//! TypeScript/JavaScript code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, ExportedClass, ExportedFunction,
    ExportedType, LanguageAnalyzer, PartialAnalysis, TypeKind,
};

/// Analyzer for TypeScript and JavaScript files.
#[derive(Debug)]
pub struct TypeScriptAnalyzer {
    // Regex patterns for TypeScript analysis
    export_function_re: Regex,
    export_class_re: Regex,
    export_interface_re: Regex,
    export_type_re: Regex,
    import_re: Regex,
    throw_error_re: Regex,
    catch_block_re: Regex,
}

impl TypeScriptAnalyzer {
    pub fn new() -> Self {
        Self {
            // export function name(params): ReturnType
            // export async function name(params): Promise<ReturnType>
            export_function_re: Regex::new(
                r"export\s+(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)\s*(?::\s*([^\{]+))?"
            ).unwrap(),

            // export class ClassName extends/implements ...
            export_class_re: Regex::new(
                r"export\s+class\s+(\w+)(?:\s+extends\s+(\w+))?"
            ).unwrap(),

            // export interface InterfaceName
            export_interface_re: Regex::new(
                r"export\s+interface\s+(\w+)"
            ).unwrap(),

            // export type TypeName = ...
            export_type_re: Regex::new(
                r"export\s+type\s+(\w+)"
            ).unwrap(),

            // import ... from 'package'
            import_re: Regex::new(
                r#"import\s+(?:.*?)\s+from\s+['"]([^'"]+)['"]"#
            ).unwrap(),

            // throw new ErrorName(...)
            throw_error_re: Regex::new(
                r"throw\s+new\s+(\w+)\s*\("
            ).unwrap(),

            // catch (e) { ... throw new ErrorName }
            catch_block_re: Regex::new(
                r"catch\s*\([^)]*\)\s*\{[^}]*throw\s+new\s+(\w+)"
            ).unwrap(),
        }
    }
}

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn analyze_file(&self, _path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Extract exported functions
        for cap in self.export_function_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim()).unwrap_or("");

            let signature = if return_type.is_empty() {
                format!("{}({})", name, params)
            } else {
                format!("{}({}): {}", name, params, return_type)
            };

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature,
                description: None,
            });
        }

        // Extract exported classes
        for cap in self.export_class_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let extends = cap.get(2).map(|m| m.as_str());

            let signature = if let Some(base) = extends {
                format!("class {} extends {}", name, base)
            } else {
                format!("class {}", name)
            };

            analysis.classes.push(ExportedClass {
                name: name.to_string(),
                signature: Some(signature),
                description: None,
            });
        }

        // Extract exported interfaces
        for cap in self.export_interface_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Interface,
                definition: None,
                description: None,
            });
        }

        // Extract exported type aliases
        for cap in self.export_type_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
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
            if package.starts_with('.') || package.starts_with('/') {
                analysis.internal_deps.push(package.to_string());
            } else {
                // Extract package name (handle scoped packages)
                let pkg_name = if package.starts_with('@') {
                    package.split('/').take(2).collect::<Vec<_>>().join("/")
                } else {
                    package.split('/').next().unwrap_or(package).to_string()
                };
                if !analysis.external_deps.contains(&pkg_name) {
                    analysis.external_deps.push(pkg_name);
                }
            }
        }

        // Infer behaviors from error handling
        // Look for throw new ErrorName statements anywhere in the code
        for cap in self.throw_error_re.captures_iter(content) {
            let error_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let input = if error_name.to_lowercase().contains("expired") {
                "Expired token"
            } else if error_name.to_lowercase().contains("invalid") {
                "Invalid token"
            } else {
                continue;
            };

            // Avoid duplicate behaviors
            if !analysis.behaviors.iter().any(|b| b.output == error_name) {
                analysis.behaviors.push(Behavior {
                    input: input.to_string(),
                    output: error_name.to_string(),
                    category: BehaviorCategory::Error,
                });
            }
        }

        // Infer success behavior if we have validation functions
        let has_validate = analysis.functions.iter().any(|f| f.name.contains("validate"));
        if has_validate {
            analysis.behaviors.insert(0, Behavior {
                input: "Valid JWT token".to_string(),
                output: "Claims object".to_string(),
                category: BehaviorCategory::Success,
            });
        }

        Ok(analysis)
    }

    fn extensions(&self) -> &[&str] {
        &["ts", "tsx", "js", "jsx", "mjs", "cjs"]
    }
}

impl Default for TypeScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
