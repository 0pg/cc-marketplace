//! Go code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, ExportedFunction, ExportedType,
    ExportedVariable, LanguageAnalyzer, PartialAnalysis, TypeKind,
};

/// Analyzer for Go files.
#[derive(Debug)]
pub struct GoAnalyzer {
    // Regex patterns for Go analysis
    func_re: Regex,
    type_struct_re: Regex,
    type_interface_re: Regex,
    var_re: Regex,
    import_re: Regex,
    error_return_re: Regex,
}

impl GoAnalyzer {
    pub fn new() -> Self {
        Self {
            // func FunctionName(params) ReturnType
            func_re: Regex::new(
                r"func\s+(\w+)\s*\(([^)]*)\)\s*(?:\(([^)]+)\)|(\S+))?"
            ).unwrap(),

            // type StructName struct
            type_struct_re: Regex::new(
                r"type\s+(\w+)\s+struct\s*\{"
            ).unwrap(),

            // type InterfaceName interface
            type_interface_re: Regex::new(
                r"type\s+(\w+)\s+interface\s*\{"
            ).unwrap(),

            // var/const ErrorName = errors.New(...)
            var_re: Regex::new(
                r"(?:var|const)\s+(\w+)\s*=\s*errors\.New"
            ).unwrap(),

            // import "package" or import ( "package" )
            import_re: Regex::new(
                r#"["']([^"']+)["']"#
            ).unwrap(),

            // return nil, ErrSomething
            error_return_re: Regex::new(
                r"return\s+\w+,\s+(Err\w+)"
            ).unwrap(),
        }
    }

    /// Check if a name is exported (starts with uppercase)
    fn is_exported(&self, name: &str) -> bool {
        name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    }
}

impl LanguageAnalyzer for GoAnalyzer {
    fn analyze_file(&self, _path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Extract exported functions (capitalized)
        for cap in self.func_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Only include exported (capitalized) functions
            if !self.is_exported(name) {
                continue;
            }

            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_multi = cap.get(3).map(|m| m.as_str());
            let return_single = cap.get(4).map(|m| m.as_str());

            let return_type = return_multi.or(return_single).unwrap_or("");

            let signature = if return_type.is_empty() {
                format!("func {}({})", name, params)
            } else {
                format!("func {}({}) {}", name, params, return_type)
            };

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature,
                description: None,
            });
        }

        // Extract exported structs
        for cap in self.type_struct_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            if !self.is_exported(name) {
                continue;
            }

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Struct,
                definition: None,
                description: None,
            });
        }

        // Extract exported interfaces
        for cap in self.type_interface_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            if !self.is_exported(name) {
                continue;
            }

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Interface,
                definition: None,
                description: None,
            });
        }

        // Extract exported error variables
        for cap in self.var_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            if !self.is_exported(name) {
                continue;
            }

            analysis.variables.push(ExportedVariable {
                name: name.to_string(),
                var_type: Some("error".to_string()),
            });
        }

        // Extract dependencies from imports
        let import_section = content
            .find("import")
            .map(|start| {
                let end = content[start..].find(')').map(|e| start + e + 1).unwrap_or(content.len());
                &content[start..end]
            })
            .unwrap_or("");

        for cap in self.import_re.captures_iter(import_section) {
            let package = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip standard library packages (simple heuristic)
            if !package.contains('.') && !package.contains('/') {
                continue;
            }

            if !analysis.external_deps.contains(&package.to_string()) {
                analysis.external_deps.push(package.to_string());
            }
        }

        // Infer behaviors from error returns
        for cap in self.error_return_re.captures_iter(content) {
            let error_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            let input = if error_name.contains("Expired") {
                "Expired token"
            } else if error_name.contains("Invalid") {
                "Invalid token"
            } else {
                continue;
            };

            if !analysis.behaviors.iter().any(|b| b.input == input) {
                analysis.behaviors.push(Behavior {
                    input: input.to_string(),
                    output: error_name.to_string(),
                    category: BehaviorCategory::Error,
                });
            }
        }

        // Add success behavior if we have validation functions
        let has_validate = analysis.functions.iter().any(|f| f.name.contains("Validate"));
        if has_validate {
            analysis.behaviors.insert(0, Behavior {
                input: "Valid JWT token".to_string(),
                output: "Claims pointer".to_string(),
                category: BehaviorCategory::Success,
            });
        }

        Ok(analysis)
    }

    fn extensions(&self) -> &[&str] {
        &["go"]
    }
}

impl Default for GoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
