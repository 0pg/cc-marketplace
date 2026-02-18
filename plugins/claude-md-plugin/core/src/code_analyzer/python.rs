//! Python code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, Contract, ExportedClass, ExportedFunction,
    ExportedType, ExportedVariable, FunctionContract, LanguageAnalyzer, PartialAnalysis, Protocol,
    TypeKind,
};

/// Analyzer for Python files.
#[derive(Debug)]
pub struct PythonAnalyzer {
    // Regex patterns for Python analysis
    all_re: Regex,
    def_re: Regex,
    class_re: Regex,
    import_re: Regex,
    from_import_re: Regex,
    raise_re: Regex,
    // Contract extraction patterns
    docstring_func_re: Regex,
    args_section_re: Regex,
    returns_section_re: Regex,
    raises_section_re: Regex,
    // Protocol patterns
    enum_class_re: Regex,
    enum_member_re: Regex,
    lifecycle_re: Regex,
    // Union type pattern for state extraction
    union_type_re: Regex,
    // Export candidates patterns
    upper_case_const_re: Regex,
    type_alias_re: Regex,
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {
            // __all__ = ['name1', 'name2']
            all_re: Regex::new(
                r"__all__\s*=\s*\[([^\]]+)\]"
            ).unwrap(),

            // def function_name(params):
            def_re: Regex::new(
                r"(?m)^def\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^:]+))?\s*:"
            ).unwrap(),

            // class ClassName:
            class_re: Regex::new(
                r"(?m)^class\s+(\w+)(?:\([^)]*\))?\s*:"
            ).unwrap(),

            // import package
            import_re: Regex::new(
                r"(?m)^import\s+(\w+)"
            ).unwrap(),

            // from package import ...
            from_import_re: Regex::new(
                r"(?m)^from\s+([\w.]+)\s+import"
            ).unwrap(),

            // raise ExceptionName(...)
            raise_re: Regex::new(
                r"raise\s+(?:(\w+)\.)?(\w+)"
            ).unwrap(),

            // Match function definition followed by docstring
            // def func(...):
            //     """docstring"""
            docstring_func_re: Regex::new(
                r#"(?s)def\s+(\w+)\s*\([^)]*\)[^:]*:\s*(?:\n\s+)?(?:"""(.*?)"""|'''(.*?)''')"#
            ).unwrap(),

            // Args: section in docstring
            // Match "Args:" followed by indented parameter lines
            args_section_re: Regex::new(
                r"(?s)Args:\s*\n((?:\s+\w+:[^\n]*\n?)+)"
            ).unwrap(),

            // Returns: section in docstring
            returns_section_re: Regex::new(
                r"(?s)Returns:\s*\n\s+(.+?)(?:\n\n|\n\s*(?:Raises|Args|$))"
            ).unwrap(),

            // Raises: section in docstring
            raises_section_re: Regex::new(
                r"(?s)Raises:\s*\n((?:\s+\w+:.*?(?:\n|$))+)"
            ).unwrap(),

            // class State(Enum):
            enum_class_re: Regex::new(
                r#"(?s)class\s+(\w+)\s*\(\s*Enum\s*\)\s*:\s*(?:""".*?"""\s*)?((?:\s*\w+\s*=.*?\n)+)"#
            ).unwrap(),

            // ENUM_VALUE = "value" or ENUM_VALUE = 1
            enum_member_re: Regex::new(
                r"(?m)^\s*(\w+)\s*="
            ).unwrap(),

            // @lifecycle N in docstring
            lifecycle_re: Regex::new(
                r"@lifecycle\s+(\d+)"
            ).unwrap(),

            // State = Union[A, B, C] or Event = Union[X, Y, Z]
            // Captures: 1=type alias name, 2=Union contents
            union_type_re: Regex::new(
                r"(?m)^(\w+)\s*=\s*Union\[([^\]]+)\]"
            ).unwrap(),

            // Top-level UPPER_CASE constants: MAX_RETRIES = 3, DEFAULT_TIMEOUT = 30
            upper_case_const_re: Regex::new(
                r"(?m)^([A-Z][A-Z0-9_]+)\s*(?::\s*\S+\s*)?=\s*(.+)"
            ).unwrap(),

            // Type alias: TypeName = Union[...] / TypeName = TypeAlias / TypeName: TypeAlias = ...
            // Captures PascalCase names assigned to type constructs
            type_alias_re: Regex::new(
                r"(?m)^([A-Z][a-zA-Z0-9]+)\s*=\s*(Union\[|Optional\[|List\[|Dict\[|Tuple\[|Set\[|Type\[|Callable\[|Literal\[|TypeVar\(|NewType\()"
            ).unwrap(),
        }
    }

    /// Check if a function name is public (doesn't start with _)
    fn is_public(&self, name: &str) -> bool {
        !name.starts_with('_')
    }

    /// Extract contracts from Python docstrings.
    fn extract_contracts(&self, content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();

        for cap in self.docstring_func_re.captures_iter(content) {
            let function_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let docstring = cap.get(2).or(cap.get(3)).map(|m| m.as_str()).unwrap_or("");

            // Skip private functions
            if !self.is_public(function_name) {
                continue;
            }

            let mut contract = Contract::default();

            // Extract preconditions from Args section
            // Look for patterns like "token: JWT token string (must be non-empty)"
            if let Some(args_cap) = self.args_section_re.captures(docstring) {
                let args_content = args_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                // Parse each argument line
                let arg_re = Regex::new(r"(\w+):\s*(.+?)(?:\n|$)").unwrap();
                for arg_cap in arg_re.captures_iter(args_content) {
                    let param_name = arg_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    let desc = arg_cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    // Look for constraint patterns in parentheses
                    if let Some(start) = desc.find('(') {
                        if let Some(end) = desc.find(')') {
                            let constraint = &desc[start + 1..end];
                            if constraint.contains("must be") || constraint.contains("required") || constraint.contains("non-empty") {
                                // Include parameter name in the precondition
                                contract.preconditions.push(format!("{} {}", param_name, constraint));
                            }
                        }
                    }
                }
            }

            // Extract postconditions from Returns section
            if let Some(returns_cap) = self.returns_section_re.captures(docstring) {
                let returns_content = returns_cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
                if !returns_content.is_empty() {
                    contract.postconditions.push(format!("returns {}", returns_content));
                }
            }

            // Extract throws from Raises section
            if let Some(raises_cap) = self.raises_section_re.captures(docstring) {
                let raises_content = raises_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                // Parse each exception line: "ExceptionName: description"
                let raise_re = Regex::new(r"(\w+):").unwrap();
                for raise_cap in raise_re.captures_iter(raises_content) {
                    if let Some(exc_name) = raise_cap.get(1) {
                        contract.throws.push(exc_name.as_str().to_string());
                    }
                }
            }

            // Only add if contract has any content
            if !contract.preconditions.is_empty()
                || !contract.postconditions.is_empty()
                || !contract.throws.is_empty()
            {
                contracts.push(FunctionContract {
                    function_name: function_name.to_string(),
                    contract,
                });
            }
        }

        contracts
    }

    /// Extract protocol information (states from Enum, Union types, lifecycle).
    fn extract_protocol(&self, content: &str) -> Option<Protocol> {
        let mut protocol = Protocol::default();

        // Extract states from Enum classes
        for cap in self.enum_class_re.captures_iter(content) {
            let enum_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let enum_body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check if this looks like a State enum
            if enum_name.to_lowercase().contains("state") ||
               enum_body.to_lowercase().contains("idle") ||
               enum_body.to_lowercase().contains("loading") {
                // Extract enum members
                for member_cap in self.enum_member_re.captures_iter(enum_body) {
                    if let Some(member) = member_cap.get(1) {
                        let member_name = member.as_str();
                        // Skip private members and special attributes
                        if !member_name.starts_with('_') && !protocol.states.contains(&member_name.to_string()) {
                            protocol.states.push(member_name.to_string());
                        }
                    }
                }
            }
        }

        // Extract states from Union type aliases (e.g., State = Union[Idle, Loading, Loaded, Error])
        for cap in self.union_type_re.captures_iter(content) {
            let type_alias = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let union_contents = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Only process type aliases that look like state/event definitions
            let type_alias_lower = type_alias.to_lowercase();
            if type_alias_lower.contains("state") || type_alias_lower.contains("event") {
                // Extract type names from Union[A, B, C]
                for type_name in union_contents.split(',') {
                    let type_name = type_name.trim();
                    // Skip empty strings and avoid duplicates
                    if !type_name.is_empty() && !protocol.states.contains(&type_name.to_string()) {
                        protocol.states.push(type_name.to_string());
                    }
                }
            }
        }

        // Extract lifecycle methods from @lifecycle docstring tags
        // Find all functions with @lifecycle N in their docstrings
        let lifecycle_func_re = Regex::new(
            r#"(?s)def\s+(\w+)\s*\([^)]*\)[^:]*:\s*(?:\n\s+)?(?:"""(.*?)"""|'''(.*?)''')"#
        ).unwrap();

        let mut lifecycle_methods: Vec<(u32, String)> = Vec::new();
        for cap in lifecycle_func_re.captures_iter(content) {
            let func_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let docstring = cap.get(2).or(cap.get(3)).map(|m| m.as_str()).unwrap_or("");

            if let Some(lifecycle_cap) = self.lifecycle_re.captures(docstring) {
                if let Some(order) = lifecycle_cap.get(1) {
                    if let Ok(order_num) = order.as_str().parse::<u32>() {
                        lifecycle_methods.push((order_num, func_name.to_string()));
                    }
                }
            }
        }

        // Sort by order and extract names
        lifecycle_methods.sort_by_key(|(order, _)| *order);
        protocol.lifecycle = lifecycle_methods.into_iter().map(|(_, name)| name).collect();

        // Only return protocol if it has content
        if !protocol.states.is_empty() || !protocol.lifecycle.is_empty() {
            Some(protocol)
        } else {
            None
        }
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Check if this is __init__.py with __all__
        let is_init = path.file_name()
            .map(|s| s.to_string_lossy() == "__init__.py")
            .unwrap_or(false);

        // Extract __all__ symbols if present
        let all_symbols: Vec<String> = if let Some(cap) = self.all_re.captures(content) {
            let symbols_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            symbols_str
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        // Extract functions
        for cap in self.def_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip private functions
            if !self.is_public(name) {
                continue;
            }

            // If __all__ is defined, only include listed symbols
            if !all_symbols.is_empty() && !all_symbols.contains(&name.to_string()) && !is_init {
                continue;
            }

            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim());

            let signature = if let Some(ret) = return_type {
                format!("{}({}) -> {}", name, params, ret)
            } else {
                format!("{}({})", name, params)
            };

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature,
                description: None,
            });
        }

        // Extract classes
        for cap in self.class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip private classes
            if !self.is_public(name) {
                continue;
            }

            // If __all__ is defined, only include listed symbols
            if !all_symbols.is_empty() && !all_symbols.contains(&name.to_string()) && !is_init {
                continue;
            }

            analysis.classes.push(ExportedClass {
                name: name.to_string(),
                signature: Some(format!("class {}", name)),
                description: None,
            });
        }

        // Collect known class/function names to avoid duplicates
        let known_names: Vec<String> = analysis.functions.iter().map(|f| f.name.clone())
            .chain(analysis.classes.iter().map(|c| c.name.clone()))
            .collect();

        // Extract UPPER_CASE constants as variables
        for cap in self.upper_case_const_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip __all__ and other dunder constants
            if name.starts_with('_') {
                continue;
            }

            // If __all__ is defined, only include listed symbols
            if !all_symbols.is_empty() && !all_symbols.contains(&name.to_string()) && !is_init {
                continue;
            }

            analysis.variables.push(ExportedVariable {
                name: name.to_string(),
                var_type: None,
            });
        }

        // Extract type aliases (PascalCase = Union[...], Optional[...], etc.)
        for cap in self.type_alias_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip if already captured as class
            if known_names.contains(&name.to_string()) {
                continue;
            }

            // If __all__ is defined, only include listed symbols
            if !all_symbols.is_empty() && !all_symbols.contains(&name.to_string()) && !is_init {
                continue;
            }

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Type,
                definition: None,
                description: None,
            });
        }

        // Extract dependencies
        for cap in self.import_re.captures_iter(content) {
            let package = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if !analysis.external_deps.contains(&package.to_string()) {
                analysis.external_deps.push(package.to_string());
            }
        }

        for cap in self.from_import_re.captures_iter(content) {
            let package = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if package.starts_with('.') {
                if !analysis.internal_deps.contains(&package.to_string()) {
                    analysis.internal_deps.push(package.to_string());
                }
            } else {
                let pkg_name = package.split('.').next().unwrap_or(package);
                if !analysis.external_deps.contains(&pkg_name.to_string()) {
                    analysis.external_deps.push(pkg_name.to_string());
                }
            }
        }

        // Extract contracts from docstrings
        analysis.contracts = self.extract_contracts(content);

        // Extract protocol information (states, lifecycle)
        analysis.protocol = self.extract_protocol(content);

        // Infer behaviors from exception handling
        for cap in self.raise_re.captures_iter(content) {
            let module = cap.get(1).map(|m| m.as_str());
            let exc_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let error_name = if let Some(mod_name) = module {
                format!("{}.{}", mod_name, exc_name)
            } else {
                exc_name.to_string()
            };

            let input = if exc_name.to_lowercase().contains("expired") || error_name.contains("ExpiredSignature") {
                "Expired token"
            } else if exc_name.to_lowercase().contains("invalid") || error_name.contains("InvalidToken") {
                "Invalid token"
            } else {
                continue;
            };

            // Avoid duplicates
            if !analysis.behaviors.iter().any(|b| b.input == input) {
                analysis.behaviors.push(Behavior {
                    input: input.to_string(),
                    output: error_name,
                    category: BehaviorCategory::Error,
                });
            }
        }

        Ok(analysis)
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
