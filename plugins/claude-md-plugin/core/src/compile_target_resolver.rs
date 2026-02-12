use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{EXCLUDED_DIRS, SOURCE_EXTENSIONS};

/// Result of incremental diff analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResult {
    /// Root directory that was scanned
    pub root: PathBuf,
    /// CLAUDE.md files that need recompilation
    pub targets: Vec<CompileTarget>,
    /// CLAUDE.md files that were skipped (up-to-date)
    pub skipped: Vec<SkippedEntry>,
    /// General warnings (e.g., not a git repo)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<DiffWarning>,
    /// Dependency cascade warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependency_warnings: Vec<DependencyWarning>,
}

/// A CLAUDE.md that needs recompilation
#[derive(Debug, Serialize, Deserialize)]
pub struct CompileTarget {
    /// Path to CLAUDE.md relative to root
    pub claude_md_path: String,
    /// Path to IMPLEMENTS.md relative to root
    pub implements_md_path: String,
    /// Directory containing the spec files
    pub dir: String,
    /// Why this target needs recompilation
    pub reason: TargetReason,
    /// Human-readable explanation
    pub details: String,
}

/// Reason why a CLAUDE.md needs recompilation
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum TargetReason {
    /// CLAUDE.md is in git staging area
    Staged,
    /// CLAUDE.md has unstaged modifications
    Modified,
    /// CLAUDE.md is untracked
    Untracked,
    /// Spec files have newer commits than source code
    SpecNewer,
    /// No source code files exist in the directory
    NoSourceCode,
}

/// A CLAUDE.md that was skipped
#[derive(Debug, Serialize, Deserialize)]
pub struct SkippedEntry {
    /// Directory path relative to root
    pub dir: String,
    /// Why it was skipped
    pub reason: String,
    /// Human-readable explanation
    pub details: String,
}

/// General warning during diff analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffWarning {
    /// Warning category
    pub warning_type: String,
    /// Human-readable message
    pub message: String,
}

/// Warning about potential cascade recompilation needs
#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyWarning {
    /// The changed dependency path
    pub changed_dep: String,
    /// Modules that depend on the changed one
    pub affected_dependents: Vec<String>,
    /// Human-readable message
    pub message: String,
}

pub struct CompileTargetResolver {
    source_extensions: HashSet<String>,
    excluded_dirs: HashSet<String>,
}

