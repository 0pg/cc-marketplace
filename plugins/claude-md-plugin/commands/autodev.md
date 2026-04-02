---
name: autodev
description: |
  Use when the user wants to autonomously develop a feature end-to-end without manual steps.
  Runs requirements → CLAUDE.md → code → validation loop until complete.
  Autonomous execution from start to finish given only requirements, without step-by-step commands.
  Trigger keywords: auto develop, end-to-end, autonomous implementation
argument-hint: '"requirement" [--path path] [--max-iter N]'
allowed-tools: [Read, Glob, Write, Task, AskUserQuestion, Bash, Skill]
---

# /autodev

Autonomously executes requirements from start to finish.
Completes the entire loop of spec definition (spec) → code generation (dev) → validation (validate) autonomously.

**Completes without human intervention, except for one initial requirement confirmation.**

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
| `--max-iter` | No | `5` | Maximum number of dev-validate cycles |

\* If no requirement is provided, it will be collected once via AskUserQuestion.

## Difference from /spec --auto

| Item | /spec --auto | /autodev |
|------|-------------|----------|
| Requirement confirmation | brainstorming + up to 2 questions | Up to 1 question |
| Mode | single=brainstorming, multi=approval | Always autonomous (parallel) |
| max_iter default | 3 | 5 |
| Usage | `/spec --auto "..."` | `/autodev "..."` |

> **Note:** dev automatically detects the language from source file extensions.
> In new projects without source files, autonomous execution may be interrupted if dev language detection fails.
> For empty projects, add a file indicating the language
> (package.json, go.mod, Cargo.toml, etc.) before running `/autodev`, or run `/dev` first.

## Workflow

### Step 1: Requirement Confirmation (up to 1 time)

If requirement text is provided, proceed directly to Step 2.

If missing or too vague, use AskUserQuestion once:
- "What feature would you like to implement? Please briefly describe the core behavior and target path."

After this, **all steps are autonomous — AskUserQuestion is prohibited.**

### Step 2: Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

Preserve the following values:
- `{original_requirement}`: requirement text
- `{impl_path}`: `--path` argument (default `.`)
- `{max_iter}`: `--max-iter` argument (default `5`)

### Step 3: Generate CLAUDE.md Index

```bash
$CLI_PATH scan-claude-md --root {impl_path} --output "${TMP_DIR}claude-md-index.json"
```

### Step 4: Decompose (Automatic Scope Determination)

Create `${TMP_DIR}decompose-session.md`:

```markdown
# Decompose Session
type: decompose | project_root: {impl_path}

## User Requirement
{original_requirement}

## Existing Modules Index
{scan-claude-md result: path, purpose pairs}

## Project Conventions
{project root Conventions or "None"}
```

```
Task(decompose):
  Session file: ${TMP_DIR}decompose-session.md
  Save results to ${TMP_DIR} and return only the path
```

Check `scope` and `modules[]` from the decompose result.

### Step 5: Spec (Spec Definition) — AskUserQuestion Prohibited

Run all impl agents in **parallel mode** (regardless of single/multi).

#### scope = single

Check if `{impl_path}/CLAUDE.md` exists in the scan-claude-md index:
- Exists → `action: update`
- Does not exist → `action: create`

Create `${TMP_DIR}spec-session.md`:

```markdown
# Spec Session
type: spec | project_root: {impl_path} | target_path: {impl_path} | action: {action} | parallel: true

## User Requirement
{original_requirement}

## Existing Modules Index
{scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

```
Task(impl):
  Session file: ${TMP_DIR}spec-session.md
  Project root: {impl_path}
  Read the session file and generate CLAUDE.md + DEVELOPERS.md.
