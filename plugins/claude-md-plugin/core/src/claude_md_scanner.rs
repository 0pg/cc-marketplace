use serde::{Deserialize, Serialize};
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

/// Lightweight index entry for a single CLAUDE.md (v3 schema)
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeMdEntry {
    /// Project-root-relative directory path (e.g., "src/auth")
    pub dir: PathBuf,
    /// First paragraph after ## Purpose (max 200 chars)
    pub purpose: String,
}

use crate::EXCLUDED_DIRS;
use std::collections::HashSet;

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

            entries.push(ClaudeMdEntry {
                dir: relative_dir,
                purpose,
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

## Requirements
"#;
        assert_eq!(extract_purpose(content), "JWT 토큰 검증 인증 모듈");
    }

    #[test]
    fn test_extract_purpose_multiline_paragraph() {
        let content = r#"## Purpose

This module handles user authentication
by verifying JWT tokens against the secret key.

## Requirements
"#;
        assert_eq!(
            extract_purpose(content),
            "This module handles user authentication by verifying JWT tokens against the secret key."
        );
    }

    #[test]
    fn test_extract_purpose_truncation() {
        let long_text = "A".repeat(250);
        let content = format!("## Purpose\n\n{}\n\n## Requirements\n", long_text);
        let result = extract_purpose(&content);
        assert_eq!(result.chars().count(), 203);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_extract_purpose_empty() {
        let content = "## Requirements\n\n- some rule\n";
        assert_eq!(extract_purpose(content), "");
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

## Requirements
None

## Domain Context
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

## Requirements
None

## Domain Context
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

        let utils_entry = result
            .entries
            .iter()
            .find(|e| e.dir.ends_with("utils"))
            .expect("should find utils CLAUDE.md");
        assert_eq!(utils_entry.purpose, "공통 유틸리티 함수");
    }

    #[test]
    fn test_scan_excludes_node_modules() {
        let temp = create_test_dir();

        let nm_dir = temp.path().join("node_modules").join("pkg");
        fs::create_dir_all(&nm_dir).unwrap();
        fs::write(
            nm_dir.join("CLAUDE.md"),
            "## Purpose\n\nShould be excluded\n",
        )
        .unwrap();

        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            src_dir.join("CLAUDE.md"),
            "## Purpose\n\nShould be included\n\n## Requirements\nNone\n",
        )
        .unwrap();

        let scanner = ClaudeMdScanner::new();
        let result = scanner.scan(temp.path());

        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].dir.ends_with("src"));
    }

    #[test]
    fn test_extract_purpose_korean_70_chars_not_truncated() {
        let korean_70 = "가".repeat(70);
        assert_eq!(korean_70.chars().count(), 70);
        let content = format!("## Purpose\n\n{}\n\n## Requirements\n", korean_70);
        let result = extract_purpose(&content);
        assert_eq!(result, korean_70);
        assert!(!result.ends_with("..."));
    }

    #[test]
    fn test_extract_purpose_korean_210_chars_truncated() {
        let korean_210 = "나".repeat(210);
        assert_eq!(korean_210.chars().count(), 210);
        let content = format!("## Purpose\n\n{}\n\n## Requirements\n", korean_210);
        let result = extract_purpose(&content);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 203);
    }

    #[test]
    fn test_scan_root_claude_md() {
        let temp = create_test_dir();

        fs::write(
            temp.path().join("CLAUDE.md"),
            "## Purpose\n\nProject root spec\n\n## Requirements\nNone\n",
        )
        .unwrap();

        let scanner = ClaudeMdScanner::new();
        let result = scanner.scan(temp.path());

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].dir, PathBuf::from(""));
        assert_eq!(result.entries[0].purpose, "Project root spec");
    }
}
