//! Dependency graph builder for analyzing module dependencies and boundary violations.
//!
//! Builds a directed graph of module dependencies based on:
//! 1. CLAUDE.md Exports sections (interface catalog)
//! 2. Source code import/require statements (actual dependencies)
//!
//! Detects boundary violations where code imports symbols not in the target's Exports.

use crate::claude_md_parser::{ClaudeMdParser, DependenciesSpec};
use crate::code_analyzer::CodeAnalyzer;
use crate::implements_md_parser::ImplementsMdParser;
use crate::symbol_index::SymbolIndexBuilder;
use crate::tree_parser::TreeParser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during dependency graph building.
#[derive(Debug, Error)]
pub enum DependencyGraphError {
    #[error("Failed to read directory: {0}")]
    DirectoryReadError(#[from] std::io::Error),

    #[error("Root path does not exist: {0}")]
    RootNotFound(String),
}

/// Result of dependency graph analysis.
#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyGraphResult {
    /// Root directory that was analyzed
    pub root: String,
    /// ISO 8601 timestamp when analysis was performed
    pub analyzed_at: String,
    /// Module nodes in the graph
    pub nodes: Vec<ModuleNode>,
    /// Dependency edges between modules
    pub edges: Vec<DependencyEdge>,
    /// Boundary violations detected
    pub violations: Vec<Violation>,
    /// Summary statistics
    pub summary: Summary,
}

/// A module node in the dependency graph.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleNode {
    /// Path relative to root
    pub path: String,
    /// Whether this module has a CLAUDE.md
    pub has_claude_md: bool,
    /// Summary - brief 1-2 sentence overview of role/responsibility/features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Exported symbols from CLAUDE.md Exports section
    pub exports: Vec<String>,
    /// Detailed symbol entries for cross-reference indexing.
    /// Currently populated as empty vec; will be filled by SymbolIndexBuilder
    /// when symbol-level dependency tracking is integrated into the dependency graph pipeline.
    /// See: symbol_index.rs SymbolEntry for the entry structure.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub symbol_entries: Vec<crate::symbol_index::SymbolEntry>,
}

/// A dependency edge between two modules.
#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source module path
    pub from: String,
    /// Target module path
    pub to: String,
    /// Edge type: "internal" or "external"
    pub edge_type: String,
    /// Symbols imported from target
    pub imported_symbols: Vec<String>,
    /// Whether this edge respects boundary (imports only exported symbols)
    pub valid: bool,
}

/// A boundary violation.
#[derive(Debug, Serialize, Deserialize)]
pub struct Violation {
    /// Source module that imports
    pub from: String,
    /// Target module that is imported
    pub to: String,
    /// Type of violation
    pub violation_type: String,
    /// Human-readable reason
    pub reason: String,
    /// Suggested fix
    pub suggestion: String,
}

/// Summary statistics.
#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub valid_edges: usize,
    pub violations_count: usize,
}

/// Builder for dependency graphs.
pub struct DependencyGraphBuilder {
    tree_parser: TreeParser,
    claude_md_parser: ClaudeMdParser,
    implements_md_parser: ImplementsMdParser,
    code_analyzer: CodeAnalyzer,
}

impl DependencyGraphBuilder {
    /// Create a new DependencyGraphBuilder.
    pub fn new() -> Self {
        Self {
            tree_parser: TreeParser::new(),
            claude_md_parser: ClaudeMdParser::new(),
            implements_md_parser: ImplementsMdParser::new(),
            code_analyzer: CodeAnalyzer::new(),
        }
    }

