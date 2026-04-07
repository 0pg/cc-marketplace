# code_analyzer (language analyzers)

## Purpose

Provide per-language static analysis implementations that extract exports, dependencies, behaviors, contracts, and protocol information from source files without executing code. Each file implements the `LanguageAnalyzer` trait for one target language, enabling the `CodeAnalyzer` orchestrator to produce language-agnostic `AnalysisResult` values used downstream by `analyze-code` and `format-analysis` CLI subcommands.

## Requirements

- REQ-1: Support six target languages — Rust, TypeScript/JavaScript, Java, Kotlin, Go, Python — each in its own source file.
- REQ-2: Extract exported public symbols (functions, structs/classes, interfaces, enums, type aliases, constants, variables, re-exports) using compiled regex patterns pre-built in `new()`.
- REQ-3: Extract external and internal dependency lists from import/use statements.
- REQ-4: Extract function-level contracts (preconditions, postconditions, throws/errors) from language-specific doc comment formats (Rust `///`, JSDoc `/** */`, Javadoc, Go `//`, Python docstrings).
- REQ-5: Infer state-machine protocol information (state names, lifecycle method order) from state enums, iota constants, and discriminated unions.
- REQ-6: Infer error behaviors (input → output pairs) from error enum variants, `throw new`, `return nil, Err*`, and `raise` patterns.
- REQ-7: Detect environment variable references (`process.env`, `os.Getenv`, `os.environ`) and return them deduplicated and sorted.
- REQ-8: All analyzer structs must implement `Default` (delegating to `new()`), and `analyze_file` must return `Result<PartialAnalysis, AnalyzerError>` with no panics in library code.

## Domain Context

- These modules form the inner layer of the two-tier CLI architecture (Tier 1: deterministic, no LLM); they must never perform network I/O, file I/O beyond the single file passed in, or any async operations.
- The `LanguageAnalyzer` trait is the single extension point; each language module is self-contained and depends only on `regex` and types from the parent `code_analyzer` module via `super::`.
- Regex patterns are compiled once in `new()` to amortize cost across multiple file analyses.
- Export detection follows each language's visibility rules: `pub` in Rust, `export` keyword in TypeScript, uppercase initial in Go, `public` modifier in Java/Kotlin, and `__all__` membership or top-level `def`/`class` in Python.
- Contract extraction from doc comments is opportunistic — only structured annotation tags (`# Arguments`, `@precondition`, `Precondition:`, Javadoc `@param`/`@return`/`@throws`, Python `Args:`/`Returns:`/`Raises:`) produce contract entries; absence of annotations yields an empty contracts list without error.
