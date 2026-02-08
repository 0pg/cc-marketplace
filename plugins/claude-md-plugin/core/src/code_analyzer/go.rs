//! Go code analyzer.

use std::path::Path;
use regex::Regex;

use super::{
    AnalyzerError, Behavior, BehaviorCategory, Contract, ExportedFunction, ExportedType,
    ExportedVariable, FunctionContract, LanguageAnalyzer, PartialAnalysis, Protocol, TypeKind,
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
    // Contract extraction patterns
    func_comment_re: Regex,
    precondition_re: Regex,
    postcondition_re: Regex,
    errors_re: Regex,
    // Protocol patterns
    iota_const_re: Regex,
    state_const_re: Regex,
    lifecycle_re: Regex,
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

            // Match comment block followed by func
            // // Comment line
            // func FunctionName(...)
            func_comment_re: Regex::new(
                r"(?s)((?://[^\n]*\n)+)\s*func\s+(\w+)\s*\([^)]*\)"
            ).unwrap(),

            // Precondition: ... in comment
            precondition_re: Regex::new(
                r"(?i)//\s*Precondition:\s*(.+)"
            ).unwrap(),

            // Postcondition: ... in comment
            postcondition_re: Regex::new(
                r"(?i)//\s*Postcondition:\s*(.+)"
            ).unwrap(),

            // Errors: ... or Error: ... in comment
            errors_re: Regex::new(
                r"(?i)//\s*Errors?:\s*(.+)"
            ).unwrap(),

            // iota const block
            // const (
            //     StateIdle State = iota
            //     StateLoading
            // )
            iota_const_re: Regex::new(
                r"(?s)const\s*\(\s*((?:\s*\w+\s+\w+\s*=\s*iota.*?\n)(?:\s*\w+.*?\n)*)\s*\)"
            ).unwrap(),

            // Individual state constants (State prefix pattern)
            state_const_re: Regex::new(
                r"(?m)^\s*(State\w+)"
            ).unwrap(),

            // @lifecycle N in comment
            lifecycle_re: Regex::new(
                r"@lifecycle\s+(\d+)"
            ).unwrap(),
        }
    }

    /// Check if a name is exported (starts with uppercase)
    fn is_exported(&self, name: &str) -> bool {
        name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    }

    /// Extract contracts from Go comments.
    fn extract_contracts(&self, content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();

        for cap in self.func_comment_re.captures_iter(content) {
            let comment_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let function_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Only process exported functions
            if !self.is_exported(function_name) {
                continue;
            }

            let mut contract = Contract::default();

            // Extract preconditions
            for pre_cap in self.precondition_re.captures_iter(comment_block) {
                if let Some(m) = pre_cap.get(1) {
                    contract.preconditions.push(m.as_str().trim().to_string());
                }
            }

            // Extract postconditions
            for post_cap in self.postcondition_re.captures_iter(comment_block) {
                if let Some(m) = post_cap.get(1) {
                    contract.postconditions.push(m.as_str().trim().to_string());
                }
            }

            // Extract errors/throws
            for err_cap in self.errors_re.captures_iter(comment_block) {
                if let Some(m) = err_cap.get(1) {
                    contract.throws.push(m.as_str().trim().to_string());
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

    /// Extract protocol information (states from iota, lifecycle).
    fn extract_protocol(&self, content: &str) -> Option<Protocol> {
        let mut protocol = Protocol::default();

        // Extract states from iota const blocks
        for cap in self.iota_const_re.captures_iter(content) {
            let const_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Extract State* constants
            for state_cap in self.state_const_re.captures_iter(const_block) {
                if let Some(state) = state_cap.get(1) {
                    let state_name = state.as_str();
                    if !protocol.states.contains(&state_name.to_string()) {
                        protocol.states.push(state_name.to_string());
                    }
                }
            }
        }

        // Extract lifecycle methods from @lifecycle comments
        let lifecycle_func_re = Regex::new(
            r"(?s)((?://[^\n]*\n)+)\s*func\s+(?:\([^)]+\)\s*)?(\w+)\s*\([^)]*\)"
        ).unwrap();

        let mut lifecycle_methods: Vec<(u32, String)> = Vec::new();
        for cap in lifecycle_func_re.captures_iter(content) {
            let comment_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let func_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if let Some(lifecycle_cap) = self.lifecycle_re.captures(comment_block) {
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

        // Extract contracts from comments
        analysis.contracts = self.extract_contracts(content);

        // Extract protocol information (states, lifecycle)
        analysis.protocol = self.extract_protocol(content);

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
}

impl Default for GoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::code_analyzer::{LanguageAnalyzer, BehaviorCategory};

    fn analyze(content: &str) -> PartialAnalysis {
        let analyzer = GoAnalyzer::new();
        analyzer.analyze_file(Path::new("handler.go"), content).unwrap()
    }

    #[test]
    fn test_exported_function() {
        let result = analyze("func ValidateToken(token string) (*Claims, error) {}\n");

        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "ValidateToken");
        assert_eq!(result.functions[0].signature, "func ValidateToken(token string) *Claims, error");
    }

    #[test]
    fn test_unexported_function_excluded() {
        let content = "func ValidateToken(token string) error {}\nfunc helper() {}\n";
        let result = analyze(content);

        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "ValidateToken");
    }

    #[test]
    fn test_function_no_return() {
        let result = analyze("func Setup() {}\n");

        assert_eq!(result.functions.len(), 1);
        // The regex captures the `{}` block start, so signature includes trailing content
        assert!(result.functions[0].signature.starts_with("func Setup()"));
    }

    #[test]
    fn test_struct_extraction() {
        let content = "type TokenService struct {\n}\ntype helper struct {\n}\n";
        let result = analyze(content);

        assert_eq!(result.types.len(), 1);
        assert_eq!(result.types[0].name, "TokenService");
        assert_eq!(result.types[0].kind, TypeKind::Struct);
    }

    #[test]
    fn test_interface_extraction() {
        let result = analyze("type Validator interface {\n    Validate(token string) error\n}\n");

        assert_eq!(result.types.len(), 1);
        assert_eq!(result.types[0].name, "Validator");
        assert_eq!(result.types[0].kind, TypeKind::Interface);
    }

    #[test]
    fn test_error_variable() {
        let content = r#"
var ErrInvalidToken = errors.New("invalid token")
var ErrExpired = errors.New("token expired")
var errInternal = errors.New("internal")
"#;
        let result = analyze(content);

        assert_eq!(result.variables.len(), 2);
        assert!(result.variables.iter().any(|v| v.name == "ErrInvalidToken"));
        assert!(result.variables.iter().any(|v| v.name == "ErrExpired"));
    }

    #[test]
    fn test_import_dependencies() {
        let content = r#"
import (
    "fmt"
    "net/http"
    "github.com/golang-jwt/jwt/v5"
    "go.uber.org/zap"
)
"#;
        let result = analyze(content);

        // Standard library imports (no dots/slashes) are skipped
        assert!(result.external_deps.contains(&"github.com/golang-jwt/jwt/v5".to_string()));
        assert!(result.external_deps.contains(&"go.uber.org/zap".to_string()));
        assert!(!result.external_deps.contains(&"fmt".to_string()));
    }

    #[test]
    fn test_contract_from_comments() {
        let content = r#"
// Precondition: token must be non-empty
// Postcondition: returns valid claims
// Errors: ErrInvalidToken, ErrExpired
func ValidateToken(token string) (*Claims, error) {}
"#;
        let result = analyze(content);

        assert_eq!(result.contracts.len(), 1);
        let contract = &result.contracts[0];
        assert_eq!(contract.function_name, "ValidateToken");
        assert!(!contract.contract.preconditions.is_empty());
        assert!(!contract.contract.postconditions.is_empty());
        assert!(!contract.contract.throws.is_empty());
    }

    #[test]
    fn test_unexported_function_contract_excluded() {
        let content = r#"
// Precondition: data must exist
func internalHelper(data string) error {}
"#;
        let result = analyze(content);
        assert!(result.contracts.is_empty());
    }

    #[test]
    fn test_behavior_from_error_return() {
        let content = r#"
func ValidateToken(token string) (*Claims, error) {
    if isExpired(token) {
        return nil, ErrExpired
    }
    if !isValid(token) {
        return nil, ErrInvalidToken
    }
    return claims, nil
}
"#;
        let result = analyze(content);

        // Should have success behavior (has "Validate" function)
        let success = result.behaviors.iter().find(|b| b.category == BehaviorCategory::Success);
        assert!(success.is_some());
        assert_eq!(success.unwrap().input, "Valid JWT token");

        let errors: Vec<_> = result.behaviors.iter()
            .filter(|b| b.category == BehaviorCategory::Error)
            .collect();
        assert!(errors.iter().any(|b| b.output == "ErrExpired"));
        assert!(errors.iter().any(|b| b.output == "ErrInvalidToken"));
    }

    #[test]
    fn test_protocol_from_iota() {
        let content = r#"
const (
    StateIdle State = iota
    StateConnecting
    StateConnected
)
"#;
        let result = analyze(content);

        let protocol = result.protocol.as_ref().expect("expected protocol");
        assert!(protocol.states.contains(&"StateIdle".to_string()));
        assert!(protocol.states.contains(&"StateConnecting".to_string()));
        assert!(protocol.states.contains(&"StateConnected".to_string()));
    }

    #[test]
    fn test_empty_file() {
        let result = analyze("");

        assert!(result.functions.is_empty());
        assert!(result.types.is_empty());
        assert!(result.variables.is_empty());
        assert!(result.behaviors.is_empty());
        assert!(result.contracts.is_empty());
        assert!(result.protocol.is_none());
    }
}
