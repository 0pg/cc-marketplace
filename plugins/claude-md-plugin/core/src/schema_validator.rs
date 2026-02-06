use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::symbol_index::SymbolIndexResult;

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
    /// Suggested fix for the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

// Include generated constants from schema-rules.yaml (SSOT)
// Not all constants are used in every module that includes them
#[allow(dead_code)]
mod schema_rules {
    include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));
}
use schema_rules::*;

pub struct SchemaValidator {
    /// Pattern to match section headers
    section_pattern: Regex,
    /// Pattern to match behavior scenarios
    behavior_pattern: Regex,
    /// Pattern to detect schema version marker
    schema_version_pattern: Regex,
    /// Pattern to match use case IDs
    usecase_id_pattern: Regex,
    /// Pattern to match include references
    include_pattern: Regex,
    /// Pattern to match extend references
    extend_pattern: Regex,
    /// Pattern to match cross-references (CLAUDE.md#symbol)
    cross_ref_pattern: Regex,
}

impl SchemaValidator {
    pub fn new() -> Self {
        // Match markdown headers like "## Purpose", "### Functions"
        let section_pattern = Regex::new(r"^#+\s+(.+)$").unwrap();

        // Match behavior scenarios: input → output
        let behavior_pattern = Regex::new(r"→|->").unwrap();

        // Schema version marker: <!-- schema: 2.0 -->
        let schema_version_pattern = Regex::new(SCHEMA_VERSION_MARKER_PATTERN).unwrap();

        // v2 behavior patterns
        let usecase_id_pattern = Regex::new(USECASE_ID_PATTERN).unwrap();
        let include_pattern = Regex::new(INCLUDE_PATTERN).unwrap();
        let extend_pattern = Regex::new(EXTEND_PATTERN).unwrap();

        // Cross-reference pattern: path/CLAUDE.md#symbolName
        let cross_ref_pattern = Regex::new(r"([A-Za-z0-9_./-]*CLAUDE\.md)#([A-Za-z_][A-Za-z0-9_]*)").unwrap();

        Self {
            section_pattern,
            behavior_pattern,
            schema_version_pattern,
            usecase_id_pattern,
            include_pattern,
            extend_pattern,
            cross_ref_pattern,
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
                        suggestion: None,
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
                        suggestion: Some(format!("Add a '## {}' section to the CLAUDE.md file", required)),
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
                            suggestion: Some(format!("Add content to the '{}' section instead of 'None'", required)),
                        });
                    }
                }
            }
        }

        // Detect schema version
        let schema_version = self.detect_schema_version(&content);

        // Validate Exports section format
        if let Some(exports) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Exports")) {
            self.validate_exports(exports, &mut errors, &mut warnings);
        }

        // Validate Behavior section format
        if let Some(behavior) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Behavior")) {
            let is_v2 = schema_version.as_deref() == Some("2.0");
            self.validate_behavior(behavior, &content, &sections, is_v2, &mut errors, &mut warnings);

            // v2-specific: validate actors and use cases
            if is_v2 {
                self.validate_v2_behavior(&content, behavior, &sections, &mut errors, &mut warnings);
            }
        }

        // Validate cross-references in all content
        self.validate_cross_references(&content, &mut warnings);

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
                suggestion: Some("Use format: `functionName(param: Type): ReturnType` or 'None' if no exports".to_string()),
            });
        }
    }

    /// Get the full content lines belonging to a ## section (including ### subsections).
    /// Since parse_sections splits at every # heading, subsections are separate Section objects.
    /// This method finds the range from the section start to the next ## section.
    fn get_full_section_lines<'a>(&self, content: &'a str, section: &Section, all_sections: &[Section]) -> Vec<(usize, &'a str)> {
        let start_line = section.start_line; // 1-based

        // Find the next ## section (same or higher level)
        let end_line = all_sections.iter()
            .filter(|s| s.start_line > start_line)
            .filter(|s| {
                // Only stop at ## level (not ### which are subsections)
                // Check the original content to determine heading level
                let line_idx = s.start_line - 1;
                content.lines().nth(line_idx)
                    .map(|l| l.starts_with("## ") && !l.starts_with("### "))
                    .unwrap_or(false)
            })
            .map(|s| s.start_line)
            .next()
            .unwrap_or(content.lines().count() + 1);

        content.lines()
            .enumerate()
            .skip(start_line) // skip the ## heading itself (0-indexed skip = start_line since it's 1-based)
            .take_while(|(i, _)| i + 1 < end_line)
            .map(|(i, line)| (i + 1, line))
            .collect()
    }

    fn validate_behavior(
        &self,
        section: &Section,
        content: &str,
        all_sections: &[Section],
        is_v2: bool,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<String>,
    ) {
        let mut found_valid_behavior = false;

        // For v2 files, scan the full section range including ### subsections
        let lines_to_check: Vec<(usize, String)> = if is_v2 {
            self.get_full_section_lines(content, section, all_sections)
                .into_iter()
                .map(|(n, l)| (n, l.to_string()))
                .collect()
        } else {
            section.content.clone()
        };

        for (_, line) in &lines_to_check {
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

            // v2 markers count as valid behavior
            if trimmed.starts_with("- Actor:") || trimmed.starts_with("- Includes:") || trimmed.starts_with("- Extends:") {
                found_valid_behavior = true;
                continue;
            }

            // Check for scenario pattern: input → output
            if self.behavior_pattern.is_match(trimmed) {
                found_valid_behavior = true;
            }
        }

        if !found_valid_behavior && !lines_to_check.is_empty() {
            errors.push(ValidationError {
                error_type: "InvalidBehavior".to_string(),
                message: "Behavior section must contain scenarios in 'input → output' format or 'None'".to_string(),
                line_number: Some(section.start_line),
                section: Some("Behavior".to_string()),
                suggestion: Some("Use format: '- input → output' (e.g., '- valid token → Claims')".to_string()),
            });
        }
    }

    /// Detect schema version from content (first 5 lines)
    fn detect_schema_version(&self, content: &str) -> Option<String> {
        for line in content.lines().take(5) {
            if let Some(caps) = self.schema_version_pattern.captures(line) {
                return caps.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    /// Validate v2-specific Behavior section (Actors, Use Cases)
    /// Uses full content range to capture ### subsections
    fn validate_v2_behavior(
        &self,
        content: &str,
        section: &Section,
        all_sections: &[Section],
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<String>,
    ) {
        let full_lines = self.get_full_section_lines(content, section, all_sections);

        let mut uc_ids: Vec<String> = Vec::new();
        let mut include_targets: Vec<String> = Vec::new();
        let mut extend_targets: Vec<String> = Vec::new();

        for (line_num, line) in &full_lines {
            let trimmed = line.trim();

            // Collect UC IDs from ### UC-N: Name headings
            if let Some(caps) = self.usecase_id_pattern.captures(trimmed) {
                let id = format!("UC-{}", caps.get(1).map(|m| m.as_str()).unwrap_or(""));
                if uc_ids.contains(&id) {
                    errors.push(ValidationError {
                        error_type: "DuplicateUseCaseId".to_string(),
                        message: format!("Duplicate use case ID: {}", id),
                        line_number: Some(*line_num),
                        section: Some("Behavior".to_string()),
                        suggestion: Some(format!("Rename one of the duplicate '{}' use cases to a unique ID (e.g., UC-N with a different number)", id)),
                    });
                }
                uc_ids.push(id);
            }

            // Collect include targets
            if let Some(caps) = self.include_pattern.captures(trimmed) {
                if let Some(targets) = caps.get(1) {
                    for target in targets.as_str().split(',').map(|s| s.trim().to_string()) {
                        include_targets.push(target);
                    }
                }
            }

            // Collect extend targets
            if let Some(caps) = self.extend_pattern.captures(trimmed) {
                if let Some(targets) = caps.get(1) {
                    for target in targets.as_str().split(',').map(|s| s.trim().to_string()) {
                        extend_targets.push(target);
                    }
                }
            }
        }

        // Validate include/extend targets exist
        for target in &include_targets {
            if !uc_ids.contains(target) {
                errors.push(ValidationError {
                    error_type: "InvalidIncludeTarget".to_string(),
                    message: format!("Include target '{}' does not match any defined use case", target),
                    line_number: None,
                    section: Some("Behavior".to_string()),
                    suggestion: Some(format!("Add '### {}: <Name>' subsection to the Behavior section, or fix the Includes reference", target)),
                });
            }
        }

        for target in &extend_targets {
            if !uc_ids.contains(target) {
                errors.push(ValidationError {
                    error_type: "InvalidExtendTarget".to_string(),
                    message: format!("Extend target '{}' does not match any defined use case", target),
                    line_number: None,
                    section: Some("Behavior".to_string()),
                    suggestion: Some(format!("Add '### {}: <Name>' subsection to the Behavior section, or fix the Extends reference", target)),
                });
            }
        }
    }

    /// Validate cross-reference syntax in content
    fn validate_cross_references(
        &self,
        content: &str,
        warnings: &mut Vec<String>,
    ) {
        for (line_num, line) in content.lines().enumerate() {
            for caps in self.cross_ref_pattern.captures_iter(line) {
                let full_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let symbol_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");

                // Warn if path looks malformed (double slashes, trailing slash before CLAUDE.md)
                if full_path.contains("//") {
                    warnings.push(format!(
                        "Line {}: Cross-reference path '{}#{}' contains double slashes",
                        line_num + 1, full_path, symbol_name
                    ));
                }

                // Warn if symbol name starts with uppercase but looks like a path component
                if symbol_name.contains('/') {
                    warnings.push(format!(
                        "Line {}: Cross-reference symbol '{}' should not contain path separators",
                        line_num + 1, symbol_name
                    ));
                }
            }
        }
    }

    /// Validate a CLAUDE.md file with cross-reference resolution against a symbol index.
    /// This extends the basic validate() with actual cross-reference verification.
    pub fn validate_with_index(&self, file: &Path, index: &SymbolIndexResult) -> ValidationResult {
        // First perform standard validation
        let mut result = self.validate(file);

        // Then verify cross-references against the symbol index
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => return result,
        };

        let anchor_set: std::collections::HashSet<&str> = index.symbols.iter()
            .map(|s| s.anchor.as_str())
            .collect();

        for (line_num, line) in content.lines().enumerate() {
            for caps in self.cross_ref_pattern.captures_iter(line) {
                let full_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let symbol_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let to_anchor = format!("{}#{}", full_path, symbol_name);

                // Skip self-references
                let file_str = file.to_string_lossy();
                if file_str.ends_with("CLAUDE.md") && to_anchor.ends_with(&format!("CLAUDE.md#{}", symbol_name)) {
                    // Check if it's truly a self-reference
                    let self_path = file.parent()
                        .and_then(|p| {
                            if let Some(root) = index.root.strip_suffix('/') {
                                p.to_string_lossy().strip_prefix(root).map(|s| s.trim_start_matches('/').to_string())
                            } else {
                                p.strip_prefix(&index.root).ok().map(|s| s.to_string_lossy().to_string())
                            }
                        })
                        .unwrap_or_default();
                    let self_anchor = if self_path.is_empty() {
                        format!("CLAUDE.md#{}", symbol_name)
                    } else {
                        format!("{}/CLAUDE.md#{}", self_path, symbol_name)
                    };
                    if to_anchor == self_anchor {
                        continue;
                    }
                }

                if !anchor_set.contains(to_anchor.as_str()) {
                    result.errors.push(ValidationError {
                        error_type: "UnresolvedCrossReference".to_string(),
                        message: format!(
                            "Cross-reference '{}' does not resolve to any known symbol",
                            to_anchor
                        ),
                        line_number: Some(line_num + 1),
                        section: None,
                        suggestion: Some(format!(
                            "Check that '{}' exists in {} Exports section, or fix the reference",
                            symbol_name, full_path
                        )),
                    });
                    result.valid = false;
                }
            }
        }

        result
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

    /// Helper: Appends required sections with None if missing
    fn with_required_sections(base: &str) -> String {
        let mut content = base.to_string();
        // Add Summary right after Purpose if not present
        if !content.contains("## Summary") {
            // Insert Summary after Purpose section
            if let Some(pos) = content.find("## Exports") {
                content.insert_str(pos, "## Summary\nTest module summary.\n\n");
            } else {
                content.push_str("\n## Summary\nTest module summary.\n");
            }
        }
        if !content.contains("## Contract") {
            content.push_str("\n## Contract\nNone\n");
        }
        if !content.contains("## Protocol") {
            content.push_str("\n## Protocol\nNone\n");
        }
        if !content.contains("## Domain Context") {
            content.push_str("\n## Domain Context\nNone\n");
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
    fn test_v2_duplicate_usecase_id() {
        let content = r#"<!-- schema: 2.0 -->
# Test Module

## Purpose
Auth module.

## Summary
Test module summary.

## Exports
- `validateToken(token: string): Claims`

## Behavior

### Actors
- User: End user

### UC-1: Token Validation
- Actor: User
- valid token → Claims

### UC-1: Duplicate Validation
- Actor: User
- expired token → Error

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);
        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.error_type == "DuplicateUseCaseId"));
    }

    #[test]
    fn test_v2_invalid_include_target() {
        let content = r#"<!-- schema: 2.0 -->
# Test Module

## Purpose
Auth module.

## Summary
Test module summary.

## Exports
- `validateToken(token: string): Claims`

## Behavior

### UC-1: Token Validation
- Actor: User
- valid token → Claims
- Includes: UC-99

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);
        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.error_type == "InvalidIncludeTarget"));
    }

    #[test]
    fn test_v2_valid_include_target() {
        let content = r#"<!-- schema: 2.0 -->
# Test Module

## Purpose
Auth module.

## Summary
Test module summary.

## Exports
- `validateToken(token: string): Claims`

## Behavior

### UC-1: Token Validation
- Actor: User
- valid token → Claims
- Includes: UC-2

### UC-2: Token Parsing
- Actor: System
- JWT string → parsed token

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);
        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_v1_file_skips_v2_validation() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Auth module.

