use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

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
/// v3.1 schema: conditions are is_project_root, is_project_or_module_root.
#[derive(Debug, Default)]
pub struct ValidationContext {
    pub is_project_root: bool,
    pub is_module_root: bool,
}

// Include generated constants from schema-rules.yaml (SSOT)
include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));

pub struct SchemaValidator;

impl SchemaValidator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate conditions by scanning the given directory.
    /// Detects module root by presence of build system markers (Cargo.toml, package.json, etc.).
    /// Detects project root by presence of `.git` directory.
    pub fn evaluate_conditions(dir: &Path) -> ValidationContext {
        let mut ctx = ValidationContext::default();
        ctx.is_module_root = MODULE_ROOT_MARKERS.iter().any(|m| dir.join(m).exists());
        ctx.is_project_root = dir.join(".git").exists();
        ctx
    }

    /// Check if a condition is met by the given context
    fn condition_met(condition: &str, ctx: &ValidationContext) -> bool {
        match condition {
            "always" => true,
            "is_project_root" => ctx.is_project_root,
            "is_project_or_module_root" => ctx.is_project_root || ctx.is_module_root,
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
            "purpose", "requirements", "domain context", "instructions",
            "conventions",
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

    /// Validate DEVELOPERS.md schema with optional validation context.
    /// v4.1: checks required sections + conditional sections (e.g., Flows at project root only).
    pub fn validate_developers_with_context(&self, developers_path: &Path, ctx: Option<&ValidationContext>) -> ValidationResult {
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
        let mut warnings = Vec::new();

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

        // Check conditional sections: warn if section exists but condition is not met
        if let Some(ctx) = ctx {
            for (name, condition) in DEVELOPERS_CONDITIONAL_SECTIONS {
                let section_found = sections.iter().find(|s| s.name.eq_ignore_ascii_case(name));
                if let Some(_section) = section_found {
                    if !Self::condition_met(condition, ctx) {
                        warnings.push(format!(
                            "'{}' section is only expected under condition '{}'. \
                             Move to project root DEVELOPERS.md or remove.",
                            name, condition
                        ));
                    }
                }
            }
        }

        // Validate Agent Observations entries (type tags + required fields)
        // Note: parse_sections splits on ALL headers including H3, so we need raw content
        if let Some(obs_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Agent Observations")) {
            if !self.is_none_marker(obs_section) {
                Self::validate_agent_observations_entries_raw(&content, obs_section.start_line, &mut warnings);
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

        if !developers_path.exists() {
            result.warnings.push(format!(
                "INV-3: DEVELOPERS.md not found at {}",
                developers_path.display()
            ));
        } else {
            // Validate DEVELOPERS.md schema, passing same ValidationContext for conditional checks
            let dev_result = self.validate_developers_with_context(&developers_path, ctx);
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
            // Propagate DEVELOPERS.md warnings (e.g., conditional section violations)
            for w in dev_result.warnings {
                result.warnings.push(format!("DEVELOPERS.md: {}", w));
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

    /// Validate Agent Observations entries from raw content.
    /// parse_sections splits on all `#` headers, so H3 entries end up as separate sections.
    /// Instead, we extract the raw block between `## Agent Observations` and the next `## `.
    fn validate_agent_observations_entries_raw(content: &str, section_start_line: usize, warnings: &mut Vec<String>) {
        static TYPE_TAG_RE: OnceLock<regex::Regex> = OnceLock::new();
        let type_tag_re = TYPE_TAG_RE.get_or_init(|| {
            regex::Regex::new(r"^\[(\w+)\]\s+.+")
                .expect("TYPE_TAG_RE is a valid hardcoded regex")
        });
        let lines: Vec<&str> = content.lines().collect();

        // Find the raw range: from section_start_line to next ## or EOF
        let start_idx = section_start_line; // 1-based, content after the ## header
        let mut end_idx = lines.len();
        for i in start_idx..lines.len() {
            let trimmed = lines[i].trim_start();
            if trimmed.starts_with("## ") {
                end_idx = i;
                break;
            }
        }

        // Parse H3 entries within the range
        let mut entries: Vec<(String, usize, Vec<String>)> = Vec::new(); // (header, line_num_1based, content_lines)
        for i in start_idx..end_idx {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("### ") {
                let header = trimmed.trim_start_matches("### ").to_string();
                entries.push((header, i + 1, Vec::new()));
            } else if let Some(entry) = entries.last_mut() {
                entry.2.push(trimmed.to_string());
            }
        }

        for (header, line_num, content_lines) in &entries {
            // Check type tag
            if let Some(caps) = type_tag_re.captures(header) {
                let entry_type = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if !AGENT_OBS_VALID_ENTRY_TYPES.iter().any(|t| t.eq_ignore_ascii_case(entry_type)) {
                    warnings.push(format!(
                        "Agent Observations: invalid entry type '{}' at line {} (valid: {:?})",
                        entry_type, line_num, AGENT_OBS_VALID_ENTRY_TYPES
                    ));
                }
            } else {
                warnings.push(format!(
                    "Agent Observations: entry at line {} missing type tag (expected ### [type] title)",
                    line_num
                ));
            }

            // Check required fields
            let content_text = content_lines.join("\n");
            for field in AGENT_OBS_REQUIRED_FIELDS {
                let field_pattern = format!("- {}:", field);
                if !content_text.contains(&field_pattern) {
                    warnings.push(format!(
                        "Agent Observations: entry '{}' at line {} missing required field '{}'",
                        header, line_num, field
                    ));
                }
            }
        }
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

    /// Converge schema (no context — backward compatible, calls with_context(None)).
    pub fn converge_schema(&self, content: &str, doc_type: &str) -> ConvergeResult {
        self.converge_schema_with_context(content, doc_type, None)
    }

    /// Converge schema with optional context for conditional section handling.
    /// - required sections: always added if missing (with None placeholder)
    /// - optional allow_none sections (e.g. Data Schemas): added if missing
    /// - conditional sections (e.g. Flows): added only when context condition is met
    pub fn converge_schema_with_context(
        &self,
        content: &str,
        doc_type: &str,
        ctx: Option<&ValidationContext>,
    ) -> ConvergeResult {
        let mut result = ConvergeResult::default();
        let mut output = content.to_string();

        // Step 1: Renames
        for &(from, to, document) in MIGRATION_RENAMES {
            if document != doc_type {
                continue;
            }
            let sections = self.parse_sections(&output);
            let has_from = sections.iter().any(|s| s.name.eq_ignore_ascii_case(from));
            let has_to = sections.iter().any(|s| s.name.eq_ignore_ascii_case(to));

            if has_from && !has_to {
                output = Self::rename_section(&output, from, to);
                result.changes.push(format!("renamed: ## {} → ## {}", from, to));
            } else if has_from && has_to {
                result.warnings.push(format!(
                    "conflict: both '## {}' and '## {}' exist — skipped rename, manual merge needed",
                    from, to
                ));
            }
        }

        // Step 2: Removals
        for &(name, document) in MIGRATION_REMOVALS {
            if document != doc_type {
                continue;
            }
            let sections = self.parse_sections(&output);
            if sections.iter().any(|s| s.name.eq_ignore_ascii_case(name)) {
                output = Self::remove_section(&output, name);
                result.changes.push(format!("removed: ## {}", name));
            }
        }

        // Step 3: Add missing required sections
        let (required_sections, allow_none_sections) = if doc_type == "developers_md" {
            (DEVELOPERS_REQUIRED_SECTIONS, DEVELOPERS_ALLOW_NONE_SECTIONS)
        } else {
            (REQUIRED_SECTIONS, ALLOW_NONE_SECTIONS)
        };

        let sections = self.parse_sections(&output);
        for required in required_sections {
            let found = sections.iter().any(|s| s.name.eq_ignore_ascii_case(required));
            if found {
                continue;
            }
            let allows_none = allow_none_sections.iter().any(|s| s.eq_ignore_ascii_case(required));
            if allows_none {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&format!("\n## {}\nNone\n", required));
                result.changes.push(format!("added: ## {} (None)", required));
            }
        }

        // Step 4: Add missing optional allow_none sections (not required, not conditional)
        // Conditional sections are handled separately in Step 5 with context check.
        if doc_type == "developers_md" {
            let sections = self.parse_sections(&output);
            for optional in DEVELOPERS_ALLOW_NONE_SECTIONS {
                if DEVELOPERS_REQUIRED_SECTIONS.iter().any(|r| r.eq_ignore_ascii_case(optional)) {
                    continue; // already handled in Step 3
                }
                // Skip conditional sections — their addition is context-dependent (Step 5)
                if DEVELOPERS_CONDITIONAL_SECTIONS.iter().any(|(name, _)| name.eq_ignore_ascii_case(optional)) {
                    continue;
                }
                // Skip agent-managed sections — only agents create these, not converge
                if DEVELOPERS_AGENT_MANAGED_SECTIONS.iter().any(|name| name.eq_ignore_ascii_case(optional)) {
                    continue;
                }
                let found = sections.iter().any(|s| s.name.eq_ignore_ascii_case(optional));
                if !found {
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                    output.push_str(&format!("\n## {}\nNone\n", optional));
                    result.changes.push(format!("added: ## {} (None)", optional));
                }
            }
        }

        // Step 5: Add conditional sections when context condition is met
        if doc_type == "developers_md" {
            if let Some(ctx) = ctx {
                let sections = self.parse_sections(&output);
                for (name, condition) in DEVELOPERS_CONDITIONAL_SECTIONS {
                    let found = sections.iter().any(|s| s.name.eq_ignore_ascii_case(name));
                    if found {
                        continue;
                    }
                    if Self::condition_met(condition, ctx) {
                        if !output.ends_with('\n') {
                            output.push('\n');
                        }
                        output.push_str(&format!("\n## {}\nNone\n", name));
                        result.changes.push(format!("added: ## {} (None, condition: {})", name, condition));
                    }
                }
            }
        }

        result.content = output;
        result
    }

    /// Rename an H2 section heading in markdown content.
    fn rename_section(content: &str, from: &str, to: &str) -> String {
        let mut output = String::with_capacity(content.len());
        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("## ") {
                let section_name = trimmed.trim_start_matches("## ").trim();
                if section_name.eq_ignore_ascii_case(from) {
                    output.push_str(&format!("## {}", to));
                    output.push('\n');
                    continue;
                }
            }
            output.push_str(line);
            output.push('\n');
        }
        // Remove trailing extra newline if original didn't end with one
        if !content.ends_with('\n') && output.ends_with('\n') {
            output.pop();
        }
        output
    }

    /// Remove an H2 section (heading + all content until next H2 or end).
    fn remove_section(content: &str, name: &str) -> String {
        let mut output = String::with_capacity(content.len());
        let mut skipping = false;
        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("## ") {
                let section_name = trimmed.trim_start_matches("## ").trim();
                if section_name.eq_ignore_ascii_case(name) {
                    skipping = true;
                    continue;
                } else {
                    skipping = false;
                }
            }
            if !skipping {
                output.push_str(line);
                output.push('\n');
            }
        }
        // Clean up double blank lines that may result from removal
        while output.contains("\n\n\n") {
            output = output.replace("\n\n\n", "\n\n");
        }
        // Remove trailing extra newline if original didn't end with one
        if !content.ends_with('\n') && output.ends_with('\n') {
            output.pop();
        }
        output
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of converge_schema operation
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConvergeResult {
    /// The converged content
    #[serde(skip)]
    pub content: String,
    /// List of changes applied (e.g., "renamed: ## X → ## Y", "removed: ## Z", "added: ## W (None)")
    pub changes: Vec<String>,
    /// Warnings (e.g., conflict: both old and new section exist)
    pub warnings: Vec<String>,
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
        // v4 schema: Requirements (not Constraints) is the always-required allow-none section
        if !content.contains("## Requirements") {
            content.push_str("\n## Requirements\nNone\n");
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
    fn test_missing_requirements_fails() {
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
            .any(|e| e.message.contains("Requirements")));
    }

    #[test]
    fn test_valid_minimal_spec() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Requirements
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
    fn test_requirements_with_content() {
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Requirements
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

## Requirements
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

## Requirements
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

## Requirements
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

## Requirements
None

## Domain Context
None

## Instructions
Always use TypeScript strict mode.

## Conventions

### Project Structure
Layered architecture.

### Module Boundaries
Each module is self-contained.

### Naming Conventions
camelCase for files.

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

        // Should add Requirements and Domain Context (always-required allow_none sections)
        assert!(added.contains(&"Requirements".to_string()));
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

## Requirements
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
        assert!(added.contains(&"Requirements".to_string()));
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

## Constraints
None

## Technical Context
None

## Decision Log
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(result.valid, "Validation failed: {:?}", result.errors);
    }

    #[test]
    fn test_developers_missing_constraints_fails() {
        let content = r#"# Test Module

## Technical Context
None

## Decision Log
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("Constraints")));
    }

    #[test]
    fn test_developers_missing_technical_context_fails() {
        let content = r#"# Test Module

## Constraints
None

## Decision Log
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("Technical Context")));
    }

    #[test]
    fn test_developers_minimal_required_sections_only() {
        // Decision Log is optional — only Constraints + Technical Context required
        let content = r#"# Test Module

## Constraints
None

## Technical Context
None
"#;
        let temp = TempDir::new().unwrap();
        let path = create_developers_file(temp.path(), content);

        let validator = SchemaValidator::new();
        let result = validator.validate_developers(&path);

        assert!(result.valid, "Minimal DEVELOPERS.md should pass: {:?}", result.errors);
    }

    #[test]
    fn test_strict_mode_missing_developers_md() {
        let content = r#"# Test Module

## Purpose
Test module.

## Requirements
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

## Requirements
None

## Domain Context
None
"#;
        let temp = TempDir::new().unwrap();
        let claude_path = temp.path().join("CLAUDE.md");
        let mut f = File::create(&claude_path).unwrap();
        write!(f, "{}", claude_content).unwrap();

        let dev_content = r#"# Test Module

## Constraints
None

## Technical Context
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

## Requirements
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

## Requirements
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
        assert!(!ctx.is_module_root);
    }

    #[test]
    fn test_evaluate_conditions_module_root() {
        let temp = TempDir::new().unwrap();
        // Create a module root marker
        File::create(temp.path().join("package.json")).unwrap();
        let ctx = SchemaValidator::evaluate_conditions(temp.path());
        assert!(!ctx.is_project_root);
        assert!(ctx.is_module_root);
    }

    #[test]
    fn test_evaluate_conditions_project_root() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        let ctx = SchemaValidator::evaluate_conditions(temp.path());
        assert!(ctx.is_project_root);
        assert!(!ctx.is_module_root);
    }

    #[test]
    fn test_evaluate_conditions_project_and_module_root() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        File::create(temp.path().join("Cargo.toml")).unwrap();
        let ctx = SchemaValidator::evaluate_conditions(temp.path());
        assert!(ctx.is_project_root);
        assert!(ctx.is_module_root);
    }

    // converge_schema tests

    #[test]
    fn test_converge_renames_constraints_to_requirements() {
        // v6-style CLAUDE.md with ## Constraints → should rename to ## Requirements
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Constraints
- Password reset limit 90 days

## Domain Context
None
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "claude_md");

        assert!(result.content.contains("## Requirements"));
        assert!(!result.content.contains("## Constraints"));
        assert!(result.content.contains("Password reset limit 90 days"));
        assert!(result.changes.iter().any(|c| c.contains("renamed")));
    }

    #[test]
    fn test_converge_idempotent_on_current_schema() {
        // Already v4-compliant CLAUDE.md → no changes
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Requirements
None

## Domain Context
None
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "claude_md");

        assert!(result.changes.is_empty(), "Expected no changes: {:?}", result.changes);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_converge_adds_missing_sections() {
        // CLAUDE.md missing Requirements and Domain Context
        let content = r#"# Test Module

## Purpose
Validates tokens.
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "claude_md");

        assert!(result.content.contains("## Requirements"));
        assert!(result.content.contains("## Domain Context"));
        assert!(result.changes.iter().any(|c| c.contains("added")));
    }

    #[test]
    fn test_converge_conflict_both_exist_warns() {
        // Both ## Constraints and ## Requirements exist → should warn, not rename
        let content = r#"# Test Module

## Purpose
Validates tokens.

## Constraints
Old constraints here.

## Requirements
New requirements here.

## Domain Context
None
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "claude_md");

        assert!(result.warnings.iter().any(|w| w.contains("conflict")),
            "Expected conflict warning: {:?}", result.warnings);
        // Both sections should still exist (unchanged)
        assert!(result.content.contains("## Constraints"));
        assert!(result.content.contains("## Requirements"));
    }

    #[test]
    fn test_converge_developers_md_renames() {
        // v6-style DEVELOPERS.md: Domain Context → Technical Context, Invariants → Constraints
        let content = r#"# Test Module

## Domain Context
Uses RS256.

## Invariants
Token must not exceed 7 days.

## Decision Log
None
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "developers_md");

        assert!(result.content.contains("## Technical Context"), "Missing Technical Context");
        assert!(result.content.contains("## Constraints"), "Missing Constraints");
        assert!(!result.content.contains("## Domain Context"), "Domain Context should be renamed");
        assert!(!result.content.contains("## Invariants"), "Invariants should be renamed");
        assert!(result.content.contains("Uses RS256."));
        assert!(result.content.contains("Token must not exceed 7 days."));
    }

    #[test]
    fn test_converge_developers_md_removes_file_map() {
        let content = r#"# Test Module

## Constraints
None

## Technical Context
None

## File Map
src/auth.ts — auth module
src/utils.ts — utilities
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "developers_md");

        assert!(!result.content.contains("## File Map"), "File Map should be removed");
        assert!(!result.content.contains("auth module"), "File Map content should be removed");
        assert!(result.changes.iter().any(|c| c.contains("removed")));
    }

    #[test]
    fn test_converge_dry_run_data() {
        // Verify ConvergeResult has correct data for dry-run JSON output
        let content = r#"# Test Module

## Purpose
Test.

## Constraints
Old data.
"#;
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "claude_md");

        // Should have rename + add changes
        assert!(!result.changes.is_empty());
        // Serializable to JSON
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("changes"));
        assert!(json.contains("warnings"));
    }

    #[test]
    fn test_converge_developers_md_adds_optional_data_schemas() {
        let content = "# Test Module\n\n## Constraints\nNone\n\n## Technical Context\nNone\n";
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "developers_md");
        assert!(
            result.content.contains("## Data Schemas"),
            "Expected Data Schemas to be added: {}", result.content
        );
        assert!(result.changes.iter().any(|c| c.contains("Data Schemas")));
    }

    #[test]
    fn test_converge_developers_md_idempotent_with_data_schemas() {
        // Content already has all required + optional (non-conditional) sections
        let content = "# Test Module\n\n## Constraints\nNone\n\n## Data Schemas\nNone\n\n## Technical Context\nNone\n\n## Decision Log\nNone\n\n## Roadmap\nNone\n";
        let validator = SchemaValidator::new();
        let result = validator.converge_schema(content, "developers_md");
        assert!(result.changes.is_empty(), "Should be idempotent: {:?}", result.changes);
    }

    #[test]
    fn test_converge_developers_md_adds_flows_at_project_root() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        let content = "# Project Root\n\n## Constraints\nNone\n\n## Technical Context\nNone\n";
        let validator = SchemaValidator::new();
        let ctx = SchemaValidator::evaluate_conditions(temp.path());
        let result = validator.converge_schema_with_context(content, "developers_md", Some(&ctx));
        assert!(
            result.content.contains("## Flows"),
            "Expected Flows to be added at project root: {}", result.content
        );
        assert!(result.changes.iter().any(|c| c.contains("Flows")));
    }

    #[test]
    fn test_converge_developers_md_no_flows_at_non_root() {
        let temp = TempDir::new().unwrap();
        // no .git → not project root
        let content = "# Module\n\n## Constraints\nNone\n\n## Technical Context\nNone\n";
        let validator = SchemaValidator::new();
        let ctx = SchemaValidator::evaluate_conditions(temp.path());
        let result = validator.converge_schema_with_context(content, "developers_md", Some(&ctx));
        assert!(
            !result.content.contains("## Flows"),
            "Flows should NOT be added at non-root: {}", result.content
        );
    }
}
