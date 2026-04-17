# Main-Ctx Orchestration (v19 draft)

How the top-level conversation (**main ctx**) drives work through the
agent tree. Companion to `delegation.md` (which defines the contract)
and to the `node-agent` subagent (which embodies one node's role).

## Core Pattern: Plan-First, Execute-Second

Node agents are **planners** during discovery: when main ctx invokes a
node-agent, the agent loads its `CLAUDE.md`, classifies the requested
work, and returns a structured plan rather than executing changes.
Main ctx collects plans recursively across the tree, then drives
execution with full visibility.

```
1. User → main ctx: instruction
2. main ctx: clarify requirements (AskUserQuestion if ambiguous)
3. main ctx → node-agent(node=root, instructions=refined task)
4. node-agent: load <node>/CLAUDE.md → return work plan
   (sections: Identity, In-Scope, Delegated, Escalated, Open Questions)
5. main ctx: record plan
   for each Delegated item:
     dispatch node-agent(node=child, instructions=forwarded task)
     ↪ recurse from step 4
   for each Open Question:
     resolve with the user (AskUserQuestion) before that branch closes
6. main ctx: when no Delegated items remain unresolved across the tree,
   execute the assembled plan
```

## Why Plan-First

- Main ctx holds the **global view** — it can detect conflicts between
  sibling plans, redundant work, and mis-ordered dependencies before
  anything is touched.
- Planning is **read-only**, execution is **side-effectful**. Separating
  them lets the model survey the whole task tree at low cost, then
  commit to changes with full information.
- If planning surfaces infeasibility or scope drift, main ctx can
  return to the user without having modified the working tree.
- Open Questions surface in planning, not mid-execution — the user
  answers before any file changes start.

## Recursion Discipline

- **One level per Task call.** A node-agent never transitively delegates
  on behalf of a child. It names the child + the forwarded instructions;
  main ctx makes the next Task call.
- **Termination.** A branch closes when its node-agent returns no
  Delegated items (only In-Scope, Escalated, or Open Questions). The
  whole tree closes when every dispatched branch has closed.
- **Open Questions block their branch.** Main ctx resolves them with
  the user (or the parent node-agent, if it's a routing question)
  before re-dispatching the affected node.
- **Escalated items return to the caller.** The caller decides whether
  to re-route to a different node, change scope, or surface to the user.

## Recording the Plan

Main ctx maintains an in-memory tree of plans during orchestration:

```
root
├── Identity: <line>
├── In-Scope: [...]
├── Delegated:
│   ├── api/
│   │   ├── Identity: <line>
│   │   ├── In-Scope: [...]
│   │   └── Delegated: ... (recurse)
│   └── billing/
│       └── ...
├── Escalated: [...]
└── Open Questions: [...]
```

Persistence beyond the session is not required by this protocol — main
ctx may keep the tree purely in conversation context. Persistent
storage (state file) is a future option if multi-session orchestration
becomes needed.

## Execution Phase (step 6)

Design currently underspecified. Reasonable directions, to be settled
when first executed:

- Main ctx walks the plan tree depth-first or in topological order of
  dependencies surfaced in the plan, dispatching node-agents in an
  **execution mode** with the specific approved plan items as
  instructions.
- The node-agent in execution mode performs Edit/Write/Bash within its
  boundary using its in-scope plan items, returns a result, and main
  ctx moves on.
- Items that span multiple nodes (e.g., a contract change that touches
  api/ and billing/ in concert) are coordinated by main ctx, which may
  serialize, parallelize, or interleave node-agent dispatches per the
  plan's stated dependencies.

The current `node-agent` definition is **planning-only**. Adding an
execution mode is a follow-up; the planning contract above does not
depend on it.

## Anti-Patterns

- **Skipping the plan.** Calling node-agent with "just do X and report
  back" defeats the global-visibility benefit. If the work is genuinely
  trivial and main-ctx-internal, do it directly without going through
  a node-agent.
- **Deep recursion in one node-agent.** A node-agent that classifies
  a task as "delegate to grandchild via child" is overstepping. It
  delegates to the child only; the child plans for itself.
- **Plan/execute interleaving.** Resist executing while planning is
  still in progress for sibling branches — execution may invalidate
  another branch's plan. Wait for the tree to close.
- **Discarding Escalated items.** Escalated work is information main
  ctx must act on (re-route, ask user, expand scope). Silently dropping
  it loses tasks the user expected.

## Relationship to delegation.md

`delegation.md` defines the general parent↔child contract: how a parent
discovers, invokes, and consumes a child. This document specifies the
**main-ctx-as-orchestrator** workflow, where main ctx (not a parent
node) holds the recursion. Both are consistent: main ctx acts as the
caller in the delegation contract; the node-agent acts as the callee.
