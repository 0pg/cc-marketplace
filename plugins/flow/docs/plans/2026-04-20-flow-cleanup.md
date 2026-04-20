# flow Cleanup (v0.4.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-remove disposable `/flow` artifacts (worktrees + stale `/tmp` session dirs) on task completion; make worktree deletion safe by relying on committed branches as SSoT; add `/flow-clean` for opt-in full purge; restore worktrees on `/flow-resume` when they are missing.

**Architecture:**
- Cleanup logic lives in **Hands-layer shell scripts** under `plugins/flow/scripts/`. SKILL prose just shells out — keeps the Brain free of filesystem mechanics.
- **Git commits are the source of truth** (CLAUDE.md §Development Principles: "Every fan-in has an explicit merge node" and INV-F4 "commit all produced changes to its branch before returning success"). Worktrees are disposable working copies; `state.json.nodes[n]` already records the branch ref.
- Cleanup is **asymmetric by outcome**: `complete` → auto-clean worktrees; `halted` → never auto-clean (worktrees are diagnostic surface for `/flow-resume`).
- `/tmp/flow/{session-id}/` GC is done by the existing `SessionStart` hook (scope-appropriate: cross-task, time-based).

**Tech Stack:** Bash (POSIX + git CLI), shellcheck for linting, existing plugin prose files (Markdown).

---

## File Structure

**New files:**
- `plugins/flow/scripts/cleanup-task.sh` — removes task artifacts. Modes: `auto` (worktrees only) | `full` (worktrees + task dir + branches).
- `plugins/flow/scripts/restore-worktree.sh` — idempotent `git worktree add` for a given node.
- `plugins/flow/scripts/gc-tmp.sh` — prunes old `/tmp/flow/{session-id}/` dirs.
- `plugins/flow/commands/flow-clean.md` — `/flow-clean <task-id> [--full] [--force]` command definition.
- `plugins/flow/tests/scripts/cleanup-task.bats` — bats tests for `cleanup-task.sh` (auto + full modes).
- `plugins/flow/tests/scripts/restore-worktree.bats` — bats tests for `restore-worktree.sh`.
- `plugins/flow/tests/scripts/gc-tmp.bats` — bats tests for `/tmp` GC.
- `plugins/flow/tests/scripts/README.md` — how to run the bats suite.

**Modified files:**
- `plugins/flow/skills/flow/SKILL.md` — add §8 "Cleanup on completion" after §7. Add callout in §6 loop about "complete → trigger §8".
- `plugins/flow/commands/flow-resume.md` — insert "1b. Worktree restoration" between steps 1 and 2; tighten "running → treat as failed" wording so remove-then-restore is explicit.
- `plugins/flow/hooks/session-start.sh` — prepend an invocation of `gc-tmp.sh` before the existing in-progress scan.
- `plugins/flow/agents/flow-worker.md` — clarify cleanup-ownership note (line 82): "SKILL auto-cleans on complete; `/flow-resume` restores via `restore-worktree.sh`; `/flow-clean --full` purges".
- `plugins/flow/CLAUDE.md` — add Cleanup row to Commands table, expand Hooks table, add "Cleanup semantics" under Architecture, bump version rules, update §Scope v0.4 in-scope list.
- `plugins/flow/.claude-plugin/plugin.json` — version `0.3.0 → 0.4.0`.
- `/Users/jhk/git/cc-marketplace/.claude-plugin/marketplace.json` — flow plugin version `0.3.0 → 0.4.0`.

**Unchanged (explicit non-goals for this plan):**
- `flow-core` Rust CLI — no Rust changes. Cleanup is pure filesystem + git; no value add from Rust.
- DAG schema, `state.json` schema — unchanged. Cleanup reads existing fields only.
- Merger / reviewer / planner / interviewer agents — unchanged.

---

## Prerequisites — before starting

- [ ] **Prereq 1: Install bats**

Run: `brew install bats-core` (macOS) or `apt install bats` (Debian/Ubuntu).

