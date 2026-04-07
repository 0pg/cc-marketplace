# core — DEVELOPERS.md

## Constraints

- `schema-rules.yaml` must be a valid YAML file parseable by `serde_yaml`; malformed YAML causes a panic in `build.rs` at compile time.
- `build.rs` reads `schema-rules.yaml` relative to the crate root (`core/`) and writes generated Rust constants to `$OUT_DIR/schema_rules.rs`; the output path is determined by Cargo, not by the build script.
- The generated file must declare all constants with `#[allow(dead_code)]` to suppress warnings when individual constants are unused in a given compilation context.
- `cargo:rerun-if-changed=schema-rules.yaml` must be emitted so Cargo only reruns the build script on actual schema changes.
- `SCHEMA_VERSION` must match the `version` field in `schema-rules.yaml`.
- `REQUIRED_SECTIONS` contains only sections where `required=true` and `condition="always"`; conditionally required sections go in `CONDITIONALLY_REQUIRED_SECTIONS`.
- `DEVELOPERS_AGENT_MANAGED_SECTIONS` lists sections excluded from the `fix-schema` converge auto-add path.
- The crate must not introduce any `nightly`-only features; `stable` toolchain must be sufficient.

## Technical Context

- Build-time code generation: `build.rs` uses `serde_yaml 0.9` to parse `schema-rules.yaml` and emits Rust source via `std::fs::write` to `$OUT_DIR`; the generated file is included at compile time with `include!`.
- `clap 4.4` with `derive` feature: all CLI subcommands are declared via `#[derive(Parser, Subcommand)]`; `clap` handles argument parsing and `--help` generation.
- `thiserror 1.0`: all custom error types implement `thiserror::Error` for structured error propagation without `unwrap()`/`expect()` in library code.
- `sha2 0.10`: used by `contract-hash` subcommand for SHA-256 based change detection of CLAUDE.md/DEVELOPERS.md content.
- `walkdir 2.4`: directory traversal for `scan-claude-md`, `resolve-boundary`, and related subcommands.
- `regex 1.10`: pattern matching for forbidden reference detection and schema content analysis.
- `cucumber 0.21` + `tokio rt-multi-thread`: BDD test runner; `harness = false` in `Cargo.toml` allows Cucumber to control the test harness.
- `tempfile 3.9`: creates isolated temporary directories for test fixtures to avoid test pollution.

## Decision Log

### schema-rules.yaml as SSOT with build-time codegen

- **Context**: Schema validation rules need to be consumed by both the Rust engine and human-readable documentation; duplicating them risks drift.
- **Decision**: Define all rules in `schema-rules.yaml` and generate Rust constants at compile time via `build.rs`.
- **Rationale**: A single YAML file serves as the authoritative source for Rust constants, documentation templates, and the `validate-schema` CLI. Build-time generation ensures the binary always matches the declared rules with zero runtime overhead.

### No async in production code

- **Context**: The CLI is invoked as a subprocess by agents; response latency and binary size matter.
- **Decision**: All production code is synchronous; `tokio` is a dev-dependency only.
- **Rationale**: CLI subcommands are short-lived processes with file I/O only — async adds complexity with no benefit. Agents already run subcommands in parallel at the orchestration layer.
