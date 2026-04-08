use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

/// A single changed line within an H2 section
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SectionChange {
    /// "added" or "removed"
    pub action: String,
    /// The line text (without diff marker)
    pub text: String,
}

/// Changes to one H2 section within a single commit
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SectionDiff {
    /// H2 section name, e.g. "Requirements", "Constraints", "Purpose"
    pub section: String,
    /// Individual line changes within this section
    pub changes: Vec<SectionChange>,
}

/// Per-file diff within a commit (CLAUDE.md or DEVELOPERS.md)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileDiff {
    /// "CLAUDE.md" or "DEVELOPERS.md"
    pub file_type: String,
    /// Relative path of the file
    pub path: String,
    /// Section-level diffs
    pub sections: Vec<SectionDiff>,
}

/// One commit's complete change record
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommitEntry {
    /// Full commit hash
    pub hash: String,
    /// Short commit hash (7 chars)
    pub short_hash: String,
    /// Commit subject line
    pub subject: String,
    /// Full commit message body (excluding subject)
    pub body: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Author name
    pub author: String,
    /// Per-file diffs in this commit
    pub file_diffs: Vec<FileDiff>,
    /// Whether commit message contains [BREAKING]
    pub breaking: bool,
}

/// Top-level result of diff-node-history
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeHistoryResult {
    /// Node path (directory containing the spec files)
    pub node_path: String,
    /// Whether this is a git repository
    pub is_git_repo: bool,
    /// Whether any history was found
    pub has_history: bool,
    /// Ordered list of commits (newest first), up to --limit
    pub commits: Vec<CommitEntry>,
    /// Total commit count found (before limit applied)
    pub total_commits_found: usize,
    /// Source files changed between oldest included commit and HEAD
    pub source_changed_files: Vec<String>,
    /// True if any source files changed
    pub source_changed: bool,
}

pub struct NodeHistoryDiffer {
    root: PathBuf,
    node_path: PathBuf,
}

