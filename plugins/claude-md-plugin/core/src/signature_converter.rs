use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for signature conversion
#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Unsupported target language: {language}")]
    UnsupportedLanguage { language: String },
    #[error("Cannot parse signature: {signature}")]
    ParseError { signature: String },
    #[error("Invalid input: {details}")]
    InvalidInput { details: String },
}

/// Supported target languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetLanguage {
    TypeScript,
    Python,
    Go,
    Rust,
    Java,
    Kotlin,
}

impl std::str::FromStr for TargetLanguage {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "typescript" | "ts" => Ok(TargetLanguage::TypeScript),
            "python" | "py" => Ok(TargetLanguage::Python),
            "go" | "golang" => Ok(TargetLanguage::Go),
            "rust" | "rs" => Ok(TargetLanguage::Rust),
            "java" => Ok(TargetLanguage::Java),
            "kotlin" | "kt" => Ok(TargetLanguage::Kotlin),
            _ => Err(ConversionError::UnsupportedLanguage {
                language: s.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for TargetLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetLanguage::TypeScript => write!(f, "typescript"),
            TargetLanguage::Python => write!(f, "python"),
            TargetLanguage::Go => write!(f, "go"),
            TargetLanguage::Rust => write!(f, "rust"),
            TargetLanguage::Java => write!(f, "java"),
            TargetLanguage::Kotlin => write!(f, "kotlin"),
        }
    }
}

/// Result of signature conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub original_signature: String,
    pub converted_signature: String,
    pub target_language: TargetLanguage,
    pub function_name: String,
    pub is_async: bool,
}

/// Parsed function signature
#[derive(Debug, Clone)]
pub struct ParsedSignature {
    pub name: String,
    pub params: Vec<ParsedParam>,
    pub return_type: String,
    pub is_async: bool,
    pub throws: Option<String>,
}

/// Parsed parameter
#[derive(Debug, Clone)]
pub struct ParsedParam {
    pub name: String,
    pub param_type: String,
    pub is_optional: bool,
}

/// Signature Converter
pub struct SignatureConverter {
    // Patterns for parsing various signature formats
    ts_pattern: Regex,
    python_pattern: Regex,
    go_pattern: Regex,
    rust_pattern: Regex,
    java_pattern: Regex,
    kotlin_pattern: Regex,
}

impl SignatureConverter {
    pub fn new() -> Self {
        Self {
            // TypeScript: funcName(param: Type): ReturnType
            ts_pattern: Regex::new(r"^(?:async\s+)?(?:function\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*:\s*(.+)$")
                .unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Python: func_name(param: type) -> ReturnType
            python_pattern: Regex::new(r"^(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*->\s*(.+):?$")
                .unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Go: FuncName(param type) (ReturnType, error)
            go_pattern: Regex::new(r"^func\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(.+)$")
                .unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Rust: pub fn func_name(param: Type) -> Result<T, E>
            rust_pattern: Regex::new(r"^(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?:->\s*(.+))?$")
                .unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Java: ReturnType funcName(Type param) throws Exception
            java_pattern: Regex::new(r"^(?:public\s+)?([A-Za-z_<>\[\]]+)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)(?:\s+throws\s+(.+))?$")
                .unwrap_or_else(|_| Regex::new(r".^").unwrap()),
            // Kotlin: suspend fun funcName(param: Type): ReturnType
            kotlin_pattern: Regex::new(r"^(?:suspend\s+)?fun\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*:\s*(.+)$")
                .unwrap_or_else(|_| Regex::new(r".^").unwrap()),
        }
    }

    /// Convert a function signature to target language
    pub fn convert(&self, signature: &str, target: TargetLanguage) -> Result<ConversionResult, ConversionError> {
        let parsed = self.parse_signature(signature)?;

        let converted = match target {
            TargetLanguage::TypeScript => self.to_typescript(&parsed),
            TargetLanguage::Python => self.to_python(&parsed),
            TargetLanguage::Go => self.to_go(&parsed),
            TargetLanguage::Rust => self.to_rust(&parsed),
            TargetLanguage::Java => self.to_java(&parsed),
            TargetLanguage::Kotlin => self.to_kotlin(&parsed),
        };

        Ok(ConversionResult {
            original_signature: signature.to_string(),
            converted_signature: converted,
            target_language: target,
            function_name: parsed.name.clone(),
            is_async: parsed.is_async,
        })
    }