Verify: `bats --version` prints a version ≥ 1.5.0. If missing, install before Task 1.

- [ ] **Prereq 2: Create a feature branch**

```bash
git checkout -b feat/flow-cleanup-v0.4.0
```

Expected: `git branch --show-current` prints `feat/flow-cleanup-v0.4.0`.

---

## Task 1: `cleanup-task.sh` — auto mode (worktrees only)

**Files:**
- Create: `plugins/flow/scripts/cleanup-task.sh`
- Test: `plugins/flow/tests/scripts/cleanup-task.bats`

- [ ] **Step 1: Write the failing test**

Create `plugins/flow/tests/scripts/cleanup-task.bats`:

```bash
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
```

- [ ] **Step 2: Run the test and confirm it fails**

Run: `bats plugins/flow/tests/scripts/cleanup-task.bats`
Expected: FAIL — "No such file or directory" on `$SCRIPT` (cleanup-task.sh does not exist yet).

- [ ] **Step 3: Write `cleanup-task.sh` (auto mode first)**

Create `plugins/flow/scripts/cleanup-task.sh`:

```bash
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
```

Then make it executable:

```bash
chmod +x plugins/flow/scripts/cleanup-task.sh
```

- [ ] **Step 4: Run the test and confirm it passes**

Run: `bats plugins/flow/tests/scripts/cleanup-task.bats`
Expected: 3 passing tests.

- [ ] **Step 5: Commit**

```bash
git add plugins/flow/scripts/cleanup-task.sh plugins/flow/tests/scripts/cleanup-task.bats
git commit -m "feat(flow): cleanup-task.sh auto mode removes worktrees"
```

---

## Task 2: `cleanup-task.sh` — full mode (task dir + branches)

**Files:**
- Modify: `plugins/flow/scripts/cleanup-task.sh`
- Modify: `plugins/flow/tests/scripts/cleanup-task.bats`

- [ ] **Step 1: Write additional failing tests**

Append to `plugins/flow/tests/scripts/cleanup-task.bats`:

```bash
@test "full mode removes task dir AND flow/{task-id}/* branches" {
  run "$SCRIPT" "$TASK_ID" full
  [ "$status" -eq 0 ]

  [ ! -d "$TASK_DIR" ]

  run git show-ref --quiet --heads "flow/$TASK_ID/node-a"
  [ "$status" -ne 0 ]
}

@test "full mode skips the currently-checked-out flow branch" {
  # Put HEAD on a flow branch to simulate the user being inside it.
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
```

- [ ] **Step 2: Run and confirm failure**

Run: `bats plugins/flow/tests/scripts/cleanup-task.bats`
Expected: the new full-mode tests FAIL with "full mode not yet implemented".

- [ ] **Step 3: Replace the `full)` branch in `cleanup-task.sh`**

Edit `plugins/flow/scripts/cleanup-task.sh`. Replace the `full)` case body with:

```bash
  full)
    remove_worktrees
    current=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
    # `git branch --list` pads with 2 leading chars ("* " or "  "); strip with sed.
    while IFS= read -r branch; do
      [ -n "$branch" ] || continue
      if [ "$branch" = "$current" ]; then
        echo "cleanup-task: skipping current branch $branch" >&2
        continue
      fi
      git branch -D "$branch" >/dev/null 2>&1 || true
    done < <(git branch --list "flow/$task_id/*" | sed 's/^..//')
    rm -rf "$task_dir"
    echo "cleanup-task: removed task dir + branches for $task_id (full)"
    ;;
```

- [ ] **Step 4: Run and confirm all tests pass**

Run: `bats plugins/flow/tests/scripts/cleanup-task.bats`
Expected: 6 passing tests.

- [ ] **Step 5: Commit**

```bash
git add plugins/flow/scripts/cleanup-task.sh plugins/flow/tests/scripts/cleanup-task.bats
git commit -m "feat(flow): cleanup-task.sh full mode removes task dir and branches"
```

