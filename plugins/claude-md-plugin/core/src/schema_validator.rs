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

const GO_BUILTIN_TYPES: &[&str] = &[
    "error", "bool", "string", "byte", "rune",
    "int", "int8", "int16", "int32", "int64",
    "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
    "float32", "float64",
    "complex64", "complex128",
];

pub struct SchemaValidator {
    /// Pattern to match section headers
    section_pattern: Regex,
    /// Pattern to match behavior scenarios
    behavior_pattern: Regex,
    /// Pre-compiled: Java/Kotlin signature pattern
    java_kotlin_sig_re: Regex,
    /// Pre-compiled: incomplete signature pattern
    incomplete_sig_re: Regex,
    /// Pre-compiled: backtick-wrapped name pattern
    backtick_name_re: Regex,
    /// Pre-compiled forbidden reference patterns from schema-rules.yaml
    forbidden_ref_patterns: Vec<(Regex, String)>,
}

impl SchemaValidator {
    pub fn new() -> Self {
        // Match markdown headers like "## Purpose", "### Functions"
        let section_pattern = Regex::new(r"^#+\s+(.+)$").unwrap();

        // Match behavior scenarios: input → output
        let behavior_pattern = Regex::new(r"→|->").unwrap();

        // Pre-compiled hot path regexes
        let java_kotlin_sig_re = Regex::new(r"^\s*[-*]?\s*`?[A-Za-z_<>\[\]]+\s+[A-Za-z_][A-Za-z0-9_]*\s*\(").unwrap();
        let incomplete_sig_re = Regex::new(r"`?[A-Za-z_][A-Za-z0-9_]*\s*\(\s*\)`?$").unwrap();
        let backtick_name_re = Regex::new(r"`[A-Za-z_][A-Za-z0-9_]*`").unwrap();

        // Compile forbidden reference patterns from SSOT (schema-rules.yaml via build.rs)
        let forbidden_ref_patterns = FORBIDDEN_REFERENCE_PATTERNS
            .iter()
            .filter_map(|(pattern, desc)| {
                Regex::new(pattern).ok().map(|re| (re, desc.to_string()))
            })
            .collect();

        Self {
            section_pattern,
            behavior_pattern,
            java_kotlin_sig_re,
            incomplete_sig_re,
            backtick_name_re,
            forbidden_ref_patterns,
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

        // Validate Dependencies for forbidden references (INV-1: tree structure)
        self.validate_dependencies(&content, &mut errors);

        ValidationResult {
            file: file_str,
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate DEVELOPERS.md schema (called in strict mode)
    pub fn validate_developers(&self, developers_path: &Path) -> ValidationResult {
        let file_str = developers_path.to_string_lossy().to_string();

        let content = match std::fs::read_to_string(developers_path) {
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
        let warnings = Vec::new();

        let sections = self.parse_sections(&content);

        // Check required sections for DEVELOPERS.md
        for required in DEVELOPERS_REQUIRED_SECTIONS {
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
                    let allows_none = DEVELOPERS_ALLOW_NONE_SECTIONS
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(required));
                    let is_none = self.is_none_marker(section);

                    if !allows_none && is_none {
                        errors.push(ValidationError {
                            error_type: "InvalidSectionContent".to_string(),
                            message: format!(
                                "Section '{}' does not allow 'None' as value",
                                required
                            ),
                            line_number: Some(section.start_line),
                            section: Some(required.to_string()),
                        });
                    }
                }
            }
        }

        ValidationResult {
            file: file_str,
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate CLAUDE.md with strict mode: also checks DEVELOPERS.md presence and schema (INV-3)
    pub fn validate_strict(&self, claude_md_path: &Path) -> ValidationResult {
        // First validate CLAUDE.md itself
        let mut result = self.validate(claude_md_path);

        // Check DEVELOPERS.md existence (INV-3)
        let developers_path = claude_md_path
            .parent()
            .map(|p| p.join("DEVELOPERS.md"))
            .unwrap_or_else(|| std::path::PathBuf::from("DEVELOPERS.md"));

        if !developers_path.exists() {
            result.warnings.push(format!(
                "INV-3: DEVELOPERS.md not found at {}",
                developers_path.display()
            ));
        } else {
            // Validate DEVELOPERS.md schema
            let dev_result = self.validate_developers(&developers_path);
            if !dev_result.valid {
                for err in dev_result.errors {
                    result.errors.push(ValidationError {
                        error_type: format!("DEVELOPERS.md:{}", err.error_type),
                        message: format!("DEVELOPERS.md: {}", err.message),
                        line_number: err.line_number,
                        section: err.section,
                    });
                }
                result.valid = false;
            }
        }

        result
    }

    fn parse_sections(&self, content: &str) -> Vec<ValidatorSection> {
        let mut sections = Vec::new();
        let mut current_section: Option<ValidatorSection> = None;

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

                current_section = Some(ValidatorSection {
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
    fn is_none_marker(&self, section: &ValidatorSection) -> bool {
        let lines: Vec<&str> = section.content.iter().map(|(_, s)| s.as_str()).collect();
        crate::is_none_marker_content(&lines)
    }

    fn validate_exports(
        &self,
        section: &ValidatorSection,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<String>,
    ) {
        let mut found_valid_export = false;
        let mut in_list = false;
        let mut in_table = false;

        for (line_num, line) in &section.content {
            let trimmed = line.trim();

            // Skip empty lines and subsection headers
            if trimmed.is_empty() || trimmed.starts_with('#') {
                in_table = false; // Reset table state at section boundaries
                continue;
            }

            // Table format: detect separator row (|------|------|) to recognize
            // that subsequent | rows are valid export data (not just headers).
            // Detect table separator row (e.g. |------|------|)
            if trimmed.starts_with('|') && trimmed.contains("---") {
                in_table = true;
                continue;
            }

            // Table data row (after separator)
            if in_table && trimmed.starts_with('|') {
                found_valid_export = true;
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
                } else if self.looks_like_enum_line(trimmed) {
                    found_valid_export = true;
                } else if self.looks_like_variable_line(trimmed) {
                    found_valid_export = true;
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
        section: &ValidatorSection,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<String>,
    ) {
        let mut found_valid_behavior = false;
        let mut in_table = false;

        for (_, line) in &section.content {
            let trimmed = line.trim();

            // Skip empty lines and headers
            if trimmed.is_empty() || trimmed.starts_with('#') {
                in_table = false; // Reset table state at section boundaries
                continue;
            }

            // Detect table separator row (e.g. |-------|--------|)
            if trimmed.starts_with('|') && trimmed.contains("---") {
                in_table = true;
                continue;
            }

            // Table data row (after separator)
            if in_table && trimmed.starts_with('|') {
                found_valid_behavior = true;
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

    /// Validate Dependencies section for forbidden reference patterns (e.g. parent `../` references).
    /// Scans raw content between `## Dependencies` and the next `##` header to handle subsections.
    fn validate_dependencies(
        &self,
        content: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        let mut in_dependencies = false;

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Detect ## Dependencies header (H2 only)
            if trimmed.starts_with("## ")
                && trimmed[3..].trim().eq_ignore_ascii_case("Dependencies")
            {
                in_dependencies = true;
                continue;
            }

            // Exit when reaching next H2 header
            if in_dependencies && trimmed.starts_with("## ") {
                break;
            }

            // Skip empty lines and H3+ subsection headers within Dependencies
            if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check each line against forbidden patterns
            for (pattern, description) in &self.forbidden_ref_patterns {
                if pattern.is_match(trimmed) {
                    errors.push(ValidationError {
                        error_type: "ForbiddenReference".to_string(),
                        message: format!("{}: {}", description, trimmed),
                        line_number: Some(idx + 1),
                        section: Some("Dependencies".to_string()),
                    });
                }
            }
        }
    }

    fn looks_like_export_line(&self, line: &str) -> bool {
        // Has a function name pattern followed by parentheses
        line.contains('(') && line.contains(')')
    }

    /// Enum export: `Status: Active | Inactive | Pending` or `Status = A | B`
    fn looks_like_enum_line(&self, line: &str) -> bool {
        if !line.contains('|') {
            return false;
        }
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`');
        (cleaned.contains(':') || cleaned.contains('='))
            && cleaned.split('|').count() >= 2
    }

    /// Variable/constant export: `MAX_RETRIES = 3` or `TIMEOUT: number`
    fn looks_like_variable_line(&self, line: &str) -> bool {
        let cleaned = line.trim_start_matches('-').trim_start_matches('*').trim();
        let cleaned = cleaned.trim_start_matches('`').trim_end_matches('`');
        if cleaned.contains('(') || cleaned.contains('|') {
            return false;
        }
        let first = match cleaned.chars().next() {
            Some(c) => c,
            None => return false,
        };
        if !first.is_uppercase() {
            return false;
        }
        cleaned.contains('=') || cleaned.contains(':')
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

        // Go: Func(param type) (ReturnType, error) — multi-return with tuple
        if line.contains(") (") {
            return true;
        }

        // Go: Func(param type) ReturnType — single return (no parens around return)
        // Match pattern: closing paren followed by space and a capitalized type or basic type
        if let Some(after_paren) = line.split(')').last() {
            let after = after_paren.trim().trim_end_matches('`');
            if !after.is_empty()
                && !after.starts_with('(')
                && !after.starts_with(':')
                && !after.starts_with('-')
                && !after.starts_with('=')
                && (after.starts_with(|c: char| c.is_ascii_uppercase())
                    || GO_BUILTIN_TYPES.contains(&after)
                    || after.starts_with('*')
                    || after.starts_with("[]")
                    || after.starts_with("map["))
            {
                return true;
            }
        }

        // Rust: func(param: Type) -> Result<T, E>
        if line.contains(") -> ") {
            return true;
        }

        // Java/Kotlin: ReturnType funcName(ParamType param)
        if self.java_kotlin_sig_re.is_match(line) {
            return true;
        }

        false
    }

    fn looks_like_incomplete_signature(&self, line: &str) -> bool {
        // Just a name with empty parens: validateToken()
        if self.incomplete_sig_re.is_match(line.trim()) {
            return true;
        }

        // Name with description but no signature: validateToken - validates token
        // or `validateToken` - validates token
        if line.contains(" - ") && !line.contains('(') {
            return true;
        }

        // Backtick-wrapped function name without params: `validateToken` or `validate_token`
        if self.backtick_name_re.is_match(line) && !line.contains('(') {
            return true;
        }

        false
    }

    /// Fix missing required sections that allow "None" by appending them with "None" content.
    /// Returns the fixed content and a list of sections that were added.
    pub fn fix_missing_sections(&self, content: &str) -> (String, Vec<String>) {
        let sections = self.parse_sections(content);
        let mut fixed = content.to_string();
        let mut added = Vec::new();

        for required in REQUIRED_SECTIONS {
            let found = sections.iter().any(|s| s.name.eq_ignore_ascii_case(required));
            if found {
                continue;
            }
            let allows_none = ALLOW_NONE_SECTIONS.iter().any(|s| s.eq_ignore_ascii_case(required));
            if allows_none {
                if !fixed.ends_with('\n') {
                    fixed.push('\n');
                }
                fixed.push_str(&format!("\n## {}\nNone\n", required));
                added.push(required.to_string());
            }
        }

        (fixed, added)
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Section representation for schema validation, tracking line numbers for error reporting.
struct ValidatorSection {
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
        if !content.contains("## Domain Context") {
            content.push_str("\n## Domain Context\nNone\n");
        }
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

    // C-1: Forbidden reference tests
    #[test]
    fn test_parent_reference_fails() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object

## Dependencies
- **Internal**: `../utils/crypto`
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.error_type == "ForbiddenReference"));
    }

    #[test]
    fn test_child_reference_passes() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object

## Dependencies
- **Internal**: `./jwt/decoder`
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_parent_reference_in_subsection_fails() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims object

## Dependencies

### Internal
- `../utils/crypto`

### External
- `jsonwebtoken@9.x`
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.error_type == "ForbiddenReference"));
    }

    // C-3: Table format tests
    #[test]
    fn test_exports_table_format_passes() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports

| Name | Signature | Description |
|------|-----------|-------------|
| `validateToken` | `(token: string): Promise<Claims>` | JWT 토큰 검증 |

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
    fn test_behavior_table_format_passes() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Validates tokens.

## Exports
- `validateToken(token: string): Promise<Claims>`

## Behavior

| Input | Output |
|-------|--------|
| 유효한 JWT 토큰 | Claims 객체 반환 |
| 만료된 토큰 | TokenExpiredError |
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_go_uint64_return_type() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Provides numeric utilities.

## Exports
- `GetCount(name string) uint64`

## Behavior
- name → count value
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "uint64 return type should be valid: {:?}", result.errors);
    }

    #[test]
    fn test_go_float32_return_type() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Provides math utilities.

## Exports
- `Calculate(x int) float32`

## Behavior
- x → calculated float value
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "float32 return type should be valid: {:?}", result.errors);
    }

    #[test]
    fn test_fix_missing_sections_adds_none_sections() {
        let content = r#"# Test Module

## Purpose
Test module.

## Exports
- `foo(x: int): string`

## Behavior
- input → output
"#;
        let validator = SchemaValidator::new();
        let (fixed, added) = validator.fix_missing_sections(content);

        // Should add Contract, Protocol, Domain Context (all allow_none)
        assert!(added.contains(&"Contract".to_string()));
        assert!(added.contains(&"Protocol".to_string()));
        assert!(added.contains(&"Domain Context".to_string()));
        assert_eq!(added.len(), 3);

        // Fixed content should pass validation
        let (_temp, path) = create_test_file(&fixed);
        let result = validator.validate(&path);
        assert!(result.valid, "Fixed content should pass: {:?}", result.errors);
    }

    #[test]
    fn test_fix_missing_sections_no_change_when_complete() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Test module.

## Exports
- `foo(x: int): string`

## Behavior
- input → output
"#,
        );
        let validator = SchemaValidator::new();
        let (_, added) = validator.fix_missing_sections(&content);

