use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of schema validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// File that was validated
    pub file: String,
    /// Whether validation passed
    pub valid: bool,
    /// List of errors found
    pub errors: Vec<ValidationError>,
    /// List of warnings
    pub warnings: Vec<String>,
}

/// Validation error details
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Line number (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
    /// Section where error was found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

// Include generated constants from schema-rules.yaml (SSOT)
include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));

pub struct SchemaValidator {
    /// Pattern to match section headers
    section_pattern: Regex,
    /// Pattern to match behavior scenarios
    behavior_pattern: Regex,
}

impl SchemaValidator {
    pub fn new() -> Self {
        // Match markdown headers like "## Purpose", "### Functions"
        let section_pattern = Regex::new(r"^#+\s+(.+)$").unwrap();

        // Match behavior scenarios: input → output
        let behavior_pattern = Regex::new(r"→|->").unwrap();

        Self {
            section_pattern,
            behavior_pattern,
        }
    }

    /// Validate a CLAUDE.md file
    pub fn validate(&self, file: &Path) -> ValidationResult {
        let file_str = file.to_string_lossy().to_string();

        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                return ValidationResult {
                    file: file_str,
                    valid: false,
                    errors: vec![ValidationError {
                        error_type: "FileError".to_string(),
                        message: format!("Cannot read file: {}", e),
                        line_number: None,
                        section: None,
                    }],
                    warnings: vec![],
                };
            }
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Parse sections
        let sections = self.parse_sections(&content);

        // Check required sections
        for required in REQUIRED_SECTIONS {
            let section_found = sections.iter().find(|s| s.name.eq_ignore_ascii_case(required));

            match section_found {
                None => {
                    errors.push(ValidationError {
                        error_type: "MissingSection".to_string(),
                        message: format!("Missing required section: {}", required),
                        line_number: None,
                        section: Some(required.to_string()),
                    });
                }
                Some(section) => {
                    // Check if section allows "None" and has valid content
                    let allows_none = ALLOW_NONE_SECTIONS.iter().any(|s| s.eq_ignore_ascii_case(required));
                    let is_none_marker = self.is_none_marker(section);

                    if !allows_none && is_none_marker {
                        errors.push(ValidationError {
                            error_type: "InvalidSectionContent".to_string(),
                            message: format!("Section '{}' does not allow 'None' as value", required),
                            line_number: Some(section.start_line),
                            section: Some(required.to_string()),
                        });
                    }
                }
            }
        }

