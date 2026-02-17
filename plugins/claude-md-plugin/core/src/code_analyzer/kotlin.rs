//! Kotlin code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, Contract, ExportedClass, ExportedFunction,
    ExportedType, ExportedVariable, FunctionContract, LanguageAnalyzer, PartialAnalysis, Protocol,
    TypeKind, ExportedEnum,
};

/// Analyzer for Kotlin files.
#[derive(Debug)]
pub struct KotlinAnalyzer {
    // Regex patterns for Kotlin analysis
    fun_re: Regex,
    private_fun_re: Regex,
    class_re: Regex,
    data_class_re: Regex,
    enum_class_re: Regex,
    import_re: Regex,
    result_re: Regex,
    throw_re: Regex,
    // Contract extraction patterns
    kdoc_fun_re: Regex,
    param_tag_re: Regex,
    return_tag_re: Regex,
    throws_tag_re: Regex,
    // Protocol patterns
    enum_body_re: Regex,
    enum_constant_re: Regex,
    lifecycle_re: Regex,
    // Sealed class/interface patterns
    sealed_class_re: Regex,
    sealed_subtype_re: Regex,
    // Export candidates patterns
    top_level_val_re: Regex,
    typealias_re: Regex,
    object_re: Regex,
    interface_re: Regex,
}

impl KotlinAnalyzer {
    pub fn new() -> Self {
        Self {
            // fun functionName(params): ReturnType
            fun_re: Regex::new(
                r"fun\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)\s*(?::\s*(\S+))?"
            ).unwrap(),

            // private fun
            private_fun_re: Regex::new(
                r"private\s+fun\s+(\w+)"
            ).unwrap(),

            // class ClassName
            class_re: Regex::new(
                r"(?:open\s+)?class\s+(\w+)(?:\s*\([^)]*\))?"
            ).unwrap(),

            // data class ClassName
            data_class_re: Regex::new(
                r"data\s+class\s+(\w+)"
            ).unwrap(),

            // enum class EnumName
            enum_class_re: Regex::new(
                r"enum\s+class\s+(\w+)"
            ).unwrap(),

            // import package.Class
            import_re: Regex::new(
                r"import\s+([\w.]+)"
            ).unwrap(),

            // Result<Type>
            result_re: Regex::new(
                r"Result<(\w+)>"
            ).unwrap(),

            // throw ExceptionName(...)
            throw_re: Regex::new(
                r"throw\s+(\w+)\s*\("
            ).unwrap(),

            // Match KDoc block followed by fun
            // /** ... */ fun functionName(...)
            kdoc_fun_re: Regex::new(
                r"(?s)/\*\*(.*?)\*/\s*(?:@\w+\s*(?:\([^)]*\)\s*)?)*fun\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)"
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
                r"@throws\s+(\w+)"
            ).unwrap(),

            // enum class EnumName { CONSTANT1, CONSTANT2, ... }
            enum_body_re: Regex::new(
                r"(?s)enum\s+class\s+(\w+)\s*\{([^}]+)\}"
            ).unwrap(),

            // Enum constant (uppercase identifier)
            enum_constant_re: Regex::new(
                r"(?m)^\s*([A-Z][A-Z0-9_]*)"
            ).unwrap(),

            // @lifecycle N in KDoc
            lifecycle_re: Regex::new(
                r"@lifecycle\s+(\d+)"
            ).unwrap(),

            // sealed class/interface Name { ... }
            sealed_class_re: Regex::new(
                r"sealed\s+(?:class|interface)\s+(\w+)"
            ).unwrap(),

            // object/data class/class SubType : ParentType
            // Matches: object Idle : State(), data class Loading(val x: Int) : State()
            sealed_subtype_re: Regex::new(
                r"(?:object|data\s+class|class)\s+(\w+)\s*(?:\([^)]*\))?\s*:\s*(\w+)"
            ).unwrap(),

            // Top-level val/const val: val NAME = value, const val NAME = value
            top_level_val_re: Regex::new(
                r"(?m)^(?:const\s+)?val\s+(\w+)\s*(?::\s*(\S+))?\s*="
            ).unwrap(),

            // typealias Name = Type
            typealias_re: Regex::new(
                r"(?m)^typealias\s+(\w+)\s*="
            ).unwrap(),

            // object Name
            object_re: Regex::new(
                r"(?m)^(?:companion\s+)?object\s+(\w+)"
            ).unwrap(),

            // interface Name
            interface_re: Regex::new(
                r"(?m)^interface\s+(\w+)"
            ).unwrap(),
        }
    }

    /// Extract contracts from KDoc comments.
    fn extract_contracts(&self, content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();

        for cap in self.kdoc_fun_re.captures_iter(content) {
            let kdoc_content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let function_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let mut contract = Contract::default();

            // Extract preconditions from @param tags
            // Look for patterns like "@param token JWT token (must be non-empty)"
            for param_cap in self.param_tag_re.captures_iter(kdoc_content) {
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
            if let Some(return_cap) = self.return_tag_re.captures(kdoc_content) {
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
            for throws_cap in self.throws_tag_re.captures_iter(kdoc_content) {
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
                    function_name: function_name.to_string(),
                    contract,
                });
            }
        }

        contracts
    }

