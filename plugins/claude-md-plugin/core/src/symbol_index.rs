use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

use crate::claude_md_parser::{ClaudeMdParser, ExportsSpec};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIndexSummary {
    pub total_modules: usize,
    pub total_symbols: usize,
    pub total_references: usize,
    pub unresolved_count: usize,
}

/// Complete symbol index result
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Cached symbol index for incremental rebuilds
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedSymbolIndex {
    /// Cache format version (for compatibility management)
    pub cache_version: u32,
    /// The full index result
    pub index: SymbolIndexResult,
    /// File modification times (relative path → epoch seconds)
    pub file_mtimes: HashMap<String, i64>,
}

const CACHE_VERSION: u32 = 2;
const CACHE_DIR: &str = ".claude/.cache";
const CACHE_FILE: &str = ".claude/.cache/symbol-index.json";

/// Builder for constructing a symbol index from a project tree
pub struct SymbolIndexBuilder {
    tree_parser: TreeParser,
    claude_md_parser: ClaudeMdParser,
    /// Pre-compiled regex for cross-reference extraction (P0.1: static regex)
    ref_pattern: Regex,
}

impl SymbolIndexBuilder {
    pub fn new() -> Self {
        Self {
            tree_parser: TreeParser::new(),
            claude_md_parser: ClaudeMdParser::new(),
            ref_pattern: Regex::new(r"([A-Za-z0-9_./-]*CLAUDE\.md)#([A-Za-z_][A-Za-z0-9_]*)").unwrap(),
        }
    }

