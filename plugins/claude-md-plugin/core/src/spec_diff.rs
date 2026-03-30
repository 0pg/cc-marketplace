use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

/// A single requirement change entry from the spec diff
#[derive(Debug, Serialize, Deserialize)]
pub struct RequirementChange {
    /// "added" or "removed"
    pub action: String,
    /// The requirement text (without leading +/- diff marker)
    pub text: String,
}

/// Result of diffing a CLAUDE.md spec file against its last committed state
#[derive(Debug, Serialize, Deserialize)]
pub struct SpecDiffResult {
    /// Absolute path of the spec file analyzed
    pub spec_file: String,
    /// Whether this is a git repository
    pub is_git_repo: bool,
    /// Hash of the last commit that touched this spec file (None if never committed)
    pub last_spec_commit: Option<String>,
    /// Requirements that were added or removed in the last spec commit
    /// Empty if all_requirements=true (first commit / non-git)
    pub changed_requirements: Vec<RequirementChange>,
    /// When true, treat all Requirements as changed (first commit, non-git, or no prior history)
    pub all_requirements: bool,
    /// Source files that changed between last_spec_commit and HEAD
    pub source_changed_files: Vec<String>,
    /// True if any source files changed since the last spec commit
    pub source_changed: bool,
}

pub struct SpecDiffer {
    root: PathBuf,
}

impl SpecDiffer {
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf() }
    }

    pub fn diff(&self, claude_md_path: &Path) -> SpecDiffResult {
        let spec_file = claude_md_path.to_string_lossy().to_string();

        if !self.is_git_repo() {
            return SpecDiffResult {
                spec_file,
                is_git_repo: false,
                last_spec_commit: None,
                changed_requirements: vec![],
                all_requirements: true,
                source_changed_files: vec![],
                source_changed: true,
            };
        }

        let last_commit = self.last_commit_for_file(claude_md_path);

        match last_commit {
            None => {
                // File never committed — treat all requirements as new
                SpecDiffResult {
                    spec_file,
                    is_git_repo: true,
                    last_spec_commit: None,
                    changed_requirements: vec![],
                    all_requirements: true,
                    source_changed_files: vec![],
                    source_changed: true,
                }
            }
            Some(ref commit_hash) => {
                let changed_requirements = self.diff_requirements(claude_md_path, commit_hash);
                let source_changed_files = self.source_files_changed_since(commit_hash);
                let source_changed = !source_changed_files.is_empty();

                SpecDiffResult {
                    spec_file,
                    is_git_repo: true,
                    last_spec_commit: last_commit,
                    changed_requirements,
                    all_requirements: false,
                    source_changed_files,
                    source_changed,
                }
            }
        }
    }

    fn is_git_repo(&self) -> bool {
        Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&self.root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Returns the commit hash of the most recent commit that changed this file.
    /// Returns None if the file has never been committed.
    fn last_commit_for_file(&self, file: &Path) -> Option<String> {
        let output = Command::new("git")
            .args(["log", "-1", "--format=%H", "--", file.to_str().unwrap_or("")])
            .current_dir(&self.root)
            .output()
            .ok()?;

        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if hash.is_empty() { None } else { Some(hash) }
        } else {
            None
        }
    }

    /// Diffs the spec file between last_commit~1 and last_commit.
    /// Extracts lines inside the ## Requirements section that were added or removed.
    fn diff_requirements(&self, file: &Path, commit_hash: &str) -> Vec<RequirementChange> {
        let parent = format!("{}~1", commit_hash);
        let output = Command::new("git")
            .args(["diff", &parent, commit_hash, "--", file.to_str().unwrap_or("")])
            .current_dir(&self.root)
            .output();

        let diff_text = match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return vec![],
        };

        self.parse_requirements_diff(&diff_text)
    }

    /// Parses a git diff output and extracts changed lines from the ## Requirements section.
    fn parse_requirements_diff(&self, diff: &str) -> Vec<RequirementChange> {
        let mut changes = Vec::new();
        let mut in_requirements = false;

        for line in diff.lines() {
            // Section boundary detection (## heading lines in diff context)
            if line.starts_with('+') || line.starts_with('-') {
                let content = &line[1..];
                let trimmed = content.trim();

                if trimmed.starts_with("## ") {
                    in_requirements = trimmed == "## Requirements";
                    continue;
                }

                if in_requirements && !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && (trimmed.starts_with("- ") || trimmed.starts_with("* "))
                {
                    let action = if line.starts_with('+') { "added" } else { "removed" };
                    // Strip list marker
                    let text = trimmed
                        .strip_prefix("- ")
                        .or_else(|| trimmed.strip_prefix("* "))
                        .unwrap_or(trimmed)
                        .to_string();
                    changes.push(RequirementChange { action: action.to_string(), text });
                }
            } else if line.starts_with("@@") {
                // Hunk headers reset section tracking
                in_requirements = false;
            }
        }

        changes
    }

    /// Returns source files (non-spec) that changed between commit and HEAD.
    fn source_files_changed_since(&self, commit_hash: &str) -> Vec<String> {
        let output = Command::new("git")
            .args(["diff", "--name-only", commit_hash, "HEAD"])
            .current_dir(&self.root)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|f| !f.is_empty())
                    .filter(|f| !f.ends_with("CLAUDE.md") && !f.ends_with("DEVELOPERS.md"))
                    .map(|f| f.to_string())
                    .collect()
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_requirements_diff_added() {
        let differ = SpecDiffer::new(Path::new("."));
        let diff = "\
+## Requirements\n\
+- 에이전트 panic 시 AgentResult::Failed 반환\n\
+- 타임아웃 설정 가능\n\
 ## Domain Context\n";
        let changes = differ.parse_requirements_diff(diff);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].action, "added");
        assert!(changes[0].text.contains("AgentResult"));
        assert_eq!(changes[1].action, "added");
        assert!(changes[1].text.contains("타임아웃"));
    }

    #[test]
    fn test_parse_requirements_diff_removed() {
        let differ = SpecDiffer::new(Path::new("."));
        let diff = "\
-## Requirements\n\
-- 구 요구사항\n\
+## Requirements\n\
+- 신 요구사항\n";
        let changes = differ.parse_requirements_diff(diff);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].action, "removed");
        assert_eq!(changes[1].action, "added");
    }

    #[test]
    fn test_parse_requirements_diff_ignores_other_sections() {
        let differ = SpecDiffer::new(Path::new("."));
        let diff = "\
+## Purpose\n\
+새로운 목적\n\
+## Requirements\n\
+- 실제 요구사항\n\
+## Domain Context\n\
+도메인 정보\n";
        let changes = differ.parse_requirements_diff(diff);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].text.contains("실제 요구사항"));
    }

    #[test]
    fn test_parse_requirements_diff_empty() {
        let differ = SpecDiffer::new(Path::new("."));
        let changes = differ.parse_requirements_diff("");
        assert!(changes.is_empty());
    }
}