impl CompileTargetResolver {
    pub fn new() -> Self {
        Self {
            source_extensions: SOURCE_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            excluded_dirs: EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Resolve which CLAUDE.md files need recompilation based on git state
    pub fn resolve(&self, root: &Path) -> DiffResult {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut targets = Vec::new();
        let mut skipped = Vec::new();
        let mut warnings = Vec::new();
        let dependency_warnings;

        // 1. Check git repo
        if !is_git_repo(&root) {
            warnings.push(DiffWarning {
                warning_type: "no-git-repo".to_string(),
                message: "Not a git repository. Use --all for full compilation.".to_string(),
            });
            return DiffResult {
                root,
                targets,
                skipped,
                warnings,
                dependency_warnings: vec![],
            };
        }

        // 2. Scan all directories containing CLAUDE.md
        let all_claude_md_dirs = self.scan_claude_md_dirs(&root);

        // 3. Get git status (one-time calls)
        let staged_files = git_staged_files(&root);
        let modified_files = git_modified_files(&root);
        let untracked_files = git_untracked_files(&root);

        // Extract dirs from staged/modified/untracked spec files
        let staged_spec_dirs = extract_spec_dirs(&staged_files);
        let modified_spec_dirs = extract_spec_dirs(&modified_files);
        let untracked_spec_dirs = extract_spec_dirs(&untracked_files);

        // 4. Evaluate each directory
        for dir in &all_claude_md_dirs {
            let dir_str = dir.to_string_lossy().to_string();

            if staged_spec_dirs.contains(&dir_str) {
                targets.push(CompileTarget {
                    claude_md_path: format!("{}/CLAUDE.md", dir_str),
                    implements_md_path: format!("{}/IMPLEMENTS.md", dir_str),
                    dir: dir_str,
                    reason: TargetReason::Staged,
                    details: "CLAUDE.md staged for commit".to_string(),
                });
            } else if modified_spec_dirs.contains(&dir_str) {
                targets.push(CompileTarget {
                    claude_md_path: format!("{}/CLAUDE.md", dir_str),
                    implements_md_path: format!("{}/IMPLEMENTS.md", dir_str),
                    dir: dir_str,
                    reason: TargetReason::Modified,
                    details: "CLAUDE.md modified but not staged".to_string(),
                });
            } else if untracked_spec_dirs.contains(&dir_str) {
                targets.push(CompileTarget {
                    claude_md_path: format!("{}/CLAUDE.md", dir_str),
                    implements_md_path: format!("{}/IMPLEMENTS.md", dir_str),
                    dir: dir_str,
                    reason: TargetReason::Untracked,
                    details: "CLAUDE.md not yet tracked by git".to_string(),
                });
            } else {
                // Compare commit timestamps
                let spec_paths = self.spec_files_in(&root, dir);
                let source_files = self.source_files_in(&root, dir);

                let spec_ts = git_last_commit_ts(&root, &spec_paths);
                let source_ts = git_last_commit_ts(&root, &source_files);

                match (spec_ts, source_ts) {
                    (Some(_), None) if source_files.is_empty() => {
                        targets.push(CompileTarget {
                            claude_md_path: format!("{}/CLAUDE.md", dir_str),
                            implements_md_path: format!("{}/IMPLEMENTS.md", dir_str),
                            dir: dir_str,
                            reason: TargetReason::NoSourceCode,
                            details: "No source code files found (first compile)".to_string(),
                        });
                    }
                    (Some(s), None) => {
                        // Source files exist but none committed yet
                        targets.push(CompileTarget {
                            claude_md_path: format!("{}/CLAUDE.md", dir_str),
                            implements_md_path: format!("{}/IMPLEMENTS.md", dir_str),
                            dir: dir_str,
                            reason: TargetReason::SpecNewer,
                            details: format!("Spec committed at {}, source not yet committed", s),
                        });
                    }
                    (Some(s), Some(c)) if s > c => {
                        targets.push(CompileTarget {
                            claude_md_path: format!("{}/CLAUDE.md", dir_str),
                            implements_md_path: format!("{}/IMPLEMENTS.md", dir_str),
                            dir: dir_str,
                            reason: TargetReason::SpecNewer,
                            details: format!("Spec updated at {} > source at {}", s, c),
                        });
                    }
                    (None, _) => {
                        skipped.push(SkippedEntry {
                            dir: dir_str,
                            reason: "spec-not-committed".to_string(),
                            details: "Spec files not committed and not staged".to_string(),
                        });
                    }
                    _ => {
                        skipped.push(SkippedEntry {
                            dir: dir_str,
                            reason: "up-to-date".to_string(),
                            details: "Source code is up-to-date with spec".to_string(),
                        });
                    }
                }
            }
        }

        // 5. Build dependency warnings
        let reverse_deps = self.build_reverse_dependency_map(&root, &all_claude_md_dirs);
        let target_dirs: HashSet<&str> = targets.iter().map(|t| t.dir.as_str()).collect();
        dependency_warnings = self.generate_dependency_warnings(&reverse_deps, &target_dirs);

        // Sort targets by dir for consistent output
        targets.sort_by(|a, b| a.dir.cmp(&b.dir));
        skipped.sort_by(|a, b| a.dir.cmp(&b.dir));

        DiffResult {
            root,
            targets,
            skipped,
            warnings,
            dependency_warnings,
        }
    }

    /// Scan for directories containing CLAUDE.md, excluding build dirs
    fn scan_claude_md_dirs(&self, root: &Path) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        let walker = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    return !e.file_name()
                        .to_str()
                        .map(|n| self.excluded_dirs.contains(n))
                        .unwrap_or(false);
                }
                true
            });

        for entry in walker.flatten() {
            if entry.file_type().is_file()
                && entry.file_name() == "CLAUDE.md"
            {
                if let Some(parent) = entry.path().parent() {
                    // Skip root-level CLAUDE.md (project root)
                    if parent == root {
                        continue;
                    }
                    let rel = parent.strip_prefix(root)
                        .unwrap_or(parent)
                        .to_path_buf();
                    dirs.push(rel);
                }
            }
        }

        dirs.sort();
        dirs
    }

    /// Get relative paths to spec files (CLAUDE.md) in a directory
    fn spec_files_in(&self, _root: &Path, dir: &Path) -> Vec<String> {
        vec![dir.join("CLAUDE.md").to_string_lossy().to_string()]
    }

    /// Get relative paths to source files in a directory (non-recursive)
    fn source_files_in(&self, root: &Path, dir: &Path) -> Vec<String> {
        let abs_dir = root.join(dir);
        let mut paths = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&abs_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    // Skip spec files and non-source files
                    if name_str == "CLAUDE.md" || name_str == "IMPLEMENTS.md" {
                        continue;
                    }
                    if let Some(ext) = Path::new(&*name_str).extension() {
                        if self.source_extensions.contains(ext.to_str().unwrap_or("")) {
                            let rel = dir.join(&*name_str);
                            paths.push(rel.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        paths
    }

    /// Build reverse dependency map: { dependency_dir -> [dependent_dirs] }
    fn build_reverse_dependency_map(
        &self,
        root: &Path,
        dirs: &[PathBuf],
    ) -> HashMap<String, Vec<String>> {
        let mut reverse_map: HashMap<String, Vec<String>> = HashMap::new();
        let dep_regex = regex::Regex::new(r"- `([^`]+/CLAUDE\.md)`").unwrap();

        for dir in dirs {
            let claude_md_path = root.join(dir).join("CLAUDE.md");
            if let Ok(content) = std::fs::read_to_string(&claude_md_path) {
                // Only parse Dependencies > Internal section
                if let Some(deps_section) = extract_internal_deps_section(&content) {
                    for cap in dep_regex.captures_iter(&deps_section) {
                        if let Some(dep_path) = cap.get(1) {
                            // Convert CLAUDE.md path to dir path
                            let dep_dir = dep_path.as_str()
                                .trim_end_matches("/CLAUDE.md")
                                .to_string();
                            let dependent_dir = dir.to_string_lossy().to_string();

                            reverse_map
                                .entry(dep_dir)
                                .or_default()
                                .push(dependent_dir);
                        }
                    }
                }
            }
        }

        reverse_map
    }

    /// Generate dependency warnings for changed targets
    fn generate_dependency_warnings(
        &self,
        reverse_deps: &HashMap<String, Vec<String>>,
        target_dirs: &HashSet<&str>,
    ) -> Vec<DependencyWarning> {
        let mut warnings = Vec::new();

        for target_dir in target_dirs {
            if let Some(dependents) = reverse_deps.get(*target_dir) {
                // Only warn about dependents not already in targets
                let affected: Vec<String> = dependents
                    .iter()
                    .filter(|d| !target_dirs.contains(d.as_str()))
                    .cloned()
                    .collect();

                if !affected.is_empty() {
                    warnings.push(DependencyWarning {
                        changed_dep: target_dir.to_string(),
                        affected_dependents: affected.clone(),
                        message: format!(
                            "{} changed; {} may need recompilation",
                            target_dir,
                            affected.join(", ")
                        ),
                    });
                }
            }
        }

        warnings.sort_by(|a, b| a.changed_dep.cmp(&b.changed_dep));
        warnings
    }
}

impl Default for CompileTargetResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============== Git helper functions ==============

/// Check if the given path is inside a git repository
fn is_git_repo(root: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get list of files in the git staging area
fn git_staged_files(root: &Path) -> Vec<String> {
    Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_string())
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Get list of untracked files (excluding standard ignores)
fn git_untracked_files(root: &Path) -> Vec<String> {
    Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_string())
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Get list of modified but unstaged files
fn git_modified_files(root: &Path) -> Vec<String> {
    Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_string())
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Get the last commit unix timestamp for the given paths
/// Returns None if no commits exist for the given paths
fn git_last_commit_ts(root: &Path, paths: &[String]) -> Option<u64> {
    if paths.is_empty() {
        return None;
    }

    let mut cmd = Command::new("git");
    cmd.args(["log", "-1", "--format=%ct", "--"]);
    for p in paths {
        cmd.arg(p);
    }
    cmd.current_dir(root);

    cmd.output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim().parse::<u64>().ok()
            } else {
                None
            }
        })
}

