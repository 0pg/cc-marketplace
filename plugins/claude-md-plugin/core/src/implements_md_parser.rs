//! IMPLEMENTS.md parser for extracting Module Integration Map and other sections.
//!
//! Parses the structured IMPLEMENTS.md format defined in schema-rules.yaml,
//! focusing on the Module Integration Map which provides explicit dependency
//! information at the export level.
//!
//! ## Module Integration Map format
//!
//! ```markdown
//! ## Module Integration Map
//!
//! ### `../auth` → auth/CLAUDE.md
//!
//! #### Exports Used
//! - `validateToken(token: string): Promise<Claims>` — API auth gatekeeper
//! - `Claims { userId: string, exp: number }` — auth info type
//!
//! #### Integration Context
//! Called as middleware for all protected API endpoints.
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Error types for IMPLEMENTS.md parsing
#[derive(Debug, Error)]
pub enum ImplementsParseError {
    #[error("Cannot read file '{path}': {source}")]
    FileReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// A single export used from a dependency module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportUsed {
    /// The export signature (e.g., `validateToken(token: string): Promise<Claims>`)
    pub signature: String,
    /// Optional role description (text after the em-dash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_description: Option<String>,
}

/// A single entry in the Module Integration Map.
///
/// Each entry represents one dependency module and lists which exports
/// are consumed and how they are integrated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationMapEntry {
    /// Relative path to the dependency (e.g., `../auth`)
    pub relative_path: String,
    /// Reference to the dependency's CLAUDE.md (e.g., `auth/CLAUDE.md`)
    pub claude_md_ref: String,
    /// Exports consumed from this dependency
    pub exports_used: Vec<ExportUsed>,
    /// Integration context describing how the dependency is used
    pub integration_context: String,
}

/// Complete specification parsed from IMPLEMENTS.md.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImplementsMdSpec {
    /// Module Integration Map entries
    pub module_integration_map: Vec<IntegrationMapEntry>,
    /// External dependencies listed in the External Dependencies section
    pub external_dependencies: Vec<String>,
    /// Validation errors (fatal format issues found during parsing)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    /// Validation warnings (non-fatal issues found during parsing)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// IMPLEMENTS.md Parser
///
/// Uses regex-based section extraction following the same patterns
/// as `ClaudeMdParser`. Focuses on the Module Integration Map section
/// for dependency graph enrichment.
pub struct ImplementsMdParser {
    /// Matches markdown headers: ## Section, ### Entry, #### Subsection
    section_pattern: Regex,
    /// Matches integration map entry headers:
    /// ### `../auth` → auth/CLAUDE.md
    entry_header_pattern: Regex,
    /// Matches export items:
    /// - `validateToken(token: string): Promise<Claims>` — description
    export_item_pattern: Regex,
}

impl ImplementsMdParser {
    pub fn new() -> Self {
        Self {
            // Match markdown headers: ## Section, ### Entry, #### Subsection
            section_pattern: Regex::new(r"^(#{1,4})\s+(.+)$")
                .expect("Failed to compile section_pattern regex"),
            // Match entry header name (after ### prefix is stripped by extract_sections):
            // `relative_path` → name/CLAUDE.md
            // SSOT: schema-rules.yaml specifies → (unicode arrow) only
            entry_header_pattern: Regex::new(
                r"^`([^`]+)`\s*→\s*(.+/CLAUDE\.md)\s*$",
            )
            .expect("Failed to compile entry_header_pattern regex"),
            // Match export item: - `signature` — role_description
            // SSOT: schema-rules.yaml specifies — (em-dash) only; role is optional
            export_item_pattern: Regex::new(r"^[-*]\s+`([^`]+)`(?:\s*—\s*(.+))?$")
                .expect("Failed to compile export_item_pattern regex"),
        }
    }

    /// Parse an IMPLEMENTS.md file from disk.
    pub fn parse(&self, file: &Path) -> Result<ImplementsMdSpec, ImplementsParseError> {
        let content =
            std::fs::read_to_string(file).map_err(|e| ImplementsParseError::FileReadError {
                path: file.to_string_lossy().to_string(),
                source: e,
            })?;

        Ok(self.parse_content(&content))
    }

