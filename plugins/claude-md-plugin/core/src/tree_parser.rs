use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::tree_utils::DirScanner;

/// Result of tree parsing
#[derive(Debug, Serialize, Deserialize)]
pub struct TreeResult {
    /// Root directory that was scanned
    pub root: PathBuf,
    /// Directories that need CLAUDE.md
    pub needs_claude_md: Vec<DirectoryInfo>,
    /// Directories that were excluded
    pub excluded: Vec<PathBuf>,
    /// Errors encountered during scanning (non-fatal)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scan_errors: Vec<ScanError>,
}

/// Error encountered during tree scanning
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanError {
    /// Path where error occurred
    pub path: String,
    /// Error message
    pub message: String,
}

/// Information about a directory
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInfo {
    /// Path relative to root
    pub path: PathBuf,
    /// Number of source files directly in this directory
    pub source_file_count: usize,
    /// Number of subdirectories
    pub subdir_count: usize,
    /// Reason why CLAUDE.md is needed
    pub reason: String,
    /// Depth from root (root = 0, first level subdirs = 1, etc.)
    /// Used for leaf-first ordering: process deepest directories first
    pub depth: usize,
}

pub struct TreeParser {
    scanner: DirScanner,
}

impl TreeParser {
    pub fn new() -> Self {
        Self {
            scanner: DirScanner::new(),
        }
    }

    /// Parse a directory tree and identify where CLAUDE.md is needed
    pub fn parse(&self, root: &Path) -> TreeResult {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut needs_claude_md = Vec::new();
        let mut scan_errors = Vec::new();

        // TreeParser needs to handle WalkDir errors for scan_errors, so we can't use
        // collect_directories directly. We still use the scanner for individual checks.
        let mut dirs_to_check: Vec<PathBuf> = Vec::new();
        let mut excluded = Vec::new();

        let walker = WalkDir::new(&root).into_iter();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let path = e.path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    scan_errors.push(ScanError {
                        path,
                        message: format!("Failed to read directory entry: {}", e),
                    });
                    continue;
                }
            };

            if entry.file_type().is_dir() {
                let path = entry.path().to_path_buf();
                if self.scanner.should_exclude(&path) {
                    excluded.push(self.scanner.make_relative(&root, &path));
                    continue;
                }

                // Check if any ancestor is excluded
                let is_under_excluded = path.ancestors().skip(1).any(|p| {
                    self.scanner.should_exclude(p)
                });

                if !is_under_excluded {
                    dirs_to_check.push(path);
                }
            }
        }

        // Check each directory
        for dir in dirs_to_check {
            if let Some(info) = self.check_directory(&root, &dir) {
                needs_claude_md.push(info);
            }
        }

        // Sort by path for consistent output
        needs_claude_md.sort_by(|a, b| a.path.cmp(&b.path));

        TreeResult {
            root,
            needs_claude_md,
            excluded,
            scan_errors,
        }
    }

    fn check_directory(&self, root: &Path, dir: &Path) -> Option<DirectoryInfo> {
        let source_file_count = self.scanner.count_source_files(dir);
        let subdir_count = self.scanner.count_subdirs(dir);

        // CON-1: CLAUDE.md needed if 1+ source files OR 2+ subdirs
        let needs_claude_md = source_file_count >= 1 || subdir_count >= 2;

        if needs_claude_md {
            let reason = if source_file_count >= 1 && subdir_count >= 2 {
                format!(
                    "{} source files and {} subdirectories",
                    source_file_count, subdir_count
                )
            } else if source_file_count >= 1 {
                format!("{} source files", source_file_count)
            } else {
                format!("{} subdirectories", subdir_count)
            };

            let relative_path = self.scanner.make_relative(root, dir);
            let depth = self.scanner.calculate_depth(&relative_path);

            Some(DirectoryInfo {
                path: relative_path,
                source_file_count,
                subdir_count,
                reason,
                depth,
            })
        } else {
            None
        }
    }
}

impl Default for TreeParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_directory_with_source_files_needs_claude_md() {
        let temp = create_test_dir();
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        File::create(auth_dir.join("token.rs")).unwrap();
        File::create(auth_dir.join("session.rs")).unwrap();

        let parser = TreeParser::new();
        let result = parser.parse(temp.path());

        let auth_info = result
            .needs_claude_md
            .iter()
            .find(|d| d.path.ends_with("auth"))
            .expect("auth should need CLAUDE.md");

        assert_eq!(auth_info.source_file_count, 2);
    }

    #[test]
    fn test_directory_with_two_subdirs_needs_claude_md() {
        let temp = create_test_dir();
        let src_dir = temp.path().join("src");
        fs::create_dir_all(src_dir.join("auth")).unwrap();
        fs::create_dir_all(src_dir.join("api")).unwrap();

        let parser = TreeParser::new();
        let result = parser.parse(temp.path());

        let src_info = result
            .needs_claude_md
            .iter()
            .find(|d| d.path == PathBuf::from("src"))
            .expect("src should need CLAUDE.md");

        assert_eq!(src_info.subdir_count, 2);
    }

    #[test]
    fn test_build_directories_are_excluded() {
        let temp = create_test_dir();
        let target_dir = temp.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        File::create(target_dir.join("main.rs")).unwrap();

        let parser = TreeParser::new();
        let result = parser.parse(temp.path());

        assert!(result.excluded.iter().any(|p| p.ends_with("target")));
        assert!(!result.needs_claude_md.iter().any(|d| d.path.to_string_lossy().contains("target")));
    }
}
