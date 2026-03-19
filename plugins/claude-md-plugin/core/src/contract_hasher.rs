use sha2::{Sha256, Digest};
use std::path::Path;

/// Calculate SHA-256 hash of the "compilable" sections of a CLAUDE.md file.
/// Only Exports, Behavior, and Contract sections are included in the hash.
/// Domain Context, Protocol, and other sections do not affect the hash,
/// preventing unnecessary recompiles when only non-compilable parts change.
pub fn contract_hash(file: &Path) -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string(file)?;
    Ok(hash_content(&content))
}

/// Calculate contract hash from content string
pub fn hash_content(content: &str) -> String {
    let sections = extract_compilable_sections(content);
    let mut hasher = Sha256::new();
    hasher.update(sections.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Extract only the compilable sections (Exports, Behavior, Contract)
/// and normalize them for deterministic hashing.
fn extract_compilable_sections(content: &str) -> String {
    let compilable_names = ["Exports", "Behavior", "Contract"];
    let mut result = String::new();
    let mut current_section: Option<&str> = None;
    let mut in_compilable = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect H2 headers
        if trimmed.starts_with("## ") {
            let section_name = trimmed[3..].trim();
            let is_compilable = compilable_names
                .iter()
                .any(|name| section_name.eq_ignore_ascii_case(name));

            if is_compilable {
                current_section = Some(section_name);
                in_compilable = true;
                result.push_str(&format!("## {}\n", section_name));
            } else {
                current_section = None;
                in_compilable = false;
            }
            continue;
        }

        // Stop collecting when we hit another H2 (already handled above)
        // or continue if we're in a compilable section
        if in_compilable && current_section.is_some() {
            // Normalize: trim trailing whitespace, skip empty lines for consistency
            let normalized = trimmed;
            if !normalized.is_empty() {
                result.push_str(normalized);
                result.push('\n');
            }
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

## Exports
- `foo(x: int): string`

## Behavior
- input → output

## Contract
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
        let content = r#"## Exports
- `foo(x: int): string`

## Behavior
- input → output

## Contract
None
"#;
        let h1 = hash_content(content);
        let h2 = hash_content(content);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_ignores_domain_context() {
        let base = r#"## Exports
- `foo(x: int): string`

## Behavior
- input → output

## Contract
None
"#;
        let with_dc = format!("{}\n## Domain Context\n- some context\n", base);
        let h1 = hash_content(base);
        let h2 = hash_content(&with_dc);
        assert_eq!(h1, h2, "Domain Context changes should not affect hash");
    }

    #[test]
    fn test_hash_changes_with_exports() {
        let content1 = r#"## Exports
- `foo(x: int): string`

## Behavior
- input → output

## Contract
None
"#;
        let content2 = r#"## Exports
- `foo(x: int): string`
- `bar(y: int): string`

## Behavior
- input → output

## Contract
None
"#;
        let h1 = hash_content(content1);
        let h2 = hash_content(content2);
        assert_ne!(h1, h2, "Export changes should change hash");
    }

    #[test]
    fn test_hash_changes_with_behavior() {
        let content1 = r#"## Exports
- `foo(x: int): string`

## Behavior
- input → output

## Contract
None
"#;
        let content2 = r#"## Exports
- `foo(x: int): string`

## Behavior
- input → output
- error → ErrorType

## Contract
None
"#;
        let h1 = hash_content(content1);
        let h2 = hash_content(content2);
        assert_ne!(h1, h2, "Behavior changes should change hash");
    }

    #[test]
    fn test_hash_ignores_protocol() {
        let base = r#"## Exports
- `foo(x: int): string`

## Behavior
- input → output

## Contract
None
"#;
        let with_protocol = format!("{}\n## Protocol\n### State Machine\nStates: A | B\n", base);
        let h1 = hash_content(base);
        let h2 = hash_content(&with_protocol);
        assert_eq!(h1, h2, "Protocol changes should not affect hash");
    }
}