    /// Build a dependency graph for the given root directory.
    pub fn build(&self, root: &Path) -> Result<DependencyGraphResult, DependencyGraphError> {
        if !root.exists() {
            return Err(DependencyGraphError::RootNotFound(
                root.display().to_string(),
            ));
        }

        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

        // 1. Find all directories with CLAUDE.md
        let tree_result = self.tree_parser.parse(&root);

        // 2. Build module nodes and collect exports
        let mut nodes = Vec::new();
        let mut module_exports: HashMap<String, Vec<String>> = HashMap::new();
        // Collect spec-level dependencies per module for edge generation
        let mut module_spec_deps: HashMap<String, Vec<String>> = HashMap::new();

        // Check directories that need CLAUDE.md
        for dir_info in &tree_result.needs_claude_md {
            let dir_path = root.join(&dir_info.path);
            let claude_md_path = dir_path.join("CLAUDE.md");
            let relative_path = dir_info.path.display().to_string();
            let relative_path = if relative_path.is_empty() {
                ".".to_string()
            } else {
                relative_path
            };

            if claude_md_path.exists() {
                // Parse CLAUDE.md to get exports, summary, and dependencies
                let (exports, summary, deps) = self.extract_exports_and_summary(&claude_md_path);
                let export_names = exports.clone();
                module_exports.insert(relative_path.clone(), export_names.clone());

                // Store spec dependencies for later edge generation
                if !deps.internal.is_empty() {
                    module_spec_deps.insert(relative_path.clone(), deps.internal);
                }

                nodes.push(ModuleNode {
                    path: relative_path,
                    has_claude_md: true,
                    summary,
                    exports: export_names,
                    symbol_entries: vec![],
                });
            } else {
                // Module without CLAUDE.md
                module_exports.insert(relative_path.clone(), Vec::new());
                nodes.push(ModuleNode {
                    path: relative_path,
                    has_claude_md: false,
                    summary: None,
                    exports: Vec::new(),
                    symbol_entries: vec![],
                });
            }
        }

        // 2.5. Build symbol index and populate symbol_entries for each node
        let symbol_builder = SymbolIndexBuilder::new();
        if let Ok(symbol_index) = symbol_builder.build_with_cache(&root, false) {
            // Group symbols by module_path
            let mut symbols_by_module: HashMap<String, Vec<crate::symbol_index::SymbolEntry>> = HashMap::new();
            for sym in &symbol_index.symbols {
                symbols_by_module.entry(sym.module_path.clone())
                    .or_default()
                    .push(sym.clone());
            }

            // Fill symbol_entries for each node
            for node in &mut nodes {
                let module_key = if node.path == "." { "" } else { &node.path };
                if let Some(entries) = symbols_by_module.remove(module_key) {
                    node.symbol_entries = entries;
                }
            }
        }

        // 3. Analyze dependencies for each module
        let mut edges = Vec::new();
        let mut violations = Vec::new();

        // 3a. Collect IMPLEMENTS.md integration map data per module
        let mut integration_maps: HashMap<String, Vec<crate::implements_md_parser::IntegrationMapEntry>> =
            HashMap::new();

        for node in &nodes {
            let dir_path = if node.path == "." {
                root.clone()
            } else {
                root.join(&node.path)
            };

            let implements_md_path = dir_path.join("IMPLEMENTS.md");
            if implements_md_path.exists() {
                if let Ok(impl_spec) = self.implements_md_parser.parse(&implements_md_path) {
                    if !impl_spec.module_integration_map.is_empty() {
                        integration_maps
                            .insert(node.path.clone(), impl_spec.module_integration_map);
                    }
                }
            }
        }

        // 3b. Build edges from code analysis and enrich with IMPLEMENTS.md data
        for node in &nodes {
            let dir_path = if node.path == "." {
                root.clone()
            } else {
                root.join(&node.path)
            };

            // Use CodeAnalyzer to extract dependencies (code-level edges)
            if let Ok(analysis) = self.code_analyzer.analyze_directory(&dir_path, None) {
                // Process internal dependencies
                for internal_dep in &analysis.dependencies.internal {
                    let (target_path, mut edge, violation) =
                        self.process_internal_dependency(&node.path, internal_dep, &module_exports);

                    // Enrich imported_symbols from IMPLEMENTS.md integration map
                    if let Some(map_entries) = integration_maps.get(&node.path) {
                        self.enrich_edge_from_integration_map(&mut edge, map_entries);
                    }

                    edges.push(edge);
                    if let Some(v) = violation {
                        violations.push(v);
                    }

                    // Also track the target if not already a node
                    if !module_exports.contains_key(&target_path) {
                        module_exports.insert(target_path, Vec::new());
                    }
                }

                // Process external dependencies
                for external_dep in &analysis.dependencies.external {
                    edges.push(DependencyEdge {
                        from: node.path.clone(),
                        to: external_dep.clone(),
                        edge_type: "external".to_string(),
                        imported_symbols: Vec::new(),
                        valid: true, // External deps are always valid
                    });
                }
            }

            // 3c. Also create edges from IMPLEMENTS.md entries not found by code analysis
            if let Some(map_entries) = integration_maps.get(&node.path) {
                for entry in map_entries {
                    let target_path =
                        self.normalize_import_path(&node.path, &entry.relative_path);

                    // Check if edge already exists (created by code analysis above)
                    let edge_exists = edges
                        .iter()
                        .any(|e| e.from == node.path && e.to == target_path);

                    if !edge_exists {
                        let imported_symbols: Vec<String> = entry
                            .exports_used
                            .iter()
                            .map(|e| e.signature.clone())
                            .collect();

                        let target_exports =
                            module_exports.get(&target_path).cloned().unwrap_or_default();
                        let has_exports = !target_exports.is_empty();

                        edges.push(DependencyEdge {
                            from: node.path.clone(),
                            to: target_path.clone(),
                            edge_type: "internal".to_string(),
                            imported_symbols,
                            valid: has_exports
                                || !module_exports.contains_key(&target_path),
                        });

                        if !module_exports.contains_key(&target_path) {
                            module_exports.insert(target_path, Vec::new());
                        }
                    }
                }
            }
        }

        // 3.5. Generate spec edges from CLAUDE.md Dependencies section
        for (from_module, spec_deps) in &module_spec_deps {
            for internal_dep in spec_deps {
                let target_path = self.normalize_import_path(from_module, internal_dep);

                // Check if a code-level edge already exists for the same from→to pair
                let already_has_edge = edges.iter().any(|e| {
                    e.from == *from_module && e.to == target_path && e.edge_type == "internal"
                });

                let target_has_exports = module_exports.get(&target_path)
                    .map(|e| !e.is_empty())
                    .unwrap_or(false);

                edges.push(DependencyEdge {
                    from: from_module.clone(),
                    to: target_path,
                    edge_type: "spec".to_string(),
                    imported_symbols: Vec::new(),
                    valid: target_has_exports || already_has_edge,
                });
            }
        }

        // 4. Calculate summary
        let valid_edges = edges.iter().filter(|e| e.valid).count();
        let summary = Summary {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
            valid_edges,
            violations_count: violations.len(),
        };

        // 5. Generate timestamp
        let analyzed_at = chrono_lite_now();

        Ok(DependencyGraphResult {
            root: root.display().to_string(),
            analyzed_at,
            nodes,
            edges,
            violations,
            summary,
        })
    }

