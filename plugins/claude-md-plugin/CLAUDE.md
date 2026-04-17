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
Observations, spec-quality reviewer) is **retired in principle**. Concrete
files under `agents/`, `skills/`, `commands/`, and much of `core/` still
reflect v18 and are **pending teardown**. Do not extend them; do not treat
them as authoritative.

### Rebuild Roadmap

| Step | Scope | Status |
|------|-------|--------|
| 1 | **Philosophy (this document)** | done (v19.0.0) |
| 2 | Agent-tree reference design — root-agent template, delegation contract, child-discovery convention | draft (v19.1.0) — see `references/agent-tree/` |
| 3 | Teardown — remove `agents/`, `skills/`, `commands/` contents | pending |
| 4 | Rebuild: new skills, commands, and reference agent files under the v19 model | pending |
| 5 | Re-scope `core/` Rust CLI — keep only subcommands the agent tree actually uses | pending |
| 6 | New invariant set — boundary, delegation, tool access (derived from v19 model, not ported from v18) | pending |

### Reference Design

- `references/agent-tree/README.md` — index and reading order
- `references/agent-tree/node-prompt-template.md` — required shape of a node's CLAUDE.md
- `references/agent-tree/source-as-tool.md` — tool invocation conventions
- `references/agent-tree/delegation.md` — parent↔child contract
- `references/agent-tree/decomposition.md` — when/where to split a node

The legacy `references/inspect/` and `references/shared/` directories remain
only until Roadmap step 3; treat them as v18 artifacts, not current design.

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
├── core/              — Rust CLI engine (scope under review; Roadmap step 5)
│   ├── src/
│   ├── tests/
│   └── Cargo.toml
├── skills/            — (pending teardown — Roadmap step 3)
├── agents/            — (pending teardown — Roadmap step 3)
├── commands/          — (pending teardown — Roadmap step 3)
├── hooks/             — Hook definitions
├── scripts/           — Shell utility scripts
└── references/        — Reference materials
```

### Language & Runtime

- Rust, edition 2021, stable toolchain — only inside `core/`.
- Skills / agents / commands / hooks: Markdown + shell, consumed by Claude Code.

### Naming Conventions

- Rust source files: `snake_case.rs`
- Skill, agent, command files: `kebab-case.md`
- CLI subcommand names: `kebab-case`
- Sub-modules under `core/src/`: `snake_case/`

### Coding Rules (Rust `core/`)

- Custom error types use `thiserror::Error`; no ad-hoc `String` or
  `Box<dyn Error>` for library errors.
- `serde` for every data type that crosses the CLI boundary (stdout JSON,
  stderr errors).
- No `unwrap()` / `expect()` in library code; return `Result`.
- No async in production code; async is restricted to the Tokio test runtime.

### Naming Rules

- Functions and locals: `snake_case`
- Structs, enums, traits: `PascalCase`
- Constants and statics: `SCREAMING_SNAKE_CASE`
- Enum variants: `PascalCase`

## Domain Context

None.

## Requirements

None.
