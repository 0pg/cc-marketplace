# core/src — Developer Specification

## Constraints

- CONST-1: `main.rs` uses `clap` derive macros; every `Commands` variant must have a matching arm in the `match` block or the code will fail to compile.
- CONST-2: `output_result<T: serde::Serialize>` writes JSON to file when `--output` is provided, otherwise to stdout; callers must not assume a specific output channel.
- CONST-3: `is_none_marker_content(lines: &[&str]) -> bool` returns `true` only when exactly one non-empty, non-header line exists and its value (after stripping markdown list prefixes `- `, `* `, `+ `, `N. `) case-insensitively equals `"none"` or `"n/a"`.
- CONST-4: `EXCLUDED_DIRS` and `SOURCE_EXTENSIONS` are `pub const` slices in `lib.rs`; any module needing them must import from `crate::` not redeclare locally.
- CONST-5: In `validate-schema --strict`, all warnings whose text starts with `"INV-3:"` are moved to `errors` and `valid` is recomputed; remaining warnings are preserved.
- CONST-6: `--min-completeness` defaults to `0` (no enforcement); when > 0, a `ValidationError { error_type: "InsufficientCompleteness" }` is appended and `valid` set to `false` if `completeness_score < min_completeness`.
- CONST-7: `fix-schema --dry-run` outputs `{"changes":[],"warnings":[]}` (JSON) when no changes are needed; otherwise outputs `{"changes":[...], "warnings":[...]}` without modifying any file.
- CONST-8: All fallible operations in library code use `Result<T, E>`; `unwrap()` and `expect()` are forbidden outside test code.
- CONST-9: `AnalyzeCode` command optionally accepts `--tree-result <path>` to enable internal dependency resolution via `DependencyResolver`; parse failures emit a warning to stderr but do not abort execution.

## Technical Context

- Binary: `claude-md-core` (Rust, edition 2021, stable toolchain)
- CLI framework: `clap 4.4` with derive macros (`Parser`, `Subcommand`)
- Serialization: `serde + serde_json` for all cross-boundary data types
- File walking: `walkdir 2.4`
- Regex: `regex 1.10`, compiled once per analyzer struct in `new()`
- Error types: `thiserror 1.0` for all custom errors
- Hashing: `sha2 0.10` for `contract-hash` SHA-256
- Statics: `OnceLock` for lazily initialized read-heavy globals; `Mutex<Option<T>>` pattern is prohibited
- Schema constants embedded at compile time from `schema-rules.yaml` via `build.rs` `include!` macro
- No async runtime in production code; async is limited to the Tokio test runtime

## Decision Log

- **Single binary entry point (`main.rs`)**: All subcommands route through one `match` block. This keeps the subprocess interface simple for SKILL callers and avoids multiple binary artifacts.
- **JSON-to-stdout contract**: Every subcommand serializes its result struct and prints it, enabling SKILL orchestrators to pipe output directly into the next step without intermediate files (unless `--output` is provided for debugging).
- **Shared constants in `lib.rs`**: `EXCLUDED_DIRS` and `SOURCE_EXTENSIONS` are declared once and re-used across `tree_parser`, `compile_target_resolver`, and `code_analyzer`, enforcing DRY and preventing drift between scanning and analysis.
- **`is_none_marker_content` in `lib.rs`**: Shared between `claude_md_parser` and `schema_validator` to avoid duplication of the "None" detection logic, which must match `none_marker.values` in `schema-rules.yaml`.

## Agent Observations

None
