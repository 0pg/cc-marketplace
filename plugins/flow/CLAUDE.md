# flow

## Purpose

A **DAG-based task execution engine** that turns a user's feature/refactor request into a directed acyclic graph of atomic code-producing subtasks, executes independent nodes in parallel under git-worktree isolation, and performs **merge+validate as an independent, cascading step** so that integration failures cannot silently corrupt main.

`flow` solves the recurring problem that linear-pipeline workflows have: when multiple agents work in parallel, their outputs get blindly concatenated and "merge" is treated as a trivial byproduct rather than a first-class concern. `flow` makes the merge step a named node with its own validation cascade and retry surface.

## Scope

**In scope (v0.1):**
- User request intake → requirement clarification → spec finalization
- Task decomposition into atomic subtasks (each with a deterministic best-effort validator)
- DAG construction (LLM-generated from spec, user-approved)
- Parallel execution with git-worktree isolation per node
- Merge+validate cascade: git-conflict check → project test command → LLM semantic review (first pass wins, break)
- Persistent state at `.claude/workflows/flow/{task-id}/` — resumable across sessions
- Auto retry (max N, default 3) per node; halt on exhaustion
- Status, graph rendering, and resume commands

**Out of scope (v0.1):**
- Distributed execution across machines
- Non-code artifact merging (docs, specs) — treated as `work` nodes whose validator is the node's own test command, not merger semantics
- Dynamic DAG mutation after execution starts
- Build-only validation (compile without tests) — explicitly excluded per design decision
- Visual UI (only mermaid text output)

## Roles

| Role | Definition | Workflow position |
|------|------------|-------------------|
| **User** | The requester of the work | Interacts during intake (Step 1–2) and DAG approval (Step 4) |
| **Orchestrator** | The `flow` SKILL running in the main session | Creates task-id, dispatches agents, owns state.json |
| **Interviewer** | `flow-interviewer` agent | Turns request into `spec.md` with explicit acceptance criteria + `test_cmd` |
| **Planner** | `flow-planner` agent | Turns `spec.md` into `dag.json` with atomic nodes + per-node validators + merge nodes |
| **Worker** | `flow-worker` agent (one per node dispatch) | Executes a single work node in an isolated git worktree, commits to its branch |
| **Merger** | `flow-merger` agent | Combines parent branches, runs the cascade validator, returns PASS/FAIL |
| **Reviewer** | `flow-reviewer` agent | LLM semantic validator used by merger as cascade step (4) |

## Requirements

- **R1 (Intake):** `/flow "<request>"` (or `/flow` prompting once) MUST create a new task-id and begin interview. When invoked without a request and `--no-ask` is NOT set, the SKILL asks exactly one question to collect the request.
- **R2 (Spec finalization):** Before task execution begins, a committed `spec.md` MUST exist under the task directory. The SKILL MUST NOT proceed to DAG generation with empty or ambiguous acceptance criteria.
- **R3 (Atomic subtasks):** Each `work` node in `dag.json` MUST have a `validator` field whose `kind ∈ {command, schema, none}`. When `kind = none`, the node MUST be followed by a review-type successor. The planner is responsible for ensuring this; the SKILL rejects `dag.json` that violates it.
- **R4 (DAG validity):** `dag.json` MUST be acyclic, MUST contain ≥1 terminal node, and every edge's source/target MUST resolve to a declared node id. The SKILL rejects violating DAGs.
- **R5 (Parallel execution + merge isolation):** Nodes whose dependencies are all `complete` MUST be dispatched in a single parallel batch. Every edge convergence point (fan-in ≥ 2) MUST have an explicit `merge` node — `work` nodes are never allowed to have ≥2 parents directly.
- **R6 (State tracking):** Every node MUST have a status ∈ `{pending, running, complete, failed}` recorded in `state.json`. Status transitions MUST be persisted before the next orchestration step proceeds.
- **R7 (Task completion):** The task is `complete` IFF every node's status is `complete`. Any `failed` node after retry exhaustion transitions the task to `halted` and surfaces the failure.

## Invariants

### INV-F1: Persistent state precedes work
```
∀ node dispatch D:
  state.json[D.node_id].status = running MUST be persisted to disk
  BEFORE the corresponding Task(agent) call is issued.
```
Prevents orphaned in-flight work after session interruption.

