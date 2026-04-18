# Agent-Tree Reference (v19)

Reference design for the v19 node-agent tree model. Companion to
`../../CLAUDE.md` (specifically the **Invariants** and **How the
Invariants Manifest** sections).

> Status: baseline complete (v19 rewrite closed as of v19.13.0).
> Expect revisions as the model is exercised against real projects.

## Reading Order

| # | Document | Purpose |
|---|----------|---------|
| 1 | `node-prompt-template.md` | Required shape of a node's `CLAUDE.md` — the agent's system prompt. |
| 2 | `source-as-tool.md` | How an agent treats source files inside its node as callable tools. |
| 3 | `delegation.md` | How a parent agent discovers, invokes, and consumes results from child agents. |
| 4 | `decomposition.md` | When and how to split a node — cohesion/boundary heuristics. |
| 5 | `orchestration.md` | Main-ctx orchestration: plan-first, execute-second protocol used with the `node-agent` subagent. |

## Core Vocabulary

| Term | Meaning |
|------|---------|
| **Node** | A directory (or package) that owns a `CLAUDE.md`. The unit of agent scope. |
| **Node agent** | The agent whose prompt is that node's `CLAUDE.md`. Operates strictly within the node's boundary (plus delegated children). |
| **Tool** (in this plugin's sense) | A source file or subset of source inside the node that the agent invokes via Bash/Edit/Read to do work. Not to be confused with Claude Code's framework tools (Bash, Edit, Read, Task, …). |
| **Delegation** | Parent agent handing off a task to a specific child agent whose domain contains the task. |
| **Boundary** | The set of files a node agent is authorized to read and modify. Equal to the node's own files plus, via delegation, whatever its children expose. |

## Non-Goals

- Re-inventing Claude Code's framework tools. The agent tree rides on top of
  existing primitives (Agent/Task, Bash, Read, Edit, Glob, Grep).
- Prescribing a deterministic verdict protocol like v18's `po-consultant`. The
  parent agent judges delegation targets; we specify the contract, not the
  algorithm.
- Shipping a deterministic CLI core. v18 had a Rust crate for schema
  validation and code analysis; v19 has no equivalent because no v19
  subagent or command needs one. If a future requirement surfaces
  (e.g., DAG cycle detection in main ctx, large-scale tree scans), a
  fresh `core/` can be introduced then — not carried forward from v18.
