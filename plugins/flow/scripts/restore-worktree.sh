#!/bin/bash
# flow restore-worktree.sh
# Recreate a missing worktree for a node from its committed branch.
# Idempotent: returns 0 if the worktree already exists.
#
# Exit codes:
#   0 — worktree present (restored or already there)
#   2 — bad invocation
#   4 — branch does not exist (task may have been fully cleaned; cannot restore)
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "usage: restore-worktree.sh <task-id> <node-id>" >&2
  exit 2
fi

task_id=$1
node_id=$2
wt_path=".claude/workflows/flow/$task_id/worktrees/$node_id"
branch="flow/$task_id/$node_id"

if [ -d "$wt_path" ]; then
  exit 0
fi

if ! git show-ref --quiet --heads "$branch"; then
  echo "restore-worktree: branch not found: $branch" >&2
  exit 4
fi

mkdir -p "$(dirname "$wt_path")"
git worktree add -q "$wt_path" "$branch"