### INV-F2: Merge node ownership of convergence
```
∀ node N ∈ dag.nodes where |N.deps| ≥ 2:
  N.type = "merge"
```
Forbids implicit multi-parent `work` nodes. Merge must be explicit and validated.

### INV-F3: Merge cascade short-circuit
```
merger(N) runs validators in order [step1, step2, step4]:
  step1 = "git merge parents → integration branch, no conflict ∧ exit=0"
  step2 = "run spec.test_cmd on integration branch, exit=0"
  step4 = "flow-reviewer semantic PASS on integration diff"

∀ step s:
  s.PASS → merger.result = valid, remaining steps SKIPPED, recorded as "skipped-due-to-earlier-pass"
  step1.FAIL → merger.result = invalid (merge itself failed; no valid state to validate further)
  step1.PASS ∧ step2.FAIL → escalate to step4
  step1.PASS ∧ step2 absent → escalate to step4
  step4.FAIL → merger.result = invalid
```
Note: the user-specified order is 1→2→4 (step 3 build-only is excluded). `spec.test_cmd` MAY invoke build as a prerequisite inside the command.

### INV-F4: Worktree isolation
```
∀ work node W:
  W executes inside a dedicated git worktree at
    .claude/workflows/flow/{task-id}/worktrees/{node-id}/
  W's branch = flow/{task-id}/{node-id}
  W MUST commit all produced changes to its branch before returning success
```
Guarantees rollback-by-branch-delete and clean merge semantics.

### INV-F5: Immutable DAG
```
Once dag.json is written at task-id/dag.json and state.json begins tracking nodes,
dag.json MUST NOT be modified for the remainder of task execution.
```
Dynamic re-planning is handled by starting a new task (possibly seeded from the failed one), not by mutating in flight.

### INV-F6: Retry bounds are a bug-guard, not a convergence criterion
```
MAX_RETRIES (default 3) is the orchestrator's safety net.
A node that fails 3 consecutive times is a halt signal, not "give up quietly":
state.json.status = halted, failure context preserved verbatim, user is surfaced
the failure at /flow-status and when /flow-resume is invoked.
```

## Architecture

### Session + state layout

```
.claude/workflows/flow/{task-id}/           ← persistent, git-ignored
├── spec.md                                 ← finalized requirements (from interviewer)
├── dag.json                                ← immutable DAG (from planner, user-approved)
├── state.json                              ← runtime: per-node status, attempts, started_at
├── nodes/{node-id}/
│   ├── meta.json                           ← worktree path, branch ref, deps, validator spec
│   ├── output.md                           ← worker's human-readable summary
│   ├── validator.json                      ← validator exec result
│   └── attempts.jsonl                      ← append-only retry log
├── merges/{merge-id}/
│   ├── meta.json                           ← integration branch, parent branches
│   ├── validators.json                     ← cascade results (step1, step2, step4) with pass/fail/skipped
│   └── status
└── worktrees/{node-id}/                    ← git worktree checkout (ephemeral, removed on cleanup)

/tmp/flow/{CLAUDE_SESSION_ID}/              ← ephemeral session files (agent inputs)
└── {interviewer|planner|worker|merger|reviewer}-session-{node-id}.md
```

### DAG schema (`dag.json`)

```json
{
  "task_id": "<uuid>",
  "created_at": "<iso8601>",
  "spec_ref": "spec.md",
  "project_test_cmd": "<shell command, optional — feeds merger step 2>",
  "nodes": [
    {
      "id": "<kebab>",
      "type": "work | merge",
      "title": "<human>",
      "deps": ["<node-id>", ...],
      "agent": "flow-worker | flow-merger",
      "spec": "<inline markdown describing the task>",
      "validator": {
        "kind": "command | schema | none",
        "command": "<shell, for kind=command>",
        "expected_exit": 0
      },
      "produces": {
        "kind": "branch",
        "ref": "flow/{task_id}/{id}"
      }
    }
  ]
}
```

### Execution loop (logical)

