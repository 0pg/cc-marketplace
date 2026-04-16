# /inspect --focus feasibility

3-layer PM/PO judgment via po-consultant. Requires `request` argument
(quoted string). `--focus feasibility` without a request → exit with
`"--focus feasibility requires a quoted request argument"`.

## F.1 Resolve target

- `--path` specified → that path
- default → current directory

If `{target}/CLAUDE.md` does not exist → exit `"No CLAUDE.md found at {target}."`

`dir_safe` = target path with `/` → `-`; root `.` → `root`.
`project_root` = nearest ancestor containing a CLAUDE.md with `## Instructions`.

## F.2 Collect 3 knowledge layers

**Layer [1] — Current Spec**: Read `{target}/CLAUDE.md` + `{target}/DEVELOPERS.md` (or `absent`).

**Layer [2] — Decision History**:

```bash
$CLI_PATH diff-node-history --path {target} --root {project_root} --limit 5 \
  --output "${TMP_DIR}inspect-history-{dir_safe}.json"
```

Filter DEVELOPERS.md `## Agent Observations` to types: `structural`,
`decision`, `improvement`.

**Layer [3] — Strategic Direction**: Extract `## Roadmap` from DEVELOPERS.md
(or `"Roadmap not defined"`).

## F.3 Session file (shared format with /spec Step 2.1c)

Write `${TMP_DIR}consult-session-{dir_safe}.md` in the unified po-consultant format:

```markdown
# Consult Session
type: consult | target: {target} | project_root: {project_root}
dir_safe: {dir_safe}

## Request
"{request}"

## [1] Current Spec

### CLAUDE.md
{full CLAUDE.md content}

### DEVELOPERS.md
{full DEVELOPERS.md content, or "absent"}

## [2] Decision History

### diff-node-history (limit 5)
{parsed JSON}

### Agent Observations (structural, decision, improvement only)
{filtered entries, or "None"}

## [3] Strategic Direction

### Roadmap
{roadmap_content}
```

**Note:** this session format is shared with `/spec` Step 2.1c. Single calling
convention for po-consultant across both SKILLs — update both when changing
the format.

## F.4 Dispatch po-consultant

```
Task(po-consultant):
  Session file: ${TMP_DIR}consult-session-{dir_safe}.md
  Save result to ${TMP_DIR} and return only the result block path
```

## F.5 Report

```
=== Feasibility: {target} ===

Request: "{request}"

Verdict: {feasible | partially_feasible | not_feasible}

Constraints:
  {CONST-N}: {conflict description}

History:
  [{date/since}] {prior attempt or decision}

Roadmap fit: {aligned | conflicts | neutral}
  "{explanation}"

Suggested path:
  Short: {achievable within current spec}
  Long:  {possible with Roadmap/spec changes}

Downstream:
  {action guidance from result file}
===
```

## Failure modes

| Situation | Response |
|-----------|----------|
| No CLAUDE.md at target | `"No CLAUDE.md found at {target}."` → exit |
| No `request` argument | `"--focus feasibility requires a quoted request argument"` → exit |
| DEVELOPERS.md absent | Note in session as `"absent"`; continue |
| diff-node-history CLI failure | Note `"History unavailable"`; continue |
| po-consultant failure | Surface raw error; show partial report |
