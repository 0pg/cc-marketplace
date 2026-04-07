# code_analyzer (language analyzers) — Developer Specification

## Constraints

- CONST-1: `LanguageAnalyzer::analyze_file` receives a `&Path` (for language detection context) and `&str` (full file content); it must return `Ok(PartialAnalysis)` for any syntactically valid or invalid source file — parse failures are silent (empty result), not errors.
- CONST-2: `AnalyzerError::UnsupportedLanguage` is the only error variant returned by `analyze_file`; it is never returned by individual language analyzers (the orchestrator gates on extension before dispatch).
- CONST-3: All regex patterns are compiled in `new()` using `Regex::new(...).unwrap()`; patterns must be valid at compile time — runtime `unwrap()` panics on invalid regex are acceptable only in `new()` (constructor contract).
- CONST-4: `PartialAnalysis::external_deps` must contain no duplicates; each analyzer guards with `.contains()` before pushing.
- CONST-5: `PartialAnalysis::env_vars` must be sorted and deduplicated before returning (`.sort()` then `.dedup()`).
- CONST-6: Re-export deduplication: single-form `pub use path::Name` must not be added if already captured by group-form `pub use path::{Name1, Name2}` (TypeScript: `export { default as X }` is handled before general re-export pass).
- CONST-7: Behavior inference for "Expired token" / "Invalid token" must check for existing entries before pushing to avoid duplicate behaviors.
- CONST-8: Protocol information is returned as `Some(Protocol)` only when `states` or `lifecycle` is non-empty; otherwise `None`.
- CONST-9: Contract entries are only added when at least one of `preconditions`, `postconditions`, `throws` (or `invariants` for TypeScript) is non-empty.
- CONST-10: Go analyzer exports only capitalized (uppercase-initial) names; Java/Kotlin use `public` modifier; Python uses `__all__` membership or top-level scope.

## Technical Context

- Language: Rust, edition 2021; no async, no unsafe.
- Key dependency: `regex 1.10` — all patterns use named-capture groups or positional groups; multiline (`(?m)`) and dotall (`(?s)`) flags are used extensively.
- `PartialAnalysis` is the accumulator struct populated by each language analyzer; the parent `CodeAnalyzer::analyze_directory` merges multiple `PartialAnalysis` values into a single `AnalysisResult`.
- TypeScript analyzer handles both `.ts` and `.js` files; JavaScript-specific patterns (no type annotations) degrade gracefully.
- Kotlin analyzer (kotlin.rs) follows the same pattern as Java but uses `fun` keyword and Kotlin visibility modifiers.
- The `extract_function_body` helper in TypeScript uses brace counting rather than a full parser — handles single-level nesting only; deeply nested lambdas may produce incomplete bodies for validation inference, which is acceptable for the current use case.

## Decision Log

- **Regex pre-compilation in `new()`**: Avoids per-call compilation overhead when analyzing many files in a directory scan. The `unwrap()` in constructors is intentional — a malformed regex is a programmer error caught at startup, not a runtime condition.
- **No AST / language parser**: The module deliberately avoids language-specific parsers (syn, tree-sitter, etc.) to keep the binary small and the build fast. Regex-based extraction is good enough for the export/dependency surface needed by downstream CLI subcommands.
- **Behavior inference is heuristic**: Error behavior inference (searching for "Expired"/"Invalid" patterns) is intentionally narrow to avoid false positives. The correct approach is structured doc-comment tags; inference is a fallback for undocumented code.
- **`is_error` detection by name suffix**: In `rust_lang.rs`, error enum variant extraction is triggered when the enum name contains "Error". This is a convention-based heuristic aligned with the project's Naming Rules (PascalCase error types end in `Error`).

## Agent Observations

### [structural] format-analysis CLI fails on analyze-code output (missing `transitions` field)
- anchor: none
- since: 2026-04-07
- refs: 2
- source: /decompile decompiler
- `format-analysis` expects a `transitions` field in the analyze-code JSON that the current `analyze-code` output does not include (protocol.transitions missing). The decompiler fell back to direct source reading. This is a schema mismatch between `analyze-code` output and `format-analysis` input that should be addressed in the parent `code_analyzer.rs` serialization.
