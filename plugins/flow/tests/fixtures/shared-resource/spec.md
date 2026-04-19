# Spec: deploy and smoke-test two Golem workers sharing port :9881

## Request (verbatim)
Two Golem workers must be deployed and each given a smoke-test invocation. Both components are published to the local Golem server at `http://localhost:9881`.

## Scope
- In scope:
  - Deploy component `worker-a` to Golem at `localhost:9881`.
  - Deploy component `worker-b` to Golem at `localhost:9881`.
  - Invoke each component and assert a response envelope.

- Out of scope:
  - Golem server setup (assume already running).

## Acceptance criteria
1. `golem component add worker-a --host localhost:9881` succeeds (exit 0).
2. `golem component add worker-b --host localhost:9881` succeeds (exit 0).
3. `golem invoke worker-a --host localhost:9881` returns `{status:"ok"}`.
4. `golem invoke worker-b --host localhost:9881` returns `{status:"ok"}`.

## project_test_cmd
`bash tests/smoke.sh`

## Notes
Both nodes write through the same Golem HTTP endpoint on port 9881. Planner MUST serialize the two deploy nodes (per flow-planner.md § Shared external resources: default-serialize when the same literal `:9881` appears in ≥2 node specs).

## Expected planner behavior
- Two work nodes, `deploy-worker-a` and `deploy-worker-b`.
- `deploy-worker-b.deps` contains `deploy-worker-a` (serial chain, no fan-out).
- No fake parallel branching.
