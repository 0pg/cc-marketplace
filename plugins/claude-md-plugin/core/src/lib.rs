/// Directories to exclude from scanning (shared across tree_parser and claude_md_scanner)
pub const EXCLUDED_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", "output",
    ".git", ".svn", ".hg", ".claude",
    "__pycache__", ".pytest_cache", ".mypy_cache",
    "vendor", "deps", "_deps",
    ".idea", ".vscode", ".vs",
    "coverage", ".nyc_output",
    "bin", "obj",
];

/// Source file extensions to detect (shared across tree_parser, compile_target_resolver)
pub const SOURCE_EXTENSIONS: &[&str] = &[
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

/// Check if the given lines represent a "None" marker (None, N/A, etc.)
/// Values should match `none_marker.values` in schema-rules.yaml (SSOT).
/// Used by both parser and validator to avoid duplication.
pub fn is_none_marker_content(lines: &[&str]) -> bool {
    let non_empty: Vec<&&str> = lines.iter()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .collect();
    if non_empty.len() == 1 {
        let trimmed = non_empty[0].trim();
        // Strip markdown list markers (- , * , + , 1. ) before checking
        let stripped = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
            .or_else(|| {
                // Ordered list markers like "1. "
                let digit_end = trimmed
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(trimmed.len());
                if digit_end > 0 && trimmed[digit_end..].starts_with(". ") {
                    Some(&trimmed[digit_end + 2..])
                } else {
                    None
                }
            })
            .unwrap_or(trimmed);
        let lower = stripped.to_lowercase();
        return lower == "none" || lower == "n/a";
    }
    false
}

pub mod tree_parser;
pub mod boundary_resolver;
pub mod schema_validator;
pub mod code_analyzer;
pub mod claude_md_parser;
pub mod bracket_utils;
pub mod convention_validator;
pub mod dependency_resolver;
pub mod claude_md_scanner;
pub mod compile_target_resolver;
pub mod exports_formatter;

pub use tree_parser::TreeParser;
pub use boundary_resolver::BoundaryResolver;
pub use schema_validator::SchemaValidator;
pub use code_analyzer::CodeAnalyzer;
pub use claude_md_parser::ClaudeMdParser;
pub use bracket_utils::{split_respecting_brackets, find_matching_bracket, extract_parenthesized};
pub use convention_validator::ConventionValidator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_none_marker_plain_none() {
        let lines = vec!["None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_plain_na() {
        let lines = vec!["N/A"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_case_insensitive() {
        let lines = vec!["none"];
        assert!(is_none_marker_content(&lines));
        let lines = vec!["NONE"];
        assert!(is_none_marker_content(&lines));
        let lines = vec!["n/a"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_dash_prefix() {
        let lines = vec!["- None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_asterisk_prefix() {
        let lines = vec!["* None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_plus_prefix() {
        let lines = vec!["+ None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_ordered_list() {
        let lines = vec!["1. None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_ordered_list_multidigit() {
        let lines = vec!["10. None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_with_empty_lines() {
        let lines = vec!["", "  ", "None", ""];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_with_header_lines() {
        let lines = vec!["### Subsection", "None"];
        assert!(is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_actual_content() {
        let lines = vec!["This is real content"];
        assert!(!is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_multiple_content_lines() {
        let lines = vec!["None", "But also other content"];
        assert!(!is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_empty() {
        let lines: Vec<&str> = vec![];
        assert!(!is_none_marker_content(&lines));
    }

    #[test]
    fn test_is_none_marker_only_whitespace() {
        let lines = vec!["  ", "   "];
        assert!(!is_none_marker_content(&lines));
    }
}
