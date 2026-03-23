use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

// Include generated constants from schema-rules.yaml (SSOT)
include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));

/// Error types for CLAUDE.md parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Cannot read file '{path}': {source}")]
    FileReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Missing required section: {section}")]
    MissingRequiredSection { section: String },
    #[error("Invalid section format in '{section}': {details}")]
    InvalidSectionFormat { section: String, details: String },
}

/// Complete specification parsed from CLAUDE.md (v3 schema)
///
/// Compact pre-learning index: Purpose, Constraints, Domain Context, Instructions.
/// Exports/Behavior/Contract/Protocol moved to .claude/index.md (auto-generated).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeMdSpec {
    /// Module name (from H1 header)
    pub name: String,
    /// Purpose description
    pub purpose: String,
    /// Constraints: rules the code must follow (bullet list, None if "None")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<String>>,
    /// Domain Context: key context summary (raw text, None if "None")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_context: Option<String>,
    /// Instructions: AI behavior directives (project root only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Validation warnings (non-fatal issues found during parsing)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// CLAUDE.md Parser (v3 schema)
pub struct ClaudeMdParser {
    section_pattern: Regex,
}

impl ClaudeMdParser {
    pub fn new() -> Self {
        Self {
            // Match markdown headers: ## Purpose, ### Functions
            section_pattern: Regex::new(r"^(#{1,4})\s+(.+)$").unwrap_or_else(|_| Regex::new(r".^").unwrap()),
        }
    }

    /// Parse a CLAUDE.md file
    pub fn parse(&self, file: &Path) -> Result<ClaudeMdSpec, ParseError> {
        let content = std::fs::read_to_string(file).map_err(|e| ParseError::FileReadError {
            path: file.to_string_lossy().to_string(),
            source: e,
        })?;

        self.parse_content(&content)
    }

    /// Parse CLAUDE.md content directly
    /// Returns Err immediately if any required section is missing.
    /// Required sections are defined in schema-rules.yaml (SSOT).
    pub fn parse_content(&self, content: &str) -> Result<ClaudeMdSpec, ParseError> {
        let mut spec = ClaudeMdSpec::default();
        let sections = self.extract_sections(content);

        // Extract module name from first H1 header
        for section in &sections {
            if section.level == 1 {
                spec.name = section.name.clone();
                break;
            }
        }

        // Check all required sections exist (from SSOT) - FAIL FAST
        for required in REQUIRED_SECTIONS {
            let section_found = sections.iter().find(|s| s.name.eq_ignore_ascii_case(required));

            match section_found {
                None => {
                    return Err(ParseError::MissingRequiredSection {
                        section: required.to_string(),
                    });
                }
                Some(section) => {
                    // For sections that allow "None", check if it's a valid None marker
                    let allows_none = ALLOW_NONE_SECTIONS
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(required));
                    let is_none_marker = self.is_none_marker(section);

                    // If section doesn't allow None but has None marker, that's an error
                    if !allows_none && is_none_marker {
                        return Err(ParseError::InvalidSectionFormat {
                            section: required.to_string(),
                            details: format!("Section '{}' does not allow 'None' as value", required),
                        });
                    }
                }
            }
        }

        // Parse Purpose section
        if let Some(purpose_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Purpose")) {
            spec.purpose = purpose_section.content.join("\n").trim().to_string();
        }

        // Parse Constraints section
        if let Some(constraints_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Constraints")) {
            if !self.is_none_marker(constraints_section) {
                spec.constraints = Some(self.parse_bullet_list(&constraints_section.content));
            }
        }

        // Parse Domain Context section
        if let Some(dc_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Domain Context")) {
            if !self.is_none_marker(dc_section) {
                let text = dc_section.content.join("\n").trim().to_string();
                if !text.is_empty() {
                    spec.domain_context = Some(text);
                }
            }
        }

        // Parse Instructions section (optional, project root only)
        if let Some(instructions_section) = sections.iter().find(|s| s.name.eq_ignore_ascii_case("Instructions")) {
            let text = instructions_section.content.join("\n").trim().to_string();
            if !text.is_empty() {
                spec.instructions = Some(text);
            }
        }

        // Warn about unrecognized sections
        let known_sections = [
            "purpose", "constraints", "domain context", "instructions",
            "project convention", "code convention",
        ];
        for section in &sections {
            if section.level == 2 {
                let name_lower = section.name.to_lowercase();
                if !known_sections.contains(&name_lower.as_str()) {
                    spec.warnings.push(format!("Unrecognized section: {}", section.name));
                }
            }
        }

