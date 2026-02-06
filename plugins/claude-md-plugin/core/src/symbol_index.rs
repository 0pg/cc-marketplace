use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::claude_md_parser::{ClaudeMdParser, ClaudeMdSpec, ExportsSpec};
use crate::tree_parser::TreeParser;

/// Error types for symbol indexing
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum SymbolIndexError {
    #[error("Cannot read directory '{path}': {source}")]
    DirectoryReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Parse error for '{path}': {message}")]
    ParseError { path: String, message: String },
}

/// Symbol kinds (matching LSP SymbolKind subset)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Type,
    Class,
    Enum,
    Variable,
}

/// A single indexed symbol from CLAUDE.md exports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// Symbol name (e.g. "validateToken")
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Module path relative to root (e.g. "src/auth")
    pub module_path: String,
    /// Full signature if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Cross-reference anchor (e.g. "src/auth/CLAUDE.md#validateToken")
    pub anchor: String,
}

/// A cross-reference found in CLAUDE.md content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    /// Module path where the reference was found
    pub from_module: String,
    /// Section where the reference was found
    pub from_section: String,
    /// Target anchor (e.g. "src/auth/CLAUDE.md#validateToken")
    pub to_anchor: String,
    /// Target symbol name
    pub to_symbol: String,
    /// Whether the reference resolves to a known symbol
    pub valid: bool,
}

/// Summary statistics for the symbol index
#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolIndexSummary {
    pub total_modules: usize,
    pub total_symbols: usize,
    pub total_references: usize,
    pub unresolved_count: usize,
}

/// Complete symbol index result
#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolIndexResult {
    /// Root directory that was indexed
    pub root: String,
    /// ISO timestamp
    pub indexed_at: String,
    /// All indexed symbols
    pub symbols: Vec<SymbolEntry>,
    /// All cross-references found
    pub references: Vec<SymbolReference>,
    /// Unresolved references
    pub unresolved: Vec<SymbolReference>,
    /// Summary statistics
    pub summary: SymbolIndexSummary,
}

/// Builder for constructing a symbol index from a project tree
pub struct SymbolIndexBuilder {
    tree_parser: TreeParser,
    claude_md_parser: ClaudeMdParser,
}

impl SymbolIndexBuilder {
    pub fn new() -> Self {
        Self {
            tree_parser: TreeParser::new(),
            claude_md_parser: ClaudeMdParser::new(),
        }
    }

    /// Build a complete symbol index for the given root directory
    pub fn build(&self, root: &Path) -> Result<SymbolIndexResult, SymbolIndexError> {
        let mut symbols = Vec::new();
        let mut references = Vec::new();

        // Step 1: Find all CLAUDE.md files via tree parser
        let tree_result = self.tree_parser.parse(root);

        // Collect all directories that might have CLAUDE.md
        let mut claude_md_paths = Vec::new();
        // Check the root itself
        let root_claude_md = root.join("CLAUDE.md");
        if root_claude_md.exists() {
            claude_md_paths.push((String::new(), root_claude_md));
        }

        // Check directories identified by tree parser
        for dir_info in &tree_result.needs_claude_md {
            let claude_md = dir_info.path.join("CLAUDE.md");
            if claude_md.exists() {
                let rel_path = dir_info.path
                    .strip_prefix(root)
                    .unwrap_or(&dir_info.path)
                    .to_string_lossy()
                    .to_string();
                claude_md_paths.push((rel_path, claude_md));
            }
        }

        // Also scan for any CLAUDE.md files we might have missed
        self.scan_claude_md_files(root, root, &mut claude_md_paths);

        // Step 2: Parse each CLAUDE.md and extract symbols
        let mut parsed_specs: Vec<(String, ClaudeMdSpec)> = Vec::new();
        for (module_path, claude_md_file) in &claude_md_paths {
            match self.claude_md_parser.parse(claude_md_file) {
                Ok(spec) => {
                    Self::extract_symbols(&spec.exports, module_path, &mut symbols);
                    parsed_specs.push((module_path.clone(), spec));
                }
                Err(_) => {
                    // Skip unparseable files silently
                    continue;
                }
            }
        }

        // Step 3: Scan all CLAUDE.md content for cross-references
        for (module_path, claude_md_file) in &claude_md_paths {
            if let Ok(content) = std::fs::read_to_string(claude_md_file) {
                Self::extract_references(&content, module_path, &symbols, &mut references);
            }
        }

        // Step 4: Classify references as resolved/unresolved
        let mut unresolved = Vec::new();
        for reference in &mut references {
            let is_valid = symbols.iter().any(|s| s.anchor == reference.to_anchor);
            reference.valid = is_valid;
            if !is_valid {
                unresolved.push(reference.clone());
            }
        }

        let summary = SymbolIndexSummary {
            total_modules: claude_md_paths.len(),
            total_symbols: symbols.len(),
            total_references: references.len(),
            unresolved_count: unresolved.len(),
        };

        Ok(SymbolIndexResult {
            root: root.to_string_lossy().to_string(),
            indexed_at: chrono::Utc::now().to_rfc3339(),
            symbols,
            references,
            unresolved,
            summary,
        })
    }