```

#### scope = multi

Sort `modules[]` by depth ASC.

Depth loop (0, 1, 2, ... order):
1. Re-run scan-claude-md → latest index
2. Create session files for each module at the current depth (`${TMP_DIR}spec-session-{dir-safe}.md`, `parallel: true`):

   ```markdown
   # Spec Session
   type: spec | project_root: {impl_path} | target_path: {module.path} | action: {module.action} | parallel: true

   ## User Requirement
   {module.requirement_refs}

   ## Purpose Hint
   {module.purpose_hint}

   ## Source Concept
   {module.source_concept}

   ## Existing Modules Index
   {latest scan-claude-md result}

   ## Project Conventions
   {project root Conventions or "None"}
   ```

3. Dispatch Task(impl) in parallel (up to 3)
4. Wait for completion → next depth

### Step 6: Auto Loop

`auto_iter = 0`

#### Auto Phase 1: Dev

```
Skill("claude-md-plugin:dev", args: "--conflict overwrite --path {impl_path}")
```

`failed` → Exit loop, report error.
`success | partial` → Proceed to Auto Phase 2.

#### Auto Phase 2: Validate

```
Skill("claude-md-plugin:validate", args: "{impl_path} --report-only")
```

```
total_violations = schema_errors + convention_issues + boundary_issues + semantic_drift
```

- `total_violations == 0` → **Success exit → Step 7**
- `auto_iter >= max_iter` → **max_iter exit → Step 7**
- Otherwise → **Extract violation details → Auto Phase 3**

**Violation Detail Extraction:**

Check the `result_files` list from validate-result:

**Case A: No `result_files` (all modules failed schema validation)**
→ Exit loop:
  "Cannot perform semantic verification due to schema errors. Resolve manually with /validate and retry."

**Case B: `result_files` exist and all files have issues=0 (CLI issues only)**
→ Use the `## Deterministic Results` section from `${TMP_DIR}validate-session-{dir-safe}.md` as Violations Detail and proceed to Phase 3
  (convention/boundary issues can be fixed by updating CLAUDE.md via spec update)

**Case C: `result_files` exist and some have issues > 0 (semantic drift exists)**
→ For each result_file where `## Summary: Total issues: N > 0` → target module for spec update
   `## Issues` section → collect per-module violation details → proceed to Phase 3

#### Auto Phase 3: Spec Update

```bash
$CLI_PATH scan-claude-md --root {impl_path} --output "${TMP_DIR}claude-md-index-auto-{auto_iter}.json"
```

Create session files per violation module:
`${TMP_DIR}spec-session-auto-{auto_iter}-{dir-safe}.md`

```markdown
# Spec Session (Auto Mode)
type: spec | project_root: {impl_path} | target_path: {path} | action: update | parallel: true

## User Requirement
{original_requirement}

## Auto-Fix Context
auto_iteration: {auto_iter}
validate_violations:
  schema_errors: {n}
  convention_issues: {n}
  boundary_issues: {n}
  semantic_drift: {n}

Validation violations were found in this module.
Read the existing CLAUDE.md and DEVELOPERS.md, and refine/supplement
Requirements and Constraints so that validate passes after dev.
Since CLAUDE.md is the SSOT, improve by describing requirements more clearly.

## Violations Detail
{whichever applies:}
  - When semantic drift exists: content of ## Issues section from result_file
  - When only CLI issues exist: content of ## Deterministic Results section from validate-session-{dir-safe}.md

## Existing Modules Index
{latest scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

Dispatch Task(impl) in parallel (up to 3). AskUserQuestion prohibited.

`auto_iter++` → Loop back to Auto Phase 1.

### Step 7: Result Report

**Success exit (`total_violations == 0`):**

```
✓ autodev complete ({auto_iter} iteration(s))
  spec:     CLAUDE.md + DEVELOPERS.md generated
  dev:      Code generation complete
  validate: All validations passed
```

```bash
git diff --stat
```

**Failure exit (max_iter reached | dev failed | schema error):**

```
⚠ autodev terminated (reason: {reason})
  Iterations: {auto_iter}/{max_iter}
  Remaining issues: schema_errors={n}, convention={n}, boundary={n}, semantic_drift={n}
  Resolve manually with /validate or /spec.
```

```bash
git diff --stat
```

## Error Handling

| Situation | Response |
|-----------|----------|
| No requirement | AskUserQuestion once in Step 1 |
| Decompose failed | Report error and exit |
| impl agent failed (single module) | Warning, continue with remaining |
| dev failed | Exit loop, report error |
| No result_files (all modules failed schema) | Exit loop, guide to manual resolution with /validate |
| result_files exist and all issues=0 (CLI issues only) | Run Phase 3 with validate-session Deterministic Results |
| max_iter exceeded | Exit loop, report remaining issues |
