---
name: impl-review
version: 1.0.0
aliases: [spec-review, review-spec, quality-review]
description: |
  This skill should be used when the user asks to "review CLAUDE.md quality", "check spec quality",
  "review requirements", "review constraints coverage", or uses "/impl-review".
  Combines deterministic CLI validation with semantic quality review by spec-quality-reviewer agent.
  Evaluates Purpose clarity, Requirements measurability, Constraints precision, and coverage.
  Trigger keywords: spec review, quality review, requirements review
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Task]
---

# /impl-review

Reviews the quality of CLAUDE.md + DEVELOPERS.md specifications.

## Triggers

- `/impl-review`
- `spec review`
- `quality review`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `<path>` | No | `.` | Target module path |
| `--all` | No | false | Review all modules |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Determine targets

**Path specified:**
Target = the specified module.

**`--all` specified:**
```bash
$CLI_PATH scan-claude-md --root {project_root}
```
All modules in the index = targets.

**Default:**
Target = current directory (`.`).

### 2. Deterministic validation

For each target:

```bash
# Schema validation
$CLI_PATH validate-schema --file {claude_md_path} --dir {dir}

# Convention validation (if project/module root)
$CLI_PATH validate-convention --project-root {project_root}

# Language validation (if configured)
$CLI_PATH validate-language --file {claude_md_path} --project-root {project_root}
```

Collect all CLI results.

### 3. Create review session file

For each target, write `${TMP_DIR}impl-review-session-${dir_safe}.md`:

```markdown
# Impl Review Session
type: impl-review | target: {path} | project_root: {project_root}
dir_safe: {dir_safe}

## CLAUDE.md Content
{full CLAUDE.md content}

## DEVELOPERS.md Content
{full DEVELOPERS.md content, or "absent"}

## Deterministic Results
### Schema Validation
{validate-schema output}

### Convention Validation
{validate-convention output, or "N/A"}

### Language Validation
{validate-language output, or "N/A"}
```

### 4. Dispatch spec-quality-reviewer

```
Task(spec-quality-reviewer):
  Session file: ${TMP_DIR}impl-review-session-${dir_safe}.md
  Save results to ${TMP_DIR} and return only the path
```

For `--all` with multiple targets, dispatch in parallel without a fixed cap.

Extract verdict: `pass` | `needs_improvement`.

### 5. Display review report

```
=== Spec Quality Review: {path} ===

Deterministic:
  Schema:      {pass | FAIL ({details})}
  Convention:  {pass | FAIL | N/A}
  Language:    {pass | FAIL | N/A}

Semantic:
  Purpose clarity:           {pass | WARN: {reason}}
  Requirements measurability: {pass | ERROR: {reason}}
  REQ → CONST coverage:     {pass | WARN: {uncovered REQ-N list}}
  Constraints precision:     {pass | ERROR: {reason}}
  Domain Context sufficiency: {pass | INFO: {reason}}

Verdict: {pass | needs_improvement}
===
```

When `--all`, show a summary table at the end:

```
=== Summary ===
  {path:<30}  {pass | needs_improvement}
  {path:<30}  {pass | needs_improvement}
  ...
Total: {N} reviewed, {M} pass, {K} needs improvement
===
```

## DO / DON'T

**DO:**
- Run deterministic CLI validation before semantic review
- Include both CLI results and document content in session file
- Show clear severity levels (ERROR, WARN, INFO)

**DON'T:**
- Modify any files — read-only operation
- Assign numeric scores — binary verdict only (pass / needs_improvement)
- Block on needs_improvement — this is advisory, not a gate
