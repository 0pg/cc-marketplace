---
name: sync
version: 1.0.0
aliases: [sync-spec, update-constraints]
description: |
  This skill should be used when the user asks to "sync DEVELOPERS.md", "update constraints for changed requirements",
  "partial spec update", or uses "/sync".
  After PM/PO modifies CLAUDE.md Requirements, updates only the DEVELOPERS.md Constraints
  without running the full /spec workflow (Self Socratic Loop + plan + review).
  Preserves Technical Context, Decision Log, and Agent Observations.
  Trigger keywords: sync, update constraints, partial update
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

# /sync

Updates DEVELOPERS.md Constraints after CLAUDE.md Requirements change, skipping the full /spec workflow.

## Triggers

- `/sync`
- `sync DEVELOPERS.md`
- `update constraints`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `<path>` | No | auto-detect | Target module path |

## Comparison with /spec

| | /spec | /sync |
|---|-------|-------|
| Self Socratic Loop | Yes | No |
| Plan + review cycle | Yes | No |
| CLAUDE.md create/modify | Yes | No (already modified) |
| DEVELOPERS.md create/modify | Full generation | Constraints only |
| state.json | Yes | No |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Determine target

**Path specified:**
Target = the specified path.

**Path not specified:**
```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

Filter for modules with CLAUDE.md changes. If multiple, list and ask which to sync.

If no changes detected → "No CLAUDE.md changes detected." → exit.

### 2. Precondition check

Check DEVELOPERS.md existence in the target directory:

```
Read: {target_path}/DEVELOPERS.md
```

If absent → "DEVELOPERS.md not found at {path}. Run /spec first to generate both documents." → exit.

### 3. Change analysis

```bash
$CLI_PATH diff-node-history --path {path} --root {project_root} --limit 10 \
  --grep "^spec({path}):" \
  --output "${TMP_DIR}sync-history-${dir_safe}.json"
```

Parse `SectionChange.text` for `REQ-\d+:` patterns in Requirements sections.

If no Requirements changes detected → "No Requirements changes detected in {path}." → exit.

### 4. Backup preserved sections

Read existing DEVELOPERS.md and extract:

```
backup_technical_context = full content of ## Technical Context
backup_decision_log = full content of ## Decision Log
backup_agent_observations = full content of ## Agent Observations
```

Store each as a string variable (SKILL-level, not written to file).

### 5. Generate synthetic plan.md

Write `${TMP_DIR}spec-plan-${dir_safe}.md` in impl agent's expected format:

```markdown
# Spec Plan
target_path: {path}
action: update
round: 1

## Proposed Requirements
{Copy current CLAUDE.md Requirements section verbatim — all REQ-N entries}

## Proposed Constraints
{Copy current DEVELOPERS.md Constraints section verbatim}

## Rationale
- Sync: Requirements changed, Constraints need corresponding update
- Preserve unaffected Constraints verbatim
- Changed Requirements: {list of changed REQ-N identifiers from Step 3}
```

### 6. Create spec-execute session file

Write `${TMP_DIR}spec-execute-session-${dir_safe}.md`:

```markdown
# Spec Execute Session
type: spec-execute | mode: execute | project_root: {project_root}
target_path: {path}
action: update
document_language: {from project root Instructions, or ""}

## Approved Plan File
plan_file: ${TMP_DIR}spec-plan-${dir_safe}.md

## User Requirement
Sync: update DEVELOPERS.md Constraints for changed Requirements

## Existing Modules Index
{scan-claude-md result}

## Project Conventions
{Conventions from resolved hierarchy, or "None"}
```

### 7. Dispatch impl agent (mode=execute)

```
Task(impl):
  Session file: ${TMP_DIR}spec-execute-session-${dir_safe}.md
  Project root: {project_root}

  Read the session file and generate CLAUDE.md + DEVELOPERS.md.
```

Extract result: status, generated files.

### 8. Preserved section verification + auto-restore

Read the updated DEVELOPERS.md and compare preserved sections:

```
current_technical_context = ## Technical Context from updated file
current_decision_log = ## Decision Log from updated file
current_agent_observations = ## Agent Observations from updated file

Compare with backups from Step 4:
  if current_technical_context != backup_technical_context → RESTORE
  if current_decision_log != backup_decision_log → RESTORE
  if current_agent_observations != backup_agent_observations → RESTORE
```

On mismatch:
1. Replace the section in DEVELOPERS.md with the backup content (Edit tool)
2. Log warning: "Restored {section}: agent modified preserved section"

This is a deterministic safeguard — it does not depend on agent behavior.

### 9. Schema validation

```bash
$CLI_PATH validate-schema --file {developers_md_path} --strict
```

If validation fails → attempt auto-fix once, then report.

### 10. Auto-commit

```bash
git add {DEVELOPERS.md path}
git commit -m "sync({path}): update Constraints for changed Requirements

Changed Requirements: {list of changed REQ-N}
Constraints updated to match."
```

## DO / DON'T

**DO:**
- Always backup preserved sections before dispatching impl agent
- Always verify and auto-restore after impl agent returns
- Use synthetic plan.md to reuse impl agent without modification
- Clearly report which sections were restored (if any)

**DON'T:**
- Run Self Socratic Loop (that's /spec's job)
- Modify CLAUDE.md (already modified by PM/PO)
- Trust that impl agent preserved sections — always verify
- Create DEVELOPERS.md from scratch (require /spec for initial creation)

## Error Handling

| Situation | Response |
|-----------|----------|
| DEVELOPERS.md not found | "Run /spec first" → exit |
| No Requirements changes | "No changes detected" → exit |
| impl agent failure | Report error, no commit |
| Preserved section modified by agent | Auto-restore from backup + warning |
| Schema validation failure | Auto-fix attempt, then report |
