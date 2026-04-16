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
| `--auto-sync` | No | OFF | Opt-in. After /dev succeeds, propagate schema changes to consumers listed in `${TMP_DIR}affected-consumers.txt` (produced by spec Step 4.5). For each consumer, dispatch `po-consultant` and execute its verdict **verbatim**: `auto_executable` runs `/spec --resync --no-ask`; anything else (`halt`, `requires_human`, etc.) halts the chain with the verdict's reason preserved and emits `git revert HEAD` as the rollback hint. No Decision enum interpretation — the consultant's execution hint drives behavior. See Step 4.7. |

\* If no requirement is provided, it will be collected once via AskUserQuestion.

## AskUserQuestion Policy

**Default:** autodev runs non-interactively. Every decision within the workflow
is delegated to an agent authority (see the `--no-ask` table below), and the
orchestrator executes those verdicts verbatim. This is the meaning of
"autonomous."

**Exceptions (judged, not arbitrary):** a user-facing prompt is permitted only
at one of two points, and only when the model judges it genuinely unavoidable:

- **Step 1 — missing requirement.** autodev cannot invent a requirement on the
  user's behalf; when none is supplied on invocation, one prompt collects it.
- **Spec Self Socratic Loop last-resort.** When the requirement-reviewer
  convergence signal fails (the loop would otherwise stall with no authority
  able to decide), spec may prompt once as a last resort.

These are alternatives, not an additive budget: invoking both means autodev
stopped being autonomous. When Step 1 uses its prompt, autodev passes
`--no-ask` to `/spec` — the autonomy exception has already been spent.
Otherwise `/spec` may exercise the last-resort option at its own discretion.

The spec skill's interior agents (requirement-reviewer, impl-reviewer, etc.)
remain free to loop on their own convergence signals — agent-internal behavior
is not user-facing interaction.

### --no-ask (internal)

`--no-ask` is propagated internally by `/autodev` to `/spec` after the single
AskUserQuestion budget is consumed at Step 1 (requirement collection). It is not
a user-facing flag.

Under `--no-ask`, user-facing decisions are **delegated** (not suppressed) to agent
authorities:

| Decision | Authority | Executed as |
|----------|-----------|-------------|
| Which node owns this requirement | each candidate's po-consultant | verdict.execution honored verbatim |
| Is this requirement feasible here | target's po-consultant | verdict.execution honored verbatim |
| Should this reroute elsewhere | target's po-consultant | verdict.redirect_to honored |
| Is the requirement concrete enough | requirement-reviewer | verdict + progress honored |
| Is the plan good | impl-reviewer | verdict + progress honored |
| Should consumers sync (with --auto-sync) | each consumer's po-consultant | verdict.execution honored verbatim |

`--no-ask` is not "proceed despite uncertainty" — it is "honor the delegated
authority's decision verbatim, halt when no authority can decide."

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
PRE_AUTODEV_SHA=$(git rev-parse HEAD 2>/dev/null || echo "")
```

`PRE_AUTODEV_SHA` anchors the rollback hint emitted by Step 4.9 when the
terminal validation gate fails (v17 Phase 1).

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
   - `auto_executable` → `Skill(/spec --resync --path $C --no-ask)`; continue to next consumer on success.
   - otherwise → stop the chain; append C's verdict reason verbatim to the result block's `## Sync Results` section, mark status `halted`, and append the rollback hint `git revert HEAD`.
4. When the chain completes or halts, emit the `## Sync Results` section listing each consumer's outcome (`synced` or `halted: <reason>`).

No Decision enum interpretation. The consultant's own execution hint drives behavior.

When `--auto-sync` is not set, Step 4.7 is skipped; consumers remain listed in spec's Step 4.5 result block for the user to sync manually.

### Step 4.9: Validation Gate (v17 Phase 1)

Terminal gate between all document/code production and the success report. Runs
after Step 4 (dev) and, when `--auto-sync` is set, after the consumer
propagation chain in Step 4.7 completes. Not subject to `--no-ask` delegation —
this is an orchestrator step, not an authority decision.

```
Skill("claude-md-plugin:validate", args: "--strict --path {spec_target}")
```

- `success` → proceed to Step 5 success report.
- `failed`  → proceed to Step 5 failure report; **do NOT auto-revert**
  (INV-15: surface state to user verbatim; destructive rollback requires user
  consent).

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

**Failure at Validation Gate (Step 4.9):**

```
⚠ autodev terminated at Validation Gate
  reason: {validate failure summary, verbatim}
  Commits created during this run:
  $(git log --oneline ${PRE_AUTODEV_SHA}..HEAD 2>/dev/null)
  To roll back: git reset --hard ${PRE_AUTODEV_SHA}
```

The rollback hint enumerates the actual commit chain produced during this run
(spec auto-commit + one dev auto-commit per target, plus any `/spec --resync` commits
when `--auto-sync` is set) rather than a single `git revert HEAD`, because the
run typically produces ≥2 commits.

## Error Handling

| Situation | Response |
|-----------|----------|
| No requirement | AskUserQuestion once in Step 1 |
| spec failed | Report error, exit |
| spec cancelled by user | Report cancellation, exit |
| dev failed | Report warning, show partial results |
| validate gate failed (Step 4.9) | Report failure with commit chain + rollback hint; do not auto-revert |