---

## Task 3: `restore-worktree.sh`

**Files:**
- Create: `plugins/flow/scripts/restore-worktree.sh`
- Test: `plugins/flow/tests/scripts/restore-worktree.bats`

- [ ] **Step 1: Write the failing test**

Create `plugins/flow/tests/scripts/restore-worktree.bats`:

```bash
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
```

- [ ] **Step 2: Run and confirm failure**

Run: `bats plugins/flow/tests/scripts/restore-worktree.bats`
Expected: FAIL — "No such file or directory".

- [ ] **Step 3: Write `restore-worktree.sh`**

Create `plugins/flow/scripts/restore-worktree.sh`:

```bash
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
```

Then:

```bash
chmod +x plugins/flow/scripts/restore-worktree.sh
```

- [ ] **Step 4: Run and confirm pass**

Run: `bats plugins/flow/tests/scripts/restore-worktree.bats`
Expected: 3 passing tests.

- [ ] **Step 5: Commit**

```bash
git add plugins/flow/scripts/restore-worktree.sh plugins/flow/tests/scripts/restore-worktree.bats
git commit -m "feat(flow): restore-worktree.sh rebuilds node worktree from committed branch"
```

---

## Task 4: `gc-tmp.sh` — `/tmp/flow/{session-id}/` GC

**Files:**
- Create: `plugins/flow/scripts/gc-tmp.sh`
- Test: `plugins/flow/tests/scripts/gc-tmp.bats`

- [ ] **Step 1: Write the failing test**

Create `plugins/flow/tests/scripts/gc-tmp.bats`:

```bash
#!/usr/bin/env bats

SCRIPT="$BATS_TEST_DIRNAME/../../scripts/gc-tmp.sh"

setup() {
  TMPROOT="$(mktemp -d)/flow"
  mkdir -p "$TMPROOT"
  export FLOW_TMP_ROOT="$TMPROOT"
  export FLOW_TMP_KEEP_DAYS=7
  export CLAUDE_SESSION_ID="current-session"

  mkdir -p "$TMPROOT/current-session"
  mkdir -p "$TMPROOT/old-session"
  mkdir -p "$TMPROOT/recent-session"
  touch -t $(date -u -v-10d +%Y%m%d%H%M 2>/dev/null || date -u -d '10 days ago' +%Y%m%d%H%M) "$TMPROOT/old-session" 2>/dev/null || true
  # macOS/BSD date uses -v; GNU uses -d. Above tries macOS first, falls back to GNU.
  # If both fail (unlikely), set a file older than 7 days manually:
  find "$TMPROOT/old-session" -exec touch -d "10 days ago" {} \; 2>/dev/null || true
}

teardown() {
  rm -rf "$(dirname "$TMPROOT")"
}

@test "removes sessions older than keep-days" {
  run "$SCRIPT"
  [ "$status" -eq 0 ]
  [ ! -d "$TMPROOT/old-session" ]
}

@test "preserves current session regardless of age" {
  find "$TMPROOT/current-session" -exec touch -d "30 days ago" {} \; 2>/dev/null || \
    touch -t $(date -u -v-30d +%Y%m%d%H%M 2>/dev/null) "$TMPROOT/current-session"
  run "$SCRIPT"
  [ "$status" -eq 0 ]
  [ -d "$TMPROOT/current-session" ]
}

@test "preserves sessions younger than keep-days" {
  run "$SCRIPT"
  [ "$status" -eq 0 ]
  [ -d "$TMPROOT/recent-session" ]
}

@test "no-op when FLOW_TMP_ROOT does not exist" {
  rm -rf "$TMPROOT"
  run "$SCRIPT"
  [ "$status" -eq 0 ]
}
```

- [ ] **Step 2: Run and confirm failure**

Run: `bats plugins/flow/tests/scripts/gc-tmp.bats`
Expected: FAIL — script not found.