    /// Build a complete symbol index for the given root directory
    pub fn build(&self, root: &Path) -> Result<SymbolIndexResult, SymbolIndexError> {
        let mut symbols = Vec::new();
        let mut references = Vec::new();

        // Step 1: Find all CLAUDE.md files via tree parser only (P0.3: single walk)
        let claude_md_paths = self.collect_claude_md_paths(root);

        // Step 2: Read each file once, parse and extract references (P0.4: single read)
        let mut file_contents: Vec<(String, String)> = Vec::new(); // (module_path, content)
        for (module_path, claude_md_file) in &claude_md_paths {
            if let Ok(content) = std::fs::read_to_string(claude_md_file) {
                // Parse from already-read content string (P0.4: no double read)
                match self.claude_md_parser.parse_content(&content) {
                    Ok(spec) => {
                        Self::extract_symbols(&spec.exports, module_path, &mut symbols);
                    }
                    Err(_) => {
                        // Skip unparseable files silently
                    }
                }
                file_contents.push((module_path.clone(), content));
            }
        }

        // Step 3: Extract cross-references using pre-compiled regex (P0.1)
        // and use HashMap for O(1) lookups (P0.2)
        let anchor_set: HashMap<&str, usize> = symbols.iter()
            .enumerate()
            .map(|(i, s)| (s.anchor.as_str(), i))
            .collect();

        for (module_path, content) in &file_contents {
            self.extract_references(content, module_path, &anchor_set, &mut references);
        }

        // Step 4: Classify references as resolved/unresolved (P0.2: HashMap lookup)
        let mut unresolved = Vec::new();
        for reference in &mut references {
            let is_valid = anchor_set.contains_key(reference.to_anchor.as_str());
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

    /// Build symbol index with caching support for incremental rebuilds
    pub fn build_with_cache(&self, root: &Path, no_cache: bool) -> Result<SymbolIndexResult, SymbolIndexError> {
        if no_cache {
            let result = self.build(root)?;
            self.save_cache(root, &result, &self.collect_claude_md_paths(root));
            return Ok(result);
        }

        // Try to load existing cache
        let cache = self.load_cache(root);

        // Collect current CLAUDE.md paths and mtimes
        let claude_md_paths = self.collect_claude_md_paths(root);
        let current_mtimes = Self::collect_claude_md_mtimes(&claude_md_paths);

        match cache {
            Some(cached) if cached.cache_version == CACHE_VERSION => {
                // Diff mtimes to find changes
                let (changed, added, removed) = Self::diff_mtimes(&cached.file_mtimes, &current_mtimes);

                if changed.is_empty() && added.is_empty() && removed.is_empty() {
                    // Cache hit - no changes
                    return Ok(cached.index);
                }

                // Incremental rebuild
                let result = self.incremental_rebuild(
                    cached,
                    &claude_md_paths,
                    &changed,
                    &added,
                    &removed,
                )?;
                self.save_cache(root, &result, &claude_md_paths);
                Ok(result)
            }
            _ => {
                // No cache or incompatible version - full rebuild
                let result = self.build(root)?;
                self.save_cache(root, &result, &claude_md_paths);
                Ok(result)
            }
        }
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

    /// Collect all CLAUDE.md file paths using tree parser only (P0.3: single walk)
    fn collect_claude_md_paths(&self, root: &Path) -> Vec<(String, std::path::PathBuf)> {
        let tree_result = self.tree_parser.parse(root);
        let mut claude_md_paths = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Check the root itself
        let root_claude_md = root.join("CLAUDE.md");
        if root_claude_md.exists() {
            seen.insert(String::new());
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
                if seen.insert(rel_path.clone()) {
                    claude_md_paths.push((rel_path, claude_md));
                }
            }
        }

        // Supplementary scan for CLAUDE.md files not in tree parser results
        Self::scan_claude_md_files_static(root, root, &mut claude_md_paths, &mut seen);

        claude_md_paths
    }

    /// Static scan for CLAUDE.md files (no &self needed)
    fn scan_claude_md_files_static(
        dir: &Path,
        root: &Path,
        result: &mut Vec<(String, std::path::PathBuf)>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
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
                        if seen.insert(rel_path.clone()) {
                            result.push((rel_path, claude_md));
                        }
                    }
                    Self::scan_claude_md_files_static(&path, root, result, seen);
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

    /// Extract cross-references using pre-compiled regex (P0.1)
    /// and HashMap-based symbol lookup (P0.2)
    fn extract_references(
        &self,
        content: &str,
        from_module: &str,
        anchor_set: &HashMap<&str, usize>,
        references: &mut Vec<SymbolReference>,
    ) {
        let mut current_section = String::from("(unknown)");

        for line in content.lines() {
            // Track current section
            if line.starts_with("## ") {
                current_section = line.trim_start_matches('#').trim().to_string();
            }

            for caps in self.ref_pattern.captures_iter(line) {
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

                let valid = anchor_set.contains_key(to_anchor.as_str());

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

    // --- Cache methods (P1) ---

    fn load_cache(&self, root: &Path) -> Option<CachedSymbolIndex> {
        let cache_path = root.join(CACHE_FILE);
        let content = std::fs::read_to_string(cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_cache(
        &self,
        root: &Path,
        result: &SymbolIndexResult,
        claude_md_paths: &[(String, std::path::PathBuf)],
    ) {
        let cache_dir = root.join(CACHE_DIR);
        if std::fs::create_dir_all(&cache_dir).is_err() {
            return;
        }

        let current_mtimes = Self::collect_claude_md_mtimes(claude_md_paths);

        let cached = CachedSymbolIndex {
            cache_version: CACHE_VERSION,
            index: result.clone(),
            file_mtimes: current_mtimes,
        };

        let cache_path = root.join(CACHE_FILE);
        if let Ok(json) = serde_json::to_string(&cached) {
            let _ = std::fs::write(cache_path, json);
        }
    }

    fn collect_claude_md_mtimes(paths: &[(String, std::path::PathBuf)]) -> HashMap<String, i64> {
        let mut mtimes = HashMap::new();
        for (module_path, file_path) in paths {
            if let Ok(metadata) = std::fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        mtimes.insert(module_path.clone(), duration.as_secs() as i64);
                    }
                }
            }
        }
        mtimes
    }

    fn diff_mtimes(
        cached: &HashMap<String, i64>,
        current: &HashMap<String, i64>,
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut changed = Vec::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();

        // Find changed and added
        for (path, &mtime) in current {
            match cached.get(path) {
                Some(&cached_mtime) if cached_mtime == mtime => {} // unchanged
                Some(_) => changed.push(path.clone()),              // mtime differs
                None => added.push(path.clone()),                    // new file
            }
        }

        // Find removed
        for path in cached.keys() {
            if !current.contains_key(path) {
                removed.push(path.clone());
            }
        }

        (changed, added, removed)
    }

    fn incremental_rebuild(
        &self,
        cached: CachedSymbolIndex,
        claude_md_paths: &[(String, std::path::PathBuf)],
        changed: &[String],
        added: &[String],
        removed: &[String],
    ) -> Result<SymbolIndexResult, SymbolIndexError> {
        let mut symbols = cached.index.symbols;

        // Remove symbols belonging to changed or removed files
        let files_to_remove: HashSet<&str> = changed.iter()
            .chain(removed.iter())
            .map(|s| s.as_str())
            .collect();
        symbols.retain(|s| !files_to_remove.contains(s.module_path.as_str()));

        // Re-parse changed + added files and add their symbols (P0.4: single read)
        let files_to_parse: Vec<&String> = changed.iter().chain(added.iter()).collect();
        let path_map: HashMap<&str, &std::path::PathBuf> = claude_md_paths.iter()
            .map(|(m, p)| (m.as_str(), p))
            .collect();

        // Read and parse changed/added files once, store content for reference extraction
        let mut parsed_contents: HashMap<String, String> = HashMap::new();
        for module_path in &files_to_parse {
            if let Some(file_path) = path_map.get(module_path.as_str()) {
                if let Ok(content) = std::fs::read_to_string(file_path.as_ref() as &Path) {
                    if let Ok(spec) = self.claude_md_parser.parse_content(&content) {
                        Self::extract_symbols(&spec.exports, module_path, &mut symbols);
                    }
                    parsed_contents.insert(module_path.to_string(), content);
                }
            }
        }

        // Re-resolve all references (HashMap makes this fast)
        let anchor_set: HashMap<&str, usize> = symbols.iter()
            .enumerate()
            .map(|(i, s)| (s.anchor.as_str(), i))
            .collect();

        let mut references = Vec::new();
        for (module_path, file_path) in claude_md_paths {
            // Reuse already-read content for changed/added files (P0.4: no double read)
            let content = if let Some(cached_content) = parsed_contents.get(module_path) {
                cached_content.clone()
            } else if let Ok(content) = std::fs::read_to_string(file_path) {
                content
            } else {
                continue;
            };
            self.extract_references(&content, module_path, &anchor_set, &mut references);
        }

        let mut unresolved = Vec::new();
        for reference in &mut references {
            let is_valid = anchor_set.contains_key(reference.to_anchor.as_str());
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
            root: cached.index.root,
            indexed_at: chrono::Utc::now().to_rfc3339(),
            symbols,
            references,
            unresolved,
            summary,
        })
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

    // --- P1: Cache tests ---

    #[test]
    fn test_cache_created_on_first_build() {
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
        let result = builder.build_with_cache(root, false).unwrap();

        assert_eq!(result.summary.total_symbols, 1);

        // Cache file should exist
        let cache_path = root.join(CACHE_FILE);
        assert!(cache_path.exists(), "Cache file should be created");

        // Verify cache contents
        let cached: CachedSymbolIndex = serde_json::from_str(
            &fs::read_to_string(&cache_path).unwrap()
        ).unwrap();
        assert_eq!(cached.cache_version, CACHE_VERSION);
        assert_eq!(cached.index.summary.total_symbols, 1);
        assert!(!cached.file_mtimes.is_empty());
    }

    #[test]
    fn test_cache_hit_returns_same_result() {
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

        // First build - creates cache
        let result1 = builder.build_with_cache(root, false).unwrap();

        // Second build - should hit cache
        let result2 = builder.build_with_cache(root, false).unwrap();

        assert_eq!(result1.summary.total_symbols, result2.summary.total_symbols);
        assert_eq!(result1.symbols.len(), result2.symbols.len());
    }

    #[test]
    fn test_no_cache_flag_rebuilds() {
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

        // Build with cache
        builder.build_with_cache(root, false).unwrap();

        // Force rebuild with no-cache
        let result = builder.build_with_cache(root, true).unwrap();
        assert_eq!(result.summary.total_symbols, 1);
    }

    #[test]
    fn test_corrupted_cache_triggers_rebuild() {
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

        // Write corrupted cache
        let cache_dir = root.join(CACHE_DIR);
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(root.join(CACHE_FILE), "not valid json").unwrap();

        let builder = SymbolIndexBuilder::new();
        let result = builder.build_with_cache(root, false).unwrap();

        assert_eq!(result.summary.total_symbols, 1);
        // Cache should be replaced with valid content
        let cached: CachedSymbolIndex = serde_json::from_str(
            &fs::read_to_string(root.join(CACHE_FILE)).unwrap()
        ).unwrap();
        assert_eq!(cached.cache_version, CACHE_VERSION);
    }

    #[test]
    fn test_incremental_rebuild_on_file_addition() {
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

        // First build
        let result1 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result1.summary.total_symbols, 1);

        // Add a new CLAUDE.md
        let payments_dir = root.join("payments");
        fs::create_dir_all(&payments_dir).unwrap();
        create_claude_md(&payments_dir, &with_required_sections(
            "payments",
            "Payments module.",
            r#"
### Functions
- `processPayment(amount: number): Receipt`"#,
            "- payment → receipt",
        ));

        // Second build - should pick up new file
        let result2 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result2.summary.total_symbols, 2);
        assert!(result2.symbols.iter().any(|s| s.name == "processPayment"));
        assert!(result2.symbols.iter().any(|s| s.name == "validateToken"));
    }

    #[test]
    fn test_incremental_rebuild_on_file_removal() {
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

        let legacy_dir = root.join("legacy");
        fs::create_dir_all(&legacy_dir).unwrap();
        create_claude_md(&legacy_dir, &with_required_sections(
            "legacy",
            "Legacy module.",
            r#"
### Functions
- `oldFunction(): void`"#,
            "- old → void",
        ));

        let builder = SymbolIndexBuilder::new();

        // First build
        let result1 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result1.summary.total_symbols, 2);

        // Remove legacy CLAUDE.md
        fs::remove_file(legacy_dir.join("CLAUDE.md")).unwrap();

        // Second build - should remove legacy symbols
        let result2 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result2.summary.total_symbols, 1);
        assert!(result2.symbols.iter().any(|s| s.name == "validateToken"));
        assert!(!result2.symbols.iter().any(|s| s.name == "oldFunction"));
    }

