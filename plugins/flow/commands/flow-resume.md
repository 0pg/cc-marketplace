---
name: flow-resume
description: |
  Resume a /flow task that halted or was interrupted. Re-enters the execution loop
  using the immutable dag.json and the last-persisted state.json. Failed nodes are
  retried (attempts counter continues from where it left off, up to --max-retries).
  Running-at-interruption nodes are re-dispatched.
argument-hint: '<task-id> [--max-retries N]'
allowed-tools: [Read, Write, Edit, Bash, Glob, Grep, Task, Skill, AskUserQuestion]
---

# /flow-resume

Resume a previously-created DAG task.

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `task-id` | Yes | — | The task id to resume. |
| `--max-retries N` | No | 3 | Reset/raise the retry cap for the remaining execution. Applies from now onward; does not erase prior attempts. |

## Behavior

1. Verify `.claude/workflows/flow/{task-id}/{dag.json,state.json}` exist.

2. Load `state.json`. Compute nodes needing work:
   - `pending` with all deps `complete` → ready queue.
   - `failed` → re-queue with `attempts` carried forward. If `attempts >= max-retries` AND the user supplies `--max-retries` raising the cap, re-arm; otherwise surface the failure and exit.
   - `running` (interrupted) → treat as `failed`; if the worktree exists, remove it via `git worktree remove --force` before step 3 restores a clean copy.

3. **Worktree restoration** — for every node scheduled for dispatch in step 2, ensure its worktree exists:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/restore-worktree.sh" "$task_id" "$node_id"
```

Exit 0 means the worktree is ready (idempotent no-op if it already existed; restored from branch if it was missing). Exit 4 means `flow/{task-id}/{node-id}` no longer exists — the task was fully cleaned by `/flow-clean --full`. Halt the resume with `reason: "task-fully-cleaned"` and instruct the user to start a new task.

4. Re-enter the `flow` skill's execution loop with the computed ready set.

5. On task completion (all nodes `complete`) or fresh halt, report. If completion, SKILL §8 runs auto-cleanup — worktrees created by restoration in step 3 are removed again.

## Invariant

`dag.json` is NEVER modified by resume (INV-F5). If the user wants to change the plan, they start a new task.

## Pre-flight check

If the main branch has advanced since the task started (`git merge-base`), warn:

```
WARNING: main has advanced since this task started. Node worktrees may be stale.
Consider starting a new /flow task instead.
Continue anyway? [y/N]
```

Skip this prompt when `--no-ask` is set.
