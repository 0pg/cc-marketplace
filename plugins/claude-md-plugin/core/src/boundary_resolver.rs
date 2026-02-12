use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// File information with language type
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: String,
}

/// Subdirectory information
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SubdirInfo {
    pub name: String,
    pub has_claude_md: bool,
}

/// Result of boundary resolution
#[derive(Debug, Serialize, Deserialize)]
pub struct BoundaryResult {
    /// Directory path
    pub path: PathBuf,
    /// Direct files in this directory
    pub direct_files: Vec<FileInfo>,
    /// Subdirectories
    pub subdirs: Vec<SubdirInfo>,
    /// Number of source files
    pub source_file_count: usize,
    /// Number of subdirectories
    pub subdir_count: usize,
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

const COMMON_SLASH_EXPRESSIONS: &[&str] = &[
    "input/output", "client/server", "pre/post", "success/failure",
    "true/false", "yes/no", "read/write", "get/set",
    "request/response", "start/stop", "on/off",
];

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
        let source_file_count = direct_files.iter()
            .filter(|f| Self::extension_to_language(&f.name) != "other")
            .count();
        let subdir_count = subdirs.len();

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
            source_file_count,
            subdir_count,
            violations,
            claude_md_error,
        }
    }

    fn get_direct_files(&self, path: &Path) -> Vec<FileInfo> {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter_map(|e| {
                        let name = e.file_name().to_str()?.to_string();
                        let file_type = Self::extension_to_language(&name);
                        Some(FileInfo { name, file_type })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_subdirs(&self, path: &Path) -> Vec<SubdirInfo> {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter_map(|e| {
                        let name = e.file_name().to_str()?.to_string();
                        if name.starts_with('.') {
                            return None;
                        }
                        let has_claude_md = e.path().join("CLAUDE.md").exists();
                        Some(SubdirInfo { name, has_claude_md })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extension_to_language(filename: &str) -> String {
        let ext = filename.rsplit('.').next().unwrap_or("");
        match ext {
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "py" | "pyi" => "python",
            "rs" => "rust",
            "go" => "go",
            "java" => "java",
            "kt" | "kts" => "kotlin",
            "rb" => "ruby",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "cs" => "csharp",
            "swift" => "swift",
            "php" => "php",
            "scala" => "scala",
            "md" => "markdown",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "html" | "htm" => "html",
            "css" | "scss" | "sass" => "css",
            _ => "other",
        }.to_string()
    }

    fn find_violations(
        &self,
        _dir_path: &Path,
        claude_md_content: &str,
        valid_subdirs: &[SubdirInfo],
    ) -> Vec<ReferenceViolation> {
        let mut violations = Vec::new();
        let subdir_names: HashSet<&str> = valid_subdirs.iter().map(|s| s.name.as_str()).collect();

        // Only check Dependencies and Structure sections for reference violations
        let section_lines = Self::extract_dependency_and_structure_lines(claude_md_content);

        for (line_num, line) in &section_lines {
            // Skip URLs
            if line.contains("http://") || line.contains("https://") {
                continue;
            }

            // Strip inline code spans and markdown link targets
            let cleaned = Self::strip_inline_code_and_links(line);

            for cap in self.reference_pattern.captures_iter(&cleaned) {
                if let Some(reference) = cap.get(1) {
                    let ref_str = reference.as_str();

                    // Check for parent references (..)
                    if ref_str.starts_with("../") {
                        violations.push(ReferenceViolation {
                            violation_type: "Parent".to_string(),
                            reference: ref_str.to_string(),
                            line_number: *line_num,
                        });
                        continue;
                    }

                    // Filter common slash expressions
                    let ref_lower = ref_str.to_lowercase();
                    if COMMON_SLASH_EXPRESSIONS.iter().any(|expr| ref_lower == *expr) {
                        continue;
                    }

                    // Minimum first segment length: 2+ chars required to be a valid path segment.
                    // Filters out false positives from prose like "n/a", "a/b comparison",
                    // "I/O" etc. while accepting real directory refs like "src/utils".
                    let first_segment = ref_str.split('/').next().unwrap_or("");
                    if first_segment.len() < 2 {
                        continue;
                    }

                    // If it starts with a valid child directory, it's allowed
                    if subdir_names.contains(first_segment) {
                        continue;
                    }

                    // Otherwise it's a sibling reference
                    violations.push(ReferenceViolation {
                        violation_type: "Sibling".to_string(),
                        reference: ref_str.to_string(),
                        line_number: *line_num,
                    });
                }
            }
        }

        violations
    }

    /// Extract lines from Dependencies and Structure sections only
    fn extract_dependency_and_structure_lines(content: &str) -> Vec<(usize, String)> {
        let mut lines = Vec::new();
        let mut in_target_section = false;
        let mut in_code_block = false;

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                continue;
            }

            // Detect H2 section headers
            if trimmed.starts_with("## ") {
                let section_name = trimmed[3..].trim().to_lowercase();
                in_target_section = section_name == "dependencies" || section_name == "structure";
                continue;
            }

            if in_target_section {
                lines.push((idx + 1, line.to_string()));
            }
        }

        lines
    }

    /// Strip inline code spans (`` ` ``) and markdown link targets `](...)` from a line
    /// to prevent false-positive sibling reference detection.
    fn strip_inline_code_and_links(line: &str) -> String {
        let mut result = String::with_capacity(line.len());
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '`' {
                // Strip backtick delimiters but keep inner text
                i += 1; // skip opening backtick
                while i < chars.len() && chars[i] != '`' {
                    result.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip closing backtick
                }
            } else if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == '(' {
                // Skip markdown link target ](...)
                result.push(chars[i]); // keep ']'
                i += 2; // skip ](
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '(' {
                        depth += 1;
                    } else if chars[i] == ')' {
                        depth -= 1;
                    }
                    i += 1;
                }
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        result
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

    #[test]
    fn test_backtick_wrapped_parent_reference_detected() {
        let temp = create_test_dir();
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();

        let claude_md = auth_dir.join("CLAUDE.md");
        let mut file = File::create(&claude_md).unwrap();
        writeln!(file, "## Dependencies").unwrap();
        writeln!(file, "- **Internal**: `../utils/crypto`").unwrap();

        let resolver = BoundaryResolver::new();
        let result = resolver.resolve(&auth_dir, Some(&claude_md));

        let violations = result.violations.unwrap();
        assert!(
            violations.iter().any(|v| v.violation_type == "Parent"),
            "Backtick-wrapped parent reference should be detected as violation"
        );
    }
}
