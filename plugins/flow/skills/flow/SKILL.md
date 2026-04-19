---
name: flow
version: 0.2.0
aliases: [dag-run, run-dag, flow-run]
description: |
  This skill should be used when the user asks to "run as DAG", "execute in parallel with merge gates",
  "fan out and merge", "run a task graph", or uses /flow. Turns a user request into atomic subtasks,
  builds a DAG (LLM-generated from spec), and executes independent nodes in parallel under
  git-worktree isolation. Merge+validate runs as an independent cascade step
  (git-conflict → project tests → LLM semantic review; first pass wins, break).
  Persistent state at .claude/workflows/flow/{task-id}/ enables resume across sessions.
  Trigger keywords: DAG, task graph, parallel execution with merge, fan-out fan-in
user_invocable: true
allowed-tools: [Read, Write, Edit, Bash, Glob, Grep, Task, Skill, AskUserQuestion]
---

# /flow

End-to-end DAG executor. Orchestrates `flow-interviewer → flow-planner → (flow-worker | flow-merger) ×N`.

## Triggers

- `/flow`
- `run as DAG`
- `fan out and merge`
- `parallel implementation with merge gate`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `request` | No* | — | Request text. *One AskUserQuestion if missing AND `--no-ask` not set. |
| `--no-ask` | No | false | Suppress user-facing prompts. Planner's self-review substitutes for user DAG approval. |
| `--max-retries` | No | 3 | Per-node retry cap. |
| `--resume <task-id>` | No | — | Delegate to `/flow-resume`. |

## Design stance

This SKILL is the **Brain layer** in Anthropic's Managed Agents architecture — it guides the model through the pipeline but does not itself reason about the work. Reasoning (requirement clarification, DAG shape, merge semantic validity) is delegated to agents. The SKILL's job is:

1. **Structural invariants** (INV-F1..F6) — persist state before action, explicit merge nodes, immutable DAG, retry bounds.
2. **Parallel dispatch mechanics** — batch-ready nodes in a single message of Task() calls.
3. **Error surfacing** — halt with full context, never silent fallback.

If a rule here could be replaced by "let the agent judge," prefer deletion. Numeric caps are bug-guards, not convergence criteria.

## Workflow

### 0. Initialization

```bash
# Ephemeral session dir for agent inputs
SESSION_TMP="/tmp/flow/${CLAUDE_SESSION_ID:-adhoc}"
mkdir -p "$SESSION_TMP"

# Persistent workflow root (task dirs live here)
WORKFLOW_ROOT=".claude/workflows/flow"
mkdir -p "$WORKFLOW_ROOT"
```

If `--resume <task-id>` is present: delegate to `/flow-resume` and return. Otherwise continue.

### 1. Intake

If `request` is empty AND `--no-ask` is false: one AskUserQuestion collects it. If `request` is still empty AND `--no-ask` is true: halt with `reason: missing-request`.

Generate a fresh `task-id` (`flow-$(date +%Y%m%d)-$(openssl rand -hex 4)` or similar) and create `$WORKFLOW_ROOT/$task_id/`.

Persist the raw request to `$WORKFLOW_ROOT/$task_id/request.txt`.

### 2. Interview — requirement clarification

Write interviewer session file: `$SESSION_TMP/interviewer-session.md` containing:

```markdown
type: interviewer
task_id: {task_id}
task_dir: {absolute path to .claude/workflows/flow/{task_id}/}
request: |
  {verbatim request}
no_ask: {true|false}
```

Dispatch `Task(subagent_type=flow-interviewer)` with the session file path. The agent MUST write `spec.md` into `task_dir` and return a result block with `status: success|needs_input|failed` + `spec_path`.

If `status = needs_input` AND `--no-ask` is true: halt with agent-reported reasons. Otherwise re-dispatch with the additional answers injected.

Gate: do NOT proceed to Step 3 without a committed `spec.md` containing at least:
- Acceptance criteria (≥1)
- `project_test_cmd` (may be literal "none" — explicitly, not missing)

### 3. Plan — DAG generation

Write planner session file: `$SESSION_TMP/planner-session.md` with the `spec.md` path + task metadata.

Dispatch `Task(subagent_type=flow-planner)`. The agent MUST:
- Produce `dag.json` at `$task_dir/dag.json`.
- Satisfy R3 (every `work` node has a validator or a review-type successor).
- Satisfy R4 (acyclic, reachable terminals, resolvable edges).
- Satisfy INV-F2 (every fan-in is an explicit `merge` node).
- Return a mermaid preview string.

**Structural validation is performed by the Hands layer — `flow-core validate-dag`.** The SKILL shells out:

```bash
FLOW_CORE="${CLAUDE_PLUGIN_ROOT}/core/target/release/flow-core"
if [ ! -x "$FLOW_CORE" ]; then
  # Halt with build instruction; do not fall back to LLM-only validation.
  echo '{"status":"halted","reason":"core-not-built","hint":"cd $CLAUDE_PLUGIN_ROOT/core && cargo build --release"}'
  exit 1
fi
"$FLOW_CORE" validate-dag "$task_dir/dag.json"
```

