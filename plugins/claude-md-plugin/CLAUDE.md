# claude-md-plugin

## Purpose

**A plugin for composing a project as a tree of node-scoped agents.**

Each directory or package in a project is treated as an autonomous **agent**. The
node's `CLAUDE.md` is that agent's system prompt; the node's source code is the
agent's tool set. Because a single god-agent cannot hold a whole project in its
context window, the plugin decomposes responsibility into child agents by domain
cohesion, and parent agents delegate to them.

## Invariants (v19)

Four foundational premises about this plugin and any project that
adopts it. Always true; never renegotiated. The SessionStart hook
(`hooks/philosophy-reminder.md`) injects them into every session on
`startup`, `resume`, `clear`, and `compact`.

1. **A project using this plugin is one agent system, composed of
   multiple agents.** Not a codebase with automation on top. The
   project's identity is multi-agent.
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
   responsibilities are signs that the agent should be split, or that
   responsibilities should be reassigned across the tree.

Everything else in v19 — Node = Agent mapping, `CLAUDE.md` as the
agent's prompt, tree-shaped domain decomposition, delegation, DAG
orchestration, the `node-agent` / `node-executor` / `node-bootstrapper`
split, main-ctx as pure orchestrator — is a **consequence** of these
four plus the practical constraints of Claude Code (context windows,
subagent non-recursion).

## How the Invariants Manifest

> Prior to v19, this plugin treated `CLAUDE.md` as the Primary SSOT and source
> code as a derived artifact. **That premise is retired.** Documents-as-SSOT did
> not materially improve what Claude can actually accomplish on a task. Source
> code is the substance; `CLAUDE.md` is prompt engineering for the agent that
> owns the node.

### Node = Agent

Every node (directory or package) with a `CLAUDE.md` is one of those
agents. The agent has authority and responsibility scoped to its node
boundary, and nothing beyond.

### CLAUDE.md = Agent Prompt

A node's `CLAUDE.md` is the **system prompt** for that node's agent —
role, responsibilities, domain context, interaction contracts with
parents and children, and any rules the agent must uphold. `CLAUDE.md`
is **instruction**, not **truth of record**.

### Source Code = Tools

Files inside a node are the agent's tools, analogous in role to Claude
Code Skills or MCP tools: capabilities the agent invokes, inspects,
modifies, and creates. Code is not "derived from spec" — code **is**
the agent's capability. (This is invariant 3, restated as mechanism.)

Because the tool **is** the capability, breaking the tool breaks the
capability. A change that leaves verification failing or the build
broken is not "progress to be finished later" — it is a broken tool,
and the agent's functional responsibility stays unfulfilled until it
is restored. `node-executor` treats `failed` verification as first-
class failure (subject to auto-retry under `/agent`), not a reportable
side note.

### Agent Tree = Domain Decomposition

A single agent cannot hold the whole project; context windows forbid
it. Cohesive sub-domains become child nodes, each with its own agent.
A parent agent knows its children's roles (by reading each child's
`CLAUDE.md` summary) and **delegates** out-of-scope work to the child
whose domain contains it. Trees are dependency-shaped: parents may
depend on children; children do not reach up to parents; siblings do
not cross-reference directly.

The tree's *shape* is governed by invariant 4: every agent must hold
hierarchical SRP and high context cohesion. This grounds the
decomposition heuristics in `references/agent-tree/decomposition.md`
— "split when the prompt is straddling two domains; merge when two
siblings have collapsed into one" is not stylistic advice but a
direct application of invariant 4.

## Node Layout

```
<node>/
├── CLAUDE.md         # agent prompt — role, responsibilities, domain context
├── <source files>    # the agent's tool set
└── <child nodes>/    # subordinate agents (by domain cohesion)
```

## Status — v19 Transition

The v18 execution model (doc-as-SSOT, `/spec`, `/dev`, `/validate`,
`/decompile`, `/bugfix`, `/impact`, `/inspect`, `/autodev`,
`/project-setup`, `/migrate`, paired `DEVELOPERS.md`, INV-1 ~ INV-15,
session-file pattern, `po-consultant` verdict protocol, Agent
Observations, spec-quality reviewer, Rust `core/` CLI) is **retired**.
v19 replaces it with the four invariants, three subagents, one
orchestration command, and a SessionStart hook. The Roadmap below is
closed.

