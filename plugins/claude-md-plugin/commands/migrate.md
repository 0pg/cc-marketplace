---
name: migrate
description: |
  Converges an existing project to the current plugin schema.
  Uses fix-schema CLI's converge_schema to deterministically handle section rename/remove/add.
  No source version detection needed — target-state-driven migration.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Edit, Write, AskUserQuestion]
---

# /migrate

Migrates an existing project to match the current plugin schema.
Does not detect the source version; instead converges to the current schema as the target state.

## Triggers

- `/migrate`
- `migration`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `project_root_path` | No | `.` | Project root path |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. File Collection

```
Glob("**/CLAUDE.md", path={project_root_path})
Glob("**/DEVELOPERS.md", path={project_root_path})
```

Exit if no CLAUDE.md found.

### 2. Dry-run (Schema Convergence Analysis)

For each file:

```bash
# CLAUDE.md
$CLI_PATH fix-schema --file "$claude_md" --type claude_md --dry-run

# DEVELOPERS.md (if exists)
$CLI_PATH fix-schema --file "$developers_md" --type developers_md --dry-run
```

Collect change details (renames, removals, additions) and warnings (conflicts).

### 3. Legacy File Detection

```
Glob("**/IMPLEMENTS.md", path={project_root_path})
Glob(".claude/index.md", path={project_root_path})
Glob("**/dev-context.md", path={project_root_path})
Glob(".claude/tmp/*/bugfix-analysis-*.md", path={project_root_path})
Glob(".claude/tmp/*/dev-session-*.md", path={project_root_path})
Glob(".claude/tmp/*/validate-session-*.md", path={project_root_path})
Glob(".claude/tmp/*/spec-session.md", path={project_root_path})
Glob(".claude/tmp/*/dedev-session-*.md", path={project_root_path})
```

Collect list of files to delete.

### 4. Content Migration Check (Operations / Public API)

For each DEVELOPERS.md where dry-run reports `"removed: ## Operations"` or `"removed: ## Public API"`:

1. Read the section content
2. If content is "None" or empty → skip (converge handles deletion automatically)
3. If **non-None content exists** → present migration options via AskUserQuestion:
   - **(a) Auto-migrate**:
     - Operations > environment variables / Configuration → append to `## Constraints`
     - Operations > gotchas / procedures → append to `## Decision Log` (ADR format: Context/Decision/Rationale)
     - Public API entries → append to `## Constraints` as export contract (e.g., `CONST-N: {symbol} must be publicly exported for {consumer} consumption`)
   - **(b) Manual**: User migrates content themselves before converge runs
   - **(c) Delete**: Proceed with converge (content will be lost)
4. If (a): execute migration via Edit tool before converge
5. If (b): pause and wait for user to complete manual migration

### 5. Display Plan + One-time Approval

If no changes: "No migration needed" → exit.

If changes exist, display the plan:
- **Schema conversion**: rename/remove/add details (per file)
- **Content migration**: migration actions from Step 4 (if any)
- **File cleanup**: list of legacy files to delete
- **Conflict warnings**: rename cases where both exist (manual resolution required)

Request one-time approval via AskUserQuestion.

### 6. Execution

```bash
# Schema convergence
$CLI_PATH fix-schema --file "$claude_md" --type claude_md
$CLI_PATH fix-schema --file "$developers_md" --type developers_md

# Delete legacy files
git rm "$legacy_file" 2>/dev/null || rm "$legacy_file"
```

### 7. Conflict Resolution (if needed)

For files that had conflict warnings in the dry-run:
- AskUserQuestion: "Both ## {from} and ## {to} exist. (a) Manual merge (b) Regenerate with /decompile"
- Process according to user's choice

### 8. Verification

```bash
$CLI_PATH validate-schema --file "$claude_md" --strict
$CLI_PATH validate-convention --project-root {project_root}
```

If verification fails: suggest "Recommend regenerating with /decompile {path}".

### 9. Result Report

```bash
git diff --stat -- "**/CLAUDE.md" "**/DEVELOPERS.md"
```

Migration results + follow-up action guidance:
- If Conventions missing → suggest `/project-setup`
- If Instructions missing → suggest `/project-setup`

## DO / DON'T

**DO:**
- Perform dry-run first, display plan → one-time approval
- Delegate to fix-schema CLI (deterministic convergence)
- Request user judgment on conflicts

**DON'T:**
- Delete files without user approval
- Request individual approval per file
- Write source version detection logic (fix-schema converges to target-state)
