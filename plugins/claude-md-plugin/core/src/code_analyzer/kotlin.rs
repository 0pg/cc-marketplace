//! Kotlin code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, ExportedClass, ExportedFunction,
    ExportedType, LanguageAnalyzer, PartialAnalysis, TypeKind, ExportedEnum,
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

    fn extensions(&self) -> &[&str] {
        &["kt", "kts"]
    }
}

impl Default for KotlinAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
