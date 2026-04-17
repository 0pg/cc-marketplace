# Agent-Tree Project — claude-md-plugin v19 reminder

This project uses the **node-agent tree** architecture. Internalize this
model before reading or modifying any code — it changes how you reason
about scope, delegation, and the meaning of `CLAUDE.md`.

## Core Model

- **Node = Agent.** Every directory (or package) that owns a `CLAUDE.md`
  is an autonomous agent, scoped to its node boundary.
- **`CLAUDE.md` = Agent Prompt.** The `CLAUDE.md` at a node is that
  agent's system prompt — role, responsibilities, domain context,
  interaction contracts. It is **instruction**, not an SSOT document and
  not a spec that code is derived from.
- **Source Code = Tools.** Files inside a node are the agent's tools,
  analogous in role to Skills or MCP tools. You invoke them (Bash),
  inspect them (Read/Grep), modify them (Edit), and verify via tests.
  Code is the capability; do not reconstruct behavior from prose.
- **Agent Tree = Domain Decomposition.** A god-agent cannot fit the
  whole project in context. Cohesive sub-domains become child nodes,
  each with its own agent. Out-of-scope work is **delegated** to the
  child whose domain contains it.

## Operating Rules While Inside a Node

1. Read the current node's `CLAUDE.md` and its ancestors' `CLAUDE.md`
   files first — they define your role, boundary, and contract.
2. Do not read or modify files outside the current node's boundary.
   The boundary is the node's own files plus whatever you reach through
   delegation.
3. Cross-boundary work is always delegation:
   - Task sits in a child → spawn a subagent for that child, pointing
     it at the child's `CLAUDE.md`.
   - Task sits in a sibling or ancestor → return to the parent; let the
     parent re-delegate. Do not reach sideways or upward directly.
4. Treat `CLAUDE.md` as the prompt, code as the capability. Never manage
   "spec vs. code" divergence — update the tool (code) when behavior
   changes; update `CLAUDE.md` only when the agent's role, scope, or
   contract changes.
5. If the current node lacks `CLAUDE.md`, either you are operating above
   the agent tree (root-level coordination) or the node hasn't been
   declared yet. Clarify scope with the user or the parent before
   proceeding — do not silently assume a boundary.

## Reference

Full reference design lives in the plugin at
`references/agent-tree/` (README → node-prompt-template →
source-as-tool → delegation → decomposition). Consult it when you need
the precise template for a node's `CLAUDE.md`, the delegation message
shape, or guidance on when to split/merge nodes.
