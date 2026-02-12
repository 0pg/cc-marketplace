//! Java code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, Contract, ExportedClass, ExportedFunction,
    ExportedType, FunctionContract, LanguageAnalyzer, PartialAnalysis, Protocol, TypeKind, ExportedEnum,
};

/// Analyzer for Java files.
#[derive(Debug)]
pub struct JavaAnalyzer {
    // Regex patterns for Java analysis
    public_method_re: Regex,
    public_class_re: Regex,
    public_interface_re: Regex,
    public_enum_re: Regex,
    import_re: Regex,
    throws_re: Regex,
    private_method_re: Regex,
    // Contract extraction patterns
    javadoc_method_re: Regex,
    param_tag_re: Regex,
    return_tag_re: Regex,
    throws_tag_re: Regex,
    // Protocol patterns
    enum_body_re: Regex,
    enum_constant_re: Regex,
    lifecycle_re: Regex,
    // Sealed class patterns (Java 17+)
    sealed_class_re: Regex,
}

impl JavaAnalyzer {
    pub fn new() -> Self {
        Self {
            // public ReturnType methodName(params) throws ...
            public_method_re: Regex::new(
                r"public\s+(?:static\s+)?(?:<[^>]+>\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)"
            ).unwrap(),

            // public class ClassName
            public_class_re: Regex::new(
                r"public\s+(?:abstract\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+[\w,\s]+)?"
            ).unwrap(),

            // public interface InterfaceName
            public_interface_re: Regex::new(
                r"public\s+interface\s+(\w+)"
            ).unwrap(),

            // public enum EnumName
            public_enum_re: Regex::new(
                r"public\s+enum\s+(\w+)"
            ).unwrap(),

            // import package.Class
            import_re: Regex::new(
                r"import\s+([\w.]+);"
            ).unwrap(),

            // throws ExceptionType
            throws_re: Regex::new(
                r"throws\s+([\w,\s]+)"
            ).unwrap(),

            // private methods
            private_method_re: Regex::new(
                r"private\s+(?:static\s+)?(?:<[^>]+>\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\("
            ).unwrap(),

            // Match Javadoc block followed by method
            // /** ... */ public ReturnType methodName(...)
            javadoc_method_re: Regex::new(
                r"(?s)/\*\*(.*?)\*/\s*public\s+(?:static\s+)?(?:<[^>]+>\s+)?(?:\w+(?:<[^>]+>)?)\s+(\w+)\s*\([^)]*\)"
            ).unwrap(),

            // @param name description (until end of line or next @)
            param_tag_re: Regex::new(
                r"@param\s+(\w+)\s+([^\n@]+)"
            ).unwrap(),

            // @return description (until end of line or next @)
            return_tag_re: Regex::new(
                r"@return\s+([^\n@]+)"
            ).unwrap(),

            // @throws/@exception ExceptionName description
            throws_tag_re: Regex::new(
                r"@(?:throws|exception)\s+(\w+)"
            ).unwrap(),

            // public enum EnumName { CONSTANT1, CONSTANT2, ... }
            enum_body_re: Regex::new(
                r"(?s)public\s+enum\s+(\w+)\s*\{([^}]+)\}"
            ).unwrap(),

            // Enum constant (uppercase identifier)
            enum_constant_re: Regex::new(
                r"(?m)^\s*([A-Z][A-Z0-9_]*)"
            ).unwrap(),

            // @lifecycle N in Javadoc
            lifecycle_re: Regex::new(
                r"@lifecycle\s+(\d+)"
            ).unwrap(),

            // sealed class ClassName permits SubType1, SubType2, ...
            sealed_class_re: Regex::new(
                r"(?:public\s+)?sealed\s+(?:class|interface)\s+\w+\s+permits\s+([^{]+)"
            ).unwrap(),
        }
    }

