use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Include generated constants from schema-rules.yaml (SSOT)
include!(concat!(env!("OUT_DIR"), "/schema_rules.rs"));

/// Result of convention validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ConventionValidationResult {
    pub project_root: String,
    pub conventions: ConventionCheck,
    pub module_roots: Vec<ModuleConventionResult>,
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Check result for a single convention section
#[derive(Debug, Serialize, Deserialize)]
pub struct ConventionCheck {
    pub valid: bool,
    pub file: String,
    pub section_found: bool,
    pub required_subsections: HashMap<String, bool>,
    pub errors: Vec<String>,
}

/// Convention check result for a module root
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleConventionResult {
    pub path: String,
    pub conventions: ConventionCheck,
}

pub struct ConventionValidator {
    h2_pattern: Regex,
    h3_pattern: Regex,
}

impl ConventionValidator {
    pub fn new() -> Self {
        Self {
            h2_pattern: Regex::new(r"^##\s+(.+)$").unwrap(),
            h3_pattern: Regex::new(r"^###\s+(.+)$").unwrap(),
        }
    }

    /// Main validation entry point
    pub fn validate(
        &self,
        project_root: &Path,
        module_roots: Option<Vec<PathBuf>>,
    ) -> ConventionValidationResult {
        let mut errors = Vec::new();

        // 1. Check project_root CLAUDE.md for Conventions
        let project_claude_md = project_root.join("CLAUDE.md");
        let conventions = self.check_file_section(
            &project_claude_md,
            "Conventions",
            CONVENTIONS_REQUIRED_SUBSECTIONS,
        );

        if !conventions.valid {
            for err in &conventions.errors {
                errors.push(err.clone());
            }
        }

        // 2. Determine module roots
        let detected_modules = match module_roots {
            Some(roots) => roots,
            None => self.find_module_roots(project_root),
        };

        // 3. Check each module root
        let mut module_results = Vec::new();
        for module_root in &detected_modules {
            let module_claude_md = module_root.join("CLAUDE.md");

            // Conventions check
            // Multi-module: optional (inherits from project_root if absent)
            // Single-module (project_root == module_root): already validated above
            let module_conventions = self.check_file_section(
                &module_claude_md,
                "Conventions",
                CONVENTIONS_REQUIRED_SUBSECTIONS,
            );

            let is_multi_module = module_root != project_root;

            if !module_conventions.valid {
                if is_multi_module && !module_conventions.section_found {
                    // Multi-module: Conventions absent = inherited from project_root → OK
                } else if module_root != project_root {
                    // Non-root module with malformed section → report errors
                    for err in &module_conventions.errors {
                        errors.push(err.clone());
                    }
                }
            }

            module_results.push(ModuleConventionResult {
                path: module_root.to_string_lossy().to_string(),
                conventions: module_conventions,
            });
        }

        let valid = errors.is_empty();

        ConventionValidationResult {
            project_root: project_root.to_string_lossy().to_string(),
            conventions,
            module_roots: module_results,
            valid,
            errors,
        }
    }

    /// Find module roots by looking for build marker files
    pub fn find_module_roots(&self, project_root: &Path) -> Vec<PathBuf> {
        let mut module_roots = Vec::new();

        // Check if project_root itself is a module root
        if self.is_module_root(project_root) {
            module_roots.push(project_root.to_path_buf());
        }

        // Walk subdirectories (one level deep for common monorepo patterns)
        self.find_module_roots_recursive(project_root, project_root, &mut module_roots, 0, 3);

        // If no module roots found, treat project_root as the sole module root
        if module_roots.is_empty() {
            module_roots.push(project_root.to_path_buf());
        }

        module_roots
    }

    fn find_module_roots_recursive(
        &self,
        dir: &Path,
        project_root: &Path,
        results: &mut Vec<PathBuf>,
        depth: usize,
        max_depth: usize,
    ) {
        if depth > max_depth {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Skip hidden directories and common non-source dirs
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with('.')
                || crate::EXCLUDED_DIRS.contains(&name.as_ref())
            {
                continue;
            }

            if self.is_module_root(&path) && path != project_root {
                if !results.contains(&path) {
                    results.push(path.clone());
                }
            }

            self.find_module_roots_recursive(&path, project_root, results, depth + 1, max_depth);
        }
    }

    fn is_module_root(&self, dir: &Path) -> bool {
        MODULE_ROOT_MARKERS
            .iter()
            .any(|marker| dir.join(marker).exists())
    }

    /// Check a CLAUDE.md file for a specific convention section and its required subsections
    fn check_file_section(
        &self,
        claude_md_path: &Path,
        section_name: &str,
        required_subsections: &[&str],
    ) -> ConventionCheck {
        let file_str = claude_md_path.to_string_lossy().to_string();

        let content = match std::fs::read_to_string(claude_md_path) {
            Ok(c) => c,
            Err(_) => {
                return ConventionCheck {
                    valid: false,
                    file: file_str.clone(),
                    section_found: false,
                    required_subsections: required_subsections
                        .iter()
                        .map(|s| (s.to_string(), false))
                        .collect(),
                    errors: vec![format!(
                        "{}: CLAUDE.md not found at {}",
                        section_name, file_str
                    )],
                };
            }
        };

        // Find the H2 section
        let section_content = self.extract_h2_section(&content, section_name);

        match section_content {
            None => ConventionCheck {
                valid: false,
                file: file_str,
                section_found: false,
                required_subsections: required_subsections
                    .iter()
                    .map(|s| (s.to_string(), false))
                    .collect(),
                errors: vec![format!(
                    "{}: section '## {}' not found",
                    claude_md_path.display(),
                    section_name
                )],
            },
            Some(section_text) => {
                let mut errors = Vec::new();
                let mut subsection_map = HashMap::new();

                // Check each required subsection (H3)
                for subsection in required_subsections {
                    let found = self.has_h3_subsection(&section_text, subsection);
                    subsection_map.insert(subsection.to_string(), found);
                    if !found {
                        errors.push(format!(
                            "{}: missing required subsection '### {}' in '## {}'",
                            claude_md_path.display(),
                            subsection,
                            section_name
                        ));
                    }
                }

                ConventionCheck {
                    valid: errors.is_empty(),
                    file: file_str,
                    section_found: true,
                    required_subsections: subsection_map,
                    errors,
                }
            }
        }
    }

