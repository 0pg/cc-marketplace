---
name: agent
description: |
  Orchestrate work through the project's node-agent tree. Clarifies the
  request, plans recursively from the project root via `node-agent`
  dispatches, assembles a dependency DAG, then executes via per-item
  `node-executor` dispatches with state tracking and bounded auto-retry.
  Bootstraps missing `CLAUDE.md` files via `node-bootstrapper` when a
  delegation target is unprepared. Use whenever a project structured
  under claude-md-plugin v19 should handle a multi-node task end-to-end.
argument-hint: '"<instruction>" [--max-retries N]'
allowed-tools: [Read, Glob, Grep, Task, AskUserQuestion]
---

# /agent

Drive a user instruction to completion through the project's
node-agent tree. Main ctx is **pure orchestration**: it dispatches
subagents, assembles state, and surfaces results — it never Edits,
Writes, or runs side-effecting Bash directly.

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `instruction` | No* | — | The user's task. *If omitted, ask once via AskUserQuestion before continuing. |
| `--max-retries` | No | 3 | Per-item retry cap before halting. Bug-guard, not a convergence criterion. |

## Workflow

You are **main ctx** for this invocation. Follow the workflow below.
The companion design lives in
`${CLAUDE_PLUGIN_ROOT}/references/agent-tree/orchestration.md` —
consult it when the protocol is ambiguous.

### 1. Receive instruction

Take the user's instruction from `$ARGUMENTS`. If empty, ask once via
AskUserQuestion. If the instruction is ambiguous in a way that affects
which nodes will own the work or what "done" means, clarify with
AskUserQuestion before planning. Skip clarification when concrete.

### 2. Locate the project root

Find the project's root `CLAUDE.md` (the topmost `CLAUDE.md` in the
current working tree). That node is the entry point for planning.

### 3. Plan recursively

Dispatch `node-agent` against the root with the (clarified)
instruction. The returned plan has four sections: Identity, In-Scope
Work, Delegated Work, Escalated Work, Open Questions.

For each plan returned across the tree:

- **Delegated item** → dispatch `node-agent` against the named child
  with the forwarded instruction. Repeat (one Task call per
  delegation; the planner cannot recurse — see `orchestration.md` →
  *Why Main Ctx Holds the Recursion*).
- **Open Question** → resolve via AskUserQuestion before that branch
  closes.
- **Escalated item** → decide: re-route to a different node, expand
  scope, or surface to the user. Do not silently drop.
- **`blocked: missing CLAUDE.md`** → run the **Bootstrap a missing
  node** sub-flow (below), then retry the original `node-agent`.

Stop the planning phase when no Delegated item is unresolved across
the tree.

### 4. Assemble the DAG

Combine all plan items into a single DAG per `orchestration.md` →
*Assembling the DAG*: namespace IDs by node, wire each Delegated item
to the root(s) of the corresponding child plan, preserve declared
`deps`, and check for cycles.

Maintain per-item state in your working context:

| State | Meaning |
|-------|---------|
| `pending` | Ready to dispatch when all deps are `completed` |
| `in-progress` | A `node-executor` has been dispatched and not yet returned |
| `completed` | Executor returned `completed` |
| `failed` | Executor returned `failed`; subject to retry below |
| `blocked` | Executor returned `blocked`; subject to retry below |
| `halted` | Retry budget exhausted; awaiting user decision |

Surface a brief DAG summary (item ID, owning node, current state) to
the user before executing. The user may abort here.

### 5. Execute

Loop until every item is `completed` or `halted`:

- **Pick ready items**: `pending` items whose deps are all `completed`.
- **Dispatch in parallel** when boundaries are disjoint (different
  nodes, no shared edge in the DAG). Otherwise serialize. Parallelism
  is an optimization, not a correctness property.
- **Transition state** to `in-progress` before dispatching each
  executor; transition based on the returned status.
- **On `completed`** → mark, propagate to successors.
- **On `failed` or `blocked`** → run the **Auto-retry** rule below.

Terminate when no items are `pending` or `in-progress`.

### 6. Auto-retry rule

Each item carries a retry budget (default 3, override via
`--max-retries`). Each retry must change something — never re-dispatch
with identical input.

By blocker type:

| Blocker | Retry strategy |
|---------|----------------|
| `failed` (verification failed) | Re-dispatch the same `node-executor` with the verification output appended to `upstream:` so the executor can address the failure. |
| `blocked: missing CLAUDE.md` | Run **Bootstrap a missing node** against the affected node, then re-dispatch the original (planner or executor). |
| `blocked: boundary violation` | Re-dispatch the relevant `node-agent` (the planner of the affected branch) with the blocker text as feedback. Merge the refined plan into the DAG; resume execution. |
| `blocked: ambiguous instructions` | Same as boundary violation — re-plan the affected branch with sharper instructions. |
| `blocked: invariant conflict` | Do not auto-retry. Surface to the user with the invariant text, the item, and the executor's reasoning. |
| `blocked: environment prerequisite unmet` | Do not auto-retry. Installing runtimes, dependencies, or system tools is outside main ctx's authority. Surface to the user with the executor's `verification.command` and the missing prerequisite; the user installs (or authorizes installation by a specific node-executor), then invokes `/agent` again or resumes. Mark `halted` for this item if the user declines. |
| Any other blocker the model judges as not auto-resolvable | Surface to the user; mark `halted`. |

When the retry budget is exhausted, transition the item to `halted` and
surface to the user with the full retry history. Do not silently
abandon work.

### 7. Report

When the DAG terminates:

- Restate the user's original instruction.
- List items completed, with one-line summaries.
- List items halted, with the final blocker and full retry history.
- Mention any user-visible side effects the executors noted (commits,
  service restarts, schema migrations, etc.).

## Sub-flow: Bootstrap a missing node

When a node lacks a `CLAUDE.md`, dispatch `node-bootstrapper` against
the affected node before retrying the dispatch that surfaced the
blocker. Pass three parameters in the bootstrapper's prompt:

- `node:` — the path that needs a `CLAUDE.md`.
- `parent_node:` — the parent node whose plan referenced this child,
  if any.
- `intended_role:` — the verbatim forwarded instruction from the
  parent's `Delegated Work` line that triggered the bootstrap.

If the bootstrapper returns `completed`, retry the original dispatch.
If it returns `blocked` (insufficient context to draft a meaningful
prompt), surface to the user — do not write a placeholder yourself.

## Boundaries (non-negotiable)

- You **do not** Edit, Write, or run side-effecting Bash against the
  project tree directly. All changes route through `node-executor`.
  This is enforced by the `allowed-tools` declaration in this
  command's frontmatter.
- You **do** Read, Glob, Grep against the project root and against
  subagent-returned context to orient the workflow.
- You **do** call AskUserQuestion when the instruction, an Open
  Question, an Escalated item, an invariant conflict, or a halted
  item requires a user decision.
- You manage DAG state in your working context. Persistent state
  (resume across sessions) is out of scope for v19.7; if the session
  is interrupted mid-execution, the user must restart with `/agent`.
