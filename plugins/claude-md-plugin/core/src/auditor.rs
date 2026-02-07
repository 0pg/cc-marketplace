//! Auditor for CLAUDE.md completeness analysis.
//!
//! Lists ALL directories and compares expected vs actual CLAUDE.md presence.
//! Unlike `parse-tree` which only returns directories meeting CON-1 criteria,
//! this module provides a complete audit of the directory tree.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::tree_utils::DirScanner;

/// Status of a directory's CLAUDE.md documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    /// meets_con1=true AND has_claude_md=true
    Complete,
    /// meets_con1=true AND has_claude_md=false
    Missing,
    /// meets_con1=false AND has_claude_md=true
    Unexpected,
    /// meets_con1=false AND has_claude_md=false
    NotRequired,
}

/// Information about a directory for audit purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditNode {
    /// Path relative to root
    pub path: PathBuf,
    /// Number of source files directly in this directory
    pub source_file_count: usize,
    /// Number of subdirectories (non-excluded)
    pub subdir_count: usize,
    /// Depth from root (root = 0)
    pub depth: usize,
    /// Whether this directory meets CON-1 criteria
    pub meets_con1: bool,
    /// Whether CLAUDE.md exists in this directory
    pub has_claude_md: bool,
    /// Whether IMPLEMENTS.md exists in this directory
    pub has_implements_md: bool,
    /// Audit status based on meets_con1 and has_claude_md
    pub status: AuditStatus,
}

/// Summary statistics for the audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    /// Total number of directories scanned
    pub total_directories: usize,
    /// Directories meeting CON-1 criteria
    pub meets_con1: usize,
    /// Directories with CLAUDE.md
    pub has_claude_md: usize,
    /// Directories with IMPLEMENTS.md
    pub has_implements_md: usize,
    /// Complete: meets_con1 && has_claude_md
    pub complete: usize,
    /// Missing: meets_con1 && !has_claude_md
    pub missing: usize,
    /// Unexpected: !meets_con1 && has_claude_md
    pub unexpected: usize,
    /// NotRequired: !meets_con1 && !has_claude_md
    pub not_required: usize,
}

/// Result of directory audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    /// Root directory that was scanned
    pub root: PathBuf,
    /// Timestamp when audit was performed
    pub audited_at: String,
    /// All audited directory nodes
    pub nodes: Vec<AuditNode>,
    /// Directories that were excluded
    pub excluded: Vec<PathBuf>,
    /// Summary statistics
    pub summary: AuditSummary,
}

/// Auditor for CLAUDE.md completeness
pub struct Auditor {
    scanner: DirScanner,
}

impl Auditor {
    pub fn new() -> Self {
        Self {
            scanner: DirScanner::new(),
        }
    }

    /// Audit a directory tree for CLAUDE.md completeness
    ///
    /// # Arguments
    /// * `root` - Root directory to scan
    /// * `only_issues` - If true, only return nodes with status "missing" or "unexpected"
    pub fn audit(&self, root: &Path, only_issues: bool) -> AuditResult {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut nodes = Vec::new();

        let (dirs_to_check, excluded) = self.scanner.collect_directories(&root);

        // Audit each directory
        for dir in dirs_to_check {
            let node = self.audit_directory(&root, &dir);

            // Filter if only_issues is true
            if only_issues {
                if node.status == AuditStatus::Missing || node.status == AuditStatus::Unexpected {
                    nodes.push(node);
                }
            } else {
                nodes.push(node);
            }
        }

        // Sort by path for consistent output
        nodes.sort_by(|a, b| a.path.cmp(&b.path));

        // Calculate summary
        let summary = Self::calculate_summary(&nodes);

        // Get current timestamp
        let audited_at = chrono::Utc::now().to_rfc3339();

        AuditResult {
            root,
            audited_at,
            nodes,
            excluded,
            summary,
        }
    }

    fn audit_directory(&self, root: &Path, dir: &Path) -> AuditNode {
        let source_file_count = self.scanner.count_source_files(dir);
        let subdir_count = self.scanner.count_subdirs(dir);
        let relative_path = self.scanner.make_relative(root, dir);
        let depth = self.scanner.calculate_depth(&relative_path);

        // CON-1: CLAUDE.md needed if 1+ source files OR 2+ subdirs
        let meets_con1 = source_file_count >= 1 || subdir_count >= 2;

        // Check file existence
        let has_claude_md = dir.join("CLAUDE.md").exists();
        let has_implements_md = dir.join("IMPLEMENTS.md").exists();

        // Calculate status
        let status = match (meets_con1, has_claude_md) {
            (true, true) => AuditStatus::Complete,
            (true, false) => AuditStatus::Missing,
            (false, true) => AuditStatus::Unexpected,
            (false, false) => AuditStatus::NotRequired,
        };

        AuditNode {
            path: relative_path,
            source_file_count,
            subdir_count,
            depth,
            meets_con1,
            has_claude_md,
            has_implements_md,
            status,
        }
    }