### Rebuild Roadmap

| Step | Scope | Status |
|------|-------|--------|
| 1 | **Philosophy (this document)** | done (v19.0.0) |
| 2 | Agent-tree reference design — root-agent template, delegation contract, child-discovery convention | draft (v19.1.0) — see `references/agent-tree/` |
| 3 | Teardown — remove v18 `agents/`, `skills/`, `commands/`, `hooks/`, `scripts/`, `core/`, and legacy references | done (v19.2.0) |
| 4 | Rebuild: new skills, commands, and reference agent files under the v19 model | done (v19.10.0 — `/agent` command + 3 subagents + 6 reference docs + SessionStart hook; no plugin-level skills are needed in the v19 baseline) |
| 5 | Re-scope `core/` Rust CLI | done (v19.13.0 — `core/` removed entirely; no v19 subagent or command needs a deterministic CLI. If a future requirement surfaces, a fresh `core/` can be introduced then) |
| 6 | New invariant set — three foundational premises (multi-agent system; per-agent purpose/context/responsibility; code as tool). Injected via SessionStart hook and documented in this file. | done (v19.8.0) |

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

- `agents/node-agent.md` — **planner**. Parameters: `node:`,
  `instructions:`. Loads the target node's `CLAUDE.md`, adopts that
  node's identity, returns a structured work plan with
  `[<id>] ... deps: [...]` line format across Identity / In-Scope /
  Delegated / Escalated / Open Questions sections. Planning-only
  (Read/Glob/Grep allowed inside boundary; no Edit/Write).
- `agents/node-executor.md` — **executor**. Parameters: `node:`,
  `item:`, `upstream:` (optional). Loads the target node's
  `CLAUDE.md`, adopts that node's identity, executes one DAG item
  inside the node's boundary (Edit/Write/Bash allowed), verifies via
  whatever the node declares as verification, returns a structured
  Result (`completed` | `failed` | `blocked`) plus notes and
  follow-ups.
- `agents/node-bootstrapper.md` — **bootstrapper**. Parameters:
  `node:`, `parent_node:` (optional), `intended_role:` (optional).
  Inspects an unprepared node and writes its `CLAUDE.md` per the v19
  template. Main ctx invokes when a planner or executor returns
  `blocked: missing CLAUDE.md`, then retries the original dispatch.

### Commands

- `commands/agent.md` — `/agent "<instruction>" [--max-retries N]`.
  End-to-end orchestration entry point: clarifies, plans recursively
  via `node-agent`, assembles a DAG, executes via `node-executor`
  with state tracking (`pending` | `in-progress` | `completed` |
  `failed` | `blocked` | `halted`) and bounded auto-retry, runs the
  bootstrap sub-flow on missing `CLAUDE.md`, surfaces halted items
  to the user. Frontmatter restricts main-ctx tools to
  `[Read, Glob, Grep, Task, AskUserQuestion]` so Edit/Write/Bash
  cannot bypass the executor.

Main ctx is pure orchestration: it dispatches `node-agent` for
planning, assembles the returned plans into a DAG, then dispatches one
`node-executor` per DAG item in topological order — auto-retrying or
bootstrapping as needed. Main ctx never edits the working tree
directly.

## Instructions

- Document language: English.
- Treat the **Invariants** and **How the Invariants Manifest** sections
  above as the authority during the v19 rewrite. Anything in this repo
  that contradicts them (stale README content, surviving v18 references)
  is legacy pending removal.
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
├── README.md          — user-facing plugin docs (v19)
├── agents/            — `node-agent` + `node-executor` + `node-bootstrapper`
├── commands/          — `/agent` (orchestration entry point)
├── hooks/             — SessionStart philosophy-reminder hook
└── references/        — reference materials
    └── agent-tree/    — v19 reference design
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
