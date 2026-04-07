# core/src

## Purpose

Implement the deterministic Rust CLI engine that powers all `claude-md-core` subcommands. This module provides the library crate (all business logic modules) and the binary entry point (`main.rs`) that routes CLI arguments to the appropriate module, enabling LLM agents to invoke static analysis, schema validation, boundary resolution, and document diffing without any network I/O or LLM calls.

## Requirements

- REQ-1: Expose all CLI subcommands (`parse-tree`, `resolve-boundary`, `validate-schema`, `parse-claude-md`, `validate-convention`, `analyze-code`, `scan-claude-md`, `diff-compile-targets`, `diff-spec-range`, `contract-hash`, `fix-schema`, `format-exports`, `format-analysis`, `validate-language`) through a single binary entry point.
- REQ-2: Write all successful subcommand results as pretty-printed JSON to stdout (or to a file when `--output` is provided); write errors to stderr and exit with code 1.
- REQ-3: Provide shared constants (`EXCLUDED_DIRS`, `SOURCE_EXTENSIONS`) and the `is_none_marker_content` utility used across multiple modules, accessible via `lib.rs`.
- REQ-4: Re-export primary public types from each sub-module so callers need only depend on the crate root (`claude_md_core::`).
- REQ-5: Language detection and code analysis must support at least six languages (Rust, TypeScript/JavaScript, Python, Go, Java, Kotlin) as determined by the `code_analyzer` child module.
- REQ-6: Schema validation must support both standard and strict modes; in strict mode, `INV-3` (DEVELOPERS.md absence) is promoted from warning to error.
- REQ-7: `validate-schema` must accept an optional `--min-completeness` threshold (0–100); validation fails when the completeness score falls below the threshold.
- REQ-8: `fix-schema` must support `--dry-run` to report changes as JSON without modifying files.

## Domain Context

- This module is the Tier 1 (deterministic CLI) layer of the two-tier claude-md-plugin architecture; it must never perform LLM calls, network I/O, or async operations in production code.
- All subcommands are invoked by SKILL entry points and Agents via subprocess; the JSON-to-stdout contract is the primary integration surface.
- Schema rules are the authoritative source of truth defined in `core/schema-rules.yaml`; generated constants are embedded at compile time via `build.rs` (`include!` macro in `schema_validator.rs`).
- The `code_analyzer` child module owns language-specific analysis logic; `core/src` only orchestrates it through the `CodeAnalyzer` facade.
