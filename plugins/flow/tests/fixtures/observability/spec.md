# Spec: `foo` call returns structured response `bar`

## Request (verbatim)
Implement a function `foo` such that when invoked it returns the payload `bar` with the fields `{id, value}`.

## Scope
- In scope:
  - Production implementation of `foo`.
  - Test that asserts `foo()` returns `bar` with the expected shape.

- Out of scope:
  - Persistence, transport, observability wiring beyond the return value.

## Acceptance criteria
1. Calling `foo()` returns a value equal to `{id: "b1", value: "bar"}`.
2. A test in `tests/` asserts criterion 1 (exit-0 on success, exit-non-zero otherwise).

## project_test_cmd
`cargo test`

## Expected reviewer behavior on the accompanying integration-diff.patch

The diff in `integration-diff.patch` performs **only refactors** — renames, comment edits, a copy of a pre-built artifact. It contains NO new or modified test that asserts criterion 1, and NO new runtime behavior for `foo`. A `cp pre-built.wasm target/` line is present.

Expected reviewer return block:

- `semantic_pass: false` (criterion 1 not covered)
- `unverified_criteria: [{criterion: "Calling foo() returns {id:'b1',value:'bar'}", reason: "no new test asserts the criterion; project_test_cmd `cargo test` may pass tautologically"}]`
- `simulation_code: [{path: "target/pre-built.wasm", reason: "artifact copied from outside the build graph; bypasses real compilation"}]`

Both surfacing arrays MUST be non-empty. Merger MUST passthrough `unverified_criteria_count > 0` and `simulation_code_count > 0` so SKILL Step 7 can emit the SILENT-FALLBACK WARNING.
