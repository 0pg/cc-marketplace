# claude-md-plugin

## Purpose

**A plugin for composing a project as a tree of node-scoped agents.**

Each directory or package in a project is treated as an autonomous **agent**. The
node's `CLAUDE.md` is that agent's system prompt; the node's source code is the
agent's tool set. Because a single god-agent cannot hold a whole project in its
context window, the plugin decomposes responsibility into child agents by domain
cohesion, and parent agents delegate to them.

## Core Philosophy (v19)

> Prior to v19, this plugin treated `CLAUDE.md` as the Primary SSOT and source
> code as a derived artifact. **That premise is retired.** Documents-as-SSOT did
> not materially improve what Claude can actually accomplish on a task. Source
> code is the substance; `CLAUDE.md` is prompt engineering for the agent that
> owns the node.

### 1. Node = Agent

Every node (directory or package) in the project corresponds to exactly one
agent. The agent has authority and responsibility scoped to its node boundary,
and nothing beyond.

### 2. CLAUDE.md = Agent Prompt

A node's `CLAUDE.md` is the **system prompt** for that node's agent. It carries
the agent's role, responsibilities, domain context, interaction contracts with
parents and children, and any invariants the agent must uphold — every piece of
context the agent needs to make judgments inside its boundary. `CLAUDE.md` is
**instruction**, not **truth of record**.

### 3. Source Code = Tools

Files inside a node are the agent's tools, analogous in role to Claude Code
Skills or MCP tools: capabilities the agent invokes, inspects, modifies, and
creates. Code is not "derived from spec" — code **is** the agent's capability.

### 4. Agent Tree = Domain Decomposition

A single agent cannot hold the whole project; context windows forbid it.
Cohesive sub-domains become child nodes, each with its own agent. A parent
agent knows its children's roles (by reading each child's `CLAUDE.md`
summary) and **delegates** out-of-scope work to the child whose domain
contains it. Trees are dependency-shaped: parents may depend on children;
children do not reach up to parents; siblings do not cross-reference directly.

## Node Layout

```
<node>/
├── CLAUDE.md         # agent prompt — role, responsibilities, domain context
├── <source files>    # the agent's tool set
└── <child nodes>/    # subordinate agents (by domain cohesion)
```

## Status — v19 Transition

This plugin is mid-rewrite. The v18 execution model (doc-as-SSOT, `/spec`,
`/dev`, `/validate`, `/decompile`, `/bugfix`, `/impact`, `/inspect`,
`/autodev`, `/project-setup`, `/migrate`, paired `DEVELOPERS.md`, INV-1 ~
INV-15, session-file pattern, `po-consultant` verdict protocol, Agent
Observations, spec-quality reviewer) is **retired**. As of Roadmap step 3,
all v18 agents, skills, commands, hooks, the `core/` Rust CLI, and v18
reference docs have been removed. `agents/`, `skills/`, `commands/`,
`hooks/`, `scripts/`, and `core/` are now empty placeholders awaiting the
v19 rebuild (steps 4–5).

### Rebuild Roadmap

| Step | Scope | Status |
|------|-------|--------|
| 1 | **Philosophy (this document)** | done (v19.0.0) |
| 2 | Agent-tree reference design — root-agent template, delegation contract, child-discovery convention | draft (v19.1.0) — see `references/agent-tree/` |
| 3 | Teardown — remove v18 `agents/`, `skills/`, `commands/`, `hooks/`, `scripts/`, `core/`, and legacy references | done (v19.2.0) |
| 4 | Rebuild: new skills, commands, and reference agent files under the v19 model | in progress (v19.4.0 — `node-agent` subagent + orchestration doc) |
| 5 | Re-scope `core/` Rust CLI — keep only subcommands the agent tree actually uses (rebuild from scratch if warranted) | pending |
| 6 | New invariant set — boundary, delegation, tool access (derived from v19 model, not ported from v18) | pending |

### SessionStart Philosophy Reminder (v19.3.0)

A plugin-owned `SessionStart` hook injects the v19 node-agent-tree
philosophy into every session on `startup`, `resume`, `clear`, and
`compact`. The purpose is to keep the model grounded in the agent-tree
model across context resets — projects that install this plugin are
opting into that architecture.

- `hooks/hooks.json` — registers the hook with matcher `"*"`
- `hooks/session-start.sh` — emits the reminder on stdout
- `hooks/philosophy-reminder.md` — the reminder content

### Reference Design

- `references/agent-tree/README.md` — index and reading order
- `references/agent-tree/node-prompt-template.md` — required shape of a node's CLAUDE.md
- `references/agent-tree/source-as-tool.md` — tool invocation conventions
- `references/agent-tree/delegation.md` — parent↔child contract
- `references/agent-tree/decomposition.md` — when/where to split a node
- `references/agent-tree/orchestration.md` — main-ctx plan-first/execute-second workflow with `node-agent`

### Subagents

- `agents/node-agent.md` — node-scoped planning agent. Loads a target
  node's `CLAUDE.md`, adopts that node's identity, and returns a
  structured work plan (Identity / In-Scope / Delegated / Escalated /
  Open Questions). Planning-only; execution mode is a follow-up.

## Instructions

- Document language: English.
- Treat **Core Philosophy** above as the authority during the v19 rewrite.
  Anything in this repo that contradicts it (v18 agent/skill/command files,
  v18 CLI subcommands, stale README content) is legacy pending removal.
- Do not author new workflows, agents, or CLI subcommands against the v18
  model.
- Version bumps remain mandatory per the marketplace rule: update
  `.claude-plugin/plugin.json` and the matching entry in the repo-root
  `.claude-plugin/marketplace.json` on every change to this plugin.

## Conventions

### Project Structure

```
claude-md-plugin/
├── .claude-plugin/    — plugin manifest (plugin.json)
├── CLAUDE.md          — this file (plugin agent prompt)
├── README.md          — (v18 legacy; rewrite deferred to step 4+)
├── DEVELOPERS.md      — (v18 legacy; rewrite deferred to step 4+)
├── agents/            — `node-agent` subagent (v19.4.0); more pending
├── skills/            — (empty — awaiting v19 rebuild, Roadmap step 4)
├── commands/          — (empty — awaiting v19 rebuild, Roadmap step 4)
├── hooks/             — SessionStart philosophy-reminder hook (v19.3.0)
├── scripts/           — (empty — utility scripts re-added as agent tools need them)
├── core/              — (empty — awaiting Roadmap step 5 re-scope / rebuild)
└── references/        — reference materials
    └── agent-tree/    — v19 reference design (step 2 draft)
```

### Naming Conventions (plugin authoring)

- Skill, agent, command files: `kebab-case.md`
- Shell scripts: `kebab-case.sh`

Language-specific conventions (Rust, etc.) will return in the relevant node's
`CLAUDE.md` when that node is rebuilt in step 5.

## Domain Context

None.

## Requirements

None.