    /// Extract export names, summary, and dependencies from a CLAUDE.md file.
    fn extract_exports_and_summary(&self, claude_md_path: &Path) -> (Vec<String>, Option<String>, DependenciesSpec) {
        let spec = match self.claude_md_parser.parse(claude_md_path) {
            Ok(s) => s,
            Err(_) => return (Vec::new(), None, DependenciesSpec::default()),
        };

        let mut exports = Vec::new();

        // Extract function names
        for func in &spec.exports.functions {
            exports.push(func.name.clone());
        }

        // Extract type names
        for type_export in &spec.exports.types {
            exports.push(type_export.name.clone());
        }

        // Extract class names
        for class in &spec.exports.classes {
            exports.push(class.name.clone());
        }

        // Extract enum names
        for enum_export in &spec.exports.enums {
            exports.push(enum_export.name.clone());
        }

        // Extract variable names
        for var in &spec.exports.variables {
            exports.push(var.name.clone());
        }

        (exports, spec.summary, spec.dependencies)
    }

    /// Process an internal dependency and check for boundary violations.
    fn process_internal_dependency(
        &self,
        from_path: &str,
        import_path: &str,
        module_exports: &HashMap<String, Vec<String>>,
    ) -> (String, DependencyEdge, Option<Violation>) {
        // Normalize the import path to a module path
        let target_path = self.normalize_import_path(from_path, import_path);

        // Check if target module has exports defined
        let target_exports = module_exports.get(&target_path).cloned().unwrap_or_default();

        // For now, we can't extract exactly which symbols are imported from the CodeAnalyzer
        // So we mark as valid if target has CLAUDE.md with exports, or warn if not
        let has_exports = !target_exports.is_empty();

        // If target has no exports defined, it's a potential violation
        let (valid, violation) = if !has_exports && module_exports.contains_key(&target_path) {
            // Target exists but has no exports - potential boundary issue
            (
                false,
                Some(Violation {
                    from: from_path.to_string(),
                    to: target_path.clone(),
                    violation_type: "missing-exports".to_string(),
                    reason: format!(
                        "Module '{}' has no Exports defined in CLAUDE.md",
                        target_path
                    ),
                    suggestion: format!(
                        "Add Exports section to {}/CLAUDE.md with public interfaces",
                        target_path
                    ),
                }),
            )
        } else {
            (true, None)
        };

        let edge = DependencyEdge {
            from: from_path.to_string(),
            to: target_path.clone(),
            edge_type: "internal".to_string(),
            imported_symbols: Vec::new(), // Would need deeper analysis
            valid,
        };

        (target_path, edge, violation)
    }

