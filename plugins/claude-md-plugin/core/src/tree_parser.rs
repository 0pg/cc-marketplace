use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

use crate::{EXCLUDED_DIRS, SOURCE_EXTENSIONS};

pub struct TreeParser {
    source_extensions: HashSet<String>,
    excluded_dirs: HashSet<String>,
}

impl TreeParser {
    pub fn new() -> Self {
        Self {
            source_extensions: SOURCE_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            excluded_dirs: EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Parse a directory tree and identify where CLAUDE.md is needed
    pub fn parse(&self, root: &Path) -> TreeResult {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut needs_claude_md = Vec::new();
        let mut excluded: Vec<PathBuf>;
        let mut scan_errors = Vec::new();

        // Collect directories, pruning excluded ones early via filter_entry.
        // This avoids traversing into node_modules, target, etc. entirely.
        let mut dirs_to_check: Vec<PathBuf> = Vec::new();

        let walker = WalkDir::new(&root).into_iter()
            .filter_entry(|e| {
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
                dirs_to_check.push(path);
            }
        }

        // Collect excluded directories from immediate children of each checked dir.
        // filter_entry skips excluded dirs entirely, so we gather them here for reporting.
        let mut excluded_set: HashSet<PathBuf> = HashSet::new();
        for dir in &dirs_to_check {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && self.should_exclude(&entry.path())
                    {
                        let rel = self.make_relative(&root, &entry.path());
                        excluded_set.insert(rel);
                    }
                }
            }
        }
        excluded = excluded_set.into_iter().collect();
        excluded.sort();

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

    fn should_exclude(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                return self.excluded_dirs.contains(name_str);
            }
        }
        false
    }

    fn check_directory(&self, root: &Path, dir: &Path) -> Option<DirectoryInfo> {
        let source_file_count = self.count_source_files(dir);
        let subdir_count = self.count_subdirs(dir);

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

            let relative_path = self.make_relative(root, dir);
            let depth = self.calculate_depth(&relative_path);

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

    /// Calculate the depth of a path from root
    /// Root = 0, "src" = 1, "src/auth" = 2, etc.
    fn calculate_depth(&self, relative_path: &Path) -> usize {
        if relative_path.as_os_str().is_empty() {
            0
        } else {
            relative_path.components().count()
        }
    }

    fn count_source_files(&self, dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter(|e| self.is_source_file(&e.path()))
                    .count()
            })
            .unwrap_or(0)
    }

    fn count_subdirs(&self, dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter(|e| {
                        let binding = e.file_name();
                        let name = binding.to_str().unwrap_or("");
                        !name.starts_with('.') && !self.should_exclude(&e.path())
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    fn is_source_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.source_extensions.contains(ext))
            .unwrap_or(false)
    }

    fn make_relative(&self, root: &Path, path: &Path) -> PathBuf {
        path.strip_prefix(root)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| path.to_path_buf())
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