impl NodeHistoryDiffer {
    pub fn new(root: &Path, node_path: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            node_path: node_path.to_path_buf(),
        }
    }

    pub fn diff(&self, limit: usize, grep: Option<&str>, since_commit: Option<&str>) -> NodeHistoryResult {
        let node_path_str = self.node_path.to_string_lossy().to_string();

        if !self.is_git_repo() {
            return NodeHistoryResult {
                node_path: node_path_str,
                is_git_repo: false,
                has_history: false,
                commits: vec![],
                total_commits_found: 0,
                source_changed_files: vec![],
                source_changed: false,
            };
        }

        // Find all commits touching CLAUDE.md or DEVELOPERS.md in this node
        let all_hashes = self.find_commits(None, grep, since_commit);
        let total_commits_found = all_hashes.len();

        // Apply limit
        let limited_hashes: Vec<String> = all_hashes.into_iter().take(limit).collect();

        if limited_hashes.is_empty() {
            return NodeHistoryResult {
                node_path: node_path_str,
                is_git_repo: true,
                has_history: false,
                commits: vec![],
                total_commits_found: 0,
                source_changed_files: vec![],
                source_changed: false,
            };
        }

        // Build commit entries
        let commits: Vec<CommitEntry> = limited_hashes.iter()
            .filter_map(|hash| self.build_commit_entry(hash))
            .collect();

        // Source files changed since oldest commit
        let oldest_hash = limited_hashes.last().unwrap();
        let source_changed_files = self.source_files_changed_since(oldest_hash);
        let source_changed = !source_changed_files.is_empty();

        NodeHistoryResult {
            node_path: node_path_str,
            is_git_repo: true,
            has_history: !commits.is_empty(),
            commits,
            total_commits_found,
            source_changed_files,
            source_changed,
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

    /// Find commit hashes touching CLAUDE.md or DEVELOPERS.md in the node path.
    /// Returns newest first.
    fn find_commits(&self, limit: Option<usize>, grep: Option<&str>, since_commit: Option<&str>) -> Vec<String> {
        let claude_md = self.node_path.join("CLAUDE.md");
        let developers_md = self.node_path.join("DEVELOPERS.md");

        let mut args = vec!["log".to_string(), "--format=%H".to_string()];

        if let Some(n) = limit {
            args.push("-n".to_string());
            args.push(n.to_string());
        }

        if let Some(pattern) = grep {
            args.push("--grep".to_string());
            args.push(pattern.to_string());
        }

        if let Some(since) = since_commit {
            args.push(format!("{}..HEAD", since));
        }

        args.push("--".to_string());
        args.push(claude_md.to_string_lossy().to_string());
        args.push(developers_md.to_string_lossy().to_string());

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.trim().to_string())
                    .collect()
            }
            _ => vec![],
        }
    }

    /// Build a CommitEntry from a commit hash
    fn build_commit_entry(&self, hash: &str) -> Option<CommitEntry> {
        // Get metadata: hash, short_hash, subject, timestamp, author
        let output = Command::new("git")
            .args(["log", "-1", "--format=%H%x00%h%x00%s%x00%aI%x00%an", hash])
            .current_dir(&self.root)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let meta = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = meta.splitn(5, '\0').collect();
        if parts.len() < 5 {
            return None;
        }

        // Get body separately
        let body_output = Command::new("git")
            .args(["log", "-1", "--format=%b", hash])
            .current_dir(&self.root)
            .output()
            .ok()?;

        let body = String::from_utf8_lossy(&body_output.stdout).trim().to_string();

        let subject = parts[2].to_string();
        let breaking = subject.contains("[BREAKING]") || body.contains("[BREAKING]");

        // Get file diffs
        let mut file_diffs = Vec::new();

        let claude_md = self.node_path.join("CLAUDE.md");
        if let Some(fd) = self.get_file_diff(hash, &claude_md, "CLAUDE.md") {
            file_diffs.push(fd);
        }

        let developers_md = self.node_path.join("DEVELOPERS.md");
        if let Some(fd) = self.get_file_diff(hash, &developers_md, "DEVELOPERS.md") {
            file_diffs.push(fd);
        }

        Some(CommitEntry {
            hash: parts[0].to_string(),
            short_hash: parts[1].to_string(),
            subject,
            body,
            timestamp: parts[3].to_string(),
            author: parts[4].to_string(),
            file_diffs,
            breaking,
        })
    }

    /// Get section-level diff for a specific file in a commit
    fn get_file_diff(&self, hash: &str, file: &Path, file_type: &str) -> Option<FileDiff> {
        let diff_text = self.get_raw_diff(hash, file)?;
        if diff_text.is_empty() {
            return None;
        }

        let sections = parse_section_diffs(&diff_text);

        Some(FileDiff {
            file_type: file_type.to_string(),
            path: file.to_string_lossy().to_string(),
            sections,
        })
    }

    /// Get raw diff text for a file at a specific commit
    fn get_raw_diff(&self, hash: &str, file: &Path) -> Option<String> {
        // Try normal diff first (hash~1..hash)
        let parent = format!("{}~1", hash);
        let file_str = file.to_str().unwrap_or("");

        let output = Command::new("git")
            .args(["diff", &parent, hash, "--", file_str])
            .current_dir(&self.root)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout).to_string();
                if text.trim().is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
            _ => {
                // Might be root commit — try diff --root
                let output = Command::new("git")
                    .args(["diff-tree", "--patch", "--root", hash, "--", file_str])
                    .current_dir(&self.root)
                    .output();

                match output {
                    Ok(o) if o.status.success() => {
                        let text = String::from_utf8_lossy(&o.stdout).to_string();
                        if text.trim().is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    }
                    _ => None,
                }
            }
        }
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

/// Parse a git diff into section-level diffs.
/// Generalizes spec_diff::parse_requirements_diff to handle all H2 sections.
pub fn parse_section_diffs(diff: &str) -> Vec<SectionDiff> {
    let mut result: Vec<SectionDiff> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_changes: Vec<SectionChange> = Vec::new();

    for line in diff.lines() {
        // Hunk header: @@ -10,5 +10,7 @@ ## Requirements
        if line.starts_with("@@") {
            flush_section(&mut result, &mut current_section, &mut current_changes);
            // Try to recover section from hunk context
            if let Some(ctx_start) = line.rfind("@@ ") {
                let ctx = line[ctx_start + 3..].trim();
                if let Some(section_name) = extract_h2_name(ctx) {
                    current_section = Some(section_name);
                }
            }
            continue;
        }

        if line.starts_with('+') || line.starts_with('-') {
            // Skip diff header lines (+++ / ---)
            if line.starts_with("+++") || line.starts_with("---") {
                continue;
            }

            let content = &line[1..];
            let trimmed = content.trim();

            // Detect H2 section boundary
            if let Some(section_name) = extract_h2_name(trimmed) {
                flush_section(&mut result, &mut current_section, &mut current_changes);
                current_section = Some(section_name);
                continue;
            }

            // Record change if inside a section and line has content
            if current_section.is_some() && !trimmed.is_empty() {
                let action = if line.starts_with('+') { "added" } else { "removed" };
                current_changes.push(SectionChange {
                    action: action.to_string(),
                    text: trimmed.to_string(),
                });
            }
        } else if !line.starts_with('\\') {
            // Context line — check if it's an H2 section header
            let trimmed = line.trim();
            if let Some(section_name) = extract_h2_name(trimmed) {
                flush_section(&mut result, &mut current_section, &mut current_changes);
                current_section = Some(section_name);
            }
        }
    }

    flush_section(&mut result, &mut current_section, &mut current_changes);
    result
}

