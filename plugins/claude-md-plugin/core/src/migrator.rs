use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of a migration operation
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationResult {
    /// File that was migrated
    pub file: String,
    /// Whether migration was performed
    pub migrated: bool,
    /// Original schema version (None if no marker found)
    pub from_version: Option<String>,
    /// Target schema version
    pub to_version: String,
    /// Changes made
    pub changes: Vec<MigrationChange>,
    /// Suggestions for manual review
    pub suggestions: Vec<String>,
}

/// A single migration change
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationChange {
    /// Type of change
    pub change_type: String,
    /// Description
    pub description: String,
}

/// Migrates CLAUDE.md files from v1 to v2 format
pub struct Migrator {
    schema_version_pattern: Regex,
}

impl Migrator {
    pub fn new() -> Self {
        Self {
            schema_version_pattern: Regex::new(r"^<!--\s*schema:\s*(\d+\.\d+)\s*-->").unwrap(),
        }
    }

    /// Migrate a CLAUDE.md file to v2 format
    pub fn migrate(&self, file: &Path, dry_run: bool) -> MigrationResult {
        let file_str = file.to_string_lossy().to_string();

        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                return MigrationResult {
                    file: file_str,
                    migrated: false,
                    from_version: None,
                    to_version: "2.0".to_string(),
                    changes: vec![],
                    suggestions: vec![format!("Cannot read file: {}", e)],
                };
            }
        };

        // Detect current version
        let from_version = self.detect_version(&content);

        // Already v2
        if from_version.as_deref() == Some("2.0") {
            return MigrationResult {
                file: file_str,
                migrated: false,
                from_version,
                to_version: "2.0".to_string(),
                changes: vec![],
                suggestions: vec!["Already at schema version 2.0".to_string()],
            };
        }

        let mut changes = Vec::new();
        let mut suggestions = Vec::new();
        let mut migrated_content = content.clone();

        // Step 1: Add schema version marker
        if from_version.is_none() {
            migrated_content = format!("<!-- schema: 2.0 -->\n{}", migrated_content);
            changes.push(MigrationChange {
                change_type: "AddVersionMarker".to_string(),
                description: "Added <!-- schema: 2.0 --> marker at top of file".to_string(),
            });
        }

        // Step 2: Convert bullet exports to heading format
        let (converted_content, export_changes) = self.convert_exports_to_headings(&migrated_content);
        if !export_changes.is_empty() {
            migrated_content = converted_content;
            changes.extend(export_changes);
        }

        // Step 3: Suggest Actor/UC structure
        if self.has_behavior_section(&migrated_content) && !self.has_actors_section(&migrated_content) {
            suggestions.push(
                "Consider adding ### Actors and ### UC-N sections to the Behavior section for UseCase diagram support".to_string()
            );
        }

        // Write if not dry-run
        if !dry_run && !changes.is_empty() {
            if let Err(e) = std::fs::write(file, &migrated_content) {
                suggestions.push(format!("Failed to write migrated file: {}", e));
                return MigrationResult {
                    file: file_str,
                    migrated: false,
                    from_version,
                    to_version: "2.0".to_string(),
                    changes,
                    suggestions,
                };
            }
        }

        MigrationResult {
            file: file_str,
            migrated: !changes.is_empty() && !dry_run,
            from_version,
            to_version: "2.0".to_string(),
            changes,
            suggestions,
        }
    }

    /// Migrate all CLAUDE.md files under a root directory
    pub fn migrate_all(&self, root: &Path, dry_run: bool) -> Vec<MigrationResult> {
        let mut results = Vec::new();
        self.scan_and_migrate(root, root, dry_run, &mut results);
        results
    }

    // --- Private helpers ---

    fn detect_version(&self, content: &str) -> Option<String> {
        for line in content.lines().take(5) {
            if let Some(caps) = self.schema_version_pattern.captures(line) {
                return caps.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    /// Convert bullet-style exports to heading-style
    /// `- \`validateToken(token: string): Claims\`` → `#### validateToken` + signature line
    fn convert_exports_to_headings(&self, content: &str) -> (String, Vec<MigrationChange>) {
        let mut changes = Vec::new();
        let mut result_lines = Vec::new();
        let mut in_exports = false;
        let mut in_export_subsection = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Track if we're in the Exports section
            if trimmed.starts_with("## ") {
                in_exports = trimmed.eq_ignore_ascii_case("## Exports");
                in_export_subsection = false;
                result_lines.push(line.to_string());
                continue;
            }

            // Track export subsections (### Functions, ### Types, etc.)
            if in_exports && trimmed.starts_with("### ") {
                in_export_subsection = true;
                result_lines.push(line.to_string());
                continue;
            }

            // Convert bullet exports to heading format
            if in_exports && in_export_subsection && trimmed.starts_with("- `") {
                // Extract function/type name from backtick content
                if let Some(name) = self.extract_export_name(trimmed) {
                    let signature = trimmed
                        .trim_start_matches("- ")
                        .trim_start_matches("* ")
                        .to_string();

                    result_lines.push(String::new());
                    result_lines.push(format!("#### {}", name));
                    result_lines.push(signature);

                    changes.push(MigrationChange {
                        change_type: "ConvertExportToHeading".to_string(),
                        description: format!("Converted '{}' from bullet to heading format", name),
                    });
                    continue;
                }
            }

            result_lines.push(line.to_string());
        }

        (result_lines.join("\n"), changes)
    }

    fn extract_export_name(&self, line: &str) -> Option<String> {
        // Match patterns like: - `Name(...` or - `Name { ...`
        let name_pattern = Regex::new(r"[`]([A-Za-z_][A-Za-z0-9_]*)[\s({\[]").unwrap();
        // Also match simple type definitions: - `Name` or - `Name = ...`
        let simple_pattern = Regex::new(r"[`]([A-Za-z_][A-Za-z0-9_]*)[`\s=]").unwrap();

        name_pattern.captures(line)
            .or_else(|| simple_pattern.captures(line))
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    }

    fn has_behavior_section(&self, content: &str) -> bool {
        content.lines().any(|l| l.trim().eq_ignore_ascii_case("## Behavior"))
    }

    fn has_actors_section(&self, content: &str) -> bool {
        content.lines().any(|l| l.trim().eq_ignore_ascii_case("### Actors"))
    }

    fn scan_and_migrate(
        &self,
        dir: &Path,
        _root: &Path,
        dry_run: bool,
        results: &mut Vec<MigrationResult>,
    ) {
        let claude_md = dir.join("CLAUDE.md");
        if claude_md.exists() {
            results.push(self.migrate(&claude_md, dry_run));
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name.starts_with('.') || name == "node_modules" || name == "target"
                        || name == "build" || name == "dist" || name == "__pycache__"
                    {
                        continue;
                    }
                    self.scan_and_migrate(&path, _root, dry_run, results);
                }
            }
        }
    }
}

impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_claude_md(dir: &Path, content: &str) -> std::path::PathBuf {
        let path = dir.join("CLAUDE.md");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_migrate_adds_version_marker() {
        let tmp = TempDir::new().unwrap();
        let content = r#"# auth

## Purpose
Auth module.

## Summary
Module summary.

## Exports

### Functions
- `validateToken(token: string): Claims`

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let path = create_claude_md(tmp.path(), content);
        let migrator = Migrator::new();
        let result = migrator.migrate(&path, false);

        assert!(result.migrated);
        assert!(result.changes.iter().any(|c| c.change_type == "AddVersionMarker"));

        let migrated = fs::read_to_string(&path).unwrap();
        assert!(migrated.starts_with("<!-- schema: 2.0 -->"));
    }

    #[test]
    fn test_migrate_converts_exports_to_headings() {
        let tmp = TempDir::new().unwrap();
        let content = r#"# auth

## Purpose
Auth module.

## Summary
Module summary.

## Exports

### Functions
- `validateToken(token: string): Claims`
- `issueToken(userId: string): string`

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let path = create_claude_md(tmp.path(), content);
        let migrator = Migrator::new();
        let result = migrator.migrate(&path, false);

        assert!(result.migrated);
        assert!(result.changes.iter().any(|c| c.change_type == "ConvertExportToHeading"));

        let migrated = fs::read_to_string(&path).unwrap();
        assert!(migrated.contains("#### validateToken"));
        assert!(migrated.contains("#### issueToken"));
    }

    #[test]
    fn test_migrate_dry_run_no_write() {
        let tmp = TempDir::new().unwrap();
        let original = r#"# auth

## Purpose
Auth module.

## Summary
Module summary.

## Exports
None

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let path = create_claude_md(tmp.path(), original);
        let migrator = Migrator::new();
        let result = migrator.migrate(&path, true);

        assert!(!result.migrated); // dry_run = true → not actually migrated
        assert!(!result.changes.is_empty()); // but changes were identified

        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, original); // file unchanged
    }

    #[test]
    fn test_already_v2_skips() {
        let tmp = TempDir::new().unwrap();
        let content = r#"<!-- schema: 2.0 -->
# auth

## Purpose
Auth module.

## Summary
Module summary.

## Exports
None

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let path = create_claude_md(tmp.path(), content);
        let migrator = Migrator::new();
        let result = migrator.migrate(&path, false);

        assert!(!result.migrated);
        assert!(result.suggestions.iter().any(|s| s.contains("Already at schema version 2.0")));
    }

    #[test]
    fn test_suggests_actors_for_behavior() {
        let tmp = TempDir::new().unwrap();
        let content = r#"# auth

## Purpose
Auth module.

## Summary
Module summary.

## Exports
None

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
None
"#;
        let path = create_claude_md(tmp.path(), content);
        let migrator = Migrator::new();
        let result = migrator.migrate(&path, true);

        assert!(result.suggestions.iter().any(|s| s.contains("Actors")));
    }
}
