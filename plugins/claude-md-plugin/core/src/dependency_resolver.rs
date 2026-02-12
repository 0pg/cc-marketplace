use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::code_analyzer::{AnalysisResult, InternalDependency, ResolutionStatus};
use crate::tree_parser::TreeResult;

/// Resolves raw import paths to CLAUDE.md directory paths.
pub struct DependencyResolver {
    /// Set of project-root-relative directories that have (or will have) CLAUDE.md
    claude_md_dirs: HashSet<PathBuf>,
}

impl DependencyResolver {
    /// Build from tree-parse results.
    pub fn new(tree_result: &TreeResult) -> Self {
        let claude_md_dirs: HashSet<PathBuf> = tree_result
            .needs_claude_md
            .iter()
            .map(|info| info.path.clone())
            .collect();

        Self { claude_md_dirs }
    }

    /// Resolve all internal deps in an AnalysisResult.
    /// `source_dir` is the project-root-relative path of the directory being analyzed.
    pub fn resolve(&self, analysis: &mut AnalysisResult, source_dir: &Path) {
        let resolved: Vec<InternalDependency> = analysis
            .dependencies
            .internal_raw
            .iter()
            .map(|raw| {
                let mut dep = self.resolve_one(raw, source_dir);
                // INV-1: child relationship check
                dep.is_child = !dep.resolved_dir.is_empty()
                    && Path::new(&dep.resolved_dir).starts_with(source_dir);
                dep
            })
            .collect();

        analysis.dependencies.internal = resolved;
    }

    /// Resolve a single raw import path to an InternalDependency.
    fn resolve_one(&self, raw_import: &str, source_dir: &Path) -> InternalDependency {
        let normalized = self.normalize(raw_import, source_dir);

        match normalized {
            Some(dir_path) => self.find_claude_md(&dir_path, raw_import),
            None => InternalDependency {
                raw_import: raw_import.to_string(),
                resolved_dir: String::new(),
                claude_md_path: String::new(),
                resolution: ResolutionStatus::Unresolved,
                is_child: false,
            },
        }
    }

    /// Normalize an import path to a project-root-relative directory path.
    fn normalize(&self, raw_import: &str, source_dir: &Path) -> Option<PathBuf> {
        let trimmed = raw_import.trim();

        if trimmed.is_empty() {
            return None;
        }

        // 1. Relative path: starts with './' or '../'
        if trimmed.starts_with("./") || trimmed.starts_with("../") {
            let resolved = source_dir.join(trimmed);
            return Some(normalize_path(&resolved));
        }

        // 2. Gradle module path: contains ':' (e.g., "vendors:vendor-common")
        if trimmed.contains(':') {
            let converted = trimmed.replace(':', "/");
            return Some(PathBuf::from(converted));
        }

        // 3. Package path: contains '.' but no '/' (e.g., "core.domain.transaction")
        //    Exclude cases that look like filenames (e.g., "file.ts")
        if trimmed.contains('.') && !trimmed.contains('/') {
            const KNOWN_EXTENSIONS: &[&str] = &[
                "ts", "js", "py", "rs", "go", "java", "kt", "tsx", "jsx",
                "json", "yaml", "yml", "toml", "md",
                "swift", "rb", "cs", "cpp", "hpp", "c", "h", "sh",
                "sql", "proto", "graphql", "dart", "scala",
                "ex", "exs", "clj", "vue", "svelte",
            ];

            let parts: Vec<&str> = trimmed.split('.').collect();
            // Known file extension → treat as file path
            if parts.len() == 2 && KNOWN_EXTENSIONS.contains(&parts[1]) {
                return Some(PathBuf::from(trimmed));
            }
            // 2-part with unknown extension → treat as file path (e.g., README.txt)
            if parts.len() == 2 {
                return Some(PathBuf::from(trimmed));
            }
            // 3+ dots → package path (e.g., com.example.module → com/example/module)
            if parts.len() >= 3 {
                let converted = trimmed.replace('.', "/");
                return Some(PathBuf::from(converted));
            }
        }

        // 4. Direct path: already looks path-like (contains '/')
        //    Or a simple module name (no separators)
        Some(PathBuf::from(trimmed))
    }

