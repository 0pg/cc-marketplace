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
AskUserQuestion.

Before dispatching the root `node-agent`, walk this **clarification
checklist**. For each axis, decide whether the instruction itself
(or the root `CLAUDE.md`'s Conventions / Invariants) already answers
it. If yes, proceed. If no and the axis materially affects planning
or execution, resolve with one AskUserQuestion call covering only
the unresolved axes. If an axis is genuinely irrelevant to this
request, skip it — the checklist is a **gap detector**, not a
question gatekeeper.

| Axis | What to pin down |
|------|------------------|
| **Actors** | Who can trigger this work end-to-end? (end user, admin, internal system, scheduled job) Affects auth/authorization planning at the api/ or equivalent boundary. |
| **Failure policy** | When an external dependency (email, webhook, payment, queue) fails partway, should the overall operation roll back, compensate, or commit-then-log-and-move-on? |
| **Side effects** | Does the work produce externally visible effects (emails, webhooks, third-party API calls, payments)? Enumerating them up front prevents sibling plans from each surfacing the same question as an Open Question. |
| **Done criteria** | What observable state or output counts as "complete"? (Already in the v18 baseline — keep.) |
| **Node ownership** | Which node(s) own the primary responsibility? (Already in the v18 baseline — keep.) |

Do not expand this checklist into a questionnaire. The goal is to
catch *missing* information that multiple planners would otherwise
rediscover independently as Open Questions. A concrete, well-scoped
instruction (e.g., "add a health-check endpoint at GET /health that
returns 200") needs zero checklist questions — proceed directly.

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
| `blocked: environment prerequisite unmet` | First, Read the root `CLAUDE.md`. If its `## Workspace Provisioning` section declares **Root-owned** (root holds the missing artifact as one of its tools), dispatch one `node-executor` against the root node with an item phrased as "scaffold `<missing artifact>` per root's declared provisioning." This consumes one retry on the *original* blocked item. If recovery succeeds, the original item becomes `pending` again and normal execution resumes. If the root is **Out-of-tree** (user/CI owns provisioning) or the section is absent, do not attempt auto-recovery: surface to the user with the executor's `verification.command` and the missing prerequisite; the user installs, then invokes `/agent` again or resumes. Mark `halted` for this item if the user declines. |
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

**After a successful bootstrap, inject a parent-update item into the
DAG automatically.** Do not leave the parent's `## Children` section
in a stale state — the bootstrapper cannot edit cross-boundary, and
recording the drift only as a follow-up means it never executes. The
injected item:

- **Owning node**: the parent node (`parent_node` from the bootstrap
  call).
- **Description**: "Add `<child>/` to the parent `CLAUDE.md`'s
  `## Children` section with a one-line role summary, derived from
  the newly written `<child>/CLAUDE.md`'s Identity."
- **Deps**: the bootstrapped child's first executed DAG item (i.e.,
  the parent-update runs after the child has begun producing real
  work, so the one-line role summary reflects settled reality, not
  the initial bootstrap guess).
- **Verification**: not applicable — documentation-only change. The
  executor will legitimately return `completed` with
  `verification.outcome: not applicable`.

This item enters the DAG as a normal `pending` state entry and
progresses through the standard execute loop. It does not consume
the original dispatch's retry budget.

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