        Ok(spec)
    }

    /// Check if a section contains only a "None" marker (None, N/A, etc.)
    fn is_none_marker(&self, section: &ParserSection) -> bool {
        let lines: Vec<&str> = section.content.iter().map(|s| s.as_str()).collect();
        crate::is_none_marker_content(&lines)
    }

    fn extract_sections(&self, content: &str) -> Vec<ParserSection> {
        let mut sections = Vec::new();
        let mut current_section: Option<ParserSection> = None;

        for line in content.lines() {
            if let Some(caps) = self.section_pattern.captures(line) {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let level = caps.get(1).map(|m| m.as_str().len()).unwrap_or(1);
                let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

                current_section = Some(ParserSection {
                    name,
                    level,
                    content: Vec::new(),
                });
            } else if let Some(ref mut section) = current_section {
                section.content.push(line.to_string());
            }
        }

        if let Some(section) = current_section {
            sections.push(section);
        }

        sections
    }

    /// Parse bullet list content into Vec<String>
    fn parse_bullet_list(&self, content: &[String]) -> Vec<String> {
        let mut items = Vec::new();
        for line in content {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Strip bullet markers
            let stripped = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .or_else(|| trimmed.strip_prefix("+ "))
                .unwrap_or(trimmed);
            if !stripped.is_empty() && !stripped.eq_ignore_ascii_case("none") && !stripped.eq_ignore_ascii_case("n/a") {
                items.push(stripped.to_string());
            }
        }
        items
    }
}

impl Default for ClaudeMdParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Section representation for CLAUDE.md parsing, storing heading level and raw content lines.
struct ParserSection {
    name: String,
    level: usize,
    content: Vec<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: Returns minimal always-required sections with allow-none as None
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
    fn test_parse_purpose() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test-module

## Purpose
Handles user authentication.
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert_eq!(spec.purpose, "Handles user authentication.");
    }

    #[test]
    fn test_parse_constraints() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Constraints
- 비밀번호 재설정 90일 제한
- 동시 세션 최대 5개

## Domain Context
None
"#;
        let spec = parser.parse_content(content).unwrap();
        let constraints = spec.constraints.expect("constraints should be Some");
        assert_eq!(constraints.len(), 2);
        assert!(constraints[0].contains("90일"));
        assert!(constraints[1].contains("5개"));
    }

    #[test]
    fn test_parse_constraints_none() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let spec = parser.parse_content(content).unwrap();
        assert!(spec.constraints.is_none());
    }

    #[test]
    fn test_parse_domain_context() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Constraints
None

## Domain Context
JWT 토큰은 PCI-DSS 준수를 위해 7일 만료.
Redis 캐시 사용으로 인증 지연 최소화.
"#;
        let spec = parser.parse_content(content).unwrap();
        let dc = spec.domain_context.expect("domain_context should be Some");
        assert!(dc.contains("PCI-DSS"));
        assert!(dc.contains("Redis"));
    }

    #[test]
    fn test_parse_domain_context_none() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Purpose
Test module.
"#,
        );
        let spec = parser.parse_content(&content).unwrap();
        assert!(spec.domain_context.is_none());
    }

    #[test]
    fn test_parse_instructions() {
        let parser = ClaudeMdParser::new();
        let content = r#"# my-project

## Purpose
My project root.

## Constraints
None

## Domain Context
None

## Instructions
Always use TypeScript strict mode.
Follow the team's code review process.
"#;
        let spec = parser.parse_content(content).unwrap();
        let instructions = spec.instructions.expect("instructions should be Some");
        assert!(instructions.contains("TypeScript strict mode"));
    }

    #[test]
    fn test_fail_fast_missing_purpose() {
        let parser = ClaudeMdParser::new();
        let content = with_required_sections(
            r#"# test

## Constraints
- some rule
"#,
        );
        let result = parser.parse_content(&content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Purpose"));
    }

    #[test]
    fn test_fail_fast_missing_constraints() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Domain Context
None
"#;
        let result = parser.parse_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Constraints"));
    }

    #[test]
    fn test_fail_fast_missing_domain_context() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Constraints
None
"#;
        let result = parser.parse_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::MissingRequiredSection { section } if section == "Domain Context"));
    }

    #[test]
    fn test_purpose_does_not_allow_none() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
None

## Constraints
None

## Domain Context
None
"#;
        let result = parser.parse_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::InvalidSectionFormat { .. }));
    }

    #[test]
    fn test_unrecognized_section_warning() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Constraints
None

## Domain Context
None

## Exports
- `foo(): void`
"#;
        let spec = parser.parse_content(content).unwrap();
        assert!(!spec.warnings.is_empty());
        assert!(spec.warnings.iter().any(|w| w.contains("Exports")));
    }

    #[test]
    fn test_minimal_valid_spec() {
        let parser = ClaudeMdParser::new();
        let content = r#"# test

## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let spec = parser.parse_content(content).unwrap();
        assert_eq!(spec.name, "test");
        assert_eq!(spec.purpose, "Test module.");
        assert!(spec.constraints.is_none());
        assert!(spec.domain_context.is_none());
        assert!(spec.instructions.is_none());
    }
}