```
SKILL(flow):
  load state.json (or init from dag.json)
  while ∃ node.status = pending:
    ready = { n | n.status = pending ∧ ∀d∈n.deps: d.status = complete }
    if ready = ∅ ∧ running = ∅: HALT (deadlock — should be impossible if DAG is valid)
    for each n in ready:
      persist state.json[n].status = running (INV-F1)
    dispatch all of `ready` in a single parallel Task() batch
    await results
    for each result:
      persist nodes/{n.id}/{output.md, validator.json}
      evaluate validator (deterministic first, then agent-reported)
      on FAIL and attempts[n] < MAX_RETRIES: requeue with attempts[n]++
      on FAIL and exhausted: state.json[n].status = failed → task.status = halted → surface
      on PASS: state.json[n].status = complete
  emit final report
```

### Agent composition

| Agent | Superpowers composition | Purpose |
|-------|-------------------------|---------|
| `flow-interviewer` | `superpowers:brainstorming` | Requirement clarification → `spec.md` |
| `flow-planner` | `superpowers:writing-plans` | `spec.md` → `dag.json` (atomic + merge nodes) |
| `flow-worker` | `superpowers:using-git-worktrees`, optionally `superpowers:test-driven-development` | Execute one node in a worktree, commit to branch |
| `flow-merger` | `superpowers:verification-before-completion` | Run cascade validator, return structured result |
| `flow-reviewer` | `superpowers:verification-before-completion` | LLM semantic review as cascade step 4 |

## Commands

| Command | Purpose |
|---------|---------|
| `/flow "<request>"` | Full pipeline: intake → interview → plan → approve → execute → report. Accepts `--no-ask` to suppress interactive confirms (DAG is auto-approved by planner's own self-review). |
| `/flow-status [task-id]` | Human-readable state summary of a task (or all tasks if omitted). |
| `/flow-resume <task-id>` | Resume a `halted` or session-interrupted task. Reuses `dag.json`, retries `failed` or `running` nodes. |
| `/flow-graph <task-id>` | Render the DAG as a mermaid diagram from `dag.json`, annotated with current status. |

## Skills

| Skill | Role |
|-------|------|
| `/flow` | Orchestrator (entry point). Holds execution loop, state persistence, parallel dispatch, retry bounds. |

Sub-workflows (resume/status/graph) are implemented as lightweight commands that delegate to state.json readers; they do not re-run the execution loop.

## Hooks

| Hook | Purpose |
|------|---------|
| `SessionStart` | Scans `.claude/workflows/flow/*/state.json` for tasks with `status ∈ {running, halted}` and emits a one-line "in-progress flows" notice so the user knows they can resume. Non-blocking, <1s. |

## Development Principles

1. **Merge is a first-class node, not a byproduct.** Every fan-in has an explicit merge node. Hiding merge inside a worker is an invariant violation.
2. **Persist state before action.** A node transition to `running` writes to state.json before the Task() call. Session interruptions leave a consistent resume point.
3. **Validators are cheap-first.** The cascade runs git-conflict (~ms) → tests (~sec/min) → LLM (~sec, but expensive tokens). First pass wins; remaining steps are recorded as `skipped-due-to-earlier-pass`, not re-run.
4. **Retry is a bug-guard.** `MAX_RETRIES=3` catches transient failures; persistent failure halts and surfaces — the user decides retry/abort/edit-spec.
5. **LLM generates the DAG; user approves it.** Planner outputs a mermaid preview before committing `dag.json`. With `--no-ask`, the planner's own self-review substitutes for user approval but MUST record rationale in `spec.md`.
6. **Version management.** Bump `.claude-plugin/plugin.json` version + `.claude-plugin/marketplace.json` entry on every release (SemVer).

## Instructions

- Document language: English.
- All per-task persistent state lives under `.claude/workflows/flow/{task-id}/`. Session-ephemeral inputs live under `/tmp/flow/{CLAUDE_SESSION_ID}/`. Never conflate the two.
- Worker agents MUST produce commits on their branch and MUST NOT attempt to merge. Merge is exclusively the merger agent's responsibility.
- The SKILL (not agents) owns transitions to `state.json`. Agents return structured results; the SKILL persists them.
- When invoking `/flow-resume`, trust `dag.json` as immutable (INV-F5). If the spec has changed, start a new task, do not edit the old DAG.
