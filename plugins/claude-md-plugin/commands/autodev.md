---
name: autodev
description: |
  Use when the user wants to autonomously develop a feature end-to-end without manual steps.
  Runs requirements → CLAUDE.md → code generation as a pipeline.
  Autonomous execution from start to finish given only requirements, without step-by-step commands.
  Trigger keywords: auto develop, end-to-end, autonomous implementation
argument-hint: '"requirement" [--path path]'
allowed-tools: [Read, Write, Bash, Skill, AskUserQuestion]
---

# /autodev

Autonomously executes requirements from start to finish.
Orchestrates spec (spec definition) and dev (code generation) as a pipeline.

**Thin orchestrator — delegates all spec logic to /spec.**

## Triggers

- `/autodev`
- `auto develop`
- `implement end-to-end`
- `autonomous implementation`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `requirement` | Yes* | - | Requirement text to implement |
| `--path` | No | `.` | Target path |

\* If no requirement is provided, it will be collected once via AskUserQuestion.

## AskUserQuestion Budget

autodev permits at most **1 AskUserQuestion** total across the entire workflow:
- Either in Step 1 (requirement collection when missing)
- Or in spec's Self Socratic Loop last-resort (when max_rounds exhausted)
- NOT both.

When Step 1 uses AskUserQuestion, autodev passes `--no-ask` to spec.
When Step 1 is skipped (requirement provided), spec runs without `--no-ask`.

## Workflow

### Step 1: Requirement Collection

If no requirement provided, AskUserQuestion once:
- "What feature would you like to implement? Briefly describe core behavior and target path."
- Set `no_ask = true`.

If requirement is provided: `no_ask = false`.

After this step, **AskUserQuestion is prohibited for all remaining steps.**

### Step 2: Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### Step 3: Spec

```
Skill("claude-md-plugin:spec", args: "{requirement} --path {impl_path} {--no-ask if no_ask}")
```

spec internally runs: Self Socratic Loop → decompose → Socratic Loop → execute.

Check spec-result:
- `status: success` → proceed to Step 4
- `status: failed | cancelled_by_user` → exit with error report

### Step 4: Dev

```
Skill("claude-md-plugin:dev", args: "--conflict overwrite --path {impl_path}")
```

Check dev-result:
- `status: success | partial` → proceed to Step 5
- `status: failed` → exit with warning

### Step 5: Result Report

**Success:**

```
✓ autodev complete
  spec: CLAUDE.md + DEVELOPERS.md generated
  dev:  Code generation complete
```

```bash
git diff --stat
```

**Failure (spec or dev failed):**

```
⚠ autodev terminated (reason: {reason})
  Resolve manually with /spec or /dev.
```

## Error Handling

| Situation | Response |
|-----------|----------|
| No requirement | AskUserQuestion once in Step 1 |
| spec failed | Report error, exit |
| spec cancelled by user | Report cancellation, exit |
| dev failed | Report warning, show partial results |
