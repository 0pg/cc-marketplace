---
name: bugfix
version: 2.0.0
aliases: [fix, debug]
description: |
  This skill should be used when the user reports a bug, unexpected behavior, or asks to
  "fix this bug", "debug this", "something is broken", "not working as expected", or uses "/bugfix".
  Delegates 3-layer root cause analysis (CLAUDE.md → DEVELOPERS.md → source code) to the
  bugfixer agent, which traces the root cause and fixes at the highest affected layer.
  Trigger keywords: fix bug, debug, unexpected behavior, broken, not working
user_invocable: true
allowed-tools: [Bash, Read, Glob, Write, Edit, Task, AskUserQuestion, Skill]
---

# /bugfix

Delegates 3-layer root cause tracing to the bugfixer agent; executes the agent's verdict.

## Triggers

- `/bugfix`
- `fix bug`
- `something is broken`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `description` | Yes | — | Bug description (expected vs actual) |
| `--path` | No | `.` | Target path |
| `--error` | No | — | Error message or stack trace |
| `--file` | No | — | File where the bug is observed |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
dir_safe=$(echo "${path:-.}" | tr '/' '-')
```

### 1. Bug report collection

Parse `description` for E (expected) and A (actual). If E is absent or unclear,
`AskUserQuestion` once to clarify expected vs actual behavior. Record `--error`
and `--file` arguments.

### 2. Target CLAUDE.md selection

- `--file` provided: walk up from `--file` directory until the nearest CLAUDE.md.
- otherwise: `$CLI_PATH scan-claude-md --root {path}` → pick shallowest node under `--path`.
- If none found: exit with `"CLAUDE.md를 찾을 수 없습니다. --path 또는 --file을 확인하세요."`

Let `selected_node_path` = directory of the selected CLAUDE.md.

### 3. Session file

`${TMP_DIR}bugfix-session-{dir-safe}.md`:

```markdown
# Bugfix Session
target_path: {selected_node_path}
error_message: {--error value or "none"}
target_file: {--file value or "none"}

## Bug Description
expected: {E}
actual: {A}
```

The bugfixer agent reads CLAUDE.md, DEVELOPERS.md, source files, and
`diff-node-history` directly from `selected_node_path` — no pre-extraction needed.

### 4. Dispatch bugfixer

```
Task(bugfixer):
  Session file: ${TMP_DIR}bugfix-session-{dir-safe}.md
  Target path: {selected_node_path}
```

Extract the result block: `status`, `root_cause_layer`, `judgment`,
`fix_type`, `fix_description`, `test_result`, `escalation` (optional),
`proposed_change` (optional).

### 5. Execute the agent's verdict

The agent decides root cause and fix type; the SKILL executes the decision.
Layer 1 / Layer 2 edits always require user approval (INV-bugfix-2).

| verdict | SKILL action |
|---------|--------------|
| `status: not_a_bug` | Print summary, exit |
| `status: escalated` | Present `escalation` context + relevant choices via `AskUserQuestion`, then re-run Step 5 as if the user's choice had come from the agent |
| `root_cause_layer: 1` | `AskUserQuestion` to approve CLAUDE.md edit → Edit → `git commit -m "spec({path}): fix requirement — {summary}"` → `Skill("claude-md-plugin:dev", args: "--path {path} --conflict overwrite")` |
| `root_cause_layer: 2` | `AskUserQuestion` to approve DEVELOPERS.md edit → Edit → `Skill("claude-md-plugin:dev", args: "--path {path} --conflict overwrite")` |
| `root_cause_layer: 3`, `test_result: passed` | Bugfix commit (Step 6) |
| `root_cause_layer: 3`, `test_result: skipped` (spec changed, code not regenerated) | `Skill("claude-md-plugin:dev", args: "--path {path} --conflict overwrite")` |
| `root_cause_layer: 3`, `test_result: failed` | Print `fix_description`, exit with status `failed` |
| `root_cause_layer: multi` | Iterate through layers in order L1 → L2 → L3 using the rows above |
| `status: failed` | Print `fix_description` + recommend `/validate` or manual review, exit |

When `status: escalated`, construct the choice set from the escalation context:
- Include "CLAUDE.md에 요구사항을 추가/명확화한다" when L1 issue
- Include "코드만 수정한다" when L3-only case (spec and code both intended E)
- Include "CLAUDE.md와 코드를 함께 수정한다" when both L1 and L3 apply
- Always include "현재 동작(A)이 올바름 (버그 아님)"

### 6. Bugfix commit (L3 fixes only)

```bash
git add {modified source files} {added test files}
git commit -m "bugfix({path}): {fix_description one-liner}

Root cause: Layer 3 — {fix_description}

Changes:
- {list of modified/added files}"
```

L1/L2 fixes are committed by `spec` / `dev` respectively — no separate bugfix commit.

### 7. Result

```
---bugfix-complete---
status: fixed | escalated | not_a_bug | failed
root_cause_layer: {N}
fix_type: {type}
summary: {one sentence}
---end-bugfix-complete---
```

## Invariants

The two bugfix invariants live inside the bugfixer agent (judgment) and this
SKILL's Step 5 table (execution). Stated here for reference only:

- **INV-bugfix-1** Code always defers to CLAUDE.md — never patch code while leaving CLAUDE.md inconsistent.
- **INV-bugfix-2** Layer 1/2 edits always require user approval; Layer 3 unambiguous fixes proceed autonomously.

## Error Handling

| Situation | Response |
|-----------|----------|
| E unclear in `description` | `AskUserQuestion` to clarify expected behavior |
| CLAUDE.md not found | Exit with guidance message |
| bugfixer failure | Surface raw error to user |
| /dev regeneration failure | Report + exit `status=failed` |
