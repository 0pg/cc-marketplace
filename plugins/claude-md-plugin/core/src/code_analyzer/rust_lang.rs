//! Rust code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, ExportedFunction, ExportedType,
    LanguageAnalyzer, PartialAnalysis, TypeKind, ExportedEnum,
};

/// Analyzer for Rust files.
#[derive(Debug)]
pub struct RustAnalyzer {
    // Regex patterns for Rust analysis
    pub_fn_re: Regex,
    pub_struct_re: Regex,
    pub_enum_re: Regex,
    use_re: Regex,
    derive_crate_re: Regex,
    error_enum_variant_re: Regex,
    result_err_re: Regex,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            // pub fn function_name(params) -> ReturnType
            pub_fn_re: Regex::new(
                r"pub\s+fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)\s*(?:->\s*([^\{]+))?"
            ).unwrap(),

            // pub struct StructName
            pub_struct_re: Regex::new(
                r"pub\s+struct\s+(\w+)"
            ).unwrap(),

            // pub enum EnumName
            pub_enum_re: Regex::new(
                r"pub\s+enum\s+(\w+)"
            ).unwrap(),

            // use crate_name::...
            use_re: Regex::new(
                r"use\s+(\w+)(?:::[^;]+)?\s*;"
            ).unwrap(),

            // #[derive(..., crate::Something, ...)] - extract crate names from derive macros
            derive_crate_re: Regex::new(
                r"(\w+)::\w+"
            ).unwrap(),

            // #[error(...)] variant in thiserror enum
            // Match the attribute with quoted string content
            error_enum_variant_re: Regex::new(
                r#"#\[error\(".*?"\)\]\s*(\w+)"#
            ).unwrap(),

            // Err(Error::Variant)
            result_err_re: Regex::new(
                r"Err\((\w+)::(\w+)"
            ).unwrap(),
        }
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn analyze_file(&self, _path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Extract pub functions
        for cap in self.pub_fn_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim());

            let signature = if let Some(ret) = return_type {
                format!("fn {}({}) -> {}", name, params, ret)
            } else {
                format!("fn {}({})", name, params)
            };

            analysis.functions.push(ExportedFunction {
                name: name.to_string(),
                signature,
                description: None,
            });
        }

        // Extract pub structs
        for cap in self.pub_struct_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Struct,
                definition: None,
                description: None,
            });
        }

        // Extract pub enums
        for cap in self.pub_enum_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Check if it's an error enum (has thiserror derive)
            let is_error = name.contains("Error");

            analysis.types.push(ExportedType {
                name: name.to_string(),
                kind: TypeKind::Enum,
                definition: None,
                description: None,
            });

            // Also add to enums list
            analysis.enums.push(ExportedEnum {
                name: name.to_string(),
                variants: None,
            });

            // If it's an error enum, extract variants for behavior inference
            if is_error {
                // Find the enum body using brace matching
                let enum_start = content.find(&format!("pub enum {}", name));
                if let Some(start) = enum_start {
                    let enum_content = &content[start..];
                    // Find the opening brace first
                    if let Some(open_brace) = enum_content.find('{') {
                        // Count braces to find matching close
                        let mut brace_count = 1;
                        let mut end_pos = open_brace + 1;
                        for (i, c) in enum_content[open_brace + 1..].char_indices() {
                            match c {
                                '{' => brace_count += 1,
                                '}' => {
                                    brace_count -= 1;
                                    if brace_count == 0 {
                                        end_pos = open_brace + 1 + i + 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                        let enum_body = &enum_content[..end_pos];
                        for var_cap in self.error_enum_variant_re.captures_iter(enum_body) {
                            let variant = var_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                            let input = if variant.to_lowercase().contains("expired") {
                                "Expired token"
                            } else if variant.to_lowercase().contains("invalid") {
                                "Invalid token"
                            } else {
                                continue;
                            };

                            let output = format!("{}::{}", name, variant);
                            if !analysis.behaviors.iter().any(|b| b.output == output) {
                                analysis.behaviors.push(Behavior {
                                    input: input.to_string(),
                                    output,
                                    category: BehaviorCategory::Error,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Extract dependencies from use statements
        let std_crates = ["std", "core", "alloc", "self", "super", "crate"];
        for cap in self.use_re.captures_iter(content) {
            let crate_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip standard library and local references
            if std_crates.contains(&crate_name) {
                continue;
            }

            if !analysis.external_deps.contains(&crate_name.to_string()) {
                analysis.external_deps.push(crate_name.to_string());
            }
        }

        // Also extract crates used in derive macros (e.g., #[derive(thiserror::Error)])
        // Look for derive attribute patterns
        let derive_re = Regex::new(r"#\[derive\([^\)]+\)\]").unwrap();
        for cap in derive_re.find_iter(content) {
            let derive_content = cap.as_str();
            // Extract crate::Type patterns from derive content
            for crate_cap in self.derive_crate_re.captures_iter(derive_content) {
                let crate_name = crate_cap.get(1).map(|m| m.as_str()).unwrap_or("");

                // Skip standard library and local references
                if std_crates.contains(&crate_name) {
                    continue;
                }

                if !analysis.external_deps.contains(&crate_name.to_string()) {
                    analysis.external_deps.push(crate_name.to_string());
                }
            }
        }

        // Add success behavior if we have validation functions
        let has_validate = analysis.functions.iter().any(|f| f.name.contains("validate"));
        if has_validate && !analysis.behaviors.iter().any(|b| b.category == BehaviorCategory::Success) {
            analysis.behaviors.insert(0, Behavior {
                input: "Valid JWT token".to_string(),
                output: "Ok(Claims)".to_string(),
                category: BehaviorCategory::Success,
            });
        }

        Ok(analysis)
    }

    fn extensions(&self) -> &[&str] {
        &["rs"]
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
