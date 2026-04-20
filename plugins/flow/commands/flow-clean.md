---
name: flow-clean
description: |
  Remove disposable artifacts for a /flow task. Default mode removes only the
  worktrees (same as SKILL §8 auto-clean). --full additionally deletes the task
  directory and all flow/{task-id}/* branches, destroying the audit trail.
argument-hint: '<task-id> [--full] [--force]'
allowed-tools: [Read, Bash]
---

# /flow-clean

Manually remove a task's disposable artifacts. Complements the auto-cleanup that runs on task `complete` (SKILL §8).

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `task-id` | Yes | — | The task id to clean. |
| `--full` | No | false | Also remove the task directory (`state.json`, `spec.md`, `dag.json`, `nodes/`, `merges/`) and every `flow/{task-id}/*` branch except the currently-checked-out one. No undo. |
| `--force` | No | false | Skip the pre-flight warning when the task is not in `complete` state. |

## Behavior

1. Read `.claude/workflows/flow/{task-id}/state.json` to determine `status`. If the task directory does not exist, treat this as "already clean" — report `already clean; nothing to do` and exit 0 (idempotent re-runs must not fail).

2. If `--full` is set AND `status != "complete"` AND `--force` is NOT set: print the warning below and halt. The user must re-run with `--force` to proceed.

```
WARNING: task {task-id} is not in 'complete' state (status={status}). Full clean
will destroy partial work including branches. Re-run with --force to proceed.
```

3. Invoke the Hands-layer script:

```bash
mode="auto"
[ "$full" = "true" ] && mode="full"
"${CLAUDE_PLUGIN_ROOT}/scripts/cleanup-task.sh" "<task-id>" "$mode"
```

4. Report what was removed (echoed verbatim from the script's stdout).

## Examples

- `/flow-clean flow-20260420-deadbeef` — remove worktrees only, preserve state + branches.
- `/flow-clean flow-20260420-deadbeef --full` — full purge; refuses when status != complete.
- `/flow-clean flow-20260420-deadbeef --full --force` — full purge regardless of status.

## Exit semantics

- 0: success (including idempotent re-runs where the task dir was already removed; see step 1).
- 2: bad invocation (missing task-id, unknown flag).
- 3: reserved by the underlying script for "task dir missing"; the command translates this to exit 0 per step 1 so users can safely re-run `/flow-clean` without scripting around it.