    /// Convert a type definition to target language
    pub fn convert_type(&self, type_def: &str, target: TargetLanguage) -> Result<String, ConversionError> {
        // Parse type: Name { field1: Type1, field2: Type2 }
        let type_pattern = Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)\s*\{([^}]*)\}$")
            .map_err(|_| ConversionError::InvalidInput {
                details: "Invalid regex".to_string(),
            })?;

        if let Some(caps) = type_pattern.captures(type_def.trim()) {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let fields_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let fields: Vec<(&str, &str)> = fields_str
                .split(',')
                .filter_map(|f| {
                    let parts: Vec<&str> = f.split(':').collect();
                    if parts.len() == 2 {
                        Some((parts[0].trim(), parts[1].trim()))
                    } else {
                        None
                    }
                })
                .collect();

            return Ok(self.format_type_definition(name, &fields, target));
        }

        Err(ConversionError::ParseError {
            signature: type_def.to_string(),
        })
    }

    /// Convert an error type to target language
    pub fn convert_error_type(&self, error_name: &str, target: TargetLanguage) -> String {
        let clean_name = error_name.trim_end_matches("Error").trim_end_matches("Exception");

        match target {
            TargetLanguage::TypeScript => format!(
                "export class {}Error extends Error {{\n  constructor(message: string = '{}') {{\n    super(message);\n    this.name = '{}Error';\n  }}\n}}",
                clean_name, self.to_error_message(clean_name), clean_name
            ),
            TargetLanguage::Python => format!(
                "class {}Error(Exception):\n    pass",
                clean_name
            ),
            TargetLanguage::Go => format!(
                "var Err{} = errors.New(\"{}\")",
                clean_name, self.to_snake_case(clean_name).replace('_', " ")
            ),
            TargetLanguage::Rust => format!(
                "#[error(\"{}\")]\n{}",
                self.to_error_message(clean_name), clean_name
            ),
            TargetLanguage::Java => format!(
                "public class {}Exception extends RuntimeException {{\n    public {}Exception(String message) {{\n        super(message);\n    }}\n}}",
                clean_name, clean_name
            ),
            TargetLanguage::Kotlin => format!(
                "class {}Exception(message: String = \"{}\") : RuntimeException(message)",
                clean_name, self.to_error_message(clean_name)
            ),
        }
    }

    /// Convert function name to target naming convention
    pub fn convert_function_name(&self, name: &str, target: TargetLanguage) -> String {
        match target {
            TargetLanguage::TypeScript | TargetLanguage::Java | TargetLanguage::Kotlin => {
                self.to_camel_case(name)
            }
            TargetLanguage::Python | TargetLanguage::Rust => self.to_snake_case(name),
            TargetLanguage::Go => self.to_pascal_case(name),
        }
    }

    /// Parse a signature string into structured form
    fn parse_signature(&self, signature: &str) -> Result<ParsedSignature, ConversionError> {
        let sig = signature.trim();

        // Detect async
        let is_async = sig.contains("Promise<")
            || sig.starts_with("async ")
            || sig.contains("async ")
            || sig.starts_with("suspend ")
            || sig.contains("suspend ")
            || sig.contains("CompletableFuture<");

        // Try parsing with different patterns
        // Generic pattern: name(params): returnType or name(params) -> returnType
        let generic_pattern = Regex::new(
            r"^(?:async\s+)?(?:function\s+)?(?:pub\s+)?(?:fn\s+)?(?:def\s+)?(?:suspend\s+)?(?:fun\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?::\s*|->|)\s*(.*)$"
        ).map_err(|_| ConversionError::ParseError {
            signature: signature.to_string(),
        })?;

        if let Some(caps) = generic_pattern.captures(sig) {
            let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let params_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

            let params = self.parse_params(params_str);

            // Check for throws clause
            let throws = if sig.contains("throws ") {
                sig.split("throws ").nth(1).map(|s| s.trim().to_string())
            } else {
                None
            };

            return Ok(ParsedSignature {
                name,
                params,
                return_type,
                is_async,
                throws,
            });
        }

        // Try Java-style: ReturnType name(params)
        let java_style = Regex::new(
            r"^(?:public\s+)?([A-Za-z_<>\[\]]+)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)(?:\s+throws\s+(.+))?$"
        ).map_err(|_| ConversionError::ParseError {
            signature: signature.to_string(),
        })?;

        if let Some(caps) = java_style.captures(sig) {
            let return_type = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let params_str = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let throws = caps.get(4).map(|m| m.as_str().to_string());

            let params = self.parse_params(params_str);
            let is_async = return_type.contains("CompletableFuture");

            return Ok(ParsedSignature {
                name,
                params,
                return_type,
                is_async,
                throws,
            });
        }

        Err(ConversionError::ParseError {
            signature: signature.to_string(),
        })
    }

    fn parse_params(&self, params_str: &str) -> Vec<ParsedParam> {
        if params_str.trim().is_empty() {
            return Vec::new();
        }

        // Use bracket-aware splitting to handle generic types like Map<K, V>
        let param_parts = self.split_params_respecting_brackets(params_str);

        param_parts
            .iter()
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    return None;
                }

                let is_optional = p.contains('?') || p.contains(" = ") || p.contains("= ");

                // Try TypeScript/Kotlin style: name: Type or name?: Type
                // But be careful with colon inside generic types
                if let Some((name, param_type)) = self.split_param_name_type(p) {
                    return Some(ParsedParam {
                        name: name.trim().trim_end_matches('?').to_string(),
                        param_type: param_type.trim().to_string(),
                        is_optional,
                    });
                }

                // Try Go/Java style: Type name or name type
                let parts: Vec<&str> = p.split_whitespace().collect();
                if parts.len() >= 2 {
                    // Reconstruct type if it contains generics
                    let (type_part, name_part) = self.split_java_go_style_param(p)?;

                    // Check if type_part looks like a type
                    let first_char = type_part.chars().next()?;
                    let first_is_type = first_char.is_uppercase()
                        || ["int", "string", "bool", "float", "double", "long", "byte", "void"]
                            .contains(&type_part.to_lowercase().as_str());

                    if first_is_type {
                        // Java style: Type name
                        return Some(ParsedParam {
                            name: name_part,
                            param_type: type_part,
                            is_optional,
                        });
                    } else {
                        // Go style: name type
                        return Some(ParsedParam {
                            name: type_part,
                            param_type: name_part,
                            is_optional,
                        });
                    }
                }

                None
            })
            .collect()
    }

    /// Split parameters by comma, respecting nested brackets <>, (), [], {}
    fn split_params_respecting_brackets(&self, s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut angle_depth: u32 = 0;   // < >
        let mut paren_depth: u32 = 0;   // ( )
        let mut bracket_depth: u32 = 0; // [ ]
        let mut brace_depth: u32 = 0;   // { }

        for c in s.chars() {
            match c {
                '<' => {
                    angle_depth += 1;
                    current.push(c);
                }
                '>' => {
                    angle_depth = angle_depth.saturating_sub(1);
                    current.push(c);
                }
                '(' => {
                    paren_depth += 1;
                    current.push(c);
                }
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    current.push(c);
                }
                '[' => {
                    bracket_depth += 1;
                    current.push(c);
                }
                ']' => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    current.push(c);
                }
                '{' => {
                    brace_depth += 1;
                    current.push(c);
                }
                '}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                    current.push(c);
                }
                ',' if angle_depth == 0
                    && paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0 =>
                {
                    result.push(current.trim().to_string());
                    current = String::new();
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }

    /// Split "name: Type" respecting brackets in Type (for TypeScript/Kotlin style)
    fn split_param_name_type(&self, s: &str) -> Option<(String, String)> {
        // Find the first colon that's not inside brackets
        let mut angle_depth: u32 = 0;
        let mut paren_depth: u32 = 0;

        for (i, c) in s.chars().enumerate() {
            match c {
                '<' => angle_depth += 1,
                '>' => angle_depth = angle_depth.saturating_sub(1),
                '(' => paren_depth += 1,
                ')' => paren_depth = paren_depth.saturating_sub(1),
                ':' if angle_depth == 0 && paren_depth == 0 => {
                    return Some((s[..i].to_string(), s[i + 1..].to_string()));
                }
                _ => {}
            }
        }
        None
    }

    /// Split Java/Go style param like "Map<K, V> name" or "name Map<K, V>"
    fn split_java_go_style_param(&self, s: &str) -> Option<(String, String)> {
        // Find the last whitespace that's not inside brackets
        let mut angle_depth: u32 = 0;
        let mut last_space_idx = None;

        for (i, c) in s.chars().enumerate() {
            match c {
                '<' => angle_depth += 1,
                '>' => angle_depth = angle_depth.saturating_sub(1),
                ' ' if angle_depth == 0 => {
                    last_space_idx = Some(i);
                }
                _ => {}
            }
        }

        last_space_idx.map(|i| (s[..i].trim().to_string(), s[i + 1..].trim().to_string()))
    }

    fn to_typescript(&self, parsed: &ParsedSignature) -> String {
        let name = self.to_camel_case(&parsed.name);
        let params = parsed
            .params
            .iter()
            .map(|p| {
                let opt = if p.is_optional { "?" } else { "" };
                format!("{}{}: {}", p.name, opt, self.convert_type_to_ts(&p.param_type))
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = self.convert_type_to_ts(&parsed.return_type);

        if parsed.is_async {
            format!("async function {}({}): Promise<{}>", name, params, return_type.trim_start_matches("Promise<").trim_end_matches('>'))
        } else {
            format!("function {}({}): {}", name, params, return_type)
        }
    }

    fn to_python(&self, parsed: &ParsedSignature) -> String {
        let name = self.to_snake_case(&parsed.name);
        let params = parsed
            .params
            .iter()
            .map(|p| {
                let pname = self.to_snake_case(&p.name);
                let ptype = self.convert_type_to_python(&p.param_type);
                if p.is_optional {
                    format!("{}: {} | None = None", pname, ptype)
                } else {
                    format!("{}: {}", pname, ptype)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        // Clean up Promise<T> -> T for Python
        let raw_return = &parsed.return_type;
        let clean_return = if raw_return.starts_with("Promise<") {
            raw_return
                .trim_start_matches("Promise<")
                .trim_end_matches('>')
                .to_string()
        } else {
            raw_return.to_string()
        };
        let return_type = self.convert_type_to_python(&clean_return);
        let prefix = if parsed.is_async { "async def" } else { "def" };

        format!("{} {}({}) -> {}:", prefix, name, params, return_type)
    }

    fn to_go(&self, parsed: &ParsedSignature) -> String {
        let name = self.to_pascal_case(&parsed.name);
        let params = parsed
            .params
            .iter()
            .map(|p| {
                if p.is_optional {
                    format!("{} ...{}", p.name, self.convert_type_to_go(&p.param_type))
                } else {
                    format!("{} {}", p.name, self.convert_type_to_go(&p.param_type))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = self.convert_type_to_go(&parsed.return_type);

        // Go typically returns (Type, error) for fallible operations
        if parsed.is_async || parsed.throws.is_some() || return_type.contains("Result") {
            let clean_type = return_type
                .replace("Promise<", "")
                .replace("Result<", "")
                .replace(", error>", "")
                .replace(">", "")
                .trim()
                .to_string();
            format!("func {}({}) ({}, error)", name, params, clean_type)
        } else if return_type.is_empty() || return_type == "void" {
            format!("func {}({})", name, params)
        } else {
            format!("func {}({}) {}", name, params, return_type)
        }
    }

    fn to_rust(&self, parsed: &ParsedSignature) -> String {
        let name = self.to_snake_case(&parsed.name);
        let params = parsed
            .params
            .iter()
            .map(|p| {
                let ptype = self.convert_type_to_rust(&p.param_type);
                if p.is_optional {
                    format!("{}: Option<{}>", p.name, ptype)
                } else {
                    format!("{}: {}", p.name, ptype)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = self.convert_type_to_rust(&parsed.return_type);
        let async_prefix = if parsed.is_async { "async " } else { "" };

        if parsed.is_async || parsed.throws.is_some() {
            let inner_type = return_type
                .trim_start_matches("Result<")
                .trim_start_matches("Promise<")
                .trim_end_matches('>');
            format!("pub {}fn {}({}) -> Result<{}, Error>", async_prefix, name, params, inner_type)
        } else if return_type.is_empty() || return_type == "void" {
            format!("pub {}fn {}({})", async_prefix, name, params)
        } else {
            format!("pub {}fn {}({}) -> {}", async_prefix, name, params, return_type)
        }
    }

    fn to_java(&self, parsed: &ParsedSignature) -> String {
        let name = self.to_camel_case(&parsed.name);
        let params = parsed
            .params
            .iter()
            .map(|p| {
                format!("{} {}", self.convert_type_to_java(&p.param_type), p.name)
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = self.convert_type_to_java(&parsed.return_type);

        let throws_clause = parsed
            .throws
            .as_ref()
            .map(|t| format!(" throws {}", t.replace("Error", "Exception")))
            .unwrap_or_default();

        if parsed.is_async {
            format!("public CompletableFuture<{}> {}({})", return_type, name, params)
        } else {
            format!("public {} {}({}){}", return_type, name, params, throws_clause)
        }
    }

    fn to_kotlin(&self, parsed: &ParsedSignature) -> String {
        let name = self.to_camel_case(&parsed.name);
        let params = parsed
            .params
            .iter()
            .map(|p| {
                let ptype = self.convert_type_to_kotlin(&p.param_type);
                if p.is_optional {
                    format!("{}: {}? = null", p.name, ptype)
                } else {
                    format!("{}: {}", p.name, ptype)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = self.convert_type_to_kotlin(&parsed.return_type);
        let prefix = if parsed.is_async { "suspend fun" } else { "fun" };

        format!("{} {}({}): {}", prefix, name, params, return_type)
    }

    fn format_type_definition(&self, name: &str, fields: &[(&str, &str)], target: TargetLanguage) -> String {
        match target {
            TargetLanguage::TypeScript => {
                let field_lines: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("  {}: {};", n, self.convert_type_to_ts(t)))
                    .collect();
                format!("interface {} {{\n{}\n}}", name, field_lines.join("\n"))
            }
            TargetLanguage::Python => {
                let field_lines: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    {}: {}", self.to_snake_case(n), self.convert_type_to_python(t)))
                    .collect();
                format!("@dataclass\nclass {}:\n{}", name, field_lines.join("\n"))
            }
            TargetLanguage::Go => {
                let field_lines: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| {
                        let go_name = self.to_pascal_case(n);
                        format!("\t{} {} `json:\"{}\"`", go_name, self.convert_type_to_go(t), n)
                    })
                    .collect();
                format!("type {} struct {{\n{}\n}}", name, field_lines.join("\n"))
            }
            TargetLanguage::Rust => {
                let field_lines: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    pub {}: {},", self.to_snake_case(n), self.convert_type_to_rust(t)))
                    .collect();
                format!(
                    "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {} {{\n{}\n}}",
                    name,
                    field_lines.join("\n")
                )
            }
            TargetLanguage::Java => {
                let field_lines: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    {} {}", self.convert_type_to_java(t), n))
                    .collect();
                format!(
                    "public record {}(\n{}\n) {{}}",
                    name,
                    field_lines.join(",\n")
                )
            }
            TargetLanguage::Kotlin => {
                let field_lines: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("    val {}: {}", n, self.convert_type_to_kotlin(t)))
                    .collect();
                format!("data class {}(\n{}\n)", name, field_lines.join(",\n"))
            }
        }
    }

    // Type conversion helpers
    fn convert_type_to_ts(&self, t: &str) -> String {
        let t = t.trim();
        match t.to_lowercase().as_str() {
            "str" | "string" => "string".to_string(),
            "int" | "int64" | "i64" | "long" | "number" => "number".to_string(),
            "bool" | "boolean" => "boolean".to_string(),
            "void" | "()" | "none" | "unit" => "void".to_string(),
            _ => t.to_string(),
        }
    }

    fn convert_type_to_python(&self, t: &str) -> String {
        let t = t.trim();
        match t.to_lowercase().as_str() {
            "string" | "str" => "str".to_string(),
            "number" | "int64" | "i64" | "long" | "int" => "int".to_string(),
            "boolean" | "bool" => "bool".to_string(),
            "void" | "()" | "unit" => "None".to_string(),
            _ => t.to_string(),
        }
    }

    fn convert_type_to_go(&self, t: &str) -> String {
        let t = t.trim();
        match t.to_lowercase().as_str() {
            "str" | "string" => "string".to_string(),
            "number" | "int" | "i64" => "int64".to_string(),
            "boolean" | "bool" => "bool".to_string(),
            "void" | "()" | "none" | "unit" => "".to_string(),
            _ => t.to_string(),
        }
    }

    fn convert_type_to_rust(&self, t: &str) -> String {
        let t = t.trim();
        match t.to_lowercase().as_str() {
            "str" => "&str".to_string(),
            "string" => "String".to_string(),
            "number" | "int" | "int64" | "long" => "i64".to_string(),
            "boolean" | "bool" => "bool".to_string(),
            "void" | "none" | "unit" => "()".to_string(),
            _ => t.to_string(),
        }
    }

    fn convert_type_to_java(&self, t: &str) -> String {
        let t = t.trim();
        match t.to_lowercase().as_str() {
            "str" | "string" => "String".to_string(),
            "number" | "int64" | "i64" => "long".to_string(),
            "int" => "int".to_string(),
            "boolean" | "bool" => "boolean".to_string(),
            "void" | "()" | "none" | "unit" => "void".to_string(),
            _ => t.to_string(),
        }
    }

    fn convert_type_to_kotlin(&self, t: &str) -> String {
        let t = t.trim();
        match t.to_lowercase().as_str() {
            "str" | "string" => "String".to_string(),
            "number" | "int64" | "i64" => "Long".to_string(),
            "int" => "Int".to_string(),
            "boolean" | "bool" => "Boolean".to_string(),
            "void" | "()" | "none" => "Unit".to_string(),
            _ => t.to_string(),
        }
    }

    // Naming convention helpers
    fn to_camel_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for (i, c) in s.chars().enumerate() {
            if c == '_' || c == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else if i == 0 {
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }

        result
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();

        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(c.to_ascii_lowercase());
            } else if c == '-' {
                result.push('_');
            } else {
                result.push(c);
            }
        }

        result
    }

    fn to_pascal_case(&self, s: &str) -> String {
        let camel = self.to_camel_case(s);
        let mut chars = camel.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    fn to_error_message(&self, name: &str) -> String {
        // Convert PascalCase to sentence: "TokenExpired" -> "Token expired"
        let snake = self.to_snake_case(name);
        let words: Vec<&str> = snake.split('_').collect();
        if words.is_empty() {
            return name.to_string();
        }
        let first = words[0].chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default()
            + &words[0][1..];
        let rest = words[1..].join(" ");
        if rest.is_empty() {
            first
        } else {
            format!("{} {}", first, rest)
        }
    }
}

impl Default for SignatureConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_typescript() {
        let converter = SignatureConverter::new();
        let result = converter
            .convert("validateToken(token: string): Promise<Claims>", TargetLanguage::TypeScript)
            .unwrap();
        assert!(result.converted_signature.contains("async function validateToken"));
        assert!(result.is_async);
    }

    #[test]
    fn test_convert_to_python() {
        let converter = SignatureConverter::new();
        let result = converter
            .convert("validateToken(token: string): Promise<Claims>", TargetLanguage::Python)
            .unwrap();
        assert!(result.converted_signature.contains("async def validate_token"));
        assert!(result.converted_signature.contains("-> Claims"));
    }

    #[test]
    fn test_convert_to_go() {
        let converter = SignatureConverter::new();
        let result = converter
            .convert("validateToken(token: string): Promise<Claims>", TargetLanguage::Go)
            .unwrap();
        assert!(result.converted_signature.contains("func ValidateToken"));
        assert!(result.converted_signature.contains("(Claims, error)"));
    }

    #[test]
    fn test_convert_to_rust() {
        let converter = SignatureConverter::new();
        let result = converter
            .convert("validateToken(token: string): Promise<Claims>", TargetLanguage::Rust)
            .unwrap();
        assert!(result.converted_signature.contains("pub async fn validate_token"));
        assert!(result.converted_signature.contains("Result<Claims, Error>"));
    }

    #[test]
    fn test_naming_conversion() {
        let converter = SignatureConverter::new();
        assert_eq!(converter.to_snake_case("validateToken"), "validate_token");
        assert_eq!(converter.to_pascal_case("validateToken"), "ValidateToken");
        assert_eq!(converter.to_camel_case("validate_token"), "validateToken");
    }

    #[test]
    fn test_type_definition_conversion() {
        let converter = SignatureConverter::new();
        let result = converter
            .convert_type("Claims { userId: string, role: Role, exp: number }", TargetLanguage::TypeScript)
            .unwrap();
        assert!(result.contains("interface Claims"));
        assert!(result.contains("userId: string"));
    }
}
