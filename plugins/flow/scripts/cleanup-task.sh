#!/bin/bash
# flow cleanup-task.sh
# Removes disposable artifacts for a /flow task.
#
# Modes:
#   auto — remove $task_dir/worktrees/ only. State, node outputs, merge metadata,
#          and branches are preserved. Safe: /flow-resume can restore worktrees
#          from committed branches.
#   full — remove everything: worktrees, task dir, flow/{task-id}/* branches.
#          Destroys the audit trail. Used only by /flow-clean --full.
#
# Exit codes:
#   0 — success (including idempotent no-op)
#   2 — bad invocation (missing args, unknown mode)
#   3 — task dir does not exist
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "usage: cleanup-task.sh <task-id> <auto|full>" >&2
  exit 2
fi

task_id=$1
mode=$2
task_dir=".claude/workflows/flow/$task_id"

if [ ! -d "$task_dir" ]; then
  echo "cleanup-task: task dir not found: $task_dir" >&2
  exit 3
fi

remove_worktrees() {
  local wt_root="$task_dir/worktrees"
  [ -d "$wt_root" ] || return 0
  for wt in "$wt_root"/*; do
    [ -d "$wt" ] || continue
    # Prefer `git worktree remove` so git's metadata stays consistent.
    git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
  done
  rmdir "$wt_root" 2>/dev/null || true
  git worktree prune 2>/dev/null || true
}

case "$mode" in
  auto)
    remove_worktrees
    echo "cleanup-task: removed worktrees for $task_id (auto)"
    ;;
  full)
    echo "cleanup-task: full mode not yet implemented" >&2
    exit 2
    ;;
  *)
    echo "cleanup-task: unknown mode: $mode" >&2
    exit 2
    ;;
esac