    /// Extract the content of an H2 section (from ## header to next ## header or EOF)
    fn extract_h2_section(&self, content: &str, section_name: &str) -> Option<String> {
        let mut in_section = false;
        let mut section_lines = Vec::new();

        for line in content.lines() {
            if let Some(caps) = self.h2_pattern.captures(line) {
                if in_section {
                    // Hit the next H2 section, stop
                    break;
                }
                let header = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if header.eq_ignore_ascii_case(section_name) {
                    in_section = true;
                    continue;
                }
            } else if in_section {
                section_lines.push(line);
            }
        }

        if in_section {
            Some(section_lines.join("\n"))
        } else {
            None
        }
    }

    /// Check if section content contains a specific H3 subsection
    fn has_h3_subsection(&self, section_content: &str, subsection_name: &str) -> bool {
        for line in section_content.lines() {
            if let Some(caps) = self.h3_pattern.captures(line) {
                let header = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if header.eq_ignore_ascii_case(subsection_name) {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for ConventionValidator {
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

    fn create_claude_md(dir: &Path, content: &str) {
        let file_path = dir.join("CLAUDE.md");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
    }

    const VALID_CONVENTIONS: &str = r#"
## Conventions

### Project Structure
Layered architecture with src/ containing all source code.

### Module Boundaries
Each module is self-contained and communicates through public APIs.

### Naming Conventions
camelCase for files, PascalCase for classes.

### Language & Runtime
TypeScript 5.0, Node.js 20 LTS

### Coding Rules
- 비동기: async/await 사용, raw Promise 금지
- 타입: strict mode, any 금지

### Naming Rules
camelCase for variables and functions, PascalCase for types.
"#;

    #[test]
    fn test_conventions_valid() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create package.json to mark as module root
        File::create(root.join("package.json")).unwrap();

        create_claude_md(
            root,
            &format!("# My Project\n\n## Purpose\nA test project.\n{}", VALID_CONVENTIONS),
        );

        let validator = ConventionValidator::new();
        let result = validator.validate(root, None);

        assert!(result.valid, "Errors: {:?}", result.errors);
        assert!(result.conventions.section_found);
        assert!(result.conventions.valid);
    }

    #[test]
    fn test_conventions_missing_section() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        File::create(root.join("package.json")).unwrap();

        create_claude_md(
            root,
            "# My Project\n\n## Purpose\nA test project.\n",
        );

        let validator = ConventionValidator::new();
        let result = validator.validate(root, None);

        assert!(!result.valid);
        assert!(!result.conventions.section_found);
    }

    #[test]
    fn test_missing_required_subsection() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        File::create(root.join("package.json")).unwrap();

        create_claude_md(
            root,
            r#"# My Project

## Purpose
A test project.

## Conventions

### Project Structure
Layered architecture.

### Language & Runtime
TypeScript
"#,
        );

        let validator = ConventionValidator::new();
        let result = validator.validate(root, None);

        assert!(!result.valid);
        assert!(result.conventions.section_found);
        assert!(!result.conventions.valid);
        // Module Boundaries, Naming Conventions, Coding Rules, Naming Rules missing
        assert!(!result.conventions.required_subsections["Module Boundaries"]);
        assert!(!result.conventions.required_subsections["Naming Conventions"]);
        assert!(!result.conventions.required_subsections["Coding Rules"]);
        assert!(!result.conventions.required_subsections["Naming Rules"]);
    }

    #[test]
    fn test_find_module_roots_single_module() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        File::create(root.join("package.json")).unwrap();

        let validator = ConventionValidator::new();
        let roots = validator.find_module_roots(root);

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], root.to_path_buf());
    }

    #[test]
    fn test_find_module_roots_multi_module() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Root is also a module
        File::create(root.join("package.json")).unwrap();

        // Create sub-module
        let sub = root.join("packages").join("api");
        fs::create_dir_all(&sub).unwrap();
        File::create(sub.join("package.json")).unwrap();

        let validator = ConventionValidator::new();
        let roots = validator.find_module_roots(root);

        assert!(roots.len() >= 2);
        assert!(roots.contains(&root.to_path_buf()));
        assert!(roots.contains(&sub));
    }

    #[test]
    fn test_module_root_conventions_missing() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        File::create(root.join("package.json")).unwrap();

        // No Conventions section
        create_claude_md(
            root,
            "# My Project\n\n## Purpose\nA test project.\n",
        );

        let validator = ConventionValidator::new();
        let result = validator.validate(root, None);

        assert!(!result.valid);
        assert!(!result.module_roots.is_empty());
        assert!(!result.module_roots[0].conventions.section_found);
    }
}
