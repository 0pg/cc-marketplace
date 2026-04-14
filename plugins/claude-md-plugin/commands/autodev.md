---
name: autodev
description: |
  Use when the user wants to autonomously develop a feature end-to-end without manual steps.
  Runs requirements → CLAUDE.md → code generation as a pipeline.
  Autonomous execution from start to finish given only requirements, without step-by-step commands.
  Trigger keywords: auto develop, end-to-end, autonomous implementation
argument-hint: '"requirement" [--path path] [--auto-sync]'
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
| `--auto-sync` | No | OFF | Opt-in. After /dev succeeds, propagate schema changes to consumers listed in `${TMP_DIR}affected-consumers.txt` (produced by spec Step 4.5). For each consumer, dispatch `po-consultant` and execute its verdict **verbatim**: `auto_executable` runs `/sync`; anything else (`halt`, `requires_human`, etc.) halts the chain with the verdict's reason preserved and emits `git revert HEAD` as the rollback hint. No Decision enum interpretation — the consultant's execution hint drives behavior. See Step 4.7. |

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
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### Step 3: Spec

```
Skill("claude-md-plugin:spec", args: "{requirement} --path {impl_path} {--no-ask if no_ask}")
```

spec internally runs: Self Socratic Loop → Socratic Loop → execute.

Check spec-result:
- `status: success` → proceed to Step 3.5
- `status: failed | cancelled_by_user` → exit with error report

### Step 3.5: Extract spec target

Parse spec-result block from Step 3:
```
spec_target = dirname(claude_md_file)
If spec_target is empty → spec_target = "."
```

Example: `claude_md_file: src/auth/CLAUDE.md` → `spec_target = "src/auth"`
Example: `claude_md_file: CLAUDE.md` → `spec_target = "."`

### Step 4: Dev

```
Skill("claude-md-plugin:dev", args: "--conflict overwrite --path {impl_path} --targets {spec_target}")
```

Check dev-result:
- `status: success | partial` → proceed to Step 4.7
- `status: failed` → exit with warning

### Step 4.7: Consumer Propagation (--auto-sync)

Runs only when `--auto-sync` is set. Reads `${TMP_DIR}affected-consumers.txt` produced by spec Step 4.5.

For each consumer C (in the order emitted):

1. Dispatch `Task(po-consultant, C)`; write result file.
2. Parse C's `Execution` field using the same extractor as Step 2.1d.
3. Execute the verdict verbatim:
   - `auto_executable` → `Skill(/sync --path $C)`; continue to next consumer on success.
   - otherwise → stop the chain; append C's verdict reason verbatim to the result block's `## Sync Results` section, mark status `halted`, and append the rollback hint `git revert HEAD`.
4. When the chain completes or halts, emit the `## Sync Results` section listing each consumer's outcome (`synced` or `halted: <reason>`).

No Decision enum interpretation. The consultant's own execution hint drives behavior.

When `--auto-sync` is not set, Step 4.7 is skipped; consumers remain listed in spec's Step 4.5 result block for the user to sync manually.

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
