//! TypeScript/JavaScript code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, Contract, ExportedClass, ExportedEnum,
    ExportedFunction, ExportedType, ExportedVariable, FunctionContract, LanguageAnalyzer,
    PartialAnalysis, Protocol, ReExport, TypeKind,
};

/// Analyzer for TypeScript and JavaScript files.
#[derive(Debug)]
pub struct TypeScriptAnalyzer {
    // Regex patterns for TypeScript analysis
    export_function_re: Regex,
    export_arrow_function_re: Regex,
    export_default_function_re: Regex,
    export_class_re: Regex,
    export_interface_re: Regex,
    export_type_re: Regex,
    re_export_re: Regex,
    re_export_default_re: Regex,
    import_re: Regex,
    throw_error_re: Regex,
    // Contract extraction patterns
    jsdoc_block_re: Regex,
    precondition_re: Regex,
    postcondition_re: Regex,
    invariant_re: Regex,
    throws_re: Regex,
    // Protocol patterns
    state_enum_re: Regex,
    // Discriminated union patterns
    discriminated_union_re: Regex,
    union_variant_re: Regex,
    // Export candidates patterns
    export_enum_re: Regex,
    export_const_var_re: Regex,
    export_let_var_re: Regex,
}

impl TypeScriptAnalyzer {
    pub fn new() -> Self {
        Self {
            // export function name(params): ReturnType
            // export async function name(params): Promise<ReturnType>
            export_function_re: Regex::new(
                r"export\s+(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)\s*(?::\s*([^\{]+))?"
            ).unwrap(),

            // export const name = (params): ReturnType => ...
            // export const name = async (params): Promise<ReturnType> => ...
            export_arrow_function_re: Regex::new(
                r"export\s+const\s+(\w+)\s*=\s*(?:async\s+)?\(([^)]*)\)\s*(?::\s*([^\s=]+))?\s*=>"
            ).unwrap(),

            // export default function name(params): ReturnType
            // export default async function name(params): Promise<ReturnType>
            export_default_function_re: Regex::new(
                r"export\s+default\s+(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)\s*(?::\s*([^\{]+))?"
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

            // export { name1, name2 } from './module'
            re_export_re: Regex::new(
                r#"export\s*\{\s*([^}]+)\s*\}\s*from\s*['"]([^'"]+)['"]"#
            ).unwrap(),

            // export { default as name } from './module'
            re_export_default_re: Regex::new(
                r#"export\s*\{\s*default\s+as\s+(\w+)\s*\}\s*from\s*['"]([^'"]+)['"]"#
            ).unwrap(),

            // import ... from 'package'
            import_re: Regex::new(
                r#"import\s+(?:.*?)\s+from\s+['"]([^'"]+)['"]"#
            ).unwrap(),

            // throw new ErrorName(...)
            throw_error_re: Regex::new(
                r"throw\s+new\s+(\w+)\s*\("
            ).unwrap(),

            // JSDoc block followed by export function
            // Matches: /** ... */ followed by export function name
            jsdoc_block_re: Regex::new(
                r"(?s)/\*\*(.*?)\*/\s*export\s+(?:async\s+)?function\s+(\w+)"
            ).unwrap(),

            // @precondition tag in JSDoc
            precondition_re: Regex::new(
                r"@precondition\s+(.+?)(?:\n|\*\/)"
            ).unwrap(),

            // @postcondition tag in JSDoc
            postcondition_re: Regex::new(
                r"@postcondition\s+(.+?)(?:\n|\*\/)"
            ).unwrap(),

            // @invariant tag in JSDoc
            invariant_re: Regex::new(
                r"@invariant\s+(.+?)(?:\n|\*\/)"
            ).unwrap(),

            // @throws tag in JSDoc
            throws_re: Regex::new(
                r"@throws?\s+(\w+)"
            ).unwrap(),

            // State enum pattern: enum State { Idle, Loading, ... }
            state_enum_re: Regex::new(
                r"(?s)export\s+enum\s+State\s*\{([^}]+)\}"
            ).unwrap(),

            // Discriminated union pattern: type State = | { kind: 'idle' } | { kind: 'loading' } ...
            // Matches the full type definition
            discriminated_union_re: Regex::new(
                r"(?s)type\s+\w+\s*=\s*((?:\s*\|?\s*\{[^}]+\}\s*)+)"
            ).unwrap(),

            // Union variant pattern: extracts discriminator values like kind: 'idle', type: 'START', status: 'loading'
            union_variant_re: Regex::new(
                r#"(?:kind|type|status)\s*:\s*['"](\w+)['"]"#
            ).unwrap(),

            // export enum EnumName { ... }
            export_enum_re: Regex::new(
                r"export\s+enum\s+(\w+)"
            ).unwrap(),

            // export const NAME = value (non-function, i.e. not followed by arrow =>)
            // Captures: name, optional type annotation
            export_const_var_re: Regex::new(
                r"export\s+const\s+(\w+)\s*(?::\s*([^=]+?))?\s*="
            ).unwrap(),

            // export let NAME: Type
            export_let_var_re: Regex::new(
                r"export\s+let\s+(\w+)\s*(?::\s*(\S+))?"
            ).unwrap(),
        }
    }

    /// Extract contracts from JSDoc comments and infer from validation patterns.
    fn extract_contracts(&self, content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();

        // Extract contracts from JSDoc blocks
        for cap in self.jsdoc_block_re.captures_iter(content) {
            let jsdoc_content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let function_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let mut contract = Contract::default();

            // Extract @precondition tags
            for pre_cap in self.precondition_re.captures_iter(jsdoc_content) {
                if let Some(m) = pre_cap.get(1) {
                    contract.preconditions.push(m.as_str().trim().to_string());
                }
            }

            // Extract @postcondition tags
            for post_cap in self.postcondition_re.captures_iter(jsdoc_content) {
                if let Some(m) = post_cap.get(1) {
                    contract.postconditions.push(m.as_str().trim().to_string());
                }
            }

            // Extract @invariant tags
            for inv_cap in self.invariant_re.captures_iter(jsdoc_content) {
                if let Some(m) = inv_cap.get(1) {
                    contract.invariants.push(m.as_str().trim().to_string());
                }
            }

            // Extract @throws tags
            for throws_cap in self.throws_re.captures_iter(jsdoc_content) {
                if let Some(m) = throws_cap.get(1) {
                    contract.throws.push(m.as_str().trim().to_string());
                }
            }

            // Only add if contract has any content
            if !contract.preconditions.is_empty()
                || !contract.postconditions.is_empty()
                || !contract.invariants.is_empty()
                || !contract.throws.is_empty()
            {
                contracts.push(FunctionContract {
                    function_name: function_name.to_string(),
                    contract,
                });
            }
        }

        // Infer contracts from validation patterns in function bodies
        self.infer_contracts_from_validation(content, &mut contracts);

        contracts
    }

    /// Infer preconditions from validation patterns like `if (!x.prop) throw`.
    fn infer_contracts_from_validation(&self, content: &str, contracts: &mut Vec<FunctionContract>) {
        // Find function definitions and extract bodies by counting braces
        let function_start_re = Regex::new(
            r"export\s+(?:async\s+)?function\s+(\w+)\s*\([^)]*\)[^{]*\{"
        ).unwrap();

        for cap in function_start_re.captures_iter(content) {
            let function_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let match_end = cap.get(0).map(|m| m.end()).unwrap_or(0);

            // Extract function body by counting braces
            let body = self.extract_function_body(&content[match_end..]);

            let mut inferred_preconditions = Vec::new();

            // Look for validation patterns: if (!order.id) throw new Error
            let validation_re = Regex::new(
                r"if\s*\(\s*!(\w+(?:\.\w+)+)\s*\)\s*\{?\s*throw"
            ).unwrap();

            for val_cap in validation_re.captures_iter(&body) {
                if let Some(prop) = val_cap.get(1) {
                    let prop_str = prop.as_str();
                    inferred_preconditions.push(format!("{} is required", prop_str));
                }
            }

            // Look for: if (x.items.length === 0) throw
            let length_re = Regex::new(
                r"if\s*\(\s*(\w+(?:\.\w+)+)\.length\s*===?\s*0\s*\)\s*\{?\s*throw"
            ).unwrap();

            for len_cap in length_re.captures_iter(&body) {
                if let Some(prop) = len_cap.get(1) {
                    let prop_str = prop.as_str();
                    inferred_preconditions.push(format!("{} not empty", prop_str));
                }
            }

            if !inferred_preconditions.is_empty() {
                // Check if we already have a contract for this function
                if let Some(existing) = contracts.iter_mut().find(|c| c.function_name == function_name) {
                    existing.contract.preconditions.extend(inferred_preconditions);
                } else {
                    contracts.push(FunctionContract {
                        function_name: function_name.to_string(),
                        contract: Contract {
                            preconditions: inferred_preconditions,
                            ..Default::default()
                        },
                    });
                }
            }
        }
    }

    /// Extract function body by counting braces.
    fn extract_function_body(&self, content: &str) -> String {
        let mut brace_count = 1;
        let mut end_idx = 0;

        for (i, c) in content.char_indices() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_idx = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        content[..end_idx].to_string()
    }

    /// Extract protocol information (states, transitions, lifecycle).
    fn extract_protocol(&self, content: &str) -> Option<Protocol> {
        let mut protocol = Protocol::default();

        // Extract states from State enum
        for cap in self.state_enum_re.captures_iter(content) {
            if let Some(body) = cap.get(1) {
                let body_str = body.as_str();
                // Parse enum variants: Idle = 'idle', Loading = 'loading', etc.
                let variant_re = Regex::new(r"(\w+)\s*(?:=\s*[^,}]+)?").unwrap();
                for var_cap in variant_re.captures_iter(body_str) {
                    if let Some(variant) = var_cap.get(1) {
                        let variant_name = variant.as_str().trim();
                        if !variant_name.is_empty() && !protocol.states.contains(&variant_name.to_string()) {
                            protocol.states.push(variant_name.to_string());
                        }
                    }
                }
            }
        }

        // Extract states from discriminated unions
        // Pattern: type State = | { kind: 'idle' } | { kind: 'loading' } ...
        for cap in self.discriminated_union_re.captures_iter(content) {
            if let Some(body) = cap.get(1) {
                let body_str = body.as_str();
                // Extract discriminator values (kind, type, status)
                for var_cap in self.union_variant_re.captures_iter(body_str) {
                    if let Some(value) = var_cap.get(1) {
                        let state_name = value.as_str().trim();
                        if !state_name.is_empty() && !protocol.states.contains(&state_name.to_string()) {
                            protocol.states.push(state_name.to_string());
                        }
                    }
                }
            }
        }

        // Extract lifecycle methods from @lifecycle JSDoc tags
        // Match JSDoc comment block containing @lifecycle N, followed by method name
        let lifecycle_jsdoc_re = Regex::new(
            r"(?s)/\*\*.*?@lifecycle\s+(\d+).*?\*/\s*(\w+)\s*\("
        ).unwrap();

        let mut lifecycle_methods: Vec<(u32, String)> = Vec::new();
        for cap in lifecycle_jsdoc_re.captures_iter(content) {
            if let (Some(order), Some(name)) = (cap.get(1), cap.get(2)) {
                if let Ok(order_num) = order.as_str().parse::<u32>() {
                    lifecycle_methods.push((order_num, name.as_str().to_string()));
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

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn analyze_file(&self, _path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError> {
        let mut analysis = PartialAnalysis::default();

        // Extract exported functions (regular function syntax)
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

        // Extract exported arrow functions (export const name = (...) => ...)
        for cap in self.export_arrow_function_re.captures_iter(content) {
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

        // Extract export default functions
        for cap in self.export_default_function_re.captures_iter(content) {
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

        // Extract re-exports: export { default as name } from './module'
        // Process this first to avoid double-matching with regular re-exports
        for cap in self.re_export_default_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let source = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            analysis.re_exports.push(ReExport {
                name: name.to_string(),
                source: source.to_string(),
            });
        }

        // Extract re-exports: export { name1, name2 } from './module'
        for cap in self.re_export_re.captures_iter(content) {
            let names_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let source = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Parse individual names, handling "default as X" pattern
            for name_part in names_str.split(',') {
                let trimmed = name_part.trim();

                // Skip "default as X" as it's handled above
                if trimmed.starts_with("default as") || trimmed.starts_with("default ") {
                    continue;
                }

                // Handle "X as Y" pattern - use Y as the name
                let name = if let Some(idx) = trimmed.find(" as ") {
                    trimmed[idx + 4..].trim()
                } else {
                    trimmed
                };

                if !name.is_empty() {
                    // Avoid duplicates
                    if !analysis.re_exports.iter().any(|r| r.name == name && r.source == source) {
                        analysis.re_exports.push(ReExport {
                            name: name.to_string(),
                            source: source.to_string(),
                        });
                    }
                }
            }
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

        // Extract exported enums
        for cap in self.export_enum_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            analysis.enums.push(ExportedEnum {
                name: name.to_string(),
                variants: None,
            });
        }

        // Collect arrow function names to exclude from const variable extraction
        let arrow_fn_names: Vec<String> = analysis.functions.iter()
            .map(|f| f.name.clone())
            .collect();

        // Extract exported const variables (non-function)
        for cap in self.export_const_var_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let var_type = cap.get(2).map(|m| m.as_str().trim().to_string());

            // Skip if already captured as an arrow function
            if arrow_fn_names.contains(&name.to_string()) {
                continue;
            }

            analysis.variables.push(ExportedVariable {
                name: name.to_string(),
                var_type,
            });
        }

        // Extract exported let variables
        for cap in self.export_let_var_re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let var_type = cap.get(2).map(|m| m.as_str().trim().to_string());

            analysis.variables.push(ExportedVariable {
                name: name.to_string(),
                var_type,
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

        // Extract contracts from JSDoc and infer from validation patterns
        analysis.contracts = self.extract_contracts(content);

        // Extract protocol information (states, transitions, lifecycle)
        analysis.protocol = self.extract_protocol(content);

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
}

impl Default for TypeScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
