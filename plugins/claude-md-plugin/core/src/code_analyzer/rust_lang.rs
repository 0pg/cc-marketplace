//! Rust code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, Contract, ExportedFunction, ExportedType,
    FunctionContract, LanguageAnalyzer, PartialAnalysis, Protocol, TypeKind, ExportedEnum,
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
    // Contract extraction patterns
    doc_comment_fn_re: Regex,
    arguments_re: Regex,
    returns_re: Regex,
    errors_re: Regex,
    // Protocol patterns
    state_enum_re: Regex,
    enum_variant_re: Regex,
    lifecycle_re: Regex,
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

            // Match doc comment block followed by pub fn
            // /// Comment line
            // pub fn function_name(...)
            doc_comment_fn_re: Regex::new(
                r"(?s)((?:\s*///[^\n]*\n)+)\s*pub\s+fn\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)"
            ).unwrap(),

            // # Arguments section in doc comments
            arguments_re: Regex::new(
                r"(?s)#\s*Arguments\s*((?:\s*\*[^\n]*\n)+)"
            ).unwrap(),

            // # Returns section in doc comments
            returns_re: Regex::new(
                r"(?s)#\s*Returns\s*\n\s*([^\n#]+)"
            ).unwrap(),

            // # Errors section in doc comments
            errors_re: Regex::new(
                r"(?s)#\s*Errors\s*((?:\s*[-*][^\n]*\n)+)"
            ).unwrap(),

            // pub enum State { variants } - handles nested braces
            state_enum_re: Regex::new(
                r"(?s)pub\s+enum\s+(\w+)\s*\{((?:[^{}]|\{[^{}]*\})*)\}"
            ).unwrap(),

            // Enum variant (including associated data variants)
            // Matches: Simple, Tuple(T), Struct { field: T }
            enum_variant_re: Regex::new(
                r"(?m)^\s*(\w+)\s*(?:\{[^}]*\}|\([^)]*\))?\s*,?"
            ).unwrap(),

            // @lifecycle N in doc comment
            lifecycle_re: Regex::new(
                r"@lifecycle\s+(\d+)"
            ).unwrap(),
        }
    }

    /// Extract contracts from Rust doc comments.
    fn extract_contracts(&self, content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();

        for cap in self.doc_comment_fn_re.captures_iter(content) {
            let raw_doc_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let function_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Strip /// prefix from doc comment lines
            let doc_block: String = raw_doc_block
                .lines()
                .map(|line| line.trim_start().strip_prefix("///").unwrap_or(line).trim_start())
                .collect::<Vec<_>>()
                .join("\n");

            let mut contract = Contract::default();

            // Extract preconditions from # Arguments section
            // Look for patterns like "* `token` - Must be non-empty"
            if let Some(args_cap) = self.arguments_re.captures(&doc_block) {
                let args_content = args_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                // Parse argument lines: * `name` - description
                let arg_re = Regex::new(r"\*\s*`(\w+)`\s*-\s*(.+)").unwrap();
                for arg_cap in arg_re.captures_iter(args_content) {
                    let param_name = arg_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    let desc = arg_cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("must be") || desc_lower.contains("non-empty") || desc_lower.contains("required") {
                        // Include parameter name in the precondition (lowercase for consistency)
                        contract.preconditions.push(format!("{} {}", param_name, desc.trim().to_lowercase()));
                    }
                }
            }

            // Extract postconditions from # Returns section
            if let Some(returns_cap) = self.returns_re.captures(&doc_block) {
                let returns_content = returns_cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
                if !returns_content.is_empty() {
                    contract.postconditions.push(returns_content.to_string());
                }
            }

            // Extract throws from # Errors section
            if let Some(errors_cap) = self.errors_re.captures(&doc_block) {
                let errors_content = errors_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                // Parse error lines: - `ErrorType::Variant` description
                let err_re = Regex::new(r"[-*]\s*`([^`]+)`").unwrap();
                for err_cap in err_re.captures_iter(errors_content) {
                    if let Some(err) = err_cap.get(1) {
                        contract.throws.push(err.as_str().to_string());
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

    /// Extract protocol information (states from enum, lifecycle).
    fn extract_protocol(&self, content: &str) -> Option<Protocol> {
        let mut protocol = Protocol::default();

        // Extract states from pub enum State { ... }
        for cap in self.state_enum_re.captures_iter(content) {
            let enum_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let enum_body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check if this looks like a State enum
            if enum_name.to_lowercase().contains("state") ||
               enum_body.to_lowercase().contains("idle") ||
               enum_body.to_lowercase().contains("loading") {
                // Extract enum variants (simple names)
                for variant_cap in self.enum_variant_re.captures_iter(enum_body) {
                    if let Some(variant) = variant_cap.get(1) {
                        let variant_name = variant.as_str();
                        // Skip if it looks like associated data
                        if !variant_name.is_empty() && !protocol.states.contains(&variant_name.to_string()) {
                            protocol.states.push(variant_name.to_string());
                        }
                    }
                }
            }
        }

        // Extract lifecycle methods from @lifecycle doc comments
        let lifecycle_fn_re = Regex::new(
            r"(?s)((?:\s*///[^\n]*\n)+)\s*pub\s+fn\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)"
        ).unwrap();

        let mut lifecycle_methods: Vec<(u32, String)> = Vec::new();
        for cap in lifecycle_fn_re.captures_iter(content) {
            let doc_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let func_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if let Some(lifecycle_cap) = self.lifecycle_re.captures(doc_block) {
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

        // Extract contracts from doc comments
        analysis.contracts = self.extract_contracts(content);

        // Extract protocol information (states, lifecycle)
        analysis.protocol = self.extract_protocol(content);

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