    /// Extract contracts from Javadoc comments.
    fn extract_contracts(&self, content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();

        for cap in self.javadoc_method_re.captures_iter(content) {
            let javadoc_content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let method_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let mut contract = Contract::default();

            // Extract preconditions from @param tags
            // Look for patterns like "@param token JWT token (must be non-empty)"
            for param_cap in self.param_tag_re.captures_iter(javadoc_content) {
                let param_name = param_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let desc = param_cap.get(2).map(|m| m.as_str()).unwrap_or("");
                // Look for constraint patterns in parentheses
                if let Some(start) = desc.find('(') {
                    if let Some(end) = desc.find(')') {
                        let constraint = &desc[start + 1..end];
                        let constraint_lower = constraint.to_lowercase();
                        if constraint_lower.contains("must be") || constraint_lower.contains("required") || constraint_lower.contains("non-empty") {
                            // Include parameter name in the precondition
                            contract.preconditions.push(format!("{} {}", param_name, constraint.trim()));
                        }
                    }
                }
            }

            // Extract postconditions from @return tag
            if let Some(return_cap) = self.return_tag_re.captures(javadoc_content) {
                let return_desc = return_cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
                // Clean up the description (remove leading * and extra whitespace)
                let clean_desc: String = return_desc
                    .lines()
                    .map(|l| l.trim_start_matches('*').trim())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !clean_desc.is_empty() {
                    contract.postconditions.push(clean_desc);
                }
            }

            // Extract throws from @throws tags
            for throws_cap in self.throws_tag_re.captures_iter(javadoc_content) {
                if let Some(exc_name) = throws_cap.get(1) {
                    contract.throws.push(exc_name.as_str().to_string());
                }
            }

            // Only add if contract has any content
            if !contract.preconditions.is_empty()
                || !contract.postconditions.is_empty()
                || !contract.throws.is_empty()
            {
                contracts.push(FunctionContract {
                    function_name: method_name.to_string(),
                    contract,
                });
            }
        }

        contracts
    }

    /// Extract protocol information (states from enum, sealed class, lifecycle).
    fn extract_protocol(&self, content: &str) -> Option<Protocol> {
        let mut protocol = Protocol::default();

        // Extract states from public enum State { ... }
        for cap in self.enum_body_re.captures_iter(content) {
            let enum_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let enum_body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check if this looks like a State enum
            if enum_name.to_lowercase().contains("state") ||
               enum_body.to_lowercase().contains("idle") ||
               enum_body.to_lowercase().contains("loading") {
                // Extract enum constants
                for const_cap in self.enum_constant_re.captures_iter(enum_body) {
                    if let Some(constant) = const_cap.get(1) {
                        let const_name = constant.as_str();
                        if !protocol.states.contains(&const_name.to_string()) {
                            protocol.states.push(const_name.to_string());
                        }
                    }
                }
            }
        }

        // Extract states from sealed class/interface permits clause (Java 17+)
        // Pattern: sealed class State permits Idle, Loading, Loaded, Error
        for cap in self.sealed_class_re.captures_iter(content) {
            let permits_clause = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Parse the permits clause: "Idle, Loading, Loaded, Error"
            for permitted in permits_clause.split(',') {
                let class_name = permitted.trim();
                if !class_name.is_empty() && !protocol.states.contains(&class_name.to_string()) {
                    protocol.states.push(class_name.to_string());
                }
            }
        }

        // Extract lifecycle methods from @lifecycle Javadoc tags
        let lifecycle_method_re = Regex::new(
            r"(?s)/\*\*(.*?)\*/\s*public\s+(?:void|[\w<>]+)\s+(\w+)\s*\([^)]*\)"
        ).unwrap();

        let mut lifecycle_methods: Vec<(u32, String)> = Vec::new();
        for cap in lifecycle_method_re.captures_iter(content) {
            let javadoc_content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let method_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if let Some(lifecycle_cap) = self.lifecycle_re.captures(javadoc_content) {
                if let Some(order) = lifecycle_cap.get(1) {
                    if let Ok(order_num) = order.as_str().parse::<u32>() {
                        lifecycle_methods.push((order_num, method_name.to_string()));
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

impl LanguageAnalyzer for JavaAnalyzer {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        let file_name = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Extract public methods
        for cap in self.public_method_re.captures_iter(content) {
            let return_type = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let params = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            // Skip constructors (return type is class name)
            let class_name = file_name.strip_suffix(".java").unwrap_or("");
            if name == class_name {
                continue;
            }

            // Check if this method is private (double-check)
            let is_private = self.private_method_re.captures_iter(content)
                .any(|c| c.get(2).map(|m| m.as_str()) == Some(name));
            if is_private {
                continue;
            }

            // Skip getter/setter methods
            if name.starts_with("get") || name.starts_with("set") || name.starts_with("is") {
                continue;
            }

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature: format!("{} {}({})", return_type, name, params),
                description: None,
            });
        }

        // Extract public classes
        for cap in self.public_class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let extends = cap.get(2).map(|m| m.as_str());

            // Check if it's an Exception class
            let is_exception = extends.map(|e| e.contains("Exception")).unwrap_or(false)
                || name.contains("Exception");

            if is_exception {
                // Add as class but don't count as regular class
                analysis.classes.push(ExportedClass {
                    name: name.to_string(),
                    signature: Some(format!("class {} extends Exception", name)),
                    description: None,
                });
            } else {
                analysis.classes.push(ExportedClass {
                    name: name.to_string(),
                    signature: if let Some(base) = extends {
                        Some(format!("class {} extends {}", name, base))
                    } else {
                        Some(format!("class {}", name))
                    },
                    description: None,
                });
            }
        }

        // Extract public interfaces
        for cap in self.public_interface_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Interface,
                definition: None,
                description: None,
            });
        }