        assert!(added.is_empty(), "No sections should be added: {:?}", added);
    }

    #[test]
    fn test_fix_missing_sections_skips_non_none_sections() {
        // Purpose does not allow None, so it should NOT be auto-added
        let content = r#"# Test Module

## Exports
- `foo(x: int): string`

## Behavior
- input → output
"#;
        let validator = SchemaValidator::new();
        let (_, added) = validator.fix_missing_sections(content);

        // Purpose is required but does NOT allow none — should not be added
        assert!(!added.contains(&"Purpose".to_string()));
        // But allow_none sections should be added
        assert!(added.contains(&"Contract".to_string()));
    }

    // DEVELOPERS.md validation tests

    fn create_developers_file(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
        let file_path = dir.join("DEVELOPERS.md");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        file_path
    }

    #[test]
    fn test_developers_valid_all_sections() {
        let content = r#"# Test Module

## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
| index.ts | 진입점 | - |

## Data Structures
None

## Decision Log
None

## Operations
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_developers_missing_file_map_fails() {
        let content = r#"# Test Module

## Data Structures
None

## Decision Log
None

## Operations
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("File Map")));
    }

    #[test]
    fn test_developers_file_map_none_not_allowed() {
        let content = r#"# Test Module

## File Map
None

## Data Structures
None

## Decision Log
None

## Operations
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e|
            e.error_type == "InvalidSectionContent" && e.message.contains("File Map")
        ));
    }

    #[test]
    fn test_developers_data_structures_allows_none() {
        let content = r#"# Test Module

## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
| index.ts | 진입점 | - |

## Data Structures
None

## Decision Log
None

## Operations
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(result.valid, "Data Structures with None should pass: {:?}", result.errors);
    }

    #[test]
    fn test_strict_mode_missing_developers_md() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Test module.

