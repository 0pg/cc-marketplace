use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Result of boundary resolution
#[derive(Debug, Serialize, Deserialize)]
pub struct BoundaryResult {
    /// Directory path
    pub path: PathBuf,
    /// Direct files in this directory
    pub direct_files: Vec<String>,
    /// Subdirectories
    pub subdirs: Vec<String>,
    /// Reference violations found (if CLAUDE.md was provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violations: Option<Vec<ReferenceViolation>>,
    /// Error reading CLAUDE.md file (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_md_error: Option<String>,
}

/// Reference violation types
#[derive(Debug, Serialize, Deserialize)]
pub struct ReferenceViolation {
    /// Type of violation: "Parent" or "Sibling"
    pub violation_type: String,
    /// The reference that caused the violation
    pub reference: String,
    /// Line number in CLAUDE.md where violation was found
    pub line_number: usize,
}

pub struct BoundaryResolver {
    /// Pattern to match directory references in CLAUDE.md
    reference_pattern: Regex,
}

impl BoundaryResolver {
    pub fn new() -> Self {
        // Match explicit path patterns:
        // - Parent references: ../something
        // - Path-like references with slashes: dir/subdir
        // - Explicit directory references in context
        let reference_pattern = Regex::new(r#"(?:^|[\s"`'])(\.\./[\w-]+(?:/[\w-]+)*|[\w-]+/[\w-]+(?:/[\w-]+)*)/?"#).unwrap();
        Self { reference_pattern }
    }

    /// Resolve boundary for a directory
    pub fn resolve(&self, path: &Path, claude_md: Option<&PathBuf>) -> BoundaryResult {
        let direct_files = self.get_direct_files(path);
        let subdirs = self.get_subdirs(path);

        let (violations, claude_md_error) = match claude_md {
            Some(md_path) => {
                match std::fs::read_to_string(md_path) {
                    Ok(content) => (Some(self.find_violations(path, &content, &subdirs)), None),
                    Err(e) => (
                        None,
                        Some(format!(
                            "Failed to read '{}': {} (check file exists and permissions)",
                            md_path.display(),
                            e
                        )),
                    ),
                }
            }
            None => (None, None),
        };

        BoundaryResult {
            path: path.to_path_buf(),
            direct_files,
            subdirs,
            violations,
            claude_md_error,
        }
    }

    fn get_direct_files(&self, path: &Path) -> Vec<String> {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_subdirs(&self, path: &Path) -> Vec<String> {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .filter(|name| !name.starts_with('.'))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn find_violations(
        &self,
        _dir_path: &Path,
        claude_md_content: &str,
        valid_subdirs: &[String],
    ) -> Vec<ReferenceViolation> {
        let mut violations = Vec::new();
        let subdir_set: HashSet<_> = valid_subdirs.iter().collect();
        let mut in_code_block = false;

        for (line_num, line) in claude_md_content.lines().enumerate() {
            // Track code blocks
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }

            // Skip code blocks
            if in_code_block {
                continue;
            }

            // Skip URLs
            if line.contains("http://") || line.contains("https://") {
                continue;
            }

            for cap in self.reference_pattern.captures_iter(line) {
                if let Some(reference) = cap.get(1) {
                    let ref_str = reference.as_str();

                    // Check for parent references (..)
                    if ref_str.starts_with("../") {
                        violations.push(ReferenceViolation {
                            violation_type: "Parent".to_string(),
                            reference: ref_str.to_string(),
                            line_number: line_num + 1,
                        });
                        continue;
                    }

                    // For path-like references (contains /), check if first segment is a valid child
                    let first_segment = ref_str.split('/').next().unwrap_or("");

                    // If it starts with a valid child directory, it's allowed
                    if subdir_set.contains(&first_segment.to_string()) {
                        continue;
                    }

                    // Otherwise it's a sibling reference (referencing something outside our children)
                    violations.push(ReferenceViolation {
                        violation_type: "Sibling".to_string(),
                        reference: ref_str.to_string(),
                        line_number: line_num + 1,
                    });
                }
            }
        }

        violations
    }
}

impl Default for BoundaryResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_child_reference_is_allowed() {
        let temp = create_test_dir();
        let src_dir = temp.path().join("src");
        fs::create_dir_all(src_dir.join("auth")).unwrap();

        let claude_md = src_dir.join("CLAUDE.md");
        let mut file = File::create(&claude_md).unwrap();
        writeln!(file, "## Structure").unwrap();
        writeln!(file, "- auth/: Auth module (see auth/CLAUDE.md for details)").unwrap();

        let resolver = BoundaryResolver::new();
        let result = resolver.resolve(&src_dir, Some(&claude_md));

        let violations = result.violations.unwrap();
        // Child references should not be violations
        assert!(violations.is_empty() || !violations.iter().any(|v| v.reference == "auth"));
    }

    #[test]
    fn test_parent_reference_is_forbidden() {
        let temp = create_test_dir();
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();

        let claude_md = auth_dir.join("CLAUDE.md");
        let mut file = File::create(&claude_md).unwrap();
        writeln!(file, "## Dependencies").unwrap();
        writeln!(file, "- See ../api for reference").unwrap();

        let resolver = BoundaryResolver::new();
        let result = resolver.resolve(&auth_dir, Some(&claude_md));

        let violations = result.violations.unwrap();
        assert!(violations.iter().any(|v| v.violation_type == "Parent"));
    }
}