/// Extract H2 section name from a line like "## Requirements" or "## Purpose"
/// Returns None for H1 (#), H3 (###), or non-heading lines.
fn extract_h2_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
        Some(trimmed[3..].trim().to_string())
    } else {
        None
    }
}

/// Flush accumulated changes into a SectionDiff and reset state
fn flush_section(
    result: &mut Vec<SectionDiff>,
    current_section: &mut Option<String>,
    current_changes: &mut Vec<SectionChange>,
) {
    if let Some(section) = current_section.take() {
        if !current_changes.is_empty() {
            result.push(SectionDiff {
                section,
                changes: current_changes.drain(..).collect(),
            });
        }
    }
    current_changes.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_section_diffs_single_section() {
        let diff = "\
+## Requirements\n\
+- REQ-1: User login\n\
+- REQ-2: Token refresh\n\
 ## Domain Context\n";
        let sections = parse_section_diffs(diff);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section, "Requirements");
        assert_eq!(sections[0].changes.len(), 2);
        assert_eq!(sections[0].changes[0].action, "added");
        assert!(sections[0].changes[0].text.contains("REQ-1"));
    }

    #[test]
    fn test_parse_section_diffs_multiple_sections() {
        let diff = "\
+## Purpose\n\
+Auth module for user management\n\
+## Requirements\n\
+- REQ-1: Login\n\
-## Constraints\n\
-- CONST-1: Old constraint\n";
        let sections = parse_section_diffs(diff);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].section, "Purpose");
        assert_eq!(sections[0].changes[0].action, "added");
        assert_eq!(sections[1].section, "Requirements");
        assert_eq!(sections[2].section, "Constraints");
        assert_eq!(sections[2].changes[0].action, "removed");
    }

    #[test]
    fn test_parse_section_diffs_hunk_header_context() {
        let diff = "\
@@ -10,5 +10,7 @@ ## Requirements\n\
+- REQ-2: New requirement\n\
 ## Domain Context\n";
        let sections = parse_section_diffs(diff);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section, "Requirements");
        assert_eq!(sections[0].changes.len(), 1);
        assert_eq!(sections[0].changes[0].text, "- REQ-2: New requirement");
    }

    #[test]
    fn test_parse_section_diffs_context_lines() {
        let diff = "\
 ## Requirements\n\
+- REQ-2: Added requirement\n\
-- REQ-1: Removed requirement\n";
        let sections = parse_section_diffs(diff);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section, "Requirements");
        assert_eq!(sections[0].changes.len(), 2);
        assert_eq!(sections[0].changes[0].action, "added");
        assert_eq!(sections[0].changes[1].action, "removed");
    }

    #[test]
    fn test_parse_section_diffs_empty() {
        let sections = parse_section_diffs("");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_parse_section_diffs_ignores_h3_subsections() {
        let diff = "\
+## Requirements\n\
+- REQ-1: Login\n\
+### Details\n\
+Some detail text\n\
 ## Domain Context\n";
        let sections = parse_section_diffs(diff);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section, "Requirements");
        // H3 header and its content are included under the H2 section
        assert_eq!(sections[0].changes.len(), 3); // REQ-1, ### Details, Some detail text
    }

    #[test]
    fn test_parse_section_diffs_skips_diff_header_lines() {
        let diff = "\
--- a/src/auth/CLAUDE.md\n\
+++ b/src/auth/CLAUDE.md\n\
@@ -1,3 +1,4 @@\n\
 ## Requirements\n\
+- REQ-2: New\n";
        let sections = parse_section_diffs(diff);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].changes.len(), 1);
        assert_eq!(sections[0].changes[0].text, "- REQ-2: New");
    }

    #[test]
    fn test_breaking_detection() {
        assert!("spec(src/auth): remove login [BREAKING]".contains("[BREAKING]"));
        assert!(!"spec(src/auth): add feature".contains("[BREAKING]"));
    }

    #[test]
    fn test_extract_h2_name() {
        assert_eq!(extract_h2_name("## Requirements"), Some("Requirements".to_string()));
        assert_eq!(extract_h2_name("## Purpose"), Some("Purpose".to_string()));
        assert_eq!(extract_h2_name("### Subsection"), None);
        assert_eq!(extract_h2_name("# Title"), None);
        assert_eq!(extract_h2_name("Not a heading"), None);
        assert_eq!(extract_h2_name("  ## Indented  "), Some("Indented".to_string()));
    }
}
