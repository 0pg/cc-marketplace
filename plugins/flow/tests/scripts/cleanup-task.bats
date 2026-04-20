#!/usr/bin/env bats

SCRIPT="$BATS_TEST_DIRNAME/../../scripts/cleanup-task.sh"

setup() {
  TMPREPO="$(mktemp -d)"
  cd "$TMPREPO"
  git init -q
  git commit --allow-empty -q -m "init"

  TASK_ID="flow-20260420-deadbeef"
  TASK_DIR=".claude/workflows/flow/$TASK_ID"
  mkdir -p "$TASK_DIR/worktrees"
  mkdir -p "$TASK_DIR/nodes/node-a" "$TASK_DIR/merges/merge-1"
  echo '{"task_id":"'"$TASK_ID"'","status":"complete"}' > "$TASK_DIR/state.json"
  echo "spec content" > "$TASK_DIR/spec.md"
  echo "dag content" > "$TASK_DIR/dag.json"
  echo "node output" > "$TASK_DIR/nodes/node-a/output.md"
  echo "merge validators" > "$TASK_DIR/merges/merge-1/validators.json"

  # Create a real git worktree so the script exercises `git worktree remove`.
  git checkout -q -b "flow/$TASK_ID/node-a"
  echo "x" > file && git add file && git commit -q -m "node-a work"
  git checkout -q main 2>/dev/null || git checkout -q master 2>/dev/null || git checkout -q -
  git worktree add -q "$TASK_DIR/worktrees/node-a" "flow/$TASK_ID/node-a"
  echo "artifact" > "$TASK_DIR/worktrees/node-a/artifact.txt"
}

teardown() {
  rm -rf "$TMPREPO"
}

@test "auto mode removes worktrees but preserves state.json/spec.md/dag.json/nodes/merges" {
  run "$SCRIPT" "$TASK_ID" auto
  [ "$status" -eq 0 ]

  [ ! -d "$TASK_DIR/worktrees/node-a" ]
  [ ! -d "$TASK_DIR/worktrees" ]

  [ -f "$TASK_DIR/state.json" ]
  [ -f "$TASK_DIR/spec.md" ]
  [ -f "$TASK_DIR/dag.json" ]
  [ -f "$TASK_DIR/nodes/node-a/output.md" ]
  [ -f "$TASK_DIR/merges/merge-1/validators.json" ]

  # Branch should remain in auto mode.
  run git show-ref --quiet --heads "flow/$TASK_ID/node-a"
  [ "$status" -eq 0 ]
}

@test "auto mode is idempotent (runs cleanly when worktrees already gone)" {
  "$SCRIPT" "$TASK_ID" auto
  run "$SCRIPT" "$TASK_ID" auto
  [ "$status" -eq 0 ]
}

@test "auto mode exits 3 when task dir does not exist" {
  run "$SCRIPT" "does-not-exist" auto
  [ "$status" -eq 3 ]
}

@test "full mode removes task dir AND flow/{task-id}/* branches" {
  run "$SCRIPT" "$TASK_ID" full
  [ "$status" -eq 0 ]

  [ ! -d "$TASK_DIR" ]

  run git show-ref --quiet --heads "flow/$TASK_ID/node-a"
  [ "$status" -ne 0 ]
}

@test "full mode skips the currently-checked-out flow branch" {
  # Put HEAD on a flow branch to simulate the user being inside it.
  # First remove the worktree so we can check out the branch.
  git worktree remove --force "$TASK_DIR/worktrees/node-a" 2>/dev/null || true
  git checkout -q "flow/$TASK_ID/node-a"

  run "$SCRIPT" "$TASK_ID" full
  [ "$status" -eq 0 ]

  # Task dir gone, but the current branch remains.
  [ ! -d "$TASK_DIR" ]
  run git show-ref --quiet --heads "flow/$TASK_ID/node-a"
  [ "$status" -eq 0 ]
}

@test "full mode is idempotent" {
  "$SCRIPT" "$TASK_ID" full
  run "$SCRIPT" "$TASK_ID" full
  # Second run: task dir already gone → exit 3 is acceptable.
  [ "$status" -eq 3 ]
}