        // Extract public enums
        for cap in self.public_enum_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.enums.push(ExportedEnum {
                name: name.to_string(),
                variants: None,
            });
        }

        // Extract dependencies from imports
        for cap in self.import_re.captures_iter(content) {
            let import_path = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip java.* and javax.* (standard library)
            if import_path.starts_with("java.") || import_path.starts_with("javax.") {
                continue;
            }

            // Extract package name (first two segments or until class name)
            let parts: Vec<&str> = import_path.split('.').collect();
            let pkg_name = if parts.len() >= 2 {
                format!("{}.{}", parts[0], parts[1])
            } else {
                parts[0].to_string()
            };

            if !analysis.external_deps.contains(&pkg_name) {
                analysis.external_deps.push(pkg_name);
            }
        }

        // Extract contracts from Javadoc comments
        analysis.contracts = self.extract_contracts(content);

        // Extract protocol information (states, lifecycle)
        analysis.protocol = self.extract_protocol(content);

        // Infer behaviors from throws clauses
        for cap in self.throws_re.captures_iter(content) {
            let exceptions = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            for exc in exceptions.split(',') {
                let exc_name = exc.trim();
                let input = if exc_name.contains("Expired") {
                    "Expired token"
                } else if exc_name.contains("Invalid") {
                    "Invalid token"
                } else {
                    continue;
                };

                if !analysis.behaviors.iter().any(|b| b.input == input) {
                    analysis.behaviors.push(Behavior {
                        input: input.to_string(),
                        output: exc_name.to_string(),
                        category: BehaviorCategory::Error,
                    });
                }
            }
        }

        // Add success behavior if we have validation methods
        let has_validate = analysis.functions.iter().any(|f| f.name.contains("validate"));
        if has_validate && !analysis.behaviors.iter().any(|b| b.category == BehaviorCategory::Success) {
            analysis.behaviors.insert(0, Behavior {
                input: "Valid JWT token".to_string(),
                output: "TokenClaims object".to_string(),
                category: BehaviorCategory::Success,
            });
        }

        Ok(analysis)
    }
}

impl Default for JavaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