    /// Find a symbol by name in the index
    pub fn find_symbol<'a>(index: &'a SymbolIndexResult, name: &str) -> Vec<&'a SymbolEntry> {
        index.symbols.iter().filter(|s| s.name == name).collect()
    }

    /// Find all references to a given anchor
    pub fn find_references<'a>(index: &'a SymbolIndexResult, anchor: &str) -> Vec<&'a SymbolReference> {
        index.references.iter().filter(|r| r.to_anchor == anchor).collect()
    }

    // --- Private helpers ---

    fn scan_claude_md_files(
        &self,
        dir: &Path,
        root: &Path,
        result: &mut Vec<(String, std::path::PathBuf)>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip excluded directories
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name.starts_with('.') || name == "node_modules" || name == "target"
                        || name == "build" || name == "dist" || name == "__pycache__"
                    {
                        continue;
                    }
                    let claude_md = path.join("CLAUDE.md");
                    if claude_md.exists() {
                        let rel_path = path
                            .strip_prefix(root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();
                        // Avoid duplicates
                        if !result.iter().any(|(p, _)| p == &rel_path) {
                            result.push((rel_path, claude_md));
                        }
                    }
                    self.scan_claude_md_files(&path, root, result);
                }
            }
        }
    }

    fn extract_symbols(exports: &ExportsSpec, module_path: &str, symbols: &mut Vec<SymbolEntry>) {
        let make_anchor = |name: &str| {
            if module_path.is_empty() {
                format!("CLAUDE.md#{}", name)
            } else {
                format!("{}/CLAUDE.md#{}", module_path, name)
            }
        };

        for func in &exports.functions {
            symbols.push(SymbolEntry {
                name: func.name.clone(),
                kind: SymbolKind::Function,
                module_path: module_path.to_string(),
                signature: Some(func.signature.clone()),
                anchor: make_anchor(&func.name),
            });
        }

        for type_export in &exports.types {
            symbols.push(SymbolEntry {
                name: type_export.name.clone(),
                kind: SymbolKind::Type,
                module_path: module_path.to_string(),
                signature: Some(type_export.definition.clone()),
                anchor: make_anchor(&type_export.name),
            });
        }

        for class in &exports.classes {
            symbols.push(SymbolEntry {
                name: class.name.clone(),
                kind: SymbolKind::Class,
                module_path: module_path.to_string(),
                signature: Some(class.constructor_signature.clone()),
                anchor: make_anchor(&class.name),
            });
        }

        for enum_export in &exports.enums {
            symbols.push(SymbolEntry {
                name: enum_export.name.clone(),
                kind: SymbolKind::Enum,
                module_path: module_path.to_string(),
                signature: None,
                anchor: make_anchor(&enum_export.name),
            });
        }

        for var in &exports.variables {
            symbols.push(SymbolEntry {
                name: var.name.clone(),
                kind: SymbolKind::Variable,
                module_path: module_path.to_string(),
                signature: var.value.clone(),
                anchor: make_anchor(&var.name),
            });
        }
    }

    fn extract_references(
        content: &str,
        from_module: &str,
        known_symbols: &[SymbolEntry],
        references: &mut Vec<SymbolReference>,
    ) {
        // Pattern: CLAUDE.md#symbolName or path/CLAUDE.md#symbolName
        let ref_pattern = regex::Regex::new(r"([A-Za-z0-9_./-]*CLAUDE\.md)#([A-Za-z_][A-Za-z0-9_]*)").unwrap();

        let mut current_section = String::from("(unknown)");

        for line in content.lines() {
            // Track current section
            if line.starts_with("## ") {
                current_section = line.trim_start_matches('#').trim().to_string();
            }

            for caps in ref_pattern.captures_iter(line) {
                let full_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let symbol_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");

                let to_anchor = format!("{}#{}", full_path, symbol_name);

                // Don't track self-references
                let self_anchor = if from_module.is_empty() {
                    format!("CLAUDE.md#{}", symbol_name)
                } else {
                    format!("{}/CLAUDE.md#{}", from_module, symbol_name)
                };
                if to_anchor == self_anchor {
                    continue;
                }

                let valid = known_symbols.iter().any(|s| s.anchor == to_anchor);

                references.push(SymbolReference {
                    from_module: from_module.to_string(),
                    from_section: current_section.clone(),
                    to_anchor,
                    to_symbol: symbol_name.to_string(),
                    valid,
                });
            }
        }
    }
}

