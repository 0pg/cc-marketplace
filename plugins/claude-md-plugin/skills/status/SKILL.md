---
name: status
version: 1.0.0
aliases: [health, dashboard, overview]
description: |
  This skill should be used when the user asks to "check project status", "show project health",
  "overview of CLAUDE.md files", "show spec coverage", "dashboard", or uses "/status".
  Aggregates schema validation, DEVELOPERS.md pairing, drift detection, and convention completeness
  into a single project health report. Pure CLI aggregation, no agent dispatch.
  Trigger keywords: project status, health check, spec coverage, dashboard
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep]
---

# /status

Displays a project health dashboard: module count, schema validity, DEVELOPERS.md pairing, drift, and convention completeness.

## Triggers

- `/status`
- `project health`
- `spec coverage`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `--path` | No | `.` | Project root path |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
```

### 1. Scan modules

```bash
$CLI_PATH scan-claude-md --root {project_root}
```

Parse output to build module index: list of directories containing CLAUDE.md.

### 2. Schema validation per module

For each module in the index:

```bash
$CLI_PATH validate-schema --file {claude_md_path} --dir {dir}
```

Record pass/fail per module.

### 3. DEVELOPERS.md pairing check (INV-3)

For each module, check if DEVELOPERS.md exists in the same directory as CLAUDE.md:

```
Read or Glob: {dir}/DEVELOPERS.md
```

Record paired/unpaired per module.

### 4. Drift detection

```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

Result branching:
- Not a git repository → Drift section shows "N/A (not a git repository)"
- No changes → all modules "up-to-date"
- Changes found → map each changed path to module, record drift status

### 5. Convention completeness

```bash
$CLI_PATH validate-convention --project-root {project_root}
```

Record: complete / incomplete (with missing subsections if any).

### 6. Aggregate and display report

```
=== Project Status: {project_root} ===

Modules:           {N} total

Schema Health:     {pass}/{total} valid ({percentage}%)
DEVELOPERS.md:     {paired}/{total} paired ({percentage}%)
Drift:             {summary}
Conventions:       {complete | incomplete ({details})}

Module Details:
  {path:<20}  schema:{pass|FAIL}  dev-md:{yes|no}  drift:{up-to-date|spec-newer|dev-pending}
  {path:<20}  schema:{pass|FAIL}  dev-md:{yes|no}  drift:{up-to-date|spec-newer|dev-pending}
  ...

===
```

Column definitions:
- `schema`: validate-schema result for the module's CLAUDE.md
- `dev-md`: DEVELOPERS.md existence in the same directory
- `drift`: from diff-compile-targets — `up-to-date` (no changes), `spec-newer` (CLAUDE.md changed since last dev), `dev-pending` (needs /dev run)

## DO / DON'T

**DO:**
- Run all CLI commands and aggregate results
- Show all modules regardless of status
- Handle non-git repos gracefully (drift = N/A)

**DON'T:**
- Dispatch any agents — this is pure CLI aggregation
- Modify any files — read-only operation
- Fail on individual module errors — report them inline and continue
