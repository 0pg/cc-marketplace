//! Java code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, ExportedClass, ExportedFunction,
    ExportedType, LanguageAnalyzer, PartialAnalysis, TypeKind, ExportedEnum,
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

    fn extensions(&self) -> &[&str] {
        &["java"]
    }
}

impl Default for JavaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