Exit code is authoritative:
- `0`: DAG is structurally valid. Proceed to Step 4.
- `1`: Violations reported as JSON `{valid: false, errors: [{code, node_id, message}, ...]}` on stdout. Inject the errors verbatim into the next planner dispatch and retry once. A second non-zero exit halts with `halted_reason: "dag-invalid-after-retry"`.
- `2`: I/O or parse error (planner produced invalid JSON). Halt immediately.

The SKILL itself does NOT reinterpret the validator output; planner-agent is the sole consumer of the error list for self-correction (per Harness Design Principles in CLAUDE.md — "Hands layer whose output the Brain reinterprets is an anti-pattern").

### 4. Approval gate

If `--no-ask` is false: show the mermaid preview + node summary to the user via `AskUserQuestion` with options `[Approve, Reject, Edit]`. On `Reject`: halt. On `Edit`: re-dispatch the planner with the user's feedback.

If `--no-ask` is true: accept the DAG as-is (planner's self-review is authoritative), BUT SKILL still runs the structural validations from Step 3.

### 5. Initialize state.json

Write `$task_dir/state.json`:

```json
{
  "task_id": "...",
  "status": "running",
  "started_at": "...",
  "max_retries": 3,
  "nodes": {
    "<node-id>": { "status": "pending", "attempts": 0, "deps": [...], "type": "work|merge" }
  }
}
```

### 6. Execution loop

```
loop:
  load state.json
  if all nodes.status == "complete":
    state.status = "complete"; persist; break
  if any nodes.status == "failed" AND attempts >= max_retries:
    state.status = "halted"; persist; break
  ready = { n | n.status == "pending" AND all(deps[d].status == "complete" for d in n.deps) }
  if ready == ∅ AND no node is running:
    # Should be impossible with a valid DAG; still guard.
    state.status = "halted"; reason = "deadlock-or-invalid-dag"; break
  # INV-F1: persist "running" BEFORE dispatching
  for n in ready:
    state.nodes[n].status = "running"
    state.nodes[n].started_at = now()
  persist state.json
  # Parallel batch dispatch — single message with multiple Task() calls
  results = Task.parallel([
    Task(subagent_type=n.agent, session_file=write_node_session(n))
    for n in ready
  ])
  for (n, result) in zip(ready, results):
    persist nodes/{n.id}/output.md and validator.json from result
    pass = evaluate_validator(n, result)  # deterministic command first; fall back to agent-reported
    if pass:
      state.nodes[n].status = "complete"
    else:
      state.nodes[n].attempts += 1
      append attempts.jsonl with failure context
      if state.nodes[n].attempts >= max_retries:
        state.nodes[n].status = "failed"
      else:
        state.nodes[n].status = "pending"  # requeue
    persist state.json
```

Per-node session file conventions:
- `$SESSION_TMP/worker-session-{node-id}.md` for `flow-worker`
- `$SESSION_TMP/merger-session-{node-id}.md` for `flow-merger`

Each session file includes the full node block from `dag.json`, `spec.md`, worktree base path (`$task_dir/worktrees/{node-id}/`), and target branch name (`flow/{task_id}/{node-id}`).

### 7. Report

On loop exit, print a summary:

- Task status (complete | halted).
- Per-node: `id | title | status | attempts | duration`.
- If `complete`: integration branch ref and suggested `git checkout` command.
- If `halted`: failure excerpt per failed node + `Use /flow-resume {task-id}` hint.

Print `state.json` path for machine-readable follow-up.

## Validator evaluation

```
evaluate_validator(node, result):
  if node.type == "merge":
    # merger returns validators.json with cascade results; SKILL just trusts it.
    return result.validators.any(step => step.status == "pass")
  if node.validator.kind == "command":
    run node.validator.command in the node's worktree
    return exit == node.validator.expected_exit
  if node.validator.kind == "schema":
    # Planner MAY emit schema-kind for structured outputs; agent returns a JSON that must match.
    return jsonschema_match(result.output, node.validator.schema)
  if node.validator.kind == "none":
    # Must be followed by a review successor (R3). Trust the successor's validator.
    return true
```

## Invariants enforced by SKILL

- **INV-F1**: `state.nodes[n].status = "running"` persists to disk before `Task()` dispatch in Step 6.
- **INV-F2**: Enforced by `flow-core validate-dag` (Hands layer). SKILL rejects planner output on non-zero exit. R4 (acyclic / ref integrity / terminal), R5 (fan-in → merge), R3 (kind=none successor), and enum correctness are all enforced by the same CLI invocation.
- **INV-F3**: Merge cascade ordering (step1 → step2 → step4) is enforced inside `flow-merger`, not here; SKILL only validates its `validators.json` schema.
- **INV-F4**: Worker session files MUST include the worktree base path; workers create the worktree before committing.
- **INV-F5**: `dag.json` at `$task_dir/dag.json` is read-only after Step 5. SKILL never edits it post-initialization; `/flow-resume` does not either.
- **INV-F6**: `max_retries` is a halt trigger, not a convergence criterion. On exhaustion, halt and surface — never silent skip.

## Halt surfaces

A halt includes:
- `state.status = "halted"`, `halted_reason` in state.json.
- Per-failed-node: last `attempts.jsonl` entry (exit code, stderr excerpt, validator output).
- `git branch --list 'flow/{task_id}/*'` so the user can inspect partial work.
- Hint: `/flow-resume {task_id}`.
