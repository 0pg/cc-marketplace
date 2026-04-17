---
name: flow
description: |
  DAG-based task execution. Turns a user request into atomic subtasks, builds a DAG,
  executes independent nodes in parallel under git-worktree isolation, and performs
  merge+validate as an independent cascade step. Persistent state at
  .claude/workflows/flow/{task-id}/ enables resume.
  Trigger keywords: DAG, parallel task, multi-branch feature, fan-out fan-in
argument-hint: '"<request>" [--no-ask] [--max-retries N]'
allowed-tools: [Read, Write, Edit, Bash, Glob, Grep, Task, Skill, AskUserQuestion]
---

# /flow

Executes a user request end-to-end as a DAG of atomic subtasks.

## Triggers

- `/flow`
- `run as DAG`
- `parallel implementation`
- `fan-out features`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `request` | No* | — | The user's request text. *If omitted and `--no-ask` is NOT set, `/flow` asks exactly once. |
| `--no-ask` | No | false | Suppress interactive prompts. Planner's self-review substitutes for user DAG approval. |
| `--max-retries` | No | 3 | Per-node retry cap before halting. |
| `--resume <task-id>` | No | — | Equivalent to `/flow-resume <task-id>`. |

## What it does

Delegates to the `flow` skill, which runs the full pipeline:

1. **Intake** — collect request (one AskUserQuestion if missing and interactive).
2. **Interview** (`flow-interviewer`) — clarify requirements, produce `spec.md` with acceptance criteria and `project_test_cmd`.
3. **Plan** (`flow-planner`) — generate `dag.json` with atomic work nodes + explicit merge nodes at every fan-in. Preview as mermaid; request approval (unless `--no-ask`).
4. **Execute** — parallel dispatch of ready nodes. Each work node runs in an isolated git worktree; each merge node runs the cascade validator.
5. **Report** — final summary of all nodes' status, merged branch reference, and full `state.json` path.

On any node's retry exhaustion, the task halts and preserves full failure context for `/flow-resume`.

## See also

- `/flow-status` — show DAG state
- `/flow-resume` — resume halted/interrupted task
- `/flow-graph` — render mermaid of current DAG + status