// ============== Utility functions ==============

/// Extract directory paths from file paths that contain CLAUDE.md or IMPLEMENTS.md
fn extract_spec_dirs(files: &[String]) -> HashSet<String> {
    let mut dirs = HashSet::new();
    for file in files {
        if file.ends_with("/CLAUDE.md") || file == "CLAUDE.md"
        {
            let dir = Path::new(file)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            // Skip root-level (empty dir)
            if !dir.is_empty() {
                dirs.insert(dir);
            }
        }
    }
    dirs
}

/// Extract the "### Internal" subsection from Dependencies section of CLAUDE.md content
fn extract_internal_deps_section(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_deps = false;
    let mut in_internal = false;
    let mut section_lines = Vec::new();

    for line in &lines {
        if line.starts_with("## Dependencies") {
            in_deps = true;
            continue;
        }
        if in_deps && line.starts_with("## ") && !line.starts_with("## Dependencies") {
            break; // Exit Dependencies section
        }
        if in_deps && line.starts_with("### Internal") {
            in_internal = true;
            continue;
        }
        if in_internal && line.starts_with("### ") {
            break; // Exit Internal subsection
        }
        if in_internal {
            section_lines.push(*line);
        }
    }

    if section_lines.is_empty() {
        None
    } else {
        Some(section_lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_spec_dirs() {
        let files = vec![
            "src/auth/CLAUDE.md".to_string(),
            "src/utils/CLAUDE.md".to_string(),
            "README.md".to_string(),
            "CLAUDE.md".to_string(), // root-level, should be skipped
        ];

        let dirs = extract_spec_dirs(&files);
        assert!(dirs.contains("src/auth"));
        assert!(dirs.contains("src/utils"));
        assert!(!dirs.contains("")); // root-level skipped
        assert_eq!(dirs.len(), 2);
    }

    #[test]
    fn test_extract_spec_dirs_ignores_implements_md() {
        let files = vec![
            "src/auth/IMPLEMENTS.md".to_string(),
            "IMPLEMENTS.md".to_string(),
        ];
        let dirs = extract_spec_dirs(&files);
        assert!(dirs.is_empty(), "IMPLEMENTS.md should not trigger compile targets");
    }

    #[test]
    fn test_extract_internal_deps_section() {
        let content = r#"# Module

## Purpose
Test module

## Dependencies
### Internal
- `core/domain/CLAUDE.md` — Domain model
- `shared/utils/CLAUDE.md` — Utilities

### External
- express@4.x
"#;
        let section = extract_internal_deps_section(content).unwrap();
        assert!(section.contains("core/domain/CLAUDE.md"));
        assert!(section.contains("shared/utils/CLAUDE.md"));
        assert!(!section.contains("express"));
    }

    #[test]
    fn test_extract_internal_deps_section_none() {
        let content = r#"# Module

## Purpose
Test module

## Exports
- foo()
"#;
        assert!(extract_internal_deps_section(content).is_none());
    }

    #[test]
    fn test_target_reason_serialization() {
        let staged = serde_json::to_string(&TargetReason::Staged).unwrap();
        assert_eq!(staged, "\"staged\"");

        let modified = serde_json::to_string(&TargetReason::Modified).unwrap();
        assert_eq!(modified, "\"modified\"");

        let spec_newer = serde_json::to_string(&TargetReason::SpecNewer).unwrap();
        assert_eq!(spec_newer, "\"spec-newer\"");

        let no_source = serde_json::to_string(&TargetReason::NoSourceCode).unwrap();
        assert_eq!(no_source, "\"no-source-code\"");
    }

    #[test]
    fn test_non_git_repo_returns_warning() {
        let temp = tempfile::TempDir::new().unwrap();
        let resolver = CompileTargetResolver::new();
        let result = resolver.resolve(temp.path());

        assert!(result.targets.is_empty());
        assert!(!result.warnings.is_empty());
        assert_eq!(result.warnings[0].warning_type, "no-git-repo");
    }
}