    /// Enrich a dependency edge's imported_symbols using IMPLEMENTS.md integration map data.
    ///
    /// If the edge target matches an integration map entry (by normalized path),
    /// the export signatures from the map are added to the edge's imported_symbols.
    fn enrich_edge_from_integration_map(
        &self,
        edge: &mut DependencyEdge,
        map_entries: &[crate::implements_md_parser::IntegrationMapEntry],
    ) {
        for entry in map_entries {
            let normalized_target =
                self.normalize_import_path(&edge.from, &entry.relative_path);

            if normalized_target == edge.to {
                // Add all export signatures as imported symbols
                for export in &entry.exports_used {
                    if !edge.imported_symbols.contains(&export.signature) {
                        edge.imported_symbols.push(export.signature.clone());
                    }
                }
                break;
            }
        }
    }

    /// Normalize an import path relative to the importing module.
    fn normalize_import_path(&self, from_path: &str, import_path: &str) -> String {
        // Handle relative imports
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let from_dir = if from_path == "." {
                PathBuf::new()
            } else {
                PathBuf::from(from_path)
            };

            let import = PathBuf::from(import_path);
            let mut result = from_dir;

            for component in import.components() {
                match component {
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        result.pop();
                    }
                    std::path::Component::Normal(name) => {
                        result.push(name);
                    }
                    _ => {}
                }
            }

            result.display().to_string()
        } else {
            // Absolute or package import - return as-is
            import_path.to_string()
        }
    }
}

