---
name: flow-status
description: |
  Show state summary of one or all /flow tasks. Reads .claude/workflows/flow/*/state.json
  and prints per-node status (pending/running/complete/failed) with retry counts and
  the latest failure excerpt for halted tasks.
argument-hint: '[task-id]'
allowed-tools: [Read, Bash, Glob]
---

# /flow-status

Read-only state inspection for DAG tasks managed by `/flow`.

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `task-id` | No | — | Specific task id. If omitted, lists all tasks under `.claude/workflows/flow/`. |

## Behavior

1. If `task-id` is given:
   - Read `.claude/workflows/flow/{task-id}/state.json`.
   - Read `.claude/workflows/flow/{task-id}/dag.json` for node titles.
   - For each node, print: `id | title | status | attempts | (failure excerpt if failed)`.
   - Print overall task status (`running | complete | halted`) and totals.
   - If task is `halted`, print the hint: `Use /flow-resume {task-id} to retry`.
2. If `task-id` is omitted:
   - `Glob`: `.claude/workflows/flow/*/state.json`.
   - Print one line per task: `task-id | overall-status | complete/total nodes | last-updated`.

This is purely a reader. It never mutates state.json.

## Output example

```
Task: 01HZXX...abcd (halted)
  Spec: Add /health and /ready endpoints with tests
  Nodes (3 total, 2 complete, 1 failed):
    health-endpoint      complete   attempts=1
    ready-endpoint       failed     attempts=3   tests exited 1: "TypeError at ready.ts:42"
    merge-endpoints      pending    (blocked by ready-endpoint)
  Hint: /flow-resume 01HZXX...abcd to retry
```