    /// Parse IMPLEMENTS.md content directly from a string.
    pub fn parse_content(&self, content: &str) -> ImplementsMdSpec {
        let mut spec = ImplementsMdSpec::default();
        let sections = self.extract_sections(content);

        // Parse Module Integration Map
        self.parse_integration_map(&sections, &mut spec);

        // Parse External Dependencies
        self.parse_external_dependencies(&sections, &mut spec);

        spec
    }

    /// Extract sections from markdown content (same approach as ClaudeMdParser).
    fn extract_sections(&self, content: &str) -> Vec<Section> {
        let mut sections = Vec::new();
        let mut current_section: Option<Section> = None;

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = self.section_pattern.captures(line) {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let level = caps.get(1).map(|m| m.as_str().len()).unwrap_or(1);
                let name = caps
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                current_section = Some(Section {
                    name,
                    level,
                    line_number: line_num + 1,
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

    /// Parse the Module Integration Map section.
    ///
    /// Structure expected:
    /// ```markdown
    /// ## Module Integration Map
    ///
    /// ### `../auth` → auth/CLAUDE.md
    ///
    /// #### Exports Used
    /// - `validateToken(token: string): Promise<Claims>` — description
    ///
    /// #### Integration Context
    /// Free-form text describing how the dependency is used.
    /// ```
    fn parse_integration_map(&self, sections: &[Section], spec: &mut ImplementsMdSpec) {
        // Find the "Module Integration Map" H2 section
        let map_section_idx = match sections
            .iter()
            .position(|s| s.level == 2 && s.name.eq_ignore_ascii_case("Module Integration Map"))
        {
            Some(idx) => idx,
            None => return, // Section not present; that's OK
        };

        // Iterate over H3 entries that follow the H2 header
        let mut current_entry: Option<IntegrationMapEntry> = None;
        let mut context_lines: Vec<String> = Vec::new();

        for section in sections.iter().skip(map_section_idx + 1) {
            // Stop when we hit the next H2 section (different major section)
            if section.level <= 2 {
                break;
            }

            // H3 = new integration map entry
            if section.level == 3 {
                // Save previous entry if any
                if let Some(mut entry) = current_entry.take() {
                    entry.integration_context = context_lines.join("\n").trim().to_string();
                    if !entry.exports_used.is_empty() {
                        spec.module_integration_map.push(entry);
                    } else {
                        spec.warnings.push(format!(
                            "Integration map entry '{}' has no exports used",
                            entry.relative_path
                        ));
                    }
                }

                // Reset state
                context_lines.clear();

                // Try to match the entry header pattern
                if let Some(caps) = self.entry_header_pattern.captures(&section.name) {
                    let relative_path = caps
                        .get(1)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    let claude_md_ref = caps
                        .get(2)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default();

                    current_entry = Some(IntegrationMapEntry {
                        relative_path,
                        claude_md_ref,
                        exports_used: Vec::new(),
                        integration_context: String::new(),
                    });
                } else {
                    spec.errors.push(format!(
                        "Line {}: H3 header '{}' does not match integration map entry pattern (`path` → name/CLAUDE.md)",
                        section.line_number, section.name
                    ));
                    continue;
                }

                // Parse content lines within the H3 section (before any H4)
                // These are typically empty or descriptive, so we skip them.
                continue;
            }

            // H4 = subsection within an entry
            if section.level == 4 {
                let name_lower = section.name.to_lowercase();

                if name_lower == "exports used" {
                    // Parse export items from this subsection's content
                    if let Some(ref mut entry) = current_entry {
                        for line in &section.content {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }

                            if let Some(caps) = self.export_item_pattern.captures(trimmed) {
                                let signature = caps
                                    .get(1)
                                    .map(|m| m.as_str().to_string())
                                    .unwrap_or_default();
                                let role_description = caps
                                    .get(2)
                                    .map(|m| m.as_str().trim().to_string())
                                    .filter(|s| !s.is_empty());

                                entry.exports_used.push(ExportUsed {
                                    signature,
                                    role_description,
                                });
                            } else {
                                spec.warnings.push(format!(
                                    "Export item does not match expected pattern: '{}'",
                                    trimmed
                                ));
                            }
                        }
                    }
                } else if name_lower == "integration context" {
                    context_lines.clear();

                    // Collect context content
                    for line in &section.content {
                        context_lines.push(line.to_string());
                    }
                } else {
                    // Unknown H4 subsection - ignore
                }
            }
        }

        // Save the last entry
        if let Some(mut entry) = current_entry.take() {
            entry.integration_context = context_lines.join("\n").trim().to_string();
            if !entry.exports_used.is_empty() {
                spec.module_integration_map.push(entry);
            } else {
                spec.warnings.push(format!(
                    "Integration map entry '{}' has no exports used",
                    entry.relative_path
                ));
            }
        }
    }

    /// Parse the External Dependencies section.
    ///
    /// Extracts dependency names from a simple list format:
    /// ```markdown
    /// ## External Dependencies
    /// - jsonwebtoken@9.0.0 — JWT signing/verification
    /// - express@4.18 — HTTP framework
    /// ```
    fn parse_external_dependencies(&self, sections: &[Section], spec: &mut ImplementsMdSpec) {
        let ext_deps_section = match sections.iter().find(|s| {
            s.level == 2 && s.name.eq_ignore_ascii_case("External Dependencies")
        }) {
            Some(s) => s,
            None => return,
        };

        for line in &ext_deps_section.content {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check for none marker
            let lower = trimmed.to_lowercase();
            if lower == "none" || lower == "n/a" {
                return;
            }

            // Strip leading bullet
            let value = trimmed
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim();

            if !value.is_empty() {
                spec.external_dependencies.push(value.to_string());
            }
        }
    }
}

impl Default for ImplementsMdParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal section representation (mirrors ClaudeMdParser's Section)
struct Section {
    name: String,
    level: usize,
    line_number: usize,
    content: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_integration_map() {
        let parser = ImplementsMdParser::new();
        let content = r#"# api/IMPLEMENTS.md

## Module Integration Map

### `../auth` → auth/CLAUDE.md

#### Exports Used
- `validateToken(token: string): Promise<Claims>` — API 요청 인증 게이트키퍼
- `Claims { userId: string, exp: number }` — 인증 정보 타입

#### Integration Context
모든 보호된 API 엔드포인트에서 미들웨어로 호출.

## External Dependencies
- jsonwebtoken@9.0.0 — JWT signing/verification
"#;

        let spec = parser.parse_content(content);
        assert_eq!(spec.module_integration_map.len(), 1);

        let entry = &spec.module_integration_map[0];
        assert_eq!(entry.relative_path, "../auth");
        assert_eq!(entry.claude_md_ref, "auth/CLAUDE.md");
        assert_eq!(entry.exports_used.len(), 2);
        assert_eq!(
            entry.exports_used[0].signature,
            "validateToken(token: string): Promise<Claims>"
        );
        assert_eq!(
            entry.exports_used[0].role_description.as_deref(),
            Some("API 요청 인증 게이트키퍼")
        );
        assert_eq!(
            entry.exports_used[1].signature,
            "Claims { userId: string, exp: number }"
        );
        assert!(entry
            .integration_context
            .contains("모든 보호된 API 엔드포인트에서 미들웨어로 호출"));

        assert_eq!(spec.external_dependencies.len(), 1);
        assert!(spec.external_dependencies[0].contains("jsonwebtoken"));
    }

    #[test]
    fn test_parse_multiple_entries() {
        let parser = ImplementsMdParser::new();
        let content = r#"# gateway/IMPLEMENTS.md

## Module Integration Map

### `../auth` → auth/CLAUDE.md

#### Exports Used
- `validateToken(token: string): Promise<Claims>` — 인증

#### Integration Context
미들웨어에서 사용.

### `../db` → db/CLAUDE.md

#### Exports Used
- `getConnection(): Connection` — DB 커넥션 풀
- `query(sql: string, params: any[]): Promise<Result>` — 쿼리 실행

#### Integration Context
모든 데이터 접근 시 사용.
"#;

        let spec = parser.parse_content(content);
        assert_eq!(spec.module_integration_map.len(), 2);
        assert_eq!(spec.module_integration_map[0].relative_path, "../auth");
        assert_eq!(spec.module_integration_map[0].exports_used.len(), 1);
        assert_eq!(spec.module_integration_map[1].relative_path, "../db");
        assert_eq!(spec.module_integration_map[1].exports_used.len(), 2);
    }

    #[test]
    fn test_parse_no_integration_map() {
        let parser = ImplementsMdParser::new();
        let content = r#"# module/IMPLEMENTS.md

## Architecture Decisions
Use event-driven approach.

## External Dependencies
None
"#;

        let spec = parser.parse_content(content);
        assert!(spec.module_integration_map.is_empty());
        assert!(spec.external_dependencies.is_empty());
    }

    #[test]
    fn test_ascii_arrow_rejected() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## Module Integration Map

### `../utils` -> utils/CLAUDE.md

#### Exports Used
- `formatDate(date: Date): string` -- 날짜 포맷팅

#### Integration Context
UI 표시용 날짜 변환.
"#;

        let spec = parser.parse_content(content);
        // ASCII arrow (->) is not SSOT-compliant; must use → (unicode)
        assert!(spec.module_integration_map.is_empty());
        assert!(!spec.errors.is_empty());
        assert!(spec.errors[0].contains("does not match"));
    }

    #[test]
    fn test_parse_export_without_role() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## Module Integration Map

### `../lib` → lib/CLAUDE.md

#### Exports Used
- `helperFn(): void`

#### Integration Context
Internal helper usage.
"#;

        let spec = parser.parse_content(content);
        assert_eq!(spec.module_integration_map.len(), 1);

        let export = &spec.module_integration_map[0].exports_used[0];
        assert_eq!(export.signature, "helperFn(): void");
        assert!(export.role_description.is_none());
    }

    #[test]
    fn test_parse_external_deps_none_marker() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## External Dependencies
None
"#;

        let spec = parser.parse_content(content);
        assert!(spec.external_dependencies.is_empty());
    }

    #[test]
    fn test_parse_multiline_integration_context() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## Module Integration Map

### `../auth` → auth/CLAUDE.md

#### Exports Used
- `validateToken(token: string): Promise<Claims>` — 인증

#### Integration Context
첫 번째 줄: 미들웨어로 사용.
두 번째 줄: 모든 API 엔드포인트에 적용.
세 번째 줄: 실패 시 401 반환.
"#;

        let spec = parser.parse_content(content);
        assert_eq!(spec.module_integration_map.len(), 1);

        let context = &spec.module_integration_map[0].integration_context;
        assert!(context.contains("첫 번째 줄"));
        assert!(context.contains("두 번째 줄"));
        assert!(context.contains("세 번째 줄"));
    }

