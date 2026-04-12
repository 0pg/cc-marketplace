---
name: consult
version: 1.0.0
aliases: [ask-po, feasibility, po-consult]
description: |
  This skill should be used when the user asks to "consult the PM/PO", "check feasibility",
  "can we add X", "is it possible to do Y", "what would it take to add Z", or uses "/consult".
  Combines current spec, decision history, and Roadmap to produce:
  verdict (feasible/partially_feasible/not_feasible) + constraints + history + roadmap_fit + suggested_path.
  Read-only — no file modifications.
  Trigger keywords: consult, feasibility, can we, is it possible, PM/PO judgment
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task]
---

# /consult

Consults a node's PM/PO about the feasibility of an abstract requirement or question.

## Triggers

- `/consult`
- `feasibility check`
- `can we add`
- `is it possible`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `<path>` | No | `.` | Target module path |
| `<request>` | Yes | — | Requirement or question (quoted string) |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Determine target

**Path specified:**
Target = specified path.

**Default:**
Target = current directory (`.`).

```bash
$CLI_PATH scan-claude-md --root {target} --output "${TMP_DIR}scan-result.json"
```

If no CLAUDE.md found → "No CLAUDE.md found at {path}. Specify a valid module path." → exit.

Set `dir_safe` = target path with `/` replaced by `-` (e.g., `src/auth` → `src-auth`).
Set `project_root` = nearest ancestor containing project-root CLAUDE.md (has `## Instructions`).

### 2. Collect knowledge layers

#### Layer [1] — Current Spec

```
Read: {target}/CLAUDE.md       → full content
Read: {target}/DEVELOPERS.md   → full content (or "absent" if not found)
```

If DEVELOPERS.md absent: note in session file as "DEVELOPERS.md absent — Constraints and Roadmap unavailable".

#### Layer [2] — Decision History

```bash
$CLI_PATH diff-node-history --path {target} --root {project_root} --limit 5 \
  --output "${TMP_DIR}consult-history-${dir_safe}.json"
```

Extract from DEVELOPERS.md `## Agent Observations` (if present):
  Filter types: `structural`, `decision`, `improvement` only (skip `tactical`, `preference`).

#### Layer [3] — Strategic Direction

Extract `## Roadmap` section from DEVELOPERS.md.
- If section absent: roadmap_content = "Roadmap not defined"
- If section present but content is `None`: roadmap_content = "Roadmap: None"
- Otherwise: roadmap_content = full section content

### 3. Create session file

Write `${TMP_DIR}consult-session-${dir_safe}.md`:

```markdown
# Consult Session
type: consult | target: {target} | project_root: {project_root}
dir_safe: {dir_safe}

## Request
"{request text}"

## [1] Current Spec

### CLAUDE.md
{full CLAUDE.md content}

### DEVELOPERS.md
{full DEVELOPERS.md content, or "absent"}

## [2] Decision History

### diff-node-history (limit 5)
{parsed JSON from consult-history-{dir_safe}.json — commits with section changes}

### Agent Observations (structural, decision, improvement only)
{filtered entries from ## Agent Observations, or "None"}

## [3] Strategic Direction

### Roadmap
{roadmap_content}
```

### 4. Dispatch po-consultant

```
Task(po-consultant):
  Session file: ${TMP_DIR}consult-session-${dir_safe}.md
  Save result to ${TMP_DIR} and return only the result block path
```

Extract: `verdict`, `result_file` from the result block.

### 5. Display result

Read `result_file` and display:

```
=== Consult: {target} ===

Request: "{request}"

Verdict: {feasible | partially_feasible | not_feasible}

Constraints:
  {CONST-N}: {conflict description}
  (or: "No conflicts found.")

History:
  [{date/since}] {prior attempt or decision}
  (or: "No prior attempts found.")

Roadmap fit: {aligned | conflicts | neutral}
  "{explanation}"
  (or: "Roadmap not defined — long-term fit unknown")

Suggested path:
  Short: {achievable within current spec}
  Long:  {possible with Roadmap/spec changes}

Downstream:
  {action guidance from result file}

===
```

## DO / DON'T

**DO:**
- Collect all three layers before dispatching po-consultant
- Clearly note when DEVELOPERS.md or Roadmap is absent (not an error, just context)
- Display the full structured result

**DON'T:**
- Modify any files — read-only operation
- Confirm Requirements — that is /spec's role
- Skip po-consultant dispatch even when the request seems straightforward

## Error Handling

| Situation | Response |
|-----------|----------|
| No CLAUDE.md at path | "No CLAUDE.md found at {path}. Specify a valid module path." → exit |
| DEVELOPERS.md absent | Note in session file as "DEVELOPERS.md absent — Constraints and Roadmap unavailable"; continue |
| diff-node-history failure | Note in session file as "History unavailable: CLI error"; continue with available layers |
| po-consultant agent failure | Report error, display partial context collected so far |
| Result file missing | "po-consultant did not produce a result. Check session file at ${TMP_DIR}consult-session-${dir_safe}.md" |
