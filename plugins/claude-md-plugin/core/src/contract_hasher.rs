use sha2::{Sha256, Digest};
use std::path::Path;

/// Calculate SHA-256 hash of the entire CLAUDE.md file content.
/// In v3 schema, CLAUDE.md is compact (no Exports/Behavior/Contract),
/// so we hash the entire file for change detection.
pub fn contract_hash(file: &Path) -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string(file)?;
    Ok(hash_content(&content))
}

/// Calculate hash from content string
pub fn hash_content(content: &str) -> String {
    let normalized = normalize_content(content);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Normalize content for deterministic hashing:
/// trim trailing whitespace per line, skip empty lines.
fn normalize_content(content: &str) -> String {
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_basic() {
        let content = r#"# Module

## Purpose
Test module.

## Constraints
None

## Domain Context
- TOKEN_EXPIRY: 7d
"#;
        let hash = hash_content(content);
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_hash_deterministic() {
        let content = r#"## Purpose
Test module.

## Constraints
None

## Domain Context
None
"#;
        let h1 = hash_content(content);
        let h2 = hash_content(content);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_changes_with_content() {
        let content1 = r#"## Purpose
Test module v1.

## Constraints
None

## Domain Context
None
"#;
        let content2 = r#"## Purpose
Test module v2.

## Constraints
None

## Domain Context
None
"#;
        let h1 = hash_content(content1);
        let h2 = hash_content(content2);
        assert_ne!(h1, h2, "Different content should produce different hashes");
    }

    #[test]
    fn test_hash_ignores_trailing_whitespace() {
        let content1 = "## Purpose\nTest module.\n\n## Constraints\nNone\n";
        let content2 = "## Purpose\nTest module.  \n\n## Constraints  \nNone  \n";
        let h1 = hash_content(content1);
        let h2 = hash_content(content2);
        assert_eq!(h1, h2, "Trailing whitespace differences should not affect hash");
    }
}