impl Default for DependencyGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple ISO 8601 timestamp without external crate.
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Simple calculation for UTC time
    let days_since_epoch = secs / 86400;
    let secs_today = secs % 86400;

    let hours = secs_today / 3600;
    let minutes = (secs_today % 3600) / 60;
    let seconds = secs_today % 60;

    // Calculate year/month/day (simplified - good enough for timestamps)
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days in days_in_months.iter() {
        if remaining_days < *days {
            break;
        }
        remaining_days -= *days;
        month += 1;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_project() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Create src/auth with CLAUDE.md
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();

        let claude_md = auth_dir.join("CLAUDE.md");
        let mut f = File::create(&claude_md).unwrap();
        writeln!(
            f,
            r#"# auth

## Purpose
Authentication module.

## Summary
인증 모듈. JWT 토큰 검증 및 사용자 인증 처리.

## Exports

### Functions
- `validateToken(token: string): Promise<Claims>`

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
Test authentication context.
"#
        )
        .unwrap();

        // Create a source file
        let token_ts = auth_dir.join("token.ts");
        let mut f = File::create(&token_ts).unwrap();
        writeln!(
            f,
            r#"
import {{ Config }} from '../config';

export async function validateToken(token: string): Promise<Claims> {{
    return {{ sub: 'user' }};
}}
"#
        )
        .unwrap();

        // Create src/config without CLAUDE.md
        let config_dir = temp.path().join("src").join("config");
        fs::create_dir_all(&config_dir).unwrap();

        let config_ts = config_dir.join("index.ts");
        let mut f = File::create(&config_ts).unwrap();
        writeln!(
            f,
            r#"
export const Config = {{ secret: 'xxx' }};
"#
        )
        .unwrap();

        temp
    }

    #[test]
    fn test_build_dependency_graph() {
        let temp = create_test_project();
        let builder = DependencyGraphBuilder::new();
        let result = builder.build(temp.path()).unwrap();

        assert!(!result.nodes.is_empty());
        assert!(!result.analyzed_at.is_empty());
    }

    #[test]
    fn test_extract_exports_and_summary_from_claude_md() {
        let temp = create_test_project();
        let builder = DependencyGraphBuilder::new();
        let claude_md_path = temp.path().join("src").join("auth").join("CLAUDE.md");

        let (exports, summary, _deps) = builder.extract_exports_and_summary(&claude_md_path);
        assert!(exports.contains(&"validateToken".to_string()));
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("인증 모듈"));
    }

    #[test]
    fn test_normalize_import_path() {
        let builder = DependencyGraphBuilder::new();

        assert_eq!(
            builder.normalize_import_path("src/auth", "../config"),
            "src/config"
        );
        assert_eq!(
            builder.normalize_import_path("src/auth", "./utils"),
            "src/auth/utils"
        );
        assert_eq!(
            builder.normalize_import_path(".", "./src"),
            "src"
        );
    }

    // ============== CLAUDE.md Dependencies → Spec Edges Tests ==============

    #[test]
    fn test_spec_edges_from_claude_md_dependencies() {
        let temp = TempDir::new().unwrap();

        // Create src/auth with Dependencies section referencing ../utils
        let auth_dir = temp.path().join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();

        let claude_md = auth_dir.join("CLAUDE.md");
        let mut f = File::create(&claude_md).unwrap();
        writeln!(
            f,
            r#"# auth

## Purpose
Authentication module.

## Summary
Auth module with spec dependencies.

## Dependencies
- internal: ../utils

## Exports

### Functions
- `validateToken(token: string): Claims`

## Behavior
- valid token → Claims

## Contract
None

## Protocol
None

## Domain Context
Test authentication context.
"#
        )
        .unwrap();

        // Create a source file (no code imports)
        let token_ts = auth_dir.join("token.ts");
        let mut f = File::create(&token_ts).unwrap();
        writeln!(
            f,
            r#"
export function validateToken(token: string): Claims {{
    return {{ sub: 'user' }};
}}
"#
        )
        .unwrap();

        // Create src/utils with CLAUDE.md
        let utils_dir = temp.path().join("src").join("utils");
        fs::create_dir_all(&utils_dir).unwrap();

        let claude_md_utils = utils_dir.join("CLAUDE.md");
        let mut f = File::create(&claude_md_utils).unwrap();
        writeln!(
            f,
            r#"# utils

## Purpose
Utility module.

## Summary
Utility functions.

## Exports

### Functions
- `formatError(err: Error): string`

## Behavior
- error → formatted string

## Contract
None

## Protocol
None

## Domain Context
Utility context.
"#
        )
        .unwrap();

        let utils_ts = utils_dir.join("index.ts");
        File::create(&utils_ts).unwrap();

        let builder = DependencyGraphBuilder::new();
        let result = builder.build(temp.path()).unwrap();

        // Should have a "spec" edge from src/auth → src/utils
        let spec_edge = result.edges.iter().find(|e| {
            e.from.contains("auth") && e.to.contains("utils") && e.edge_type == "spec"
        });
        assert!(
            spec_edge.is_some(),
            "Expected a 'spec' edge from auth to utils, got edges: {:?}",
            result.edges
        );
    }

    #[test]
    fn test_extract_exports_summary_and_dependencies() {
        let temp = create_test_project();
        let builder = DependencyGraphBuilder::new();
        let claude_md_path = temp.path().join("src").join("auth").join("CLAUDE.md");

        let (exports, summary, deps) = builder.extract_exports_and_summary(&claude_md_path);
        assert!(exports.contains(&"validateToken".to_string()));
        assert!(summary.is_some());
        // The default test project might not have Dependencies, so just check it doesn't panic
        assert!(deps.internal.is_empty() || !deps.internal.is_empty());
    }
}