## Exports
- `validateToken(token: string): Claims`

## Behavior
- valid token → Claims
"#,
        );
        let (_temp, path) = create_test_file(&content);
        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        // v1 file should pass without v2-specific checks
        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_cross_reference_double_slash_warning() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
Auth module. Uses src//auth/CLAUDE.md#validateToken.

## Exports
- `validateToken(token: string): Claims`

## Behavior
- valid token → Claims
"#,
        );
        let (_temp, path) = create_test_file(&content);
        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.warnings.iter().any(|w| w.contains("double slashes")));
    }

    #[test]
    fn test_validate_with_index_valid_reference() {
        use crate::symbol_index::{SymbolEntry, SymbolKind, SymbolIndexResult, SymbolIndexSummary};

        let content = with_required_sections(
            r#"# Test Module

## Purpose
API module. Uses auth/CLAUDE.md#validateToken for auth.

## Exports
- `handleRequest(req: Request): Response`

## Behavior
- request → response
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let index = SymbolIndexResult {
            root: _temp.path().to_string_lossy().to_string(),
            indexed_at: String::new(),
            symbols: vec![SymbolEntry {
                name: "validateToken".to_string(),
                kind: SymbolKind::Function,
                module_path: "auth".to_string(),
                signature: Some("validateToken(token: string): Claims".to_string()),
                anchor: "auth/CLAUDE.md#validateToken".to_string(),
            }],
            references: vec![],
            unresolved: vec![],
            summary: SymbolIndexSummary {
                total_modules: 1,
                total_symbols: 1,
                total_references: 0,
                unresolved_count: 0,
            },
        };

        let validator = SchemaValidator::new();
        let result = validator.validate_with_index(&path, &index);

        assert!(result.valid, "Expected validation to pass, got: {:?}", result.errors);
    }

    #[test]
    fn test_validate_with_index_unresolved_reference() {
        use crate::symbol_index::{SymbolIndexResult, SymbolIndexSummary};

        let content = with_required_sections(
            r#"# Test Module

## Purpose
API module. Uses auth/CLAUDE.md#nonExistent for auth.

## Exports
- `handleRequest(req: Request): Response`

## Behavior
- request → response
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let index = SymbolIndexResult {
            root: _temp.path().to_string_lossy().to_string(),
            indexed_at: String::new(),
            symbols: vec![],
            references: vec![],
            unresolved: vec![],
            summary: SymbolIndexSummary {
                total_modules: 0,
                total_symbols: 0,
                total_references: 0,
                unresolved_count: 0,
            },
        };

        let validator = SchemaValidator::new();
        let result = validator.validate_with_index(&path, &index);

        assert!(!result.valid, "Expected validation to fail");
        assert!(result.errors.iter().any(|e| e.error_type == "UnresolvedCrossReference"));
        assert!(result.errors.iter().any(|e| e.suggestion.as_deref().unwrap_or("").contains("auth/CLAUDE.md")));
    }

    #[test]
    fn test_validate_without_index_passes_syntax_only() {
        let content = with_required_sections(
            r#"# Test Module

## Purpose
API module. Uses auth/CLAUDE.md#anything for auth.

## Exports
- `handleRequest(req: Request): Response`

## Behavior
- request → response
"#,
        );
        let (_temp, path) = create_test_file(&content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        // Without index, only syntax check is performed, cross-references pass
        assert!(result.valid, "Expected syntax-only validation to pass, got: {:?}", result.errors);
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
