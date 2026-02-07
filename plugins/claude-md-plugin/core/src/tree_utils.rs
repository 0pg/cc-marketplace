use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Source file extensions to detect
const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs",  // JavaScript/TypeScript
    "py", "pyi",                              // Python
    "go",                                     // Go
    "rs",                                     // Rust
    "java", "kt", "kts", "scala",            // JVM
    "c", "cpp", "cc", "cxx", "h", "hpp",     // C/C++
    "cs",                                     // C#
    "rb",                                     // Ruby
    "swift",                                  // Swift
    "php",                                    // PHP
];

/// Directories to exclude from scanning
const EXCLUDED_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", "output",
    ".git", ".svn", ".hg",
    "__pycache__", ".pytest_cache", ".mypy_cache",
    "vendor", "deps", "_deps",
    ".idea", ".vscode", ".vs",
    "coverage", ".nyc_output",
    "bin", "obj",
];

/// Shared directory scanner used by TreeParser and Auditor.
/// Provides common logic for detecting source files, excluded dirs,
/// and walking directory trees.
pub(crate) struct DirScanner {
    source_extensions: HashSet<String>,
    excluded_dirs: HashSet<String>,
}

impl DirScanner {
    pub(crate) fn new() -> Self {
        Self {
            source_extensions: SOURCE_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            excluded_dirs: EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub(crate) fn should_exclude(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                return self.excluded_dirs.contains(name_str);
            }
        }
        false
    }

    pub(crate) fn count_source_files(&self, dir: &Path) -> usize {
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

    pub(crate) fn count_subdirs(&self, dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter(|e| !self.should_exclude(&e.path()))
                    .count()
            })
            .unwrap_or(0)
    }

    pub(crate) fn is_source_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.source_extensions.contains(ext))
            .unwrap_or(false)
    }

    pub(crate) fn make_relative(&self, root: &Path, path: &Path) -> PathBuf {
        path.strip_prefix(root)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| path.to_path_buf())
    }

    pub(crate) fn calculate_depth(&self, relative_path: &Path) -> usize {
        if relative_path.as_os_str().is_empty() {
            0
        } else {
            relative_path.components().count()
        }
    }

    /// Walk the directory tree rooted at `root` and collect:
    /// - directories to check (not excluded, not under excluded ancestors)
    /// - excluded directory paths (relative to root)
    pub(crate) fn collect_directories(&self, root: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut dirs_to_check: Vec<PathBuf> = Vec::new();
        let mut excluded: Vec<PathBuf> = Vec::new();

        let walker = WalkDir::new(root).into_iter();

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_dir() {
                let path = entry.path().to_path_buf();

                if self.should_exclude(&path) {
                    excluded.push(self.make_relative(root, &path));
                    continue;
                }

                // Check if any ancestor is excluded
                let is_under_excluded = path.ancestors().skip(1).any(|p| {
                    if let Some(name) = p.file_name() {
                        if let Some(name_str) = name.to_str() {
                            return self.excluded_dirs.contains(name_str);
                        }
                    }
                    false
                });

                if !is_under_excluded {
                    dirs_to_check.push(path);
                }
            }
        }

        (dirs_to_check, excluded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (TempDir, DirScanner) {
        let dir = TempDir::new().unwrap();
        let scanner = DirScanner::new();
        (dir, scanner)
    }

    #[test]
    fn test_should_exclude() {
        let (_, scanner) = setup();
        assert!(scanner.should_exclude(Path::new("/project/node_modules")));
        assert!(scanner.should_exclude(Path::new("/project/target")));
        assert!(scanner.should_exclude(Path::new("/project/.git")));
        assert!(!scanner.should_exclude(Path::new("/project/src")));
        assert!(!scanner.should_exclude(Path::new("/project/lib")));
    }

    #[test]
    fn test_count_source_files() {
        let (dir, scanner) = setup();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::File::create(src.join("main.ts")).unwrap();
        fs::File::create(src.join("utils.py")).unwrap();
        fs::File::create(src.join("README.md")).unwrap();

        assert_eq!(scanner.count_source_files(&src), 2);
    }

    #[test]
    fn test_count_subdirs() {
        let (dir, scanner) = setup();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("lib")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();

        assert_eq!(scanner.count_subdirs(root), 2);
    }

    #[test]
    fn test_is_source_file() {
        let (_, scanner) = setup();
        assert!(scanner.is_source_file(Path::new("main.ts")));
        assert!(scanner.is_source_file(Path::new("app.py")));
        assert!(scanner.is_source_file(Path::new("lib.rs")));
        assert!(!scanner.is_source_file(Path::new("README.md")));
        assert!(!scanner.is_source_file(Path::new("config.yaml")));
    }

    #[test]
    fn test_make_relative() {
        let (_, scanner) = setup();
        let root = Path::new("/project");
        let path = Path::new("/project/src/main.rs");
        assert_eq!(scanner.make_relative(root, path), PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_calculate_depth_root() {
        let (_, scanner) = setup();
        assert_eq!(scanner.calculate_depth(Path::new("")), 0);
    }

    #[test]
    fn test_calculate_depth_nested() {
        let (_, scanner) = setup();
        assert_eq!(scanner.calculate_depth(Path::new("src")), 1);
        assert_eq!(scanner.calculate_depth(Path::new("src/auth")), 2);
        assert_eq!(scanner.calculate_depth(Path::new("src/auth/jwt")), 3);
    }

    #[test]
    fn test_collect_directories() {
        let (dir, scanner) = setup();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("lib")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join(".git/objects")).unwrap();

        let (dirs, excluded) = scanner.collect_directories(root);

        // root, src, lib should be in dirs
        assert!(dirs.len() >= 3, "Expected at least 3 dirs, got {}: {:?}", dirs.len(), dirs);

        // node_modules and .git should be in excluded
        let excluded_names: Vec<String> = excluded.iter()
            .filter_map(|p| p.file_name())
            .filter_map(|n| n.to_str())
            .map(|s| s.to_string())
            .collect();
        assert!(excluded_names.contains(&"node_modules".to_string()));
        assert!(excluded_names.contains(&".git".to_string()));

        // Subdirs of excluded should NOT be in dirs
        let dir_strs: Vec<String> = dirs.iter().map(|p| p.display().to_string()).collect();
        for d in &dir_strs {
            assert!(!d.contains("node_modules"), "Found excluded subdir in dirs: {}", d);
            assert!(!d.contains(".git"), "Found excluded subdir in dirs: {}", d);
        }
    }
}
