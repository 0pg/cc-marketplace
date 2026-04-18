# claude-md-plugin (v19)

> **A project is one agent system, composed of multiple agents.**
> Source code is a tool; `CLAUDE.md` is the agent's prompt.

## What this plugin does

`claude-md-plugin` lets you structure a codebase as a **tree of
node-scoped agents**. Every directory (or package) with a `CLAUDE.md`
becomes an autonomous agent:

- The `CLAUDE.md` **is that agent's system prompt** — role,
  responsibilities, domain context, interaction contract.
- The source files inside the node **are that agent's tools** — things
  it invokes, inspects, modifies, and verifies.
- Cohesive sub-domains become **child agents**; a parent delegates
  out-of-scope work to the child whose domain contains it.

Because a single agent can't hold a whole project in its context
window, work flows through a tree of agents. A main-ctx orchestrator
plans across the tree, assembles a **dependency DAG**, and executes
via per-item subagent dispatches — never touching files directly.

## The four invariants

1. **A project using this plugin is one agent system, composed of
   multiple agents.** Not a codebase with automation on top.
2. **Every agent has its own purpose, domain context, and
   responsibilities.** Agents are not interchangeable.
3. **Source code is a tool** — the means by which an agent fulfills
   its responsibilities. Broken code is a broken tool, and the agent's
   responsibility stays unfulfilled until it is restored.
4. **Each agent satisfies hierarchical single-responsibility, and the
   context it owns has high cohesion.** One clear job per agent; split
   when cohesion breaks.

These are injected into every session (startup / resume / clear /
compact) by the plugin's SessionStart hook.

## Install

Requires Claude Code (CLI, desktop, web, or IDE extension). Add this
plugin via the marketplace:

```
/plugin marketplace add 0pg/cc-marketplace
/plugin install claude-md-plugin@jhk-plugins
```

Once installed and enabled in your project, the SessionStart hook
starts injecting the four invariants into every session.

## Quick start

### 1. Declare the root agent

Create a `CLAUDE.md` at your project root describing the root agent's
role, responsibilities, tools, and children. Use the template at
`references/agent-tree/node-prompt-template.md` (inside this plugin).

### 2. Declare child agents as the domain grows

When a sub-domain starts competing for the root's prompt (invariant
4), give it its own directory and `CLAUDE.md`. Child CLAUDE.md files
inherit the parent's context via Claude Code's hierarchical auto-load;
write only what differs.

### 3. Run work through the agent tree

```
/agent "<your instruction>"
```

This runs the full orchestration:

1. **Clarify** the instruction if ambiguous.
2. **Plan recursively**: dispatch `node-agent` against the root,
   follow each delegated item down to the child, collect plans back.
3. **Assemble a DAG** from all returned plans (IDs namespaced per
   node; `deps:` edges preserved; cycle check).
4. **Execute** the DAG in topological order via per-item
   `node-executor` dispatches. Independent items may run in parallel.
5. **Auto-retry** on failure or blocker (bounded, default 3); on
   `blocked: missing CLAUDE.md`, dispatch `node-bootstrapper` and
   retry.
6. **Report** completed and halted items to the user.

Main ctx never edits files itself — the `/agent` command's
`allowed-tools` restricts it to `[Read, Glob, Grep, Task,
AskUserQuestion]` so Edit/Write/Bash must route through an executor.

## Components

| Component | Role |
|-----------|------|
| `commands/agent.md` | `/agent` — orchestration entry point |
| `agents/node-agent.md` | **Planner** — loads a node's CLAUDE.md, returns a structured plan (In-Scope / Delegated / Escalated / Open Questions). Read-only. |
| `agents/node-executor.md` | **Executor** — executes one DAG item inside a node's boundary. Edit/Write/Bash permitted only inside the node. |
| `agents/node-bootstrapper.md` | **Bootstrapper** — writes a `CLAUDE.md` for a node that lacks one, inheriting from the parent's intended role and the node's actual contents. |
| `hooks/` | SessionStart hook — injects the four invariants every session. |
| `references/agent-tree/` | Design documents (reading order in its README.md). |

## Reference

Start with `references/agent-tree/README.md`, then read in order:

1. `node-prompt-template.md` — required shape of a node's CLAUDE.md
2. `source-as-tool.md` — tool invocation conventions
3. `delegation.md` — parent↔child contract
4. `decomposition.md` — when to split a node (operationalizes invariant 4)
5. `orchestration.md` — main-ctx plan-first / execute-second workflow

## Version

v19 — a complete redesign from v18's "CLAUDE.md as Primary SSOT"
model. Everything about v18 (`/spec`, `/dev`, `/validate`,
`/decompile`, `/bugfix`, `/impact`, `/inspect`, `/autodev`, the
`DEVELOPERS.md` pairing, schema validation, INV-1~15, the Rust CLI
core) is retired. See the commit history on the
`claude/redesign-agent-architecture-*` branch for the transition.

## License

MIT — see `LICENSE`.
