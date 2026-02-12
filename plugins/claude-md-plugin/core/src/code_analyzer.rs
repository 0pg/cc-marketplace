//! Code analyzer module for extracting exports, dependencies, and behaviors from source files.
//!
//! Supports multiple languages: TypeScript, Python, Go, Rust, Java, Kotlin

mod typescript;
mod python;
mod go;
mod rust_lang;
mod java;
mod kotlin;

use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use typescript::TypeScriptAnalyzer;
pub use python::PythonAnalyzer;
pub use go::GoAnalyzer;
pub use rust_lang::RustAnalyzer;
pub use java::JavaAnalyzer;
pub use kotlin::KotlinAnalyzer;

/// Errors that can occur during code analysis.
#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Unsupported language for file: {0}")]
    UnsupportedLanguage(String),

}

/// Result of code analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalysisResult {
    /// Path that was analyzed
    pub path: String,
    /// Exported symbols
    pub exports: Exports,
    /// Dependencies
    pub dependencies: Dependencies,
    /// Inferred behaviors
    pub behaviors: Vec<Behavior>,
    /// Contracts for functions (preconditions, postconditions, etc.)
    pub contracts: Vec<FunctionContract>,
    /// Protocol (state machines, lifecycle)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,
    /// List of files that were analyzed
    pub analyzed_files: Vec<String>,
}

/// Exported symbols from code.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Exports {
    /// Exported functions
    pub functions: Vec<ExportedFunction>,
    /// Exported types (interfaces, type aliases, etc.)
    pub types: Vec<ExportedType>,
    /// Exported classes
    pub classes: Vec<ExportedClass>,
    /// Exported enums
    pub enums: Vec<ExportedEnum>,
    /// Exported variables/constants
    pub variables: Vec<ExportedVariable>,
    /// Re-exported symbols from other modules
    pub re_exports: Vec<ReExport>,
}

/// A re-exported symbol from another module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReExport {
    pub name: String,
    pub source: String,
}

/// An exported function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedFunction {
    pub name: String,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An exported type (interface, type alias, struct, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedType {
    pub name: String,
    pub kind: TypeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Kind of exported type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TypeKind {
    Interface,
    Type,
    Struct,
    Enum,
    Class,
    DataClass,
}

/// An exported class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedClass {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An exported enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedEnum {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,
}

/// An exported variable or constant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedVariable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var_type: Option<String>,
}

/// A resolved internal dependency pointing to a specific CLAUDE.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalDependency {
    /// Original import path from source code (e.g., "../utils", "core.domain.pkg")
    pub raw_import: String,

    /// Project-root-relative directory path (e.g., "core/domain/transaction")
    pub resolved_dir: String,

    /// Project-root-relative CLAUDE.md path (e.g., "core/domain/transaction/CLAUDE.md")
    pub claude_md_path: String,

    /// Resolution quality
    pub resolution: ResolutionStatus,

    /// Whether resolved_dir is a child of the source directory (INV-1 compliant).
    /// false means sibling/parent/external — agent should not write to Dependencies section.
    #[serde(default)]
    pub is_child: bool,
}

/// Resolution quality for an internal dependency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResolutionStatus {
    /// CLAUDE.md exists at exact resolved directory
    Exact,
    /// Using nearest ancestor's CLAUDE.md (distance = levels up)
    Ancestor { distance: usize },
    /// No CLAUDE.md found
    Unresolved,
}

/// Dependencies extracted from code.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dependencies {
    /// External dependencies (third-party packages)
    pub external: Vec<String>,
    /// Resolved internal dependencies (populated by DependencyResolver)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub internal: Vec<InternalDependency>,
    /// Raw internal import paths (populated by analyzers, before resolution)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub internal_raw: Vec<String>,
}

/// A behavior inferred from code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub input: String,
    pub output: String,
    pub category: BehaviorCategory,
}

/// Category of behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BehaviorCategory {
    Success,
    Error,
}

/// Contract information for a function (preconditions, postconditions, invariants).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contract {
    /// Preconditions that must hold before the function executes
    pub preconditions: Vec<String>,
    /// Postconditions that are guaranteed after successful execution
    pub postconditions: Vec<String>,
    /// Invariants that are maintained throughout execution
    pub invariants: Vec<String>,
    /// Exceptions/errors that may be thrown
    pub throws: Vec<String>,
}

/// A function with its associated contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionContract {
    pub function_name: String,
    pub contract: Contract,
}

/// Protocol information (state machines, lifecycle).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Protocol {
    /// State machine states (from enum)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<String>,
    /// State transitions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<StateTransition>,
    /// Lifecycle methods in order
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<String>,
}

/// A state transition in a state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub trigger: String,
}

/// Trait that all language-specific analyzers must implement.
pub trait LanguageAnalyzer {
    /// Analyze a single file and return partial results.
    fn analyze_file(&self, path: &Path, content: &str) -> Result<PartialAnalysis, AnalyzerError>;
}

/// Partial analysis result from a single file.
#[derive(Debug, Clone, Default)]
pub struct PartialAnalysis {
    pub functions: Vec<ExportedFunction>,
    pub types: Vec<ExportedType>,
    pub classes: Vec<ExportedClass>,
    pub enums: Vec<ExportedEnum>,
    pub variables: Vec<ExportedVariable>,
    pub re_exports: Vec<ReExport>,
    pub contracts: Vec<FunctionContract>,
    pub protocol: Option<Protocol>,
    pub external_deps: Vec<String>,
    pub internal_deps: Vec<String>,
    pub behaviors: Vec<Behavior>,
}

