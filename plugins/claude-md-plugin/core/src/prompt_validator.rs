use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

// === Valid values ===

const VALID_TOOLS: &[&str] = &[
    "Bash", "Read", "Write", "Glob", "Grep", "Task", "Skill",
    "AskUserQuestion", "Edit", "WebFetch", "WebSearch",
    "NotebookRead", "TodoWrite",
];

const VALID_STATUSES: &[&str] = &["approve", "feedback", "warning", "error"];

// === Data Models ===

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PromptKind {
    Skill,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Vec<String>,
    pub version: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub trigger: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentFrontmatter {
    pub name: String,
    pub description: String,
    pub tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptValidationIssue {
    pub severity: Severity,
    pub kind: PromptKind,
    pub file: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CrossReferenceSummary {
    pub task_references: usize,
    pub skill_references: usize,
    pub unresolved_task_refs: Vec<(String, String)>,
    pub unresolved_skill_refs: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptValidationResult {
    pub root: String,
    pub skills_count: usize,
    pub agents_count: usize,
    pub valid: bool,
    pub issues: Vec<PromptValidationIssue>,
    pub cross_reference_summary: CrossReferenceSummary,
}

// === Validator ===

pub struct PromptValidator {
    task_ref_re: Regex,
    skill_ref_re: Regex,
    result_start_re: Regex,
    result_end_re: Regex,
}

impl PromptValidator {
    pub fn new() -> Self {
        Self {
            task_ref_re: Regex::new(r"Task\(([a-z][-a-z0-9]*)").unwrap(),
            skill_ref_re: Regex::new(r#"Skill\("(?:claude-md-plugin:)?([a-z][-a-z0-9]*)"\)"#).unwrap(),
            result_start_re: Regex::new(r"---([a-z][-a-z0-9]*)-result---").unwrap(),
            result_end_re: Regex::new(r"---end-([a-z][-a-z0-9]*)-result---").unwrap(),
        }
    }

    pub fn validate(&self, root: &Path) -> PromptValidationResult {
        let mut issues = Vec::new();
        let mut skill_names = HashSet::new();
        let mut agent_names = HashSet::new();
        let mut all_contents: Vec<(String, String)> = Vec::new(); // (file_path, content)

        let skills_dir = root.join("skills");
        let agents_dir = root.join("agents");

        // Phase 1: Scan skills
        if skills_dir.is_dir() {
            self.scan_skills(&skills_dir, &mut issues, &mut skill_names, &mut all_contents);
        }

        // Phase 2: Scan agents
        if agents_dir.is_dir() {
            self.scan_agents(&agents_dir, &mut issues, &mut agent_names, &mut all_contents);
        }

        // Phase 3: Cross-reference validation
        let cross_ref = self.validate_cross_references(&all_contents, &skill_names, &agent_names, &mut issues);

        // Phase 4: Result block validation
        self.validate_result_blocks(&all_contents, &mut issues);

        let valid = !issues.iter().any(|i| i.severity == Severity::Error);

        PromptValidationResult {
            root: root.to_string_lossy().to_string(),
            skills_count: skill_names.len(),
            agents_count: agent_names.len(),
            valid,
            issues,
            cross_reference_summary: cross_ref,
        }
    }

    fn scan_skills(
        &self,
        skills_dir: &Path,
        issues: &mut Vec<PromptValidationIssue>,
        skill_names: &mut HashSet<String>,
        all_contents: &mut Vec<(String, String)>,
    ) {
        let entries = match std::fs::read_dir(skills_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
            let file_str = skill_md.to_string_lossy().to_string();

            let content = match std::fs::read_to_string(&skill_md) {
                Ok(c) => c,
                Err(_) => {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Skill,
                        file: file_str,
                        message: "Failed to read SKILL.md".to_string(),
                    });
                    continue;
                }
            };

            all_contents.push((file_str.clone(), content.clone()));

            let (frontmatter_str, _body) = match Self::split_frontmatter(&content) {
                Some(parts) => parts,
                None => {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Skill,
                        file: file_str,
                        message: "Missing YAML frontmatter (no --- delimiters)".to_string(),
                    });
                    continue;
                }
            };

            let fm: SkillFrontmatter = match serde_yaml::from_str(&frontmatter_str) {
                Ok(f) => f,
                Err(e) => {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Skill,
                        file: file_str,
                        message: format!("Invalid YAML frontmatter: {}", e),
                    });
                    continue;
                }
            };

            skill_names.insert(fm.name.clone());

            // Validate name matches directory
            if fm.name != dir_name {
                issues.push(PromptValidationIssue {
                    severity: Severity::Error,
                    kind: PromptKind::Skill,
                    file: file_str.clone(),
                    message: format!(
                        "Skill name '{}' does not match directory name '{}'",
                        fm.name, dir_name
                    ),
                });
            }

            // Validate tools
            let valid_tools: HashSet<&str> = VALID_TOOLS.iter().copied().collect();
            for tool in &fm.allowed_tools {
                if !valid_tools.contains(tool.as_str()) {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Skill,
                        file: file_str.clone(),
                        message: format!("Invalid tool '{}' in allowed-tools", tool),
                    });
                }
            }
        }
    }

    fn scan_agents(
        &self,
        agents_dir: &Path,
        issues: &mut Vec<PromptValidationIssue>,
        agent_names: &mut HashSet<String>,
        all_contents: &mut Vec<(String, String)>,
    ) {
        let entries = match std::fs::read_dir(agents_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("md") {
                continue;
            }

            let file_stem = path.file_stem().unwrap().to_string_lossy().to_string();
            let file_str = path.to_string_lossy().to_string();

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Agent,
                        file: file_str,
                        message: "Failed to read agent file".to_string(),
                    });
                    continue;
                }
            };

            all_contents.push((file_str.clone(), content.clone()));

            let (frontmatter_str, _body) = match Self::split_frontmatter(&content) {
                Some(parts) => parts,
                None => {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Agent,
                        file: file_str,
                        message: "Missing YAML frontmatter (no --- delimiters)".to_string(),
                    });
                    continue;
                }
            };

            let fm: AgentFrontmatter = match serde_yaml::from_str(&frontmatter_str) {
                Ok(f) => f,
                Err(e) => {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Agent,
                        file: file_str,
                        message: format!("Invalid YAML frontmatter: {}", e),
                    });
                    continue;
                }
            };

            agent_names.insert(fm.name.clone());

            // Validate name matches filename
            if fm.name != file_stem {
                issues.push(PromptValidationIssue {
                    severity: Severity::Error,
                    kind: PromptKind::Agent,
                    file: file_str.clone(),
                    message: format!(
                        "Agent name '{}' does not match filename '{}'",
                        fm.name, file_stem
                    ),
                });
            }

            // Warn if tools missing
            if fm.tools.is_none() {
                issues.push(PromptValidationIssue {
                    severity: Severity::Warning,
                    kind: PromptKind::Agent,
                    file: file_str.clone(),
                    message: "Agent missing 'tools' field".to_string(),
                });
            }

            // Validate tool names if present
            if let Some(ref tools) = fm.tools {
                let valid_tools: HashSet<&str> = VALID_TOOLS.iter().copied().collect();
                for tool in tools {
                    if !valid_tools.contains(tool.as_str()) {
                        issues.push(PromptValidationIssue {
                            severity: Severity::Error,
                            kind: PromptKind::Agent,
                            file: file_str.clone(),
                            message: format!("Invalid tool '{}' in tools", tool),
                        });
                    }
                }
            }
        }
    }

    fn validate_cross_references(
        &self,
        all_contents: &[(String, String)],
        skill_names: &HashSet<String>,
        agent_names: &HashSet<String>,
        issues: &mut Vec<PromptValidationIssue>,
    ) -> CrossReferenceSummary {
        let mut summary = CrossReferenceSummary::default();

        for (file, content) in all_contents {
            // Extract Task() references
            for cap in self.task_ref_re.captures_iter(content) {
                let agent_name = cap[1].to_string();
                summary.task_references += 1;

                if !agent_names.contains(&agent_name) {
                    summary
                        .unresolved_task_refs
                        .push((agent_name.clone(), file.clone()));
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Skill,
                        file: file.clone(),
                        message: format!(
                            "Unresolved Task reference: agent '{}' not found",
                            agent_name
                        ),
                    });
                }
            }

            // Extract Skill() references
            for cap in self.skill_ref_re.captures_iter(content) {
                let skill_name = cap[1].to_string();
                summary.skill_references += 1;

                if !skill_names.contains(&skill_name) {
                    summary
                        .unresolved_skill_refs
                        .push((skill_name.clone(), file.clone()));
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Skill,
                        file: file.clone(),
                        message: format!(
                            "Unresolved Skill reference: skill '{}' not found",
                            skill_name
                        ),
                    });
                }
            }
        }

        summary
    }

    fn validate_result_blocks(
        &self,
        all_contents: &[(String, String)],
        issues: &mut Vec<PromptValidationIssue>,
    ) {
        for (file, content) in all_contents {
            let starts: Vec<(usize, String)> = self
                .result_start_re
                .captures_iter(content)
                .filter(|cap| !cap[1].starts_with("end-"))
                .map(|cap| {
                    let m = cap.get(0).unwrap();
                    (m.start(), cap[1].to_string())
                })
                .collect();

            let ends: Vec<(usize, String)> = self
                .result_end_re
                .captures_iter(content)
                .map(|cap| {
                    let m = cap.get(0).unwrap();
                    (m.start(), cap[1].to_string())
                })
                .collect();

            // Check that each start has a matching end
            for (start_pos, start_name) in &starts {
                let has_matching_end = ends.iter().any(|(end_pos, end_name)| {
                    end_name == start_name && end_pos > start_pos
                });

                if !has_matching_end {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Agent,
                        file: file.clone(),
                        message: format!(
                            "Result block '---{}-result---' has no matching end delimiter",
                            start_name
                        ),
                    });
                }
            }

            // Check for end without start
            for (end_pos, end_name) in &ends {
                let has_matching_start = starts.iter().any(|(start_pos, start_name)| {
                    start_name == end_name && start_pos < end_pos
                });

                if !has_matching_start {
                    issues.push(PromptValidationIssue {
                        severity: Severity::Error,
                        kind: PromptKind::Agent,
                        file: file.clone(),
                        message: format!(
                            "Result block '---end-{}-result---' has no matching start delimiter",
                            end_name
                        ),
                    });
                }
            }

            // Check status fields within result blocks
            self.validate_status_in_result_blocks(file, content, &starts, &ends, issues);
        }
    }

    fn validate_status_in_result_blocks(
        &self,
        file: &str,
        content: &str,
        starts: &[(usize, String)],
        ends: &[(usize, String)],
        issues: &mut Vec<PromptValidationIssue>,
    ) {
        // For each matched start-end pair, check the top-level status field only
        for (start_pos, start_name) in starts {
            // Find the nearest matching end
            let end_pos = ends
                .iter()
                .filter(|(ep, en)| en == start_name && ep > start_pos)
                .map(|(ep, _)| *ep)
                .min();

            if let Some(end_pos) = end_pos {
                let block_content = &content[*start_pos..end_pos];

                // Find the first status: line (top-level status only)
                // Top-level status is the one with minimal indentation
                let status_line = block_content.lines().find(|line| {
                    let trimmed = line.trim_start();
                    if !trimmed.starts_with("status:") {
                        return false;
                    }
                    // Only match top-level status (not indented sub-item status)
                    let indent = line.len() - trimmed.len();
                    indent <= 2 // Allow 0-2 spaces for top-level
                });

                match status_line {
                    None => {
                        issues.push(PromptValidationIssue {
                            severity: Severity::Error,
                            kind: PromptKind::Agent,
                            file: file.to_string(),
                            message: format!(
                                "Result block '{}' missing 'status' field",
                                start_name
                            ),
                        });
                    }
                    Some(line) => {
                        let trimmed = line.trim_start();
                        let status_value = trimmed.trim_start_matches("status:").trim();

                        // Handle template patterns like "approve | feedback" or "{approve|warning}"
                        let status_parts: Vec<&str> = status_value
                            .trim_matches(|c| c == '{' || c == '}')
                            .split('|')
                            .map(|s| s.trim())
                            .collect();

                        let valid_set: HashSet<&str> = VALID_STATUSES.iter().copied().collect();
                        for part in &status_parts {
                            if !part.is_empty() && !valid_set.contains(part) {
                                issues.push(PromptValidationIssue {
                                    severity: Severity::Error,
                                    kind: PromptKind::Agent,
                                    file: file.to_string(),
                                    message: format!(
                                        "Invalid status value '{}' in result block '{}'",
                                        part, start_name
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn split_frontmatter(content: &str) -> Option<(String, String)> {
        let content = content.trim_start();
        if !content.starts_with("---") {
            return None;
        }

        let after_first = &content[3..];
        let end_pos = after_first.find("\n---")?;

        let frontmatter = after_first[..end_pos].trim().to_string();
        let body = after_first[end_pos + 4..].to_string();

        Some((frontmatter, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("skills")).unwrap();
        fs::create_dir_all(dir.path().join("agents")).unwrap();
        dir
    }

    #[test]
    fn test_split_frontmatter_valid() {
        let content = "---\nname: test\ndescription: hello\n---\nBody here";
        let (fm, body) = PromptValidator::split_frontmatter(content).unwrap();
        assert_eq!(fm, "name: test\ndescription: hello");
        assert!(body.contains("Body here"));
    }

    #[test]
    fn test_split_frontmatter_missing() {
        let content = "No frontmatter here";
        assert!(PromptValidator::split_frontmatter(content).is_none());
    }

    #[test]
    fn test_empty_directory() {
        let dir = setup_test_dir();
        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(result.valid);
        assert_eq!(result.skills_count, 0);
        assert_eq!(result.agents_count, 0);
    }

    #[test]
    fn test_valid_skill() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("skills").join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A test skill\nallowed-tools: [Read, Write]\n---\nBody",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(result.valid);
        assert_eq!(result.skills_count, 1);
    }

    #[test]
    fn test_skill_name_mismatch() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("skills").join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: wrong-name\ndescription: A test skill\nallowed-tools: [Read]\n---\nBody",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| i.message.contains("does not match directory")));
    }

    #[test]
    fn test_invalid_tool() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("skills").join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\nallowed-tools: [Read, InvalidTool]\n---\nBody",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| i.message.contains("InvalidTool")));
    }

    #[test]
    fn test_valid_agent() {
        let dir = setup_test_dir();
        fs::write(
            dir.path().join("agents").join("my-agent.md"),
            "---\nname: my-agent\ndescription: A test agent\ntools: [Read, Task]\n---\nBody",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(result.valid);
        assert_eq!(result.agents_count, 1);
    }

    #[test]
    fn test_agent_missing_tools_warning() {
        let dir = setup_test_dir();
        fs::write(
            dir.path().join("agents").join("my-agent.md"),
            "---\nname: my-agent\ndescription: A test agent\n---\nBody",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        // Should pass (warning only)
        assert!(result.valid);
        assert!(result.issues.iter().any(|i| {
            i.severity == Severity::Warning && i.message.contains("tools")
        }));
    }

    #[test]
    fn test_cross_reference_task() {
        let dir = setup_test_dir();
        // Create agent
        fs::write(
            dir.path().join("agents").join("my-agent.md"),
            "---\nname: my-agent\ndescription: test\ntools: [Read]\n---\nBody",
        )
        .unwrap();
        // Create skill referencing agent
        let skill_dir = dir.path().join("skills").join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\nallowed-tools: [Task]\n---\nTask(my-agent)",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(result.valid);
        assert_eq!(result.cross_reference_summary.task_references, 1);
        assert_eq!(result.cross_reference_summary.unresolved_task_refs.len(), 0);
    }

    #[test]
    fn test_cross_reference_unresolved() {
        let dir = setup_test_dir();
        let skill_dir = dir.path().join("skills").join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\nallowed-tools: [Task]\n---\nTask(nonexistent-agent)",
        )
        .unwrap();

        let validator = PromptValidator::new();
        let result = validator.validate(dir.path());
        assert!(!result.valid);
        assert_eq!(result.cross_reference_summary.unresolved_task_refs.len(), 1);
    }
}