    #[test]
    fn test_diff_mtimes() {
        let mut cached = HashMap::new();
        cached.insert("auth".to_string(), 100);
        cached.insert("api".to_string(), 200);
        cached.insert("legacy".to_string(), 300);

        let mut current = HashMap::new();
        current.insert("auth".to_string(), 100);  // unchanged
        current.insert("api".to_string(), 250);   // changed
        current.insert("payments".to_string(), 400); // added

        let (changed, added, removed) = SymbolIndexBuilder::diff_mtimes(&cached, &current);

        assert_eq!(changed, vec!["api".to_string()]);
        assert_eq!(added, vec!["payments".to_string()]);
        assert_eq!(removed, vec!["legacy".to_string()]);
    }

    #[test]
    fn test_sequential_incremental_rebuilds() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Setup: 3 modules (auth, payments, api) with 1 symbol each
        let auth_dir = root.join("auth");
        let payments_dir = root.join("payments");
        let api_dir = root.join("api");
        fs::create_dir_all(&auth_dir).unwrap();
        fs::create_dir_all(&payments_dir).unwrap();
        fs::create_dir_all(&api_dir).unwrap();

        create_claude_md(&auth_dir, &with_required_sections(
            "auth", "Auth module.",
            "\n### Functions\n- `validateToken(token: string): Claims`",
            "- token → Claims",
        ));
        create_claude_md(&payments_dir, &with_required_sections(
            "payments", "Payments module.",
            "\n### Functions\n- `processPayment(amount: number): Receipt`",
            "- amount → Receipt",
        ));
        create_claude_md(&api_dir, &with_required_sections(
            "api", "API module.",
            "\n### Functions\n- `handleRequest(req: Request): Response`",
            "- req → Response",
        ));