/// Main code analyzer that delegates to language-specific analyzers.
#[derive(Debug)]
pub struct CodeAnalyzer {
    typescript: TypeScriptAnalyzer,
    python: PythonAnalyzer,
    go: GoAnalyzer,
    rust: RustAnalyzer,
    java: JavaAnalyzer,
    kotlin: KotlinAnalyzer,
}

impl CodeAnalyzer {
    /// Create a new CodeAnalyzer.
    pub fn new() -> Self {
        Self {
            typescript: TypeScriptAnalyzer::new(),
            python: PythonAnalyzer::new(),
            go: GoAnalyzer::new(),
            rust: RustAnalyzer::new(),
            java: JavaAnalyzer::new(),
            kotlin: KotlinAnalyzer::new(),
        }
    }

    /// Analyze a single file.
    pub fn analyze_file(&self, path: &Path) -> Result<AnalysisResult, AnalyzerError> {
        let content = std::fs::read_to_string(path)?;
        let language = self.detect_language(path)?;

        let partial = match language {
            "typescript" | "javascript" => self.typescript.analyze_file(path, &content)?,
            "python" => self.python.analyze_file(path, &content)?,
            "go" => self.go.analyze_file(path, &content)?,
            "rust" => self.rust.analyze_file(path, &content)?,
            "java" => self.java.analyze_file(path, &content)?,
            "kotlin" => self.kotlin.analyze_file(path, &content)?,
            _ => return Err(AnalyzerError::UnsupportedLanguage(path.display().to_string())),
        };

        let file_name = path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(AnalysisResult {
            path: path.display().to_string(),
            exports: Exports {
                functions: partial.functions,
                types: partial.types,
                classes: partial.classes,
                enums: partial.enums,
                variables: partial.variables,
                re_exports: partial.re_exports,
            },
            dependencies: Dependencies {
                external: partial.external_deps,
                internal: Vec::new(),
                internal_raw: partial.internal_deps,
            },
            behaviors: partial.behaviors,
            contracts: partial.contracts,
            protocol: partial.protocol,
            analyzed_files: vec![file_name],
        })
    }

    /// Analyze a directory with optional file filter.
    pub fn analyze_directory(
        &self,
        path: &Path,
        files: Option<&[&str]>,
    ) -> Result<AnalysisResult, AnalyzerError> {
        let mut result = AnalysisResult {
            path: path.display().to_string(),
            ..Default::default()
        };

        let files_to_analyze: Vec<_> = if let Some(filter) = files {
            filter.iter()
                .map(|f| path.join(f))
                .filter(|p| p.exists())
                .collect()
        } else {
            self.find_source_files(path)?
        };

        for file_path in files_to_analyze {
            if let Ok(file_result) = self.analyze_file(&file_path) {
                self.merge_results(&mut result, file_result);
            }
        }

        Ok(result)
    }

    /// Detect the programming language from file extension.
    fn detect_language(&self, path: &Path) -> Result<&'static str, AnalyzerError> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "ts" | "tsx" => Ok("typescript"),
            "js" | "jsx" | "mjs" | "cjs" => Ok("javascript"),
            "py" => Ok("python"),
            "go" => Ok("go"),
            "rs" => Ok("rust"),
            "java" => Ok("java"),
            "kt" | "kts" => Ok("kotlin"),
            _ => Err(AnalyzerError::UnsupportedLanguage(path.display().to_string())),
        }
    }

    /// Find all source files in a directory (non-recursive).
    /// INV-2: Self-contained boundary — only analyzes direct files;
    /// subdirectories are handled by their own CLAUDE.md.
    fn find_source_files(&self, path: &Path) -> Result<Vec<std::path::PathBuf>, AnalyzerError> {
        let mut files = Vec::new();

        if path.is_file() {
            files.push(path.to_path_buf());
            return Ok(files);
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                if self.detect_language(&entry_path).is_ok() {
                    files.push(entry_path);
                }
            }
        }

        Ok(files)
    }

    /// Merge partial results into the main result.
    fn merge_results(&self, target: &mut AnalysisResult, source: AnalysisResult) {
        target.exports.functions.extend(source.exports.functions);
        target.exports.types.extend(source.exports.types);
        target.exports.classes.extend(source.exports.classes);
        target.exports.enums.extend(source.exports.enums);
        target.exports.variables.extend(source.exports.variables);
        target.exports.re_exports.extend(source.exports.re_exports);

        // Deduplicate dependencies
        for dep in source.dependencies.external {
            if !target.dependencies.external.contains(&dep) {
                target.dependencies.external.push(dep);
            }
        }
        for dep in source.dependencies.internal_raw {
            if !target.dependencies.internal_raw.contains(&dep) {
                target.dependencies.internal_raw.push(dep);
            }
        }

        target.behaviors.extend(source.behaviors);
        target.contracts.extend(source.contracts);

        // Merge protocol (take the first non-empty one or merge)
        if let Some(src_protocol) = source.protocol {
            if let Some(ref mut tgt_protocol) = target.protocol {
                tgt_protocol.states.extend(src_protocol.states);
                tgt_protocol.transitions.extend(src_protocol.transitions);
                tgt_protocol.lifecycle.extend(src_protocol.lifecycle);
            } else {
                target.protocol = Some(src_protocol);
            }
        }

        target.analyzed_files.extend(source.analyzed_files);
    }
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