    #[test]
    fn test_errors_on_invalid_entry_header() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## Module Integration Map

### Invalid Header Format

#### Exports Used
- `something(): void`

#### Integration Context
Some context.
"#;

        let spec = parser.parse_content(content);
        assert!(spec.module_integration_map.is_empty());
        assert!(!spec.errors.is_empty());
        assert!(spec.errors[0].contains("does not match"));
    }

    #[test]
    fn test_warnings_on_empty_exports() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## Module Integration Map

### `../auth` → auth/CLAUDE.md

#### Exports Used

#### Integration Context
Some context.
"#;

        let spec = parser.parse_content(content);
        assert!(spec.module_integration_map.is_empty());
        assert!(!spec.warnings.is_empty());
        assert!(spec.warnings[0].contains("no exports used"));
    }

    #[test]
    fn test_parse_integration_map_stops_at_next_h2() {
        let parser = ImplementsMdParser::new();
        let content = r#"# test/IMPLEMENTS.md

## Module Integration Map

### `../auth` → auth/CLAUDE.md

#### Exports Used
- `validateToken(token: string): Promise<Claims>` — 인증

#### Integration Context
미들웨어 사용.

## Implementation Approach
Some other section that should not be parsed as integration map.
"#;

        let spec = parser.parse_content(content);
        assert_eq!(spec.module_integration_map.len(), 1);
        assert_eq!(spec.module_integration_map[0].relative_path, "../auth");
    }
}
