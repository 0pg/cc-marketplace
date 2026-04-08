---
name: decompile
version: 1.0.0
aliases: [decom, extract, document]
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", "extract documentation from source",
  or uses "/decompile" or "/decom".
  Analyzes existing source code and creates CLAUDE.md + DEVELOPERS.md documentation for each directory.
  Uses parse-tree CLI for directory discovery, then runs decompiler agent per directory in leaf-first order.
  Trigger keywords: decompile, extract docs from code, document existing code
user_invocable: true
allowed-tools: [Bash, Read, Write, Glob, Task, AskUserQuestion]
---

# /decompile

Analyzes source code to extract CLAUDE.md + DEVELOPERS.md.

## Triggers

- `/decompile`
- `extract docs from code`
- `document existing code`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `path` | No | `.` | Target path |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 0.5. Check for `## Instructions` existence and read document language

If the project root CLAUDE.md does not have a `## Instructions` section:
```
⚠ Instructions section not configured. After decompile is complete, it is recommended to set it up with `/project-setup`.
```
Output the warning and continue with decompile. Creating Instructions is the role of `/project-setup`.

Read the `## Instructions` section from project root CLAUDE.md and extract the `Document language` value.
If not found, set `document_language` to empty (the agent will ask the user).

### 1. Directory tree parsing (inline)

Handle tree-parse via direct CLI invocation rather than as a separate skill:

```bash
$CLI_PATH parse-tree --root {path} --output .claude/extract-tree.json
```

### 2. Determine execution order

Sort the `needs_claude_md` array by depth DESC (leaf-first).

### 3. Create session files + run decompiler

For each directory in sorted order (leaf-first):

1. Generate child CLAUDE.md list (subdirectories that already have generated CLAUDE.md)
2. Read project conventions (if available)
3. Write session file → `${TMP_DIR}decompile-session-{dir-safe}.md`:

```markdown
# Decompile Task: {path}
type: decompile | target: {path}
document_language: {document_language or ""}

## Tree Info
source_file_count: {n}
subdir_count: {n}
depth: {n}

## Children CLAUDE.md
{List of already-generated child CLAUDE.md paths, or "None" if empty}

## Project Conventions
{project root Conventions or "None"}
```

4. Invoke `Task(decompiler)`:
```
Session file: ${TMP_DIR}decompile-session-{dir-safe}.md
Target: {path}
Save results to ${TMP_DIR} and return only the path
```

Independent directories at the same depth can be executed in parallel (up to 3).

Check decompiler result status:
- `success`: proceed to next module
- `failed_with_warnings`: collect warnings, proceed to next module

### 4. Display changes

```bash
git diff --stat
```

### 5. Result

```
---decompile-result---
status: success | partial | failed
total: {n}
generated: {n}
failed: {n}
warnings: {Instructions not configured, etc. — omit if none}
---end-decompile-result---
```

## DO / DON'T

**DO:**
- Follow leaf-first order
- Include child CLAUDE.md list in session files
- Delegate to decompiler agent via session files

**DON'T:**
- Modify code (this is an extraction task)
- Copy source code directly into generated CLAUDE.md
- Pass the entire raw tree.json to the decompiler agent (include only necessary info in session files)

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| Directory with no source files | skip |
| decompiler agent failure (single module) | warn, continue with the rest |
