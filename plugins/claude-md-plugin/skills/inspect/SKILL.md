---
name: inspect
version: 1.1.0
aliases: [status, health, dashboard, overview, impl-review, spec-review, quality-review, consult, feasibility, po-consult]
description: |
  This skill should be used when the user asks to "check project status", "review spec quality",
  "check feasibility", "consult the PM/PO", "can we add X", "show dashboard", or uses any of
  `/inspect`, `/status`, `/impl-review`, `/consult`. Unified read-only inspection entry point
  covering project health (schema/pairing/drift/conventions), spec quality (CLI + 5-criteria review),
  and feasibility consultation (3-layer PM/PO judgment). Single calling convention replaces three
  legacy SKILLs.
  Trigger keywords: status, health, dashboard, spec quality, review, feasibility, consult, can we
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task]
---

# /inspect

Unified read-only inspection: project health, spec quality, or feasibility consultation.

## Triggers

- `/inspect` (or aliases `/status`, `/impl-review`, `/consult`)
- `project health`, `spec quality review`, `feasibility check`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `--focus` | No | `health` | `health` \| `quality` \| `feasibility` \| `all` |
| `--path` | No | `.` | Target module path (or project root for `health`) |
| `--all` | No | false | For `quality`: review every module. Ignored for other focuses. |
| `request` | Required for `feasibility` | — | Quoted request text for feasibility consultation |

`--focus all` (opt-in) runs `health` + `quality` sequentially. Default
`health` matches the alias semantics (`/status`, `/health`, `/dashboard`);
callers who want the heavier `quality` review must request it explicitly.
`feasibility` is never implicit because it requires a `request` argument.

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
project_root=$(pwd)
```

### 1. Dispatch by focus

Load only the reference file(s) matching `--focus`. This keeps per-invocation
context cost proportional to what was asked — `--focus health` does not load
the quality or feasibility playbooks, and vice versa.

```bash
case "${focus}" in
  health)
    cat "${CLAUDE_PLUGIN_ROOT}/references/inspect/health.md"
    ;;
  quality)
    cat "${CLAUDE_PLUGIN_ROOT}/references/inspect/quality.md"
    ;;
  feasibility)
    cat "${CLAUDE_PLUGIN_ROOT}/references/inspect/feasibility.md"
    ;;
  all)
    cat "${CLAUDE_PLUGIN_ROOT}/references/inspect/health.md"
    cat "${CLAUDE_PLUGIN_ROOT}/references/inspect/quality.md"
    ;;
esac
```

Follow the loaded reference(s) step by step. Each reference file owns its own
Report format and Failure modes table.

For `--focus all`, run Health first, then Quality; print each report block as
it completes.

## DO / DON'T

**DO:**
- Run all three focuses through the same entry point — one calling convention
- Show all modules regardless of individual failures (report inline, continue)
- Handle non-git repos gracefully (drift = N/A, in Health reference)

**DON'T:**
- Modify any files — read-only operation across all focuses
- Silently expand scope: default `health` matches the lightweight alias
  semantics; callers requesting `quality` or `all` must do so explicitly
- Silently skip feasibility when no request provided — error with guidance

## Error Handling

| Situation | Response |
|-----------|----------|
| No CLAUDE.md at path | `"No CLAUDE.md found at {path}."` → exit |
| `--focus feasibility` without request | Error with guidance → exit |
| Reference file missing | Surface raw error (indicates a broken install) |
| Per-focus failures | Defer to the `Failure modes` table in the loaded reference |
