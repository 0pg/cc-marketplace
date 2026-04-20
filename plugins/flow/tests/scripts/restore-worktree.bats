#!/usr/bin/env bats

SCRIPT="$BATS_TEST_DIRNAME/../../scripts/restore-worktree.sh"

setup() {
  TMPREPO="$(mktemp -d)"
  cd "$TMPREPO"
  git init -q
  git commit --allow-empty -q -m "init"

  TASK_ID="flow-20260420-cafe1234"
  TASK_DIR=".claude/workflows/flow/$TASK_ID"
  BRANCH="flow/$TASK_ID/node-a"

  git checkout -q -b "$BRANCH"
  echo "work" > w.txt && git add w.txt && git commit -q -m "work"
  git checkout -q -
}

teardown() {
  rm -rf "$TMPREPO"
}

@test "creates worktree when missing" {
  run "$SCRIPT" "$TASK_ID" "node-a"
  [ "$status" -eq 0 ]
  [ -f "$TASK_DIR/worktrees/node-a/w.txt" ]
}

@test "no-op when worktree already exists" {
  "$SCRIPT" "$TASK_ID" "node-a"
  run "$SCRIPT" "$TASK_ID" "node-a"
  [ "$status" -eq 0 ]
}

@test "exits 4 when branch is missing" {
  run "$SCRIPT" "$TASK_ID" "ghost-node"
  [ "$status" -eq 4 ]
}
