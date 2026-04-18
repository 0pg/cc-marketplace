# Main-Ctx Orchestration (v19 draft)

How the top-level conversation (**main ctx**) drives work through the
agent tree. Companion to `delegation.md` (which defines the contract)
and to the `node-agent` subagent (which embodies one node's role).

## Core Pattern: Plan-First, Execute-Second

Node agents are **planners** during discovery: when main ctx invokes a
node-agent, the agent loads its `CLAUDE.md`, classifies the requested
work, and returns a structured plan rather than executing changes.
Main ctx drives recursion across the tree, assembling the returned
plans into a **dependency DAG**, then executes with full visibility.

```
1. User → main ctx: instruction
2. main ctx: clarify requirements (AskUserQuestion if ambiguous)
3. main ctx → node-agent(node=root, instructions=refined task)
4. node-agent: load <node>/CLAUDE.md → return work plan
   (sections: Identity, In-Scope, Delegated, Escalated, Open Questions;
    optional [id] + deps: [...] per item)
5. main ctx: record plan
   for each Delegated item:
     dispatch node-agent(node=child, instructions=forwarded task)
     ↪ recurse from step 4
   for each Open Question:
     resolve with the user (AskUserQuestion) before that branch closes
6. main ctx: once every Delegated item across the tree has either been
   dispatched and returned, or been resolved via Open Questions /
   Escalations, assemble all returned plans into a single DAG and
   execute it.
```

## Why Main Ctx Holds the Recursion

This is **not** a philosophical preference — it is a Claude Code
constraint. Subagents cannot recursively spawn other subagents via the
Task tool. If a node-agent needed to delegate to a grandchild, it
would have no way to do so from inside a subagent invocation.

The protocol works around this by inverting the recursion: each
node-agent **names** the direct child it would delegate to (and the
instructions it would forward), and main ctx — which does have Task
access — makes the subsequent Task call. Main ctx is effectively the
recursion driver that node-agents cannot be.

Consequences:

- Every node-agent invocation is one level deep.
- The returned plans are intentionally partial: they surface delegated
  work as declarations, not as nested results.
- Main ctx's role during discovery is mechanical: dispatch node-agents
  for each Delegated item and collect their plans. Judgment lives inside
  each node-agent.

## Why Plan-First

Beyond the recursion constraint, separating plan from execute pays off:

- **Global visibility.** Once the DAG is assembled, main ctx can detect
  conflicts between sibling plans, redundant work, mis-ordered
  dependencies, and scope drift — all before a single file changes.
- **Read-only vs. side-effectful separation.** Planning is cheap and
  safe; execution has lasting effects. Running planning to completion
  first means execution starts with full information.
- **Early failure surface.** If planning reveals infeasibility or an
  unresolvable Open Question, main ctx returns to the user without
  touching the working tree.
- **Deterministic execution ordering.** The DAG encodes dependencies
  explicitly, so execution order is derived from the graph (not from
  chat-time heuristics).

## Assembling the DAG

Each node-agent returns a list of plan items (In-Scope, Delegated,
Escalated, Open Questions). Items may carry an optional kebab-case
`[id]` and a `deps: [...]` reference list. Main ctx combines every
returned plan into one DAG by:

1. **Namespacing IDs.** A raw `[root-1]` from the root node becomes
   e.g. `root/in/root-1` in the global DAG. A `[deleg-1]` from an
   `api/` node becomes `api/in/deleg-1`. Collisions are impossible by
   construction.
2. **Wiring delegated items to their child subplans.** A `Delegated`
   item in node N referencing child C is replaced in the DAG by the
   **root(s) of C's returned plan**, with the Delegated item's own
   dependents re-pointed to C's terminal items (or to the Delegated
   item's resolved outcome, whichever the parent plan intended).
   In practice: a Delegated item acts as a placeholder whose
   concrete dependencies are materialized when C's plan arrives.
3. **Preserving declared deps.** Any explicit `deps: [x, y]` on a plan
   item becomes an edge in the DAG.
4. **Cycle check.** If DAG construction produces a cycle, main ctx
   halts and surfaces the cycle path to the user — almost always a
   misdeclared dependency or a scope overlap between two nodes.

Items without IDs or deps are treated as leaf nodes with no
predecessors (they still may be successors via a Delegated wiring).

The DAG is the execution artifact — linear transcripts are not.

## Recursion Discipline

- **One level per Task call.** A node-agent never transitively delegates
  on behalf of a child. It names the child + the forwarded instructions;
  main ctx makes the next Task call. (See "Why Main Ctx Holds the
  Recursion" above.)
- **Termination.** A branch closes when its node-agent returns no
  Delegated items (only In-Scope, Escalated, or Open Questions). The
  whole tree closes when every dispatched branch has closed.
- **Open Questions block their branch.** Main ctx resolves them with
  the user (or the parent node-agent, if it's a routing question)
  before re-dispatching the affected node.
- **Escalated items return to the caller.** The caller decides whether
  to re-route to a different node, change scope, or surface to the user.

## Execution Phase (step 6)

Design currently underspecified; the DAG framing above constrains the
design space. Likely shape:

- Main ctx performs topological execution of the DAG. Independent
  nodes (no edge between them) **may** be executed in parallel; nodes
  on a dependency edge must be serialized.
- Per-item execution still goes through a node-agent call (because
  Edit/Write must happen inside the owning node's boundary). A
  future `node-agent` execution mode, or a dedicated executor
  subagent, would receive the specific plan item(s) to execute and
  return a result.
- Failure handling, retries, and plan invalidation on failure are
  open design questions. The `flow` plugin's DAG execution loop is a
  useful precedent; whether to integrate or re-implement remains open.

The current `node-agent` definition is **planning-only**. Adding an
execution mode is a follow-up; the planning contract does not depend
on it.

## Anti-Patterns

- **Skipping the plan.** Calling node-agent with "just do X and report
  back" defeats the global-visibility benefit. If the work is genuinely
  trivial and main-ctx-internal, do it directly without going through
  a node-agent.
- **Deep recursion in one node-agent.** A node-agent that classifies a
  task as "delegate to grandchild via child" is overstepping. It names
  the child only; the child plans for itself.
- **Plan/execute interleaving.** Resist executing while planning is
  still in progress for sibling branches — execution may invalidate
  another branch's plan. Wait for the DAG to close.
- **Discarding Escalated items.** Escalated work is information main
  ctx must act on (re-route, ask user, expand scope). Silently dropping
  it loses tasks the user expected.
- **Treating the DAG as a tree.** Dependencies can converge (two
  siblings finishing unlocks a merge item) — that's a DAG, not a tree.
  Code that assumes tree shape will lose convergence information.

## Relationship to delegation.md

`delegation.md` defines the general parent↔child contract: how a parent
discovers, invokes, and consumes a child. This document specifies the
**main-ctx-as-orchestrator** workflow, where main ctx (not a parent
node) holds the recursion because Claude Code subagents cannot
transitively spawn subagents. Both are consistent: main ctx acts as
the caller in the delegation contract; the node-agent acts as the
callee; the parent→child language in delegation.md describes the
logical relationship, while the Task-call mechanics route through
main ctx.