    /// Find the CLAUDE.md for a normalized directory path.
    /// Tries exact match first, then walks up ancestors.
    fn find_claude_md(&self, dir_path: &Path, raw_import: &str) -> InternalDependency {
        let dir_str = dir_path.to_string_lossy().to_string();

        // Exact match
        if self.claude_md_dirs.contains(dir_path) {
            return InternalDependency {
                raw_import: raw_import.to_string(),
                resolved_dir: dir_str.clone(),
                claude_md_path: format!("{}/CLAUDE.md", dir_str),
                resolution: ResolutionStatus::Exact,
                is_child: false, // set by resolve()
            };
        }

        // Walk up ancestors
        let mut current = dir_path.parent();
        let mut distance = 1usize;

        while let Some(ancestor) = current {
            if ancestor.as_os_str().is_empty() {
                break;
            }

            if self.claude_md_dirs.contains(ancestor) {
                let ancestor_str = ancestor.to_string_lossy().to_string();
                return InternalDependency {
                    raw_import: raw_import.to_string(),
                    resolved_dir: ancestor_str.clone(),
                    claude_md_path: format!("{}/CLAUDE.md", ancestor_str),
                    resolution: ResolutionStatus::Ancestor { distance },
                    is_child: false, // set by resolve()
                };
            }

            current = ancestor.parent();
            distance += 1;
        }

        // Unresolved
        InternalDependency {
            raw_import: raw_import.to_string(),
            resolved_dir: dir_str,
            claude_md_path: String::new(),
            resolution: ResolutionStatus::Unresolved,
            is_child: false,
        }
    }
}