        let builder = SymbolIndexBuilder::new();

        // Build 1: Full build → cache created
        let result1 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result1.summary.total_symbols, 3);
        assert!(result1.symbols.iter().any(|s| s.name == "validateToken"));
        assert!(result1.symbols.iter().any(|s| s.name == "processPayment"));
        assert!(result1.symbols.iter().any(|s| s.name == "handleRequest"));

        // Build 2: Modify auth → incremental rebuild
        // Sleep briefly to ensure mtime changes
        std::thread::sleep(std::time::Duration::from_millis(1100));
        create_claude_md(&auth_dir, &with_required_sections(
            "auth", "Auth module.",
            "\n### Functions\n- `verifyJWT(token: string): Claims`",
            "- token → Claims",
        ));

        let result2 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result2.summary.total_symbols, 3);
        assert!(result2.symbols.iter().any(|s| s.name == "verifyJWT"), "New auth symbol should exist");
        assert!(!result2.symbols.iter().any(|s| s.name == "validateToken"), "Old auth symbol should be gone");
        assert!(result2.symbols.iter().any(|s| s.name == "processPayment"), "payments symbol should be unchanged");
        assert!(result2.symbols.iter().any(|s| s.name == "handleRequest"), "api symbol should be unchanged");

        // Build 3: Modify payments → incremental rebuild
        std::thread::sleep(std::time::Duration::from_millis(1100));
        create_claude_md(&payments_dir, &with_required_sections(
            "payments", "Payments module.",
            "\n### Functions\n- `chargeCard(card: Card): Transaction`",
            "- card → Transaction",
        ));

        let result3 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result3.summary.total_symbols, 3);
        assert!(result3.symbols.iter().any(|s| s.name == "chargeCard"), "New payments symbol should exist");
        assert!(!result3.symbols.iter().any(|s| s.name == "processPayment"), "Old payments symbol should be gone");
        assert!(result3.symbols.iter().any(|s| s.name == "verifyJWT"), "auth symbol from build 2 should be preserved");
        assert!(result3.symbols.iter().any(|s| s.name == "handleRequest"), "api symbol should be unchanged");

        // Build 4: Remove api → incremental rebuild
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::remove_file(api_dir.join("CLAUDE.md")).unwrap();

        let result4 = builder.build_with_cache(root, false).unwrap();
        assert_eq!(result4.summary.total_symbols, 2);
        assert!(result4.symbols.iter().any(|s| s.name == "verifyJWT"), "auth symbol should remain");
        assert!(result4.symbols.iter().any(|s| s.name == "chargeCard"), "payments symbol should remain");
        assert!(!result4.symbols.iter().any(|s| s.name == "handleRequest"), "api symbol should be gone");
    }
}
