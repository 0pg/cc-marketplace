# flow — behavioral fixtures

Manual-dispatch fixtures covering Brain-layer prose additions introduced in v0.3.0. Each fixture asserts a single agent's behavioral claim and is intended to be dispatched by hand via `Task(subagent_type=...)` or equivalent harness.

Automation (CI integration, agent dispatch simulation) is a follow-up — see "Open follow-ups" below.

## Fixtures

### `fixtures/shared-resource/` — planner shared-resource detection (Patch 1)

Asserts `flow-planner` serializes nodes whose `spec` share an external literal (port, service name, connection string) via a `deps` edge by default.

- Input: `spec.md` — two nodes referencing `:9881` (Golem deploy-target analogue).
- Dispatch: `Task(subagent_type=flow-planner)`, `task_dir = <fixture path>`.
- Expected output: `dag.json` where the second node's `deps` contains the first node's id. Mermaid preview visualizes the serial chain.
- Failure signal: the two nodes emit `deps: []` and run in parallel — indicates the planner prose did not fire.

### `fixtures/command-shape/` — interviewer command-shape rigor (Patch 2)

Asserts `flow-interviewer` rejects or rewrites a `project_test_cmd` that is static-only (`cargo build` with no runtime assertion).

- Input: `request.txt` — a request containing an observable-runtime claim ("GET /health returns 200") paired with a draft `project_test_cmd = "cargo build"`.
- Dispatch: `Task(subagent_type=flow-interviewer)`, `no_ask: true`.
- Expected output: `spec.md` where `project_test_cmd` has been replaced with a runtime-asserting command, OR `## Assumptions` documents the static-only flag + recommended runtime command.
- Failure signal: `spec.md`'s `project_test_cmd` is still literal `cargo build` with no assumption note — the shape-rigor branch in step 5 did not fire.

### `fixtures/observability/` — reviewer observability + simulation detection (Patch 3)

Asserts `flow-reviewer` populates `unverified_criteria` / `simulation_code` when an integration diff fails to produce observable evidence for an acceptance criterion or contains sim-mode code.

- Input: `spec.md` (acceptance criterion: "calling `foo` returns `bar`") + `integration-diff.patch` (refactor-only diff, or diff including `cp pre-built.wasm target/`).
- Dispatch: `Task(subagent_type=flow-reviewer)`.
- Expected output: return block contains `unverified_criteria: [...]` (non-empty) when the criterion is not observable, and/or `simulation_code: [...]` (non-empty) when sim-mode artifacts are detected.
- Failure signal: reviewer returns `semantic_pass: true` with both arrays empty — the new Acceptance-criterion observability branch did not fire.

### End-to-end SKILL Step 7 surfacing (cross-Patch, SKILL 5b + merger 6)

Fixture C's reviewer output fed through the merger passthrough and SKILL Step 7 must produce a **SILENT-FALLBACK WARNING** section in the final SKILL report. This is a full-pipeline dry-run using `fixtures/observability/` as the merge input. Manual dispatch only in v0.3.0.

## Open follow-ups

- Automated dispatch harness (CI) — currently the fixtures are evaluated by hand.
- `cargo workspace member` / `package.json workspaces` auto-detection feeding the planner with buildable partitionability hints — Hands-layer candidate.
- Happy-path observability (step 1 clean merge → reviewer never runs) is a known gap in v0.1 cascade design. Not covered by any fixture here. Requires `flow-merger.md` cascade policy change → separate user-approved work.