## Exports
- `foo(x: int): string`

## Behavior
- input → output
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate_strict(&path);

        // Should have INV-3 warning (DEVELOPERS.md not found)
        assert!(result.warnings.iter().any(|w| w.starts_with("INV-3:")));
    }

    #[test]
    fn test_strict_mode_with_valid_developers_md() {
        let claude_content = with_required_sections(
            r#"# Test Module

## Purpose
Test module.

## Exports
- `foo(x: int): string`

## Behavior
- input → output
"#,
        );
        let temp = TempDir::new().unwrap();
        let claude_path = temp.path().join("CLAUDE.md");
        let mut f = File::create(&claude_path).unwrap();
        write!(f, "{}", claude_content).unwrap();

        let dev_content = r#"# Test Module

## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
| index.ts | 진입점 | - |

## Data Structures
None

## Decision Log
None

## Operations
None
"#;
        create_developers_file(temp.path(), dev_content);

        let validator = SchemaValidator::new();
        let result = validator.validate_strict(&claude_path);

        assert!(result.valid, "Strict validation with valid DEVELOPERS.md should pass: {:?}", result.errors);
        assert!(!result.warnings.iter().any(|w| w.starts_with("INV-3:")));
    }

    #[test]
    fn test_strict_mode_with_invalid_developers_md() {
        let claude_content = with_required_sections(
            r#"# Test Module

## Purpose
Test module.

## Exports
- `foo(x: int): string`

## Behavior
- input → output
"#,
        );
        let temp = TempDir::new().unwrap();
        let claude_path = temp.path().join("CLAUDE.md");
        let mut f = File::create(&claude_path).unwrap();
        write!(f, "{}", claude_content).unwrap();

        // DEVELOPERS.md missing File Map
        let dev_content = r#"# Test Module

## Data Structures
None

## Decision Log
None

## Operations
None
"#;
        create_developers_file(temp.path(), dev_content);

        let validator = SchemaValidator::new();
        let result = validator.validate_strict(&claude_path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e|
            e.error_type.starts_with("DEVELOPERS.md:") && e.message.contains("File Map")
        ));
    }

    #[test]
    fn test_go_pointer_return_type() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Provides server management.

## Exports
- `NewServer(addr string) *Server`

## Behavior
- addr → server instance
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "*Server pointer return type should be valid: {:?}", result.errors);
    }
}
