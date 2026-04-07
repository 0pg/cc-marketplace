# core/tests — Developer Specification

## Constraints

- CONST-1: `cucumber.rs` is the single entry point for all BDD step definitions; it must re-export or inline every step required by the `.feature` files in `features/`.
- CONST-2: `TestWorld` implements `cucumber::World` and `Default`; all scenario-scoped state is held in `Option<T>` fields so the derive macro can zero-initialize between scenarios.
- CONST-3: Temporary file system operations use `tempfile::TempDir`; the `TempDir` handle must remain alive for the duration of the scenario (stored in `TestWorld.temp_dir`).
- CONST-4: Feature files in `features/` reference fixture files under `features/` subdirectories (e.g., `fixtures/typescript/index.ts`); the paths are resolved relative to `CARGO_MANIFEST_DIR/tests` via `get_tests_path()`.
- CONST-5: No `unwrap()` is permitted in step-definition helper functions; use `expect("…")` with a descriptive message.
- CONST-6: The Cucumber runner uses the Tokio multi-thread runtime; step functions must remain synchronous (`fn`, not `async fn`) unless the scenario explicitly requires async behavior.
- CONST-7: Each `.feature` file is self-contained with its own `Background` block; shared setup steps must not depend on ordering with steps from other feature files.

## Technical Context

- **Test framework**: `cucumber 0.21` with `#[derive(World)]` macro for scenario state management.
- **Async runtime**: `tokio` (rt-multi-thread feature) driving the Cucumber runner.
- **Temp file management**: `tempfile 3.9` (`TempDir`) for isolated filesystem scenarios.
- **Subject under test**: `claude_md_core` crate — all public types and subcommand entry points are imported directly (no FFI, no subprocess).
- **Fixture languages**: Go, Java, Kotlin, Python, Rust, TypeScript — one subdirectory each under `fixtures/`, plus `parser/`, `empty/`, and `expected/` for schema/parser edge cases.
- **Feature coverage** (18 files): `agent_observations`, `boundary_resolver`, `bugfix`, `claude_md_parser`, `code_analyze`, `commit_hash_handoff`, `compile_target_resolver`, `convention_validator`, `dev_green_refactor_pipeline`, `dev_test_review_loop`, `developers_md_validator`, `format_exports`, `impl_inter_session`, `impl_socratic_loop`, `language_validator`, `schema_rules`, `schema_validator`, `tree_parser`.

## Decision Log

- **Single `cucumber.rs` file**: All step definitions are consolidated in one file to avoid Rust's integration-test linking restrictions; splitting across modules would require a `mod` tree that complicates the `#[given/when/then]` attribute routing.
- **`Option<T>` fields in `TestWorld`**: Using `Option` instead of bare values allows `Default` derivation and makes missing-state bugs surface as panics with descriptive messages rather than silent wrong answers.

## Agent Observations

None