impl Default for SymbolIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_claude_md(dir: &Path, content: &str) {
        fs::write(dir.join("CLAUDE.md"), content).unwrap();
    }

    /// Helper to wrap minimal exports/behaviors with all required sections
    fn with_required_sections(name: &str, purpose: &str, exports: &str, behavior: &str) -> String {
        format!(
            r#"# {}

## Purpose
{}

## Summary
Module summary.

## Exports
{}

## Behavior
{}

## Contract
None

## Protocol
None

## Domain Context
None
"#,
            name, purpose, exports, behavior
        )
    }

    #[test]
    fn test_symbol_index_basic() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let auth_dir = root.join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        create_claude_md(&auth_dir, &with_required_sections(
            "auth",
            "Authentication module.",
            r#"
### Functions
- `validateToken(token: string): Promise<Claims>`

### Types
- `Claims { userId: string, role: Role }`"#,
            "- valid token → Claims",
        ));

        let builder = SymbolIndexBuilder::new();
        let result = builder.build(root).unwrap();

        assert_eq!(result.summary.total_symbols, 2);
        assert!(result.symbols.iter().any(|s| s.name == "validateToken" && s.kind == SymbolKind::Function));
        assert!(result.symbols.iter().any(|s| s.name == "Claims" && s.kind == SymbolKind::Type));
    }

    #[test]
    fn test_find_symbol() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let auth_dir = root.join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        create_claude_md(&auth_dir, &with_required_sections(
            "auth",
            "Auth module.",
            r#"
### Functions
- `validateToken(token: string): Claims`"#,
            "- token → Claims",
        ));

        let builder = SymbolIndexBuilder::new();
        let result = builder.build(root).unwrap();

        let found = SymbolIndexBuilder::find_symbol(&result, "validateToken");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].anchor, "auth/CLAUDE.md#validateToken");
    }

    #[test]
    fn test_cross_reference_detection() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let auth_dir = root.join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        create_claude_md(&auth_dir, &with_required_sections(
            "auth",
            "Auth module.",
            r#"
### Functions
- `validateToken(token: string): Claims`"#,
            "- token → Claims",
        ));

        let api_dir = root.join("api");
        fs::create_dir_all(&api_dir).unwrap();
        // For cross-reference detection, the reference is in the content,
        // which is read as raw text, not parsed through ClaudeMdSpec
        create_claude_md(&api_dir, &with_required_sections(
            "api",
            "API module. Uses auth/CLAUDE.md#validateToken for authentication.",
            r#"
### Functions
- `handleRequest(req: Request): Response`"#,
            "- request → response",
        ));

        let builder = SymbolIndexBuilder::new();
        let result = builder.build(root).unwrap();

        assert!(result.summary.total_references > 0);
        let refs = SymbolIndexBuilder::find_references(&result, "auth/CLAUDE.md#validateToken");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].from_module, "api");
        assert!(refs[0].valid);
    }

    #[test]
    fn test_unresolved_reference() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let api_dir = root.join("api");
        fs::create_dir_all(&api_dir).unwrap();
        create_claude_md(&api_dir, &with_required_sections(
            "api",
            "References nonexistent/CLAUDE.md#missingSymbol.",
            r#"
### Functions
- `serve(): void`"#,
            "- start → running",
        ));

        let builder = SymbolIndexBuilder::new();
        let result = builder.build(root).unwrap();

        assert_eq!(result.summary.unresolved_count, 1);
        assert_eq!(result.unresolved[0].to_symbol, "missingSymbol");
    }
}
