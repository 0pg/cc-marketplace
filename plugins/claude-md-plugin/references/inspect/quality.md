# /inspect --focus quality

Deterministic CLI validation + semantic 5-criteria review per module.

## Q.1 Determine targets

- `--all` → every module from `scan-claude-md`
- `--path` specified → that single module
- default → current directory

## Q.2 Deterministic validation per target

```bash
$CLI_PATH validate-schema --file {claude_md_path} --dir {dir}
$CLI_PATH validate-convention --project-root {project_root}   # if project/module root
$CLI_PATH validate-language --file {claude_md_path} --project-root {project_root}   # if Document language configured
```

## Q.3 Review session file + dispatch

For each target, write `${TMP_DIR}inspect-quality-session-{dir_safe}.md`:

```markdown
# Inspect Quality Session
type: inspect-quality | target: {path} | project_root: {project_root}
dir_safe: {dir_safe}

## CLAUDE.md Content
{full CLAUDE.md content}

## DEVELOPERS.md Content
{full DEVELOPERS.md content, or "absent"}

## Deterministic Results
### Schema
{validate-schema output}

### Convention
{validate-convention output, or "N/A"}

### Language
{validate-language output, or "N/A"}
```

Dispatch:

```
Task(spec-quality-reviewer):
  Session file: ${TMP_DIR}inspect-quality-session-{dir_safe}.md
  Save results to ${TMP_DIR} and return only the path
```

For `--all`, dispatch all targets in parallel (single batch).

## Q.4 Report

```
=== Spec Quality: {path} ===

Deterministic:
  Schema:      {pass | FAIL ({details})}
  Convention:  {pass | FAIL | N/A}
  Language:    {pass | FAIL | N/A}

Semantic:
  Purpose clarity:            {pass | WARN: {reason}}
  Requirements measurability: {pass | ERROR: {reason}}
  REQ → CONST coverage:       {pass | WARN: {uncovered REQ-N list}}
  Constraints precision:      {pass | ERROR: {reason}}
  Domain Context sufficiency: {pass | INFO: {reason}}

Verdict: {pass | needs_improvement}
===
```

For `--all`, append a summary table:

```
=== Summary ===
  {path}  {pass | needs_improvement}
  ...
Total: {N} reviewed, {M} pass, {K} needs improvement
===
```

## Failure modes

| Situation | Response |
|-----------|----------|
| No CLAUDE.md at target | `"No CLAUDE.md found at {path}."` → exit |
| Individual CLI validation error | Report inline in Deterministic block; continue to reviewer |
| spec-quality-reviewer failure | Surface raw error; show Deterministic section only |
