use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Result of scanning existing CLAUDE.md files
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    /// Root directory that was scanned
    pub root: PathBuf,
    /// Entries for each found CLAUDE.md
    pub entries: Vec<ClaudeMdEntry>,
}

/// Lightweight index entry for a single CLAUDE.md
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeMdEntry {
    /// Project-root-relative directory path (e.g., "src/auth")
    pub dir: PathBuf,
    /// First paragraph after ## Purpose (max 200 chars)
    pub purpose: String,
    /// Export names only (no signatures)
    pub export_names: Vec<String>,
}

use crate::EXCLUDED_DIRS;

pub struct ClaudeMdScanner {
    excluded_dirs: HashSet<String>,
}

impl ClaudeMdScanner {
    pub fn new() -> Self {
        Self {
            excluded_dirs: EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Scan for existing CLAUDE.md files and extract lightweight index
    pub fn scan(&self, root: &Path) -> ScanResult {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut entries = Vec::new();

        let walker = WalkDir::new(&root).into_iter()
            .filter_entry(|e| {
                // Prune excluded directories during traversal
                if e.file_type().is_dir() {
                    return !e.file_name()
                        .to_str()
                        .map(|n| self.excluded_dirs.contains(n))
                        .unwrap_or(false);
                }
                true
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Only process files named CLAUDE.md
            if !entry.file_type().is_file() {
                continue;
            }
            let file_name = match entry.file_name().to_str() {
                Some(name) => name,
                None => continue,
            };
            if file_name != "CLAUDE.md" {
                continue;
            }

            let file_path = entry.path();
            let dir_path = match file_path.parent() {
                Some(p) => p,
                None => continue,
            };

            let relative_dir = dir_path
                .strip_prefix(&root)
                .map(|p| p.to_path_buf())
                .unwrap_or_default();

            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: failed to read {}: {}", file_path.display(), e);
                    continue;
                }
            };

            let purpose = extract_purpose(&content);
            let export_names = extract_export_names(&content);

            entries.push(ClaudeMdEntry {
                dir: relative_dir,
                purpose,
                export_names,
            });
        }

        // Sort by dir path for consistent output
        entries.sort_by(|a, b| a.dir.cmp(&b.dir));

        ScanResult { root, entries }
    }

}

impl Default for ClaudeMdScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the first paragraph after `## Purpose`, truncated to 200 chars
fn extract_purpose(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_purpose = false;
    let mut paragraph = String::new();

    for line in &lines {
        if in_purpose {
            // Stop at next heading
            if line.starts_with("## ") {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                // If we already have content, the paragraph is done
                if !paragraph.is_empty() {
                    break;
                }
                continue;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        } else if line.starts_with("## Purpose") {
            in_purpose = true;
        }
    }

    // Truncate to 200 chars (char-based, not byte-based)
    if paragraph.chars().count() > 200 {
        let truncated: String = paragraph.chars().take(200).collect();
        format!("{}...", truncated)
    } else {
        paragraph
    }
}

/// Extract export names from `## Exports` section
/// Looks for identifiers in backtick code spans (first word before `(`, `{`, `extends`, `=`)
fn extract_export_names(content: &str) -> Vec<String> {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut in_exports = false;
    let mut names = Vec::new();

    // INTENTIONAL: Backticks required to distinguish export identifiers from prose.
    // e.g., `validateToken(...)` → matched ✓
    // e.g., "The validateToken function..." → not matched (prevents false positives)
    // Pattern: backtick-wrapped identifier at start of line content
    // e.g., `validateToken(token: string): Claims` → "validateToken"
    // e.g., `Claims { userId: string }` → "Claims"
    // e.g., `TokenError extends Error` → "TokenError"
    // e.g., `Role = "admin" | "user"` → "Role"
    let re = RE.get_or_init(|| Regex::new(r"^-?\s*`([A-Za-z_][A-Za-z0-9_]*)").unwrap());

    for line in &lines {
        if in_exports {
            if line.starts_with("## ") {
                break;
            }
            if let Some(caps) = re.captures(line.trim()) {
                if let Some(name) = caps.get(1) {
                    names.push(name.as_str().to_string());
                }
            }
        } else if line.starts_with("## Exports") {
            in_exports = true;
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_extract_purpose_basic() {
        let content = r#"# my-module

## Purpose

JWT 토큰 검증 인증 모듈

## Exports
"#;
        assert_eq!(extract_purpose(content), "JWT 토큰 검증 인증 모듈");
    }

    #[test]
    fn test_extract_purpose_multiline_paragraph() {
        let content = r#"## Purpose

This module handles user authentication
by verifying JWT tokens against the secret key.

## Exports
"#;
        assert_eq!(
            extract_purpose(content),
            "This module handles user authentication by verifying JWT tokens against the secret key."
        );
    }

    #[test]
    fn test_extract_purpose_truncation() {
        let long_text = "A".repeat(250);
        let content = format!("## Purpose\n\n{}\n\n## Exports\n", long_text);
        let result = extract_purpose(&content);
        // 200 chars of content + "..." = 203 chars
        assert_eq!(result.chars().count(), 203);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_extract_purpose_empty() {
        let content = "## Exports\n\n`foo()`\n";
        assert_eq!(extract_purpose(content), "");
    }

    #[test]
    fn test_extract_export_names_functions() {
        let content = r#"## Exports

- `validateToken(token: string): Promise<Claims>`
- `refreshToken(old: string): string`

## Behavior
"#;
        let names = extract_export_names(content);
        assert_eq!(names, vec!["validateToken", "refreshToken"]);
    }

    #[test]
    fn test_extract_export_names_types_and_classes() {
        let content = r#"## Exports

- `Claims { userId: string, role: Role }`
- `TokenError extends Error`
- `Role = "admin" | "user"`

## Behavior
"#;
        let names = extract_export_names(content);
        assert_eq!(names, vec!["Claims", "TokenError", "Role"]);
    }

    #[test]
    fn test_extract_export_names_empty() {
        let content = "## Purpose\n\nSomething\n";
        let names = extract_export_names(content);
        assert!(names.is_empty());
    }

    #[test]
    fn test_scan_finds_claude_md_files() {
        let temp = create_test_dir();

        // Create src/auth/CLAUDE.md
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        fs::write(
            auth_dir.join("CLAUDE.md"),
            r#"# auth

## Purpose

JWT 토큰 검증 인증 모듈

## Exports

- `validateToken(token: string): Claims`
- `Claims { userId: string }`

## Behavior

None
"#,
        )
        .unwrap();

        // Create src/utils/CLAUDE.md
        let utils_dir = temp.path().join("src").join("utils");
        fs::create_dir_all(&utils_dir).unwrap();
        fs::write(
            utils_dir.join("CLAUDE.md"),
            r#"# utils

## Purpose

공통 유틸리티 함수

## Exports

- `hashPassword(pw: string): string`
- `Logger extends BaseLogger`

## Behavior

None
"#,
        )
        .unwrap();

        let scanner = ClaudeMdScanner::new();
        let result = scanner.scan(temp.path());

        assert_eq!(result.entries.len(), 2);

        let auth_entry = result
            .entries
            .iter()
            .find(|e| e.dir.ends_with("auth"))
            .expect("should find auth CLAUDE.md");
        assert_eq!(auth_entry.purpose, "JWT 토큰 검증 인증 모듈");
        assert_eq!(auth_entry.export_names, vec!["validateToken", "Claims"]);

        let utils_entry = result
            .entries
            .iter()
            .find(|e| e.dir.ends_with("utils"))
            .expect("should find utils CLAUDE.md");
        assert_eq!(utils_entry.purpose, "공통 유틸리티 함수");
        assert_eq!(utils_entry.export_names, vec!["hashPassword", "Logger"]);
    }

    #[test]
    fn test_scan_excludes_node_modules() {
        let temp = create_test_dir();

        // CLAUDE.md in node_modules should be excluded
        let nm_dir = temp.path().join("node_modules").join("pkg");
        fs::create_dir_all(&nm_dir).unwrap();
        fs::write(
            nm_dir.join("CLAUDE.md"),
            "## Purpose\n\nShould be excluded\n",
        )
        .unwrap();

        // CLAUDE.md in src should be included
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            src_dir.join("CLAUDE.md"),
            "## Purpose\n\nShould be included\n\n## Exports\n\n- `foo()`\n",
        )
        .unwrap();

        let scanner = ClaudeMdScanner::new();
        let result = scanner.scan(temp.path());

        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].dir.ends_with("src"));
    }

    #[test]
    fn test_extract_purpose_korean_70_chars_not_truncated() {
        // 70 Korean chars = 210 bytes, but only 70 chars -> should NOT be truncated
        let korean_70 = "가".repeat(70);
        assert_eq!(korean_70.len(), 210); // 210 bytes
        assert_eq!(korean_70.chars().count(), 70); // 70 chars
        let content = format!("## Purpose\n\n{}\n\n## Exports\n", korean_70);
        let result = extract_purpose(&content);
        assert_eq!(result, korean_70);
        assert!(!result.ends_with("..."), "70 Korean chars should NOT be truncated");
    }

    #[test]
    fn test_extract_purpose_korean_210_chars_truncated() {
        // 210 Korean chars = 630 bytes, 210 chars -> should be truncated to 200 chars + "..."
        let korean_210 = "나".repeat(210);
        assert_eq!(korean_210.chars().count(), 210);
        let content = format!("## Purpose\n\n{}\n\n## Exports\n", korean_210);
        let result = extract_purpose(&content);
        assert!(result.ends_with("..."), "210 Korean chars should be truncated");
        // Should be 200 chars of Korean + "..." (3 chars) = 203 chars total
        assert_eq!(result.chars().count(), 203);
        // Verify the first 200 chars are the Korean text
        let expected_prefix: String = "나".repeat(200);
        assert!(result.starts_with(&expected_prefix));
    }

    #[test]
    fn test_scan_root_claude_md() {
        let temp = create_test_dir();

        // CLAUDE.md at project root
        fs::write(
            temp.path().join("CLAUDE.md"),
            "## Purpose\n\nProject root spec\n\n## Exports\n\n- `main()`\n",
        )
        .unwrap();

        let scanner = ClaudeMdScanner::new();
        let result = scanner.scan(temp.path());

        assert_eq!(result.entries.len(), 1);
        // Root dir should be empty path
        assert_eq!(result.entries[0].dir, PathBuf::from(""));
        assert_eq!(result.entries[0].purpose, "Project root spec");
        assert_eq!(result.entries[0].export_names, vec!["main"]);
    }
}