    fn calculate_summary(nodes: &[AuditNode]) -> AuditSummary {
        let mut summary = AuditSummary {
            total_directories: nodes.len(),
            meets_con1: 0,
            has_claude_md: 0,
            has_implements_md: 0,
            complete: 0,
            missing: 0,
            unexpected: 0,
            not_required: 0,
        };

        for node in nodes {
            if node.meets_con1 {
                summary.meets_con1 += 1;
            }
            if node.has_claude_md {
                summary.has_claude_md += 1;
            }
            if node.has_implements_md {
                summary.has_implements_md += 1;
            }
            match node.status {
                AuditStatus::Complete => summary.complete += 1,
                AuditStatus::Missing => summary.missing += 1,
                AuditStatus::Unexpected => summary.unexpected += 1,
                AuditStatus::NotRequired => summary.not_required += 1,
            }
        }

        summary
    }
}

impl Default for Auditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_complete_status() {
        let temp = create_test_dir();
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        File::create(auth_dir.join("token.rs")).unwrap();
        File::create(auth_dir.join("CLAUDE.md")).unwrap();

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        let auth_node = result
            .nodes
            .iter()
            .find(|n| n.path.ends_with("auth"))
            .expect("auth should be audited");

        assert!(auth_node.meets_con1);
        assert!(auth_node.has_claude_md);
        assert_eq!(auth_node.status, AuditStatus::Complete);
    }

    #[test]
    fn test_missing_status() {
        let temp = create_test_dir();
        let utils_dir = temp.path().join("src").join("utils");
        fs::create_dir_all(&utils_dir).unwrap();
        File::create(utils_dir.join("helpers.ts")).unwrap();
        // No CLAUDE.md

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        let utils_node = result
            .nodes
            .iter()
            .find(|n| n.path.ends_with("utils"))
            .expect("utils should be audited");

        assert!(utils_node.meets_con1);
        assert!(!utils_node.has_claude_md);
        assert_eq!(utils_node.status, AuditStatus::Missing);
    }

    #[test]
    fn test_unexpected_status() {
        let temp = create_test_dir();
        let empty_dir = temp.path().join("docs");
        fs::create_dir_all(&empty_dir).unwrap();
        File::create(empty_dir.join("CLAUDE.md")).unwrap();
        // No source files, no subdirs, but has CLAUDE.md

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        let docs_node = result
            .nodes
            .iter()
            .find(|n| n.path.ends_with("docs"))
            .expect("docs should be audited");

        assert!(!docs_node.meets_con1);
        assert!(docs_node.has_claude_md);
        assert_eq!(docs_node.status, AuditStatus::Unexpected);
    }

    #[test]
    fn test_not_required_status() {
        let temp = create_test_dir();
        let empty_dir = temp.path().join("assets");
        fs::create_dir_all(&empty_dir).unwrap();
        File::create(empty_dir.join("logo.png")).unwrap();
        // No source files, no subdirs, no CLAUDE.md

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        let assets_node = result
            .nodes
            .iter()
            .find(|n| n.path.ends_with("assets"))
            .expect("assets should be audited");

        assert!(!assets_node.meets_con1);
        assert!(!assets_node.has_claude_md);
        assert_eq!(assets_node.status, AuditStatus::NotRequired);
    }

    #[test]
    fn test_only_issues_filter() {
        let temp = create_test_dir();

        // Complete (should be filtered out)
        let complete_dir = temp.path().join("complete");
        fs::create_dir_all(&complete_dir).unwrap();
        File::create(complete_dir.join("main.rs")).unwrap();
        File::create(complete_dir.join("CLAUDE.md")).unwrap();

        // Missing (should be included)
        let missing_dir = temp.path().join("missing");
        fs::create_dir_all(&missing_dir).unwrap();
        File::create(missing_dir.join("main.rs")).unwrap();

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), true);

        // Should only contain "missing" directory
        assert!(result.nodes.iter().any(|n| n.path.ends_with("missing")));
        assert!(!result.nodes.iter().any(|n| n.path.ends_with("complete")));
    }

    #[test]
    fn test_implements_md_detection() {
        let temp = create_test_dir();
        let auth_dir = temp.path().join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        File::create(auth_dir.join("token.rs")).unwrap();
        File::create(auth_dir.join("CLAUDE.md")).unwrap();
        File::create(auth_dir.join("IMPLEMENTS.md")).unwrap();

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        let auth_node = result
            .nodes
            .iter()
            .find(|n| n.path.ends_with("auth"))
            .expect("auth should be audited");

        assert!(auth_node.has_implements_md);
    }

    #[test]
    fn test_summary_calculation() {
        let temp = create_test_dir();

        // Complete
        let complete_dir = temp.path().join("complete");
        fs::create_dir_all(&complete_dir).unwrap();
        File::create(complete_dir.join("main.rs")).unwrap();
        File::create(complete_dir.join("CLAUDE.md")).unwrap();

        // Missing
        let missing_dir = temp.path().join("missing");
        fs::create_dir_all(&missing_dir).unwrap();
        File::create(missing_dir.join("main.rs")).unwrap();

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        assert!(result.summary.complete >= 1);
        assert!(result.summary.missing >= 1);
    }

    #[test]
    fn test_excluded_directories() {
        let temp = create_test_dir();
        let node_modules = temp.path().join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();
        File::create(node_modules.join("index.js")).unwrap();

        let auditor = Auditor::new();
        let result = auditor.audit(temp.path(), false);

        assert!(result.excluded.iter().any(|p| p.ends_with("node_modules")));
        assert!(!result.nodes.iter().any(|n| n.path.to_string_lossy().contains("node_modules")));
    }
}
