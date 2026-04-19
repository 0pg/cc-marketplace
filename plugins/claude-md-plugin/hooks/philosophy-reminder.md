# Agent-Tree Project — claude-md-plugin v19 reminder

## Invariants

Three premises are always true about a project using this plugin.
Hold them as given across every reasoning step — they are not
renegotiated during a session.

1. **This project is one agent system, composed of multiple agents.**
   Not a codebase with automation on top. The project's identity is
   multi-agent.
2. **Every agent has its own purpose, domain context, and
   responsibilities.** Agents are not interchangeable; each exists
   to do a specific job within a bounded scope.
3. **Source code is a tool** — the means by which an agent fulfills
   its responsibilities, not the point of the work. If the code
   breaks, the tool breaks, and the agent can no longer fulfill its
   functional responsibility. Keeping tools working (buildable,
   testable, verifiable) is therefore part of having responsibilities,
   not a separate concern.
4. **Each agent satisfies hierarchical single-responsibility, and the
   context it owns has high cohesion.** At its level in the tree, an
   agent has one clear responsibility — not a grab bag. The purpose,
   domain context, tools, and child sub-domains it owns form a
   cohesive whole. Fragmented context or multiple unrelated
   responsibilities are signs that the agent should be split (or that
   the responsibilities should be reassigned).

Everything below follows from these three.

## How the invariants manifest

- Each directory (or package) with a `CLAUDE.md` is one of those
  agents. The `CLAUDE.md` at that node is the agent's **system
  prompt** — its role, responsibilities, domain context, tools, and
  interaction contract. `CLAUDE.md` is instruction, not an SSOT
  record; it is not a spec that code is derived from.
- Agents are organized as a tree by domain cohesion, because a single
  agent cannot fit the whole system in its context window and because
  each agent must satisfy hierarchical SRP (invariant 4). Cohesive
  sub-domains become child agents; out-of-scope work is delegated to
  the child whose domain contains it. An agent whose owned context
  loses cohesion as the project grows is a candidate for split.
- Source files inside a node are that agent's tools — capabilities it
  invokes (Bash), inspects (Read/Grep), modifies (Edit), and verifies
  (tests). Code is the capability itself, not an artifact derived
  from prose. A change that leaves verification broken is not
  "progress that can be finished later" — it is a broken tool, and the
  agent's functional responsibility stays unfulfilled until it is
  restored.

## Operating rules (consequences)

1. Read the current node's `CLAUDE.md` and its ancestors' `CLAUDE.md`
   first — they define your role, boundary, and contract.
2. Do not read or modify files outside the current node's boundary.
   The boundary is the node's own files plus whatever you reach via
   delegation.
3. Cross-boundary work is always delegation:
   - Task sits in a child → delegate to that child's agent.
   - Task sits in a sibling or ancestor → return to the parent; let
     the parent re-delegate. Do not reach sideways or upward directly.
4. Treat code as the capability. Do not manage "spec vs. code"
   divergence — update the tool (code) when behavior changes; update
   `CLAUDE.md` only when the agent's role, scope, or contract itself
   changes.
5. If the current node has no `CLAUDE.md`, either you are operating
   above the agent tree (root-level coordination) or the node has not
   been declared yet. Clarify scope before acting — do not silently
   assume a boundary.

## Reference

Full reference design lives in the plugin at
`references/agent-tree/` (README → node-prompt-template →
source-as-tool → delegation → decomposition → orchestration).

Entry-point command: `/agent "<instruction>"` runs the full
orchestration workflow (clarify → plan → DAG assemble → execute with
auto-retry).