- [ ] **Step 3: Write `gc-tmp.sh`**

Create `plugins/flow/scripts/gc-tmp.sh`:

```bash
#!/bin/bash
# flow gc-tmp.sh
# Prune /tmp/flow/{session-id}/ directories older than FLOW_TMP_KEEP_DAYS (default 7),
# preserving the current session ($CLAUDE_SESSION_ID).
#
# Variables (all optional, sensible defaults):
#   FLOW_TMP_ROOT      — default "/tmp/flow"
#   FLOW_TMP_KEEP_DAYS — default 7
#   CLAUDE_SESSION_ID  — preserved if set
set -euo pipefail

tmp_root="${FLOW_TMP_ROOT:-/tmp/flow}"
keep_days="${FLOW_TMP_KEEP_DAYS:-7}"
current="${CLAUDE_SESSION_ID:-}"

[ -d "$tmp_root" ] || exit 0

# `find -mtime +N` matches entries strictly older than N days.
while IFS= read -r -d '' d; do
  name=$(basename "$d")
  if [ -n "$current" ] && [ "$name" = "$current" ]; then
    continue
  fi
  rm -rf "$d"
done < <(find "$tmp_root" -mindepth 1 -maxdepth 1 -type d -mtime +"$keep_days" -print0 2>/dev/null)
```

Then:

```bash
chmod +x plugins/flow/scripts/gc-tmp.sh
```

- [ ] **Step 4: Run and confirm pass**

Run: `bats plugins/flow/tests/scripts/gc-tmp.bats`
Expected: 4 passing tests.

- [ ] **Step 5: Commit**

```bash
git add plugins/flow/scripts/gc-tmp.sh plugins/flow/tests/scripts/gc-tmp.bats
git commit -m "feat(flow): gc-tmp.sh prunes stale /tmp/flow session dirs"
```

---

## Task 5: Wire `gc-tmp.sh` into `session-start.sh`

**Files:**
- Modify: `plugins/flow/hooks/session-start.sh`

- [ ] **Step 1: Read current hook**

Run: `cat plugins/flow/hooks/session-start.sh` to confirm the current structure (lines 1–54).

- [ ] **Step 2: Insert gc-tmp call between stdin-discard and workflow_root check**

Open `plugins/flow/hooks/session-start.sh`. Find these lines:

```bash
# Discard stdin (hook input JSON — we don't need it).
cat > /dev/null || true

# Only runs inside a repo with a flow workflow directory. Bail silently otherwise.
workflow_root=".claude/workflows/flow"
```

Replace with:

```bash
# Discard stdin (hook input JSON — we don't need it).
cat > /dev/null || true

# /tmp/flow GC — best-effort, silent on failure so we never block session start.
gc_script="${CLAUDE_PLUGIN_ROOT:-}/scripts/gc-tmp.sh"
if [ -x "$gc_script" ]; then
  "$gc_script" 2>/dev/null || true
fi

# Only runs inside a repo with a flow workflow directory. Bail silently otherwise.
workflow_root=".claude/workflows/flow"
```

- [ ] **Step 3: Manual verify**

Run (with a stale /tmp session seeded):

```bash
mkdir -p /tmp/flow/gc-test-old
touch -d "30 days ago" /tmp/flow/gc-test-old 2>/dev/null || \
  touch -t $(date -u -v-30d +%Y%m%d%H%M) /tmp/flow/gc-test-old

CLAUDE_PLUGIN_ROOT="$PWD/plugins/flow" bash plugins/flow/hooks/session-start.sh < /dev/null
test ! -d /tmp/flow/gc-test-old && echo OK
```

Expected: `OK` prints (stale dir removed by the hook).

- [ ] **Step 4: Commit**

```bash
git add plugins/flow/hooks/session-start.sh
git commit -m "feat(flow): session-start hook prunes stale /tmp/flow session dirs"
```

---

## Task 6: SKILL §8 — Cleanup on completion