/// Normalize a path by resolving `.` and `..` components without filesystem access.
/// Guards against path traversal beyond the root (excessive `..` components are ignored).
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
                // If components is empty, ignore the ParentDir to prevent
                // traversal beyond the project boundary (INV-1 guard)
            }
            other => {
                components.push(other);
            }
        }
    }

    if components.is_empty() {
        return PathBuf::from(".");
    }

    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_analyzer::Dependencies;
    use crate::tree_parser::DirectoryInfo;

    fn make_tree_result(dirs: Vec<&str>) -> TreeResult {
        TreeResult {
            root: PathBuf::from("."),
            needs_claude_md: dirs
                .into_iter()
                .map(|d| DirectoryInfo {
                    path: PathBuf::from(d),
                    source_file_count: 1,
                    subdir_count: 0,
                    reason: "test".to_string(),
                    depth: d.matches('/').count(),
                })
                .collect(),
            excluded: vec![],
            scan_errors: vec![],
        }
    }

    #[test]
    fn test_resolve_relative_path() {
        let tree = make_tree_result(vec!["src/utils", "src/auth"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec!["../utils".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("src/auth"));

        assert_eq!(analysis.dependencies.internal.len(), 1);
        let dep = &analysis.dependencies.internal[0];
        assert_eq!(dep.raw_import, "../utils");
        assert_eq!(dep.resolved_dir, "src/utils");
        assert_eq!(dep.claude_md_path, "src/utils/CLAUDE.md");
        assert_eq!(dep.resolution, ResolutionStatus::Exact);
    }

    #[test]
    fn test_resolve_gradle_module_path() {
        let tree = make_tree_result(vec!["vendors/vendor-common", "core/domain"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec!["vendors:vendor-common".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("core/domain"));

        assert_eq!(analysis.dependencies.internal.len(), 1);
        let dep = &analysis.dependencies.internal[0];
        assert_eq!(dep.raw_import, "vendors:vendor-common");
        assert_eq!(dep.resolved_dir, "vendors/vendor-common");
        assert_eq!(dep.claude_md_path, "vendors/vendor-common/CLAUDE.md");
        assert_eq!(dep.resolution, ResolutionStatus::Exact);
    }

    #[test]
    fn test_resolve_package_path() {
        let tree = make_tree_result(vec!["core/domain/transaction"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec!["core.domain.transaction".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("some/other/dir"));

        assert_eq!(analysis.dependencies.internal.len(), 1);
        let dep = &analysis.dependencies.internal[0];
        assert_eq!(dep.raw_import, "core.domain.transaction");
        assert_eq!(dep.resolved_dir, "core/domain/transaction");
        assert_eq!(dep.claude_md_path, "core/domain/transaction/CLAUDE.md");
        assert_eq!(dep.resolution, ResolutionStatus::Exact);
    }

    #[test]
    fn test_resolve_ancestor_fallback() {
        let tree = make_tree_result(vec!["core/domain"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec!["core/domain/transaction/sub".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("src"));

        assert_eq!(analysis.dependencies.internal.len(), 1);
        let dep = &analysis.dependencies.internal[0];
        assert_eq!(dep.resolution, ResolutionStatus::Ancestor { distance: 2 });
        assert_eq!(dep.claude_md_path, "core/domain/CLAUDE.md");
    }

    #[test]
    fn test_resolve_unresolved() {
        let tree = make_tree_result(vec!["src/auth"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec!["nonexistent/module".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("src/auth"));

        assert_eq!(analysis.dependencies.internal.len(), 1);
        let dep = &analysis.dependencies.internal[0];
        assert_eq!(dep.resolution, ResolutionStatus::Unresolved);
        assert!(dep.claude_md_path.is_empty());
    }

    #[test]
    fn test_normalize_unknown_extension_file() {
        let tree = make_tree_result(vec!["src"]);
        let resolver = DependencyResolver::new(&tree);

        // Unknown extensions should remain as file paths, not convert to package paths
        let result = resolver.normalize("README.txt", Path::new("src"));
        assert_eq!(result, Some(PathBuf::from("README.txt")));

        let result = resolver.normalize("config.ini", Path::new("src"));
        assert_eq!(result, Some(PathBuf::from("config.ini")));

        let result = resolver.normalize("data.csv", Path::new("src"));
        assert_eq!(result, Some(PathBuf::from("data.csv")));

        // 3-part should still convert to package path
        let result = resolver.normalize("com.example.module", Path::new("src"));
        assert_eq!(result, Some(PathBuf::from("com/example/module")));
    }

    #[test]
    fn test_sibling_dependency_marked_as_non_child() {
        let tree = make_tree_result(vec!["src/auth", "src/utils", "src/auth/types"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec![
                    "../utils".to_string(),      // sibling
                    "./types".to_string(),        // child
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("src/auth"));

        assert_eq!(analysis.dependencies.internal.len(), 2);

        // ../utils from src/auth → src/utils → is_child = false (sibling)
        let sibling_dep = &analysis.dependencies.internal[0];
        assert_eq!(sibling_dep.resolved_dir, "src/utils");
        assert!(!sibling_dep.is_child, "Sibling dependency should have is_child=false");

        // ./types from src/auth → src/auth/types → is_child = true (child)
        let child_dep = &analysis.dependencies.internal[1];
        assert_eq!(child_dep.resolved_dir, "src/auth/types");
        assert!(child_dep.is_child, "Child dependency should have is_child=true");
    }

    #[test]
    fn test_resolve_direct_path() {
        let tree = make_tree_result(vec!["core/domain/transaction"]);
        let resolver = DependencyResolver::new(&tree);

        let mut analysis = AnalysisResult {
            dependencies: Dependencies {
                internal_raw: vec!["core/domain/transaction".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        resolver.resolve(&mut analysis, Path::new("src"));

        assert_eq!(analysis.dependencies.internal.len(), 1);
        let dep = &analysis.dependencies.internal[0];
        assert_eq!(dep.resolved_dir, "core/domain/transaction");
        assert_eq!(dep.claude_md_path, "core/domain/transaction/CLAUDE.md");
        assert_eq!(dep.resolution, ResolutionStatus::Exact);
    }
}