        // Validate Exports section format
        if let Some(exports) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Exports")) {
            self.validate_exports(exports, &mut errors, &mut warnings);
        }

        // Validate Behavior section format
        if let Some(behavior) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Behavior")) {
            self.validate_behavior(behavior, &mut errors, &mut warnings);
        }

        ValidationResult {
            file: file_str,
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn parse_sections(&self, content: &str) -> Vec<Section> {
        let mut sections = Vec::new();
        let mut current_section: Option<Section> = None;

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = self.section_pattern.captures(line) {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                // Start new section - use map().unwrap_or() instead of unwrap()
                let section_name = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| line.trim_start_matches('#').trim().to_string());

                current_section = Some(Section {
                    name: section_name,
                    start_line: line_num + 1,
                    content: Vec::new(),
                });
            } else if let Some(ref mut section) = current_section {
                section.content.push((line_num + 1, line.to_string()));
            }
        }

        // Save last section
        if let Some(section) = current_section {
            sections.push(section);
        }

        sections
    }

    /// Check if a section contains only a "None" marker (None, N/A, etc.)
    fn is_none_marker(&self, section: &Section) -> bool {
        let non_empty_lines: Vec<&str> = section
            .content
            .iter()
            .map(|(_, line)| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        // If section has only one non-empty line and it's a none marker
        if non_empty_lines.len() == 1 {
            let line = non_empty_lines[0].to_lowercase();
            return line == "none" || line == "n/a";
        }

        false
    }

    fn validate_exports(
        &self,
        section: &Section,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<String>,
    ) {
        let mut found_valid_export = false;
        let mut in_list = false;

        for (line_num, line) in &section.content {
            let trimmed = line.trim();

            // Skip empty lines and subsection headers
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check if we're in a list
            if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('`') {
                in_list = true;
            }

            // Skip if marked as "None" or similar
            if trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("n/a") {
                found_valid_export = true;
                continue;
            }

            // Check if this looks like an export definition
            if in_list {
                if self.looks_like_export_line(trimmed) {
                    // Has parentheses - validate signature format
                    if self.has_valid_signature(trimmed) {
                        found_valid_export = true;
                    } else if self.looks_like_incomplete_signature(trimmed) {
                        warnings.push(format!(
                            "Line {}: Export may be missing parameter types or return type",
                            line_num
                        ));
                    }
                } else if self.looks_like_incomplete_signature(trimmed) {
                    // No parentheses but looks like incomplete function definition
                    warnings.push(format!(
                        "Line {}: Export may be missing parameter types or return type",
                        line_num
                    ));
                }
            }
        }

        if !found_valid_export && !section.content.is_empty() {
            errors.push(ValidationError {
                error_type: "InvalidExports".to_string(),
                message: "Exports section must contain valid function signatures or 'None'".to_string(),
                line_number: Some(section.start_line),
                section: Some("Exports".to_string()),
            });
        }
    }

    fn validate_behavior(
        &self,
        section: &Section,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<String>,
    ) {
        let mut found_valid_behavior = false;

        for (_, line) in &section.content {
            let trimmed = line.trim();

            // Skip empty lines and headers
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Skip if marked as "None"
            if trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("n/a") {
                found_valid_behavior = true;
                continue;
            }

            // Check for scenario pattern: input → output
            if self.behavior_pattern.is_match(trimmed) {
                found_valid_behavior = true;
            }
        }

        if !found_valid_behavior && !section.content.is_empty() {
            errors.push(ValidationError {
                error_type: "InvalidBehavior".to_string(),
                message: "Behavior section must contain scenarios in 'input → output' format or 'None'".to_string(),
                line_number: Some(section.start_line),
                section: Some("Behavior".to_string()),
            });
        }
    }

    fn looks_like_export_line(&self, line: &str) -> bool {
        // Has a function name pattern followed by parentheses
        line.contains('(') && line.contains(')')
    }

    fn has_valid_signature(&self, line: &str) -> bool {
        // Check for common signature patterns across languages

        // TypeScript/JavaScript: func(param: Type): ReturnType or func(param): ReturnType
        if line.contains("):") && (line.contains(": ") || line.contains("=>")) {
            return true;
        }

        // Python: func(param: type) -> ReturnType
        if line.contains(") ->") || line.contains(")->") {
            return true;
        }

        // Go: Func(param type) (ReturnType, error)
        if line.contains(") (") || (line.contains(')') && !line.trim().ends_with(')')) {
            return true;
        }

        // Rust: func(param: Type) -> Result<T, E>
        if line.contains(") -> ") {
            return true;
        }

        // Java/Kotlin: ReturnType funcName(ParamType param)
        let re = Regex::new(r"^\s*[-*]?\s*`?[A-Za-z_<>\[\]]+\s+[A-Za-z_][A-Za-z0-9_]*\s*\(").unwrap();
        if re.is_match(line) {
            return true;
        }

        false
    }

    fn looks_like_incomplete_signature(&self, line: &str) -> bool {
        // Just a name with empty parens: validateToken()
        let incomplete_re = Regex::new(r"`?[A-Za-z_][A-Za-z0-9_]*\s*\(\s*\)`?$").unwrap();
        if incomplete_re.is_match(line.trim()) {
            return true;
        }

        // Name with description but no signature: validateToken - validates token
        // or `validateToken` - validates token
        if line.contains(" - ") && !line.contains('(') {
            return true;
        }

        // Backtick-wrapped function name without params: `validateToken` or `validate_token`
        let backtick_name_re = Regex::new(r"`[A-Za-z_][A-Za-z0-9_]*`").unwrap();
        if backtick_name_re.is_match(line) && !line.contains('(') {
            return true;
        }

        false
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

struct Section {
    name: String,
    start_line: usize,
    content: Vec<(usize, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(content: &str) -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        (temp, file_path)
    }

    /// Helper: Appends Contract and Protocol sections with None if missing
    fn with_required_sections(base: &str) -> String {
        let mut content = base.to_string();
        if !content.contains("## Contract") {
            content.push_str("\n## Contract\nNone\n");
        }
        if !content.contains("## Protocol") {
            content.push_str("\n## Protocol\nNone\n");
        }
        content
    }

    #[test]
    fn test_missing_purpose_fails() {
        let content = with_required_sections(
            r#"# Test Module

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Purpose")));
    }

    #[test]
    fn test_valid_typescript_exports() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_exports_missing_signature_warns() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken` - validates the token

## Behavior
- valid token → Claims object
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        // Should have warnings about missing signature
        assert!(!result.warnings.is_empty() || !result.errors.is_empty());
    }

    #[test]
    fn test_behavior_without_arrow_fails() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- 토큰을 검증합니다
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.error_type == "InvalidBehavior"));
    }

    #[test]
    fn test_missing_contract_fails() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object

## Protocol
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Contract")));
    }

    #[test]
    fn test_missing_protocol_fails() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object

## Contract
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Protocol")));
    }

    #[test]
    fn test_contract_allows_none() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        // Contract with None should pass
        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }
}