**Files:**
- Modify: `plugins/flow/skills/flow/SKILL.md`

- [ ] **Step 1: Append §8 after §7 Report**

Open `plugins/flow/skills/flow/SKILL.md`. Find the end of §7 (the line `Print \`state.json\` path for machine-readable follow-up.` and everything up to the ending `## Validator evaluation` heading).

Immediately before `## Validator evaluation`, insert:

```markdown
### 8. Cleanup on completion

After emitting the report in Step 7, branch on final status:

- `state.status = "complete"` → invoke the Hands layer to remove disposable artifacts:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/cleanup-task.sh" "$task_id" auto
```

Auto mode removes `$task_dir/worktrees/` only (can be >1GB per task). Preserved: `state.json`, `dag.json`, `spec.md`, `nodes/*/output.md`, `merges/*/validators.json`, and every `flow/{task_id}/*` branch. This is the full audit trail and the committed work — enough for `/flow-status` and for re-entry via `/flow-resume` after manual inspection.

- `state.status = "halted"` → do NOT auto-clean. Worktrees are diagnostic surface: the user needs them intact for `/flow-resume` or manual triage. Cleanup is opt-in via `/flow-clean`.

**Design rationale (why auto-clean is safe on complete):** git commits on `flow/{task_id}/{node-id}` branches are the source of truth (INV-F4). `state.json.nodes[n]` records the branch ref. If `/flow-resume` is later invoked on an auto-cleaned task, `scripts/restore-worktree.sh` rebuilds each worktree from its committed branch before re-dispatch. Worktree contents are a disposable working copy, not canonical state.
```

- [ ] **Step 2: Cross-reference from §6 loop exit**

In the same file, find the execution-loop block (around line 149). At the line:

```
  if all nodes.status == "complete":
    state.status = "complete"; persist; break
```

Change it to:

```
  if all nodes.status == "complete":
    state.status = "complete"; persist; break  # §8 cleanup runs after §7
```

- [ ] **Step 3: Manual verify — read §8 end-to-end**

Run: `grep -n "### 8" plugins/flow/skills/flow/SKILL.md`

Expected: shows the new §8 header with the correct line number and is followed by the `## Validator evaluation` heading.

- [ ] **Step 4: Commit**

```bash
git add plugins/flow/skills/flow/SKILL.md
git commit -m "docs(flow): SKILL §8 auto-clean worktrees on task complete"
```

---

## Task 7: `/flow-resume` — worktree restoration step

**Files:**
- Modify: `plugins/flow/commands/flow-resume.md`

- [ ] **Step 1: Read current state**

Run: `cat plugins/flow/commands/flow-resume.md`

Confirm the current "Behavior" section has steps 1–4 and that step 2 mentions "running (interrupted) → treat as failed (re-dispatch from scratch; existing worktree is removed first if present)."

- [ ] **Step 2: Rewrite the Behavior section**

Open `plugins/flow/commands/flow-resume.md`. Replace the entire `## Behavior` section with:

```markdown
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
```

- [ ] **Step 3: Manual verify**

Run: `grep -n "Worktree restoration" plugins/flow/commands/flow-resume.md`

Expected: single hit on the new step 3 header.

- [ ] **Step 4: Commit**

```bash
git add plugins/flow/commands/flow-resume.md
git commit -m "docs(flow): /flow-resume restores worktrees via restore-worktree.sh"
```

---

## Task 8: `/flow-clean` command

**Files:**
- Create: `plugins/flow/commands/flow-clean.md`

- [ ] **Step 1: Write the command file**

Create `plugins/flow/commands/flow-clean.md`:

```markdown
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

1. Read `.claude/workflows/flow/{task-id}/state.json` to determine `status`.
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

- 0: success.
- 2: bad invocation (missing task-id, unknown flag).
- 3: task dir does not exist.
```

- [ ] **Step 2: Manual verify**

Run: `ls plugins/flow/commands/flow-clean.md && head -5 plugins/flow/commands/flow-clean.md`