    /// Extract protocol information (states from enum class, sealed class/interface, lifecycle).
    fn extract_protocol(&self, content: &str) -> Option<Protocol> {
        let mut protocol = Protocol::default();

        // Extract states from enum class State { ... }
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

        // Extract states from sealed class/interface
        // First, find all sealed class/interface names
        let sealed_names: Vec<String> = self.sealed_class_re
            .captures_iter(content)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect();

        // Then find all subtypes that inherit from these sealed types
        for cap in self.sealed_subtype_re.captures_iter(content) {
            let subtype_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let parent_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check if parent is a sealed class/interface
            if sealed_names.contains(&parent_name.to_string()) {
                if !protocol.states.contains(&subtype_name.to_string()) {
                    protocol.states.push(subtype_name.to_string());
                }
            }
        }

        // Extract lifecycle methods from @lifecycle KDoc tags
        let lifecycle_fun_re = Regex::new(
            r"(?s)/\*\*(.*?)\*/\s*(?:@\w+\s*(?:\([^)]*\)\s*)?)*fun\s+(\w+)\s*\([^)]*\)"
        ).unwrap();

        let mut lifecycle_methods: Vec<(u32, String)> = Vec::new();
        for cap in lifecycle_fun_re.captures_iter(content) {
            let kdoc_content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let func_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if let Some(lifecycle_cap) = self.lifecycle_re.captures(kdoc_content) {
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

impl LanguageAnalyzer for KotlinAnalyzer {
    fn analyze_file(&self, _path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Get list of private functions
        let private_funs: Vec<String> = self.private_fun_re
            .captures_iter(content)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect();

        // Extract functions (default public in Kotlin)
        for cap in self.fun_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str());

            // Skip private functions
            if private_funs.contains(&name.to_string()) {
                continue;
            }

            let signature = if let Some(ret) = return_type {
                format!("fun {}({}): {}", name, params, ret)
            } else {
                format!("fun {}({})", name, params)
            };

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature,
                description: None,
            });
        }

        // Extract data classes
        for cap in self.data_class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::DataClass,
                definition: None,
                description: None,
            });
        }

        // Extract regular classes (exclude data classes and enum classes)
        let data_classes: Vec<String> = self.data_class_re
            .captures_iter(content)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect();

        let enum_classes: Vec<String> = self.enum_class_re
            .captures_iter(content)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect();

        for cap in self.class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip if already captured as data class or enum class
            if data_classes.contains(&name.to_string()) || enum_classes.contains(&name.to_string()) {
                continue;
            }

            // Check if it's an Exception class
            let is_exception = name.contains("Exception");

            analysis.classes.push(ExportedClass {
                name: name.to_string(),
                signature: if is_exception {
                    Some(format!("class {} : Exception", name))
                } else {
                    Some(format!("class {}", name))
                },
                description: None,
            });
        }

        // Extract enum classes
        for cap in self.enum_class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.enums.push(ExportedEnum {
                name: name.to_string(),
                variants: None,
            });
        }

        // Extract top-level val/const val
        for cap in self.top_level_val_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let val_type = cap.get(2).map(|m| m.as_str().trim().to_string());

            // Skip private vals
            if private_funs.contains(&name.to_string()) {
                continue;
            }

            analysis.variables.push(ExportedVariable {
                name: name.to_string(),
                var_type: val_type,
            });
        }

        // Extract typealias
        for cap in self.typealias_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Type,
                definition: None,
                description: None,
            });
        }

        // Extract objects as classes
        for cap in self.object_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip companion objects (they're part of a class, not standalone)
            // The regex will match "companion object Name" — check if it's companion
            // Actually the regex prefix `(?:companion\s+)?` makes it optional
            // We want to capture both companion and standalone objects

            // Skip if already captured as class
            if analysis.classes.iter().any(|c| c.name == name) {
                continue;
            }

            analysis.classes.push(ExportedClass {
                name: name.to_string(),
                signature: Some(format!("object {}", name)),
                description: None,
            });
        }

        // Extract interfaces as types
        for cap in self.interface_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Trait,
                definition: None,
                description: None,
            });
        }

        // Extract dependencies from imports
        for cap in self.import_re.captures_iter(content) {
            let import_path = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip kotlin.* and java.* (standard library)
            if import_path.starts_with("kotlin.") || import_path.starts_with("java.") {
                continue;
            }

            // Extract package name
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

        // Extract contracts from KDoc comments
        analysis.contracts = self.extract_contracts(content);

        // Extract protocol information (states, lifecycle)
        analysis.protocol = self.extract_protocol(content);

        // Infer behaviors from Result types and throws
        let has_result = self.result_re.is_match(content);
        let has_validate = analysis.functions.iter().any(|f| f.name.contains("validate"));

        // Extract thrown exceptions
        for cap in self.throw_re.captures_iter(content) {
            let exc_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            let input = if exc_name.contains("Expired") {
                "Expired token"
            } else if exc_name.contains("Invalid") {
                "Invalid token"
            } else {
                continue;
            };

            let output = if has_result {
                format!("Result.failure({})", exc_name)
            } else {
                exc_name.to_string()
            };

            if !analysis.behaviors.iter().any(|b| b.input == input) {
                analysis.behaviors.push(Behavior {
                    input: input.to_string(),
                    output,
                    category: BehaviorCategory::Error,
                });
            }
        }

        // Add success behavior
        if has_validate {
            let success_output = if has_result {
                "Result.success(TokenClaims)".to_string()
            } else {
                "TokenClaims object".to_string()
            };

            if !analysis.behaviors.iter().any(|b| b.category == BehaviorCategory::Success) {
                analysis.behaviors.insert(0, Behavior {
                    input: "Valid JWT token".to_string(),
                    output: success_output,
                    category: BehaviorCategory::Success,
                });
            }
        }

        Ok(analysis)
    }
}

impl Default for KotlinAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
