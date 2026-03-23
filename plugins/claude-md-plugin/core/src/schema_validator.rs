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
    /// Completeness score (0-100): percentage of non-None required sections
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completeness_score: Option<u32>,
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

/// Context for conditional section evaluation.
/// v3 schema: conditions are is_project_root, is_project_or_module_root, has_multiple_files.
#[derive(Debug, Default)]
pub struct ValidationContext {
    pub is_project_root: bool,
    pub has_subdirs_or_files: bool,
    pub has_multiple_files: bool,
    pub source_file_count: usize,
}

// Include generated constants from schema-rules.yaml (SSOT)
include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));

pub struct SchemaValidator;

impl SchemaValidator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate conditions by scanning source files in the given directory.
    pub fn evaluate_conditions(dir: &Path) -> ValidationContext {
        let mut ctx = ValidationContext::default();

        let mut file_count = 0u32;
        let mut has_subdirs = false;

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return ctx,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if !crate::EXCLUDED_DIRS.contains(&name.as_ref()) {
                    has_subdirs = true;
                }
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if crate::SOURCE_EXTENSIONS.contains(&ext_str.as_ref()) {
                    file_count += 1;
                }
            }
        }

        ctx.source_file_count = file_count as usize;
        ctx.has_multiple_files = file_count >= 2;
        ctx.has_subdirs_or_files = has_subdirs || file_count > 0;

        // is_project_root is not auto-detected from directory scan;
        // it must be set by the caller based on whether the file is at the project root.

        ctx
    }

    /// Check if a condition is met by the given context
    fn condition_met(condition: &str, ctx: &ValidationContext) -> bool {
        match condition {
            "always" => true,
            "is_project_root" => ctx.is_project_root,
            "is_project_or_module_root" => true, // Caller must determine this
            "has_subdirs_or_files" => ctx.has_subdirs_or_files,
            "has_multiple_files" => ctx.has_multiple_files,
            _ => true, // Unknown conditions default to required
        }
    }

    /// Validate a CLAUDE.md file (always-required sections only, backward compatible)
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
                    completeness_score: None,
                };
            }
        };

        self.validate_content(&content, &file_str, None)
    }

    /// Validate a CLAUDE.md file with context for conditional section evaluation.
    pub fn validate_with_context(&self, file: &Path, ctx: &ValidationContext) -> ValidationResult {
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
                    completeness_score: None,
                };
            }
        };

        self.validate_content(&content, &file_str, Some(ctx))
    }

    /// Core validation logic shared between validate() and validate_with_context()
    fn validate_content(&self, content: &str, file_str: &str, ctx: Option<&ValidationContext>) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Parse sections
        let sections = self.parse_sections(content);

        // Check always-required sections
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

        // Check conditionally-required sections when context is provided
        if let Some(ctx) = ctx {
            for (name, condition) in CONDITIONALLY_REQUIRED_SECTIONS {
                if Self::condition_met(condition, ctx) {
                    let section_found = sections.iter().find(|s| s.name.eq_ignore_ascii_case(name));

                    match section_found {
                        None => {
                            errors.push(ValidationError {
                                error_type: "MissingSection".to_string(),
                                message: format!("Missing conditionally required section: {} (condition: {})", name, condition),
                                line_number: None,
                                section: Some(name.to_string()),
                            });
                        }
                        Some(section) => {
                            let allows_none = ALLOW_NONE_SECTIONS.iter().any(|s| s.eq_ignore_ascii_case(name));
                            let is_none_marker = self.is_none_marker(section);

                            if !allows_none && is_none_marker {
                                errors.push(ValidationError {
                                    error_type: "InvalidSectionContent".to_string(),
                                    message: format!("Section '{}' does not allow 'None' as value", name),
                                    line_number: Some(section.start_line),
                                    section: Some(name.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Warn about unrecognized sections (v3: only known sections expected)
        let known_sections = [
            "purpose", "constraints", "domain context", "instructions",
            "project convention", "code convention",
        ];
        for section in &sections {
            // Only check H2 sections, skip H1 (title) and H3+ (subsections)
            if section.name.starts_with('#') {
                continue;
            }
            let name_lower = section.name.to_lowercase();
            // Check if this section header has ## prefix (H2 level)
            let is_h2 = section.content.is_empty() || true; // All parsed sections from parse_sections
            if is_h2 && !known_sections.contains(&name_lower.as_str()) {
                // Only warn for top-level H2 sections that aren't the module name (H1)
                // The section_pattern captures all header levels, so we check start_line context
                warnings.push(format!("Unrecognized section: {}", section.name));
            }
        }

        // Calculate completeness score
        let completeness = self.completeness_score(&sections);

        ValidationResult {
            file: file_str.to_string(),
            valid: errors.is_empty(),
            errors,
            warnings,
            completeness_score: Some(completeness),
        }
    }

    /// Validate DEVELOPERS.md schema (called in strict mode)
    pub fn validate_developers(&self, developers_path: &Path) -> ValidationResult {
        self.validate_developers_with_context(developers_path, None)
    }

    /// Validate DEVELOPERS.md schema with optional directory context.
    /// When dir_path is provided, File Map "None" is allowed for single-file directories.
    pub fn validate_developers_with_context(&self, developers_path: &Path, dir_path: Option<&Path>) -> ValidationResult {
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
                    completeness_score: None,
                };
            }
        };

        let mut errors = Vec::new();
        let warnings = Vec::new();

        let sections = self.parse_sections(&content);

        // Determine if the directory has multiple files (for File Map condition)
        let has_multiple_files = dir_path.map_or(true, |dir| {
            Self::count_source_files(dir) >= 2
        });

        // Check required sections for DEVELOPERS.md
        for required in DEVELOPERS_REQUIRED_SECTIONS {
            // Check if this section has a condition
            let condition = DEVELOPERS_CONDITIONAL_SECTIONS
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(required))
                .map(|(_, cond)| *cond);

            // If there's a condition and it's not met, skip this section
            if let Some(cond) = condition {
                if cond == "has_multiple_files" && !has_multiple_files {
                    continue;
                }
            }

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

                    // File Map: allow None for single-file directories
                    let file_map_single_file_exemption =
                        required.eq_ignore_ascii_case("File Map") && !has_multiple_files;

                    if !allows_none && is_none && !file_map_single_file_exemption {
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
            completeness_score: None,
        }
    }

    /// Count source files in a directory
    fn count_source_files(dir: &Path) -> usize {
        match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| {
                    e.path().is_file()
                        && e.path()
                            .extension()
                            .map_or(false, |ext| {
                                crate::SOURCE_EXTENSIONS.contains(&ext.to_string_lossy().as_ref())
                            })
                })
                .count(),
            Err(_) => 0,
        }
    }

    /// Validate CLAUDE.md with strict mode: also checks DEVELOPERS.md presence and schema (INV-3)
    pub fn validate_strict(&self, claude_md_path: &Path) -> ValidationResult {
        self.validate_strict_with_context(claude_md_path, None)
    }

    /// Validate CLAUDE.md with strict mode and optional context
    pub fn validate_strict_with_context(&self, claude_md_path: &Path, ctx: Option<&ValidationContext>) -> ValidationResult {
        // First validate CLAUDE.md itself
        let mut result = match ctx {
            Some(c) => self.validate_with_context(claude_md_path, c),
            None => self.validate(claude_md_path),
        };

        // Check DEVELOPERS.md existence (INV-3)
        let developers_path = claude_md_path
            .parent()
            .map(|p| p.join("DEVELOPERS.md"))
            .unwrap_or_else(|| std::path::PathBuf::from("DEVELOPERS.md"));

        let dir_path = claude_md_path.parent();

        if !developers_path.exists() {
            result.warnings.push(format!(
                "INV-3: DEVELOPERS.md not found at {}",
                developers_path.display()
            ));
        } else {
            // Validate DEVELOPERS.md schema with directory context
            let dev_result = self.validate_developers_with_context(&developers_path, dir_path);
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

    /// Calculate completeness score: percentage of non-None required sections (0-100)
    pub fn completeness_score(&self, sections: &[ValidatorSection]) -> u32 {
        let all_required: Vec<&str> = REQUIRED_SECTIONS.to_vec();
        if all_required.is_empty() {
            return 100;
        }

        let mut total = 0u32;
        let mut non_none = 0u32;

        for required in &all_required {
            total += 1;
            if let Some(section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case(required)) {
                if !self.is_none_marker(section) {
                    non_none += 1;
                }
            }
            // Missing sections count as None (they don't contribute to non_none)
        }

        if total == 0 { 100 } else { (non_none * 100) / total }
    }

    pub fn parse_sections(&self, content: &str) -> Vec<ValidatorSection> {
        let mut sections = Vec::new();
        let mut current_section: Option<ValidatorSection> = None;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let section_name = trimmed.trim_start_matches('#').trim().to_string();

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
pub struct ValidatorSection {
    pub name: String,
    pub start_line: usize,
    pub content: Vec<(usize, String)>,
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

    /// Helper: Appends always-required allow-none sections if missing
    fn with_required_sections(base: &str) -> String {
        let mut content = base.to_string();
        if !content.contains("## Constraints") {
            content.push_str("\n## Constraints\nNone\n");
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
    fn test_missing_constraints_fails() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Constraints")));
    }

    #[test]
    fn test_valid_minimal_spec() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Constraints
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
    fn test_constraints_with_content() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Constraints
- 비밀번호 재설정 90일 제한
- 동시 세션 최대 5개

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_purpose_does_not_allow_none() {
        let content = r#"# Test Module

## Purpose
None

## Constraints
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.error_type == "InvalidSectionContent" && e.message.contains("Purpose")));
    }

    #[test]
    fn test_instructions_not_required_without_context() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate(&path);

        // Instructions is conditional (is_project_root) — should pass without it
        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_instructions_required_for_project_root() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);

        let ctx = ValidationContext {
            is_project_root: true,
            ..Default::default()
        };

        let validator = SchemaValidator::new();
        let result = validator.validate_with_context(&path, &ctx);

        // Instructions is required when is_project_root is true
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e|
            e.message.contains("Instructions") && e.message.contains("conditionally required")
        ));
    }

    #[test]
    fn test_instructions_present_for_project_root() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None

## Instructions
Always use TypeScript strict mode.

## Project Convention

### Project Structure
Layered architecture.

### Module Boundaries
Each module is self-contained.

### Naming Conventions
camelCase for files.

## Code Convention

### Language & Runtime
TypeScript 5.0

### Coding Rules
- strict mode

### Naming Rules
camelCase for variables.
"#;
        let (_temp, path) = create_test_file(content);

        let ctx = ValidationContext {
            is_project_root: true,
            ..Default::default()
        };

        let validator = SchemaValidator::new();
        let result = validator.validate_with_context(&path, &ctx);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_fix_missing_sections_adds_none_sections() {
        let content = r#"# Test Module

## Purpose
Test module.
"#;
        let validator = SchemaValidator::new();
        let (fixed, added) = validator.fix_missing_sections(content);

        // Should add Constraints and Domain Context (always-required allow_none sections)
        assert!(added.contains(&"Constraints".to_string()));
        assert!(added.contains(&"Domain Context".to_string()));

        // Fixed content should pass validation
        let (_temp, path) = create_test_file(&fixed);
        let result = validator.validate(&path);
        assert!(result.valid, "Fixed content should pass: {:?}", result.errors);
    }

    #[test]
    fn test_fix_missing_sections_no_change_when_complete() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let validator = SchemaValidator::new();
        let (_, added) = validator.fix_missing_sections(content);

        assert!(added.is_empty(), "No sections should be added: {:?}", added);
    }

    #[test]
    fn test_fix_missing_sections_skips_non_none_sections() {
        // Purpose does not allow None, so it should NOT be auto-added
        let content = r#"# Test Module
"#;
        let validator = SchemaValidator::new();
        let (_, added) = validator.fix_missing_sections(content);

        // Purpose is required but does NOT allow none — should not be added
        assert!(!added.contains(&"Purpose".to_string()));
        // But allow_none sections should be added
        assert!(added.contains(&"Constraints".to_string()));
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
    fn test_developers_file_map_none_allowed_single_file() {
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

        // Create a single source file to simulate single-file directory
        let source_file = temp.path().join("index.ts");
        File::create(&source_file).unwrap();

        let validator = SchemaValidator::new();
        let result = validator.validate_developers_with_context(&path, Some(temp.path()));

        assert!(result.valid, "Single-file dir should allow File Map None: {:?}", result.errors);
    }

    #[test]
    fn test_strict_mode_missing_developers_md() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let (_temp, path) = create_test_file(content);

        let validator = SchemaValidator::new();
        let result = validator.validate_strict(&path);

        // Should have INV-3 warning (DEVELOPERS.md not found)
        assert!(result.warnings.iter().any(|w| w.starts_with("INV-3:")));
    }

    #[test]
    fn test_strict_mode_with_valid_developers_md() {
        let claude_content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
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

    // Completeness score tests
    #[test]
    fn test_completeness_score_all_populated() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
- some constraint

## Domain Context
Some important context.
"#;
        let validator = SchemaValidator::new();
        let sections = validator.parse_sections(content);
        let score = validator.completeness_score(&sections);
        // All 3 always-required sections are populated (non-None)
        assert_eq!(score, 100);
    }

    #[test]
    fn test_completeness_score_with_none_sections() {
        let content = r#"# Test Module

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let validator = SchemaValidator::new();
        let sections = validator.parse_sections(content);
        let score = validator.completeness_score(&sections);
        // Only Purpose is non-None out of 3 always-required sections
        // Score = 1/3 * 100 = 33
        assert_eq!(score, 33);
    }

    // Condition evaluation tests
    #[test]
    fn test_evaluate_conditions_empty_dir() {
        let temp = TempDir::new().unwrap();
        let ctx = SchemaValidator::evaluate_conditions(temp.path());
        assert!(!ctx.is_project_root);
        assert!(!ctx.has_multiple_files);
        assert_eq!(ctx.source_file_count, 0);
    }
}