Expected: file exists; frontmatter begins with `name: flow-clean`.

- [ ] **Step 3: Commit**

```bash
git add plugins/flow/commands/flow-clean.md
git commit -m "feat(flow): add /flow-clean command with auto and --full modes"
```

---

## Task 9: Update `flow-worker.md` cleanup-ownership note

**Files:**
- Modify: `plugins/flow/agents/flow-worker.md`

- [ ] **Step 1: Locate and replace**

In `plugins/flow/agents/flow-worker.md`, find this line (approx line 82):

```markdown
6. **Do NOT delete the worktree.** The merger needs it accessible. Cleanup is the SKILL's responsibility on task completion.
```

Replace with:

```markdown
6. **Do NOT delete the worktree.** The merger needs it accessible during the cascade. Cleanup is handled by SKILL §8 on task `complete` (via `scripts/cleanup-task.sh`); `/flow-resume` restores missing worktrees from committed branches via `scripts/restore-worktree.sh`. Full purge is opt-in via `/flow-clean --full`.
```

- [ ] **Step 2: Manual verify**

Run: `grep -n "cleanup-task.sh" plugins/flow/agents/flow-worker.md`

Expected: one hit on the updated line.

- [ ] **Step 3: Commit**

```bash
git add plugins/flow/agents/flow-worker.md
git commit -m "docs(flow): clarify worker cleanup-ownership contract"
```

---

## Task 10: Update `plugins/flow/CLAUDE.md`

**Files:**
- Modify: `plugins/flow/CLAUDE.md`

- [ ] **Step 1: Scope block — add v0.4 bullet**

Find the `**In scope (v0.1):**` list. Immediately after the closing blank line of the `**Out of scope (v0.1):**` list, append:

```markdown
**Added in v0.4:**
- Auto-cleanup of `$task_dir/worktrees/` on task `complete` (SKILL §8).
- `scripts/restore-worktree.sh` — `/flow-resume` rebuilds missing worktrees from committed branches.
- `scripts/gc-tmp.sh` — `SessionStart` hook prunes `/tmp/flow/{session-id}/` dirs older than `FLOW_TMP_KEEP_DAYS` (default 7).
- `/flow-clean <task-id> [--full] [--force]` — opt-in purge.
```

- [ ] **Step 2: Commands table — add flow-clean row**

Find the `## Commands` table. Add a row after the `/flow-graph` row:

```markdown
| `/flow-clean <task-id> [--full] [--force]` | Remove disposable artifacts. Default = worktrees only; `--full` = task dir + branches. Refuses non-complete tasks without `--force`. |
```

- [ ] **Step 3: Hooks table — update SessionStart description**

Find the `## Hooks` table. Replace the single existing `SessionStart` row with:

```markdown
| `SessionStart` | (1) Prunes `/tmp/flow/{session-id}/` dirs older than `FLOW_TMP_KEEP_DAYS` (default 7), preserving the current session. (2) Scans `.claude/workflows/flow/*/state.json` for tasks with `status ∈ {running, halted}` and emits a one-line notice. Non-blocking, <1s. |
```

- [ ] **Step 4: Architecture — add Cleanup semantics section**

Find the `### Execution loop (logical)` code block. Immediately **after** the closing ``` of that block, insert:

```markdown
### Cleanup semantics

The plugin treats files on disk as two layers:

| Artifact | Authority | Cleanup policy |
|----------|-----------|----------------|
| `flow/{task_id}/{node-id}` branches + commits | **Canonical** — source of truth (INV-F4) | Auto-kept on `complete`; purged only by `/flow-clean --full`. |
| `$task_dir/{state.json, dag.json, spec.md, nodes/, merges/}` | **Audit trail** | Kept on `complete` (small; enables `/flow-status`); purged by `/flow-clean --full`. |
| `$task_dir/worktrees/` | **Disposable working copy** (reconstructible via `git worktree add`) | **Auto-removed on `complete`** (SKILL §8); preserved on `halted` for triage; restored by `/flow-resume` when missing. |
| `/tmp/flow/{session-id}/*.md` | **Ephemeral agent inputs** | GC'd by `SessionStart` hook when older than `FLOW_TMP_KEEP_DAYS` and not the current session. |

This partition makes "commits are SSoT" operational: removing a worktree is always safe because it can be rebuilt from the branch. Removing a branch is destructive and requires explicit opt-in (`--full`).
```

- [ ] **Step 5: Invariants — add INV-F7**

Find the `## Invariants` section. After `### INV-F6: Retry bounds are a bug-guard, not a convergence criterion`, append:

```markdown
### INV-F7: Commits are canonical; worktrees are disposable
```
For any node N with `state.nodes[N].status = "complete"`:
  the branch flow/{task_id}/{N} MUST exist and its tip MUST reflect
  the committed work. The worktree at $task_dir/worktrees/{N} MAY be
  absent (auto-cleaned post-complete or removed by /flow-clean).
  Any reader that needs the files MUST reconstruct the worktree via
  scripts/restore-worktree.sh rather than assuming it is present.
```
Justifies SKILL §8 auto-cleanup and `/flow-resume` restoration.
```

- [ ] **Step 6: Manual verify**

Run these three greps; each should have ≥1 hit:

```bash
grep -n "Added in v0.4" plugins/flow/CLAUDE.md
grep -n "flow-clean" plugins/flow/CLAUDE.md
grep -n "INV-F7" plugins/flow/CLAUDE.md
```

Expected: all three return line numbers.

- [ ] **Step 7: Commit**

```bash
git add plugins/flow/CLAUDE.md
git commit -m "docs(flow): document cleanup semantics, INV-F7, /flow-clean, hook GC"
```

---

## Task 11: Version bump + marketplace sync

**Files:**
- Modify: `plugins/flow/.claude-plugin/plugin.json`
- Modify: `/Users/jhk/git/cc-marketplace/.claude-plugin/marketplace.json`

- [ ] **Step 1: Bump `plugin.json`**

Open `plugins/flow/.claude-plugin/plugin.json`. Change:

```json
  "version": "0.3.0",
```

to:

```json
  "version": "0.4.0",
```

Leave all other fields untouched.

- [ ] **Step 2: Bump marketplace entry**

Open `/Users/jhk/git/cc-marketplace/.claude-plugin/marketplace.json`. In the `flow` entry (around line 33), change:

```json
      "version": "0.3.0",
```

to:

```json
      "version": "0.4.0",
```

- [ ] **Step 3: Verify both JSONs parse**

Run:

```bash
python3 -c "import json; json.load(open('plugins/flow/.claude-plugin/plugin.json'))" && echo "plugin.json OK"
python3 -c "import json; json.load(open('.claude-plugin/marketplace.json'))" && echo "marketplace.json OK"
```

Expected: both "OK" lines print.

- [ ] **Step 4: Commit**

```bash
git add plugins/flow/.claude-plugin/plugin.json .claude-plugin/marketplace.json
git commit -m "chore(flow): bump to v0.4.0 (cleanup feature)"
```

---

## Task 12: End-to-end smoke test

**Files:**
- None (verification only)

- [ ] **Step 1: Run the full bats suite**

Run: `bats plugins/flow/tests/scripts/`

Expected: all tests across `cleanup-task.bats`, `restore-worktree.bats`, `gc-tmp.bats` pass (13 tests total).

- [ ] **Step 2: Simulate auto-cleanup against a seeded task**

Run:

```bash
TMP=$(mktemp -d); cd "$TMP"
git init -q && git commit --allow-empty -q -m init
TASK=flow-20260420-smoke00
mkdir -p ".claude/workflows/flow/$TASK/worktrees" ".claude/workflows/flow/$TASK/nodes/n1"
echo '{"status":"complete"}' > ".claude/workflows/flow/$TASK/state.json"
echo "node output" > ".claude/workflows/flow/$TASK/nodes/n1/output.md"
git checkout -q -b "flow/$TASK/n1"; echo x>f; git add f; git commit -q -m w
git checkout -q -
git worktree add -q ".claude/workflows/flow/$TASK/worktrees/n1" "flow/$TASK/n1"

"/Users/jhk/git/cc-marketplace/plugins/flow/scripts/cleanup-task.sh" "$TASK" auto

test ! -d ".claude/workflows/flow/$TASK/worktrees" && \
test -f ".claude/workflows/flow/$TASK/state.json" && \
test -f ".claude/workflows/flow/$TASK/nodes/n1/output.md" && \
git show-ref --quiet --heads "flow/$TASK/n1" && \
echo "SMOKE OK"
cd - >/dev/null; rm -rf "$TMP"
```

Expected: `SMOKE OK` prints.

- [ ] **Step 3: Simulate restore-after-cleanup**

Run (continuing from a clean `/tmp` repo seeded as in step 2, but don't delete before this):

```bash
TMP=$(mktemp -d); cd "$TMP"
git init -q && git commit --allow-empty -q -m init
TASK=flow-20260420-smoke01
git checkout -q -b "flow/$TASK/n1"; echo x>f; git add f; git commit -q -m w
git checkout -q -

# No worktree yet — this is the post-auto-clean state.
"/Users/jhk/git/cc-marketplace/plugins/flow/scripts/restore-worktree.sh" "$TASK" "n1"

test -f ".claude/workflows/flow/$TASK/worktrees/n1/f" && echo "RESTORE OK"
cd - >/dev/null; rm -rf "$TMP"
```

Expected: `RESTORE OK` prints.

- [ ] **Step 4: Commit (if any incidental fixes came up)**

If steps 1–3 pass without edits, nothing to commit. If any edit was required (e.g., shell portability), commit it here with `fix(flow): cleanup smoke fix — <reason>`.

---

## Self-Review Checklist

Before handing off to a reviewer:

- [ ] **Spec coverage**: Every bullet of the user's request is covered.
  - Auto-delete on completion → Task 6 (SKILL §8) + Task 1–2 (cleanup-task.sh).
  - `/flow-resume` can restore from commits → Task 3 (restore-worktree.sh) + Task 7 (flow-resume.md).
  - `/tmp/flow` GC → Task 4–5.
  - `docs/archive/` garbage → **NOT YET COVERED** — requires user to point at the specific repo so we can tell if it's a worker-spec problem (spec said "write to docs/archive/") or a cleanup gap. Treat as an open question; do not auto-extend the plan.
- [ ] **Placeholder scan**: search for `TBD`, `TODO`, `implement later`, `handle edge cases` — none present.
- [ ] **Type consistency**: script names match everywhere — `cleanup-task.sh`, `restore-worktree.sh`, `gc-tmp.sh`. Mode names match — `auto`, `full`. Flag names match — `--full`, `--force`.
- [ ] **Branch naming**: `flow/{task-id}/{node-id}` used consistently (matches existing INV-F4).
- [ ] **Invariant naming**: `INV-F7` is the next free number after `INV-F6`.
- [ ] **Exit-code contract**: 0 = success, 2 = bad invocation, 3 = task dir missing, 4 = branch missing. Consistent across scripts.

## Open Question (for the user, not part of execution)

`docs/archive/` — you mentioned flow-related files are piling up there. I could not find that dir in `/Users/jhk/git/golemclaw/` or `/Users/jhk/git/cc-marketplace/`. Please share the repo path; depending on whether workers were instructed to write there (spec problem) vs. left output behind (cleanup gap), this may warrant an extension to Task 6 or a separate plan.

---

## Execution Handoff

Plan complete and saved to `plugins/flow/docs/plans/2026-04-20-flow-cleanup.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch with checkpoints for review.

Which approach?
