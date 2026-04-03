---
name: spec
version: 1.2.0
aliases: [define, requirements, impl]
description: |
  This skill should be used when the user asks to "define requirements", "write spec",
  "create CLAUDE.md from requirements", "define behavior before coding", or uses "/spec".
  Analyzes natural language requirements and generates CLAUDE.md without implementing code.
  Follows ATDD principle: specification first, then code generation via /dev.
  Trigger keywords: define requirements, write spec, spec first
user_invocable: true
allowed-tools: [Read, Glob, Write, Task, AskUserQuestion, Bash, Skill]
---

# /spec

Analyzes requirements (natural language or User Story) to generate/update **CLAUDE.md + DEVELOPERS.md**.
Performs requirement definition only **without code implementation**, following the "spec first" principle.

## Triggers

- `/spec`
- `define requirements`
- `write spec`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `requirement` | Yes | - | Requirement text |
| `--path` | No | `.` | Target path |
| `--auto` | No | false | Run spec→dev→validate autonomous loop |
| `--auto-max-iter` | No | `3` | Maximum spec update retry count. After N attempts, includes one final dev+validate for a total of N+1 verifications |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Generate existing CLAUDE.md index

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index.json"
```

### 2. Read project conventions and document language

Read the `## Conventions` section from project root CLAUDE.md if present.

Read the `## Instructions` section from project root CLAUDE.md and extract the `Document language` value.
If not found, set `document_language` to empty (the agent will ask the user).

### 3. Create Decompose session file

`${TMP_DIR}decompose-session.md`:

```markdown
# Decompose Session
type: decompose | project_root: {project_root}

## User Requirement
{user requirement text}

## Existing Modules Index
{scan-claude-md result: path, purpose pairs}

## Project Conventions
{project root Conventions or "None"}
```

### 4. Dispatch Decompose agent

```
Task(decompose):
  Session file: ${TMP_DIR}decompose-session.md
  Save results to ${TMP_DIR} and return only the path
```

### 5. Read Decompose result

Extract the `result_file` path from the decompose result block and Read.

Identify `scope`, `modules[]`, `unassigned[]`, `ambiguous[]` from the result JSON.

If `unassigned` items exist, notify the user:
```
⚠ The following requirements were not assigned to any module:
  - {unassigned items}
After impl is complete, add them manually or re-run /spec.
```

### 6. Scope branching

#### scope = single

**6a. Create Plan session file**

`${TMP_DIR}spec-plan-session-{dir-safe}.md`:

```markdown
# Spec Plan Session
type: spec-plan | mode: plan | round: 1 | project_root: {project_root}
target_path: TBD
action: TBD
document_language: {document_language or ""}

## User Requirement
{full user requirement text}

## Existing Modules Index
{scan-claude-md result: path, purpose pairs}

## Project Conventions
{project root Conventions or "None"}
```

**6b. Dispatch Task(impl, mode=plan)**

```
Task(impl):
  Session file: ${TMP_DIR}spec-plan-session-{dir-safe}.md
  Project root: {project_root}

  Read the session file and generate an execution plan (plan.md) in mode=plan.
```

Extract `plan_file`, `target_path`, `action`, `dir-safe` from the result block.

> `dir_safe`: replace slashes with hyphens in target_path (e.g., `src/auth` → `src-auth`)

**6b-1. Initialize workflow state**

```bash
WORKFLOW_DIR=".claude/workflows/{dir-safe}"
mkdir -p "$WORKFLOW_DIR"
cp "{plan_file}" "$WORKFLOW_DIR/spec-plan.md"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
cat > "$WORKFLOW_DIR/state.json" << 'STATEOF'
{
  "workflow_id": "{dir-safe}-TIMESTAMP_PLACEHOLDER",
  "target_path": "{target_path}",
  "dir_safe": "{dir-safe}",
  "action": "{action}",
  "status": "awaiting-review",
  "round": 1,
  "plan_file": ".claude/workflows/{dir-safe}/spec-plan.md",
  "last_reviewer_result": "",
  "project_root": "{project_root}",
  "user_requirement": "{first 500 chars of user requirement text — escape JSON special chars (\" \\ newlines)}",
  "created_at": "TIMESTAMP_PLACEHOLDER",
  "updated_at": "TIMESTAMP_PLACEHOLDER"
}
STATEOF
# Replace TIMESTAMP_PLACEHOLDER with actual timestamp
sed -i '' "s/TIMESTAMP_PLACEHOLDER/$TIMESTAMP/g" "$WORKFLOW_DIR/state.json"
```

**6c. Socratic Loop**

`round = 1`, `max_safety = 5`

```
loop:
  1. Create Reviewer session file:
     ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md:
       # Spec Reviewer Session
       type: spec-reviewer | round: {round}
       plan_file: {plan_file}
       dir_safe: {dir-safe}

  2. Dispatch Task(impl-reviewer):
       Session file: ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md
       Save results to ${TMP_DIR} and return only the path

     Extract verdict from result block.

     2-1. Promote artifact + update state.json (reflecting verdict):
     ```bash
     cp "${TMP_DIR}spec-reviewer-result-{dir-safe}-v{round}.md" \
        ".claude/workflows/{dir-safe}/reviewer-v{round}.md"
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'approved' if '{verdict}' == 'approved' else 'awaiting-revise'
     s['last_reviewer_result'] = '.claude/workflows/{dir-safe}/reviewer-v{round}.md'
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```

  3. if verdict == "approved":
       break

  4. if round >= max_safety:
       ⚠ Socratic loop terminated after {max_safety} iterations.
         Proceeding with the best available plan.
     ```bash
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'max-safety-exceeded'
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```
       break

  5. Create Revise session file:
     ${TMP_DIR}spec-plan-session-{dir-safe}.md (overwrite):
       # Spec Plan Session
       type: spec-plan | mode: revise | round: {round+1} | project_root: {project_root}
       target_path: {target_path}
       action: {action}
       document_language: {document_language or ""}

       ## User Requirement
       {full user requirement text}

       ## Reviewer Feedback File
       feedback_file: ${TMP_DIR}spec-reviewer-result-{dir-safe}-v{round}.md

       ## Existing Plan File
       existing_plan_file: {plan_file}

       ## Existing Modules Index
       {scan-claude-md result}

       ## Project Conventions
       {project root Conventions or "None"}

  6. Dispatch Task(impl, mode=revise):
       Session file: ${TMP_DIR}spec-plan-session-{dir-safe}.md
       Project root: {project_root}

       Read the session file and improve the execution plan in mode=revise.

     Verify plan_file update from result block.

     6-1. Promote revise artifact + update state.json:
     ```bash
     cp "${TMP_DIR}spec-plan-{dir-safe}.md" ".claude/workflows/{dir-safe}/spec-plan.md"
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'awaiting-review'
     s['round'] = {round} + 1
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```

  7. round++
  → return to 1
```

**6d. Create Execute session file**

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-exec.json"
```

`${TMP_DIR}spec-execute-session-{dir-safe}.md`:

```markdown
# Spec Execute Session
type: spec-execute | mode: execute | project_root: {project_root}
target_path: {target_path}
action: {action}
document_language: {document_language or ""}

## Approved Plan File
plan_file: {plan_file}

## User Requirement
{full user requirement text}

## Existing Modules Index
{latest scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

**6e. Dispatch Task(impl, mode=execute)**

```
Task(impl):
  Session file: ${TMP_DIR}spec-execute-session-{dir-safe}.md
  Project root: {project_root}

  Read the session file and generate CLAUDE.md + DEVELOPERS.md in mode=execute.
```

**6e-1. Update state + auto-commit after Execute completion**

**Commit message construction:**

The SKILL executor constructs the commit message after Execute completion following these rules:

1. **summary**: one-line summary based on Purpose and Requirements from the CLAUDE.md generated by impl agent
2. **[BREAKING]** (optional): include only when Requirements are deleted or there is a major direction change
3. **Transition context**: 1-2 sentences
   - `create` action: "New module creation" + Purpose summary
   - `update` action: describe transition direction based on Requirements changes from `git diff HEAD -- {target_path}/CLAUDE.md`
   - Good example: "Introducing OAuth2 as an additional authentication path alongside session-based auth. Maintaining sessions for legacy client support."
   - Bad example: "Update authentication system" (no directionality)
4. **Changes**: derived from `git diff HEAD -- {target_path}/CLAUDE.md {target_path}/DEVELOPERS.md`
   - added: newly added Requirements/Constraints items
   - modified: changed Requirements/Constraints items
   - removed: deleted Requirements/Constraints items
   - omit categories that don't apply

```bash
python3 -c "
import json
from datetime import datetime, timezone
with open('.claude/workflows/{dir-safe}/state.json') as f:
    s = json.load(f)
s['status'] = 'executed'
s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
    json.dump(s, f, indent=2, ensure_ascii=False)
"

# Commit only CLAUDE.md + DEVELOPERS.md (exclude TMP files and workflow state)
git add "{target_path}/CLAUDE.md" "{target_path}/DEVELOPERS.md"
git commit -m "spec({target_path}): [BREAKING] {summary}

{transition context — from where to where, why this change, 1-2 sentences}

Changes:
- added: {list of added Requirements/Constraints}
- modified: {list of modified Requirements/Constraints}
- removed: {list of removed Requirements/Constraints}"
```

#### scope = multi

**6a. User approval**

Present the decomposition plan via AskUserQuestion and request approval:

```
Decomposition plan:
  • {path} ({action}) — {purpose_hint}
  • {path} ({action}) — {purpose_hint}
  ...

{if ambiguous items exist}
⚠ Ambiguous decisions:
  - {ambiguous items}

Proceed with this plan? (Let me know if modifications are needed)
```

Handling varies by modification type (max 1 loop):

| Modification type | Handling approach |
|-------------------|-------------------|
| Path change, purpose_hint edit, module add/delete | SKILL directly edits `decompose-result.json` |
| Requirement redistribution, module merge/split | Add `## User Modification` section to `decompose-session.md` and re-invoke Task(decompose) |

Section to add to session file on re-invocation:
```markdown
## User Modification
{user's modification request content}
```
The decompose agent reads this section and re-executes in a direction that modifies the previous decomposition result.

On cancel: exit.

**6b. root-first sorting**

Sort `modules[]` by `depth` ASC. At the same depth, prioritize those without `depends_on`.

**6c+6d. Depth loop — execute session file creation and dispatch per depth**

Process each depth in order. **Session file creation is performed just before each depth** so that
impl results (CLAUDE.md) from previous depths are reflected in the index.

```
for depth in sorted_depths:  # 0, 1, 2, ...

  1. Create session files for modules at current depth:
     Re-run scan-claude-md to get the latest index
     (includes CLAUDE.md generated by previous depth impl)

     Create ${TMP_DIR}spec-session-{dir-safe}.md for each module:

     ---
     # Spec Session
     type: spec | project_root: {project_root} | target_path: {module.path} | action: {module.action} | parallel: true
     document_language: {document_language or ""}

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
     ---

  2. Current depth modules: Create Plan session files + dispatch Task(impl, mode=plan) in parallel (up to 3)

     Create `${TMP_DIR}spec-plan-session-{dir-safe}.md` for each module:
     ```
     # Spec Plan Session
     type: spec-plan | mode: plan | round: 1 | project_root: {project_root} | parallel: true
     target_path: {module.path}
     action: {module.action}
     document_language: {document_language or ""}

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

     Dispatch Task(impl, mode=plan) in parallel:
     ```
     Task(impl) — ${TMP_DIR}spec-plan-session-{dir-safe-A}.md
     Task(impl) — ${TMP_DIR}spec-plan-session-{dir-safe-B}.md  (if exists)
     Task(impl) — ${TMP_DIR}spec-plan-session-{dir-safe-C}.md  (if exists)
     ```

     Instructions for each Task:
       Session file: ${TMP_DIR}spec-plan-session-{dir-safe}.md
       Project root: {project_root}
       Read the session file and generate an execution plan (plan.md) in mode=plan.
       (parallel mode — AskUserQuestion prohibited)

  3. Socratic Loop per module (sequential per module, round=1, max_safety=5):

     Execute the following sequentially for each module:

     ```
     loop:
       a. Create Reviewer session file:
          ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md:
            # Spec Reviewer Session
            type: spec-reviewer | round: {round}
            plan_file: ${TMP_DIR}spec-plan-{dir-safe}.md
            dir_safe: {dir-safe}

       b. Dispatch Task(impl-reviewer):
            Session file: ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md
            Save results to ${TMP_DIR} and return only the path

          Extract verdict from result block.

       c. if verdict == "approved" → break

       d. if round >= max_safety:
            ⚠ Module {module.path}: Socratic loop terminated after {max_safety} iterations.
            break

       e. Create Revise session file:
          ${TMP_DIR}spec-plan-session-{dir-safe}.md (overwrite):
            # Spec Plan Session
            type: spec-plan | mode: revise | round: {round+1} | project_root: {project_root} | parallel: true
            target_path: {module.path}
            action: {module.action}
            document_language: {document_language or ""}

            ## User Requirement
            {module.requirement_refs}

            ## Reviewer Feedback File
            feedback_file: ${TMP_DIR}spec-reviewer-result-{dir-safe}-v{round}.md

            ## Existing Plan File
            existing_plan_file: ${TMP_DIR}spec-plan-{dir-safe}.md

            ## Existing Modules Index
            {scan-claude-md result}

            ## Project Conventions
            {project root Conventions or "None"}

       f. Dispatch Task(impl, mode=revise):
            Session file: ${TMP_DIR}spec-plan-session-{dir-safe}.md
            Read the session file and improve the execution plan in mode=revise.
            (parallel mode — AskUserQuestion prohibited)

       g. round++
     ```

     > **Why sequential per module:** Each module's reviewer loop iteration depends on previous results,
     > so sequential execution within the loop is unavoidable. While loops between modules are independent
     > and could run in parallel, they are processed sequentially to protect SKILL context.

  4. Create Execute session files + dispatch Task(impl, mode=execute) in parallel (up to 3):

     Create `${TMP_DIR}spec-execute-session-{dir-safe}.md` for each module:
     ```
     # Spec Execute Session
     type: spec-execute | mode: execute | project_root: {project_root} | parallel: true
     target_path: {module.path}
     action: {module.action}
     document_language: {document_language or ""}

     ## Approved Plan File
     plan_file: ${TMP_DIR}spec-plan-{dir-safe}.md

     ## User Requirement
     {module.requirement_refs}

     ## Existing Modules Index
     {latest scan-claude-md result}

     ## Project Conventions
     {project root Conventions or "None"}
     ```

     Dispatch Task(impl, mode=execute) in parallel:
     ```
     Task(impl) — ${TMP_DIR}spec-execute-session-{dir-safe-A}.md
     Task(impl) — ${TMP_DIR}spec-execute-session-{dir-safe-B}.md  (if exists)
     Task(impl) — ${TMP_DIR}spec-execute-session-{dir-safe-C}.md  (if exists)
     ```

     Instructions for each Task:
       Session file: ${TMP_DIR}spec-execute-session-{dir-safe}.md
       Project root: {project_root}
       Read the session file and generate CLAUDE.md + DEVELOPERS.md in mode=execute.
       (parallel mode — AskUserQuestion prohibited)

  5. Wait for current depth completion → proceed to next depth
```

> **Why split by depth:** impl agents at depth=1 modules (children) need to Read the CLAUDE.md of depth=0 modules (parents)
> during Phase 1.5 (Dependency Exploration). If session files are created before parent impl completes,
> the index becomes stale and parent context is missed.

### 7. Display changes

```bash
git diff --stat
```

### 8. Result

```
---spec-result---
scope: single | multi
modules:
  - {path}: {status} ({action})
unassigned_count: N
---end-spec-result---
```

## DO / DON'T

**DO:**
- Always invoke decompose first to delegate scope determination
- Do not skip decompose even for scope=single
- For multi scope, dispatch parallel impl after user approval
- Notify user about unassigned requirements

**DON'T:**
- Dispatch Task(impl) directly without decompose
- Delegate decomposition decisions to impl agent
- Auto-execute multi mode without user approval

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| No requirement argument | Collect requirements via AskUserQuestion |
| decompose agent failure | Report error and exit |
| impl agent failure (single module) | warn, continue with remaining modules |
| User cancels approval | Return status: cancelled_by_user |

---

## Auto Mode (--auto)

When the `--auto` flag is present, automatically run dev → validate → spec update loop after spec completion.
**AskUserQuestion prohibited after Phase 0.**

> **Note:** dev auto-detects language from source file extensions.
> For new projects with no source files, a language prompt may appear during the first dev run.
> In that case, autonomous execution will be interrupted. For empty projects, add a file indicating
> the language (package.json, go.mod, Cargo.toml, etc.) or run `/dev` once before using `--auto`.

Preserve the following values upon entering Auto Mode:
- `{original_requirement}`: user requirement text (extracted in Phase 0)
- `{impl_path}`: `--path` argument value (default `.`)

### Phase 0: Initial spec (same as normal workflow)

- Execute full Workflow Steps 0-8 above
- single mode: AskUserQuestion allowed (brainstorming + clarification)
- multi mode: one-time user approval (decomposition plan)
- CLAUDE.md + DEVELOPERS.md generation complete → enter Auto Loop


### Auto Loop

`auto_iter = 0`

#### Auto Phase 1: Dev

```
Skill("claude-md-plugin:dev", args: "--conflict overwrite --path {impl_path}")
```

Check `status` from dev-result:
- `failed` → warn and exit Auto Loop (cannot validate without code)
- `success | partial` → proceed to Auto Phase 2

#### Auto Phase 2: Validate

```
Skill("claude-md-plugin:validate", args: "{impl_path} --report-only")
```

Parse validate-result:

```
total_violations = schema_errors + convention_issues + boundary_issues + semantic_drift
```

- `total_violations == 0` → **success exit**
- `auto_iter >= auto_max_iter` → **max_iter exit**
- Otherwise → extract violation details → Auto Phase 3

**Violation detail extraction:**

Read each file from validate-result's `result_files` list:
- Files where `## Summary`'s `Total issues: N` > 0 → that module is a spec update target
- `## Issues` section → collect per-module violation details (REQUIREMENTS_NOT_IMPLEMENTED, etc.)
- If result_files is empty (no semantic verification targets): skip Phase 3 even if total_violations > 0

#### Auto Phase 3: Spec Update

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-auto-{iter}.json"
```

Create session files for each module where violations were found:
`${TMP_DIR}spec-session-auto-{iter}-{dir-safe}.md`

```markdown
# Spec Session (Auto Mode)
type: spec | project_root: {project_root} | target_path: {path} | action: update | parallel: true
document_language: {document_language or ""}

## User Requirement
{original_requirement}

## Auto-Fix Context
auto_iteration: {n}
validate_violations:
  schema_errors: {n}
  convention_issues: {n}
  boundary_issues: {n}
  semantic_drift: {n}

Validation violations were found in this module.
Read the existing CLAUDE.md and DEVELOPERS.md, and refine/supplement
Requirements and Constraints so that validate passes after dev.
Since CLAUDE.md is the SSOT, improve by making requirements more explicitly stated.

## Violations Detail
{violation details for this module extracted from result_file's ## Issues section}

## Existing Modules Index
{latest scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

Dispatch Task(impl) in parallel (up to 3). **AskUserQuestion prohibited.**

`auto_iter++` → loop back to Auto Phase 1

### Auto Phase 4: Exit report

**Success exit (`total_violations == 0`):**

```
✓ Auto mode complete ({auto_iter} iteration(s))
  spec: CLAUDE.md + DEVELOPERS.md generated
  dev: code generation complete
  validate: all verifications passed
```

**Failure exit (max_iter reached | dev failed):**

```
⚠ Auto mode terminated (reason: {reason})
  Iterations: {auto_iter}/{auto_max_iter}
  Remaining issues: schema_errors={n}, convention={n}, boundary={n}, semantic_drift={n}
  Run /validate or /spec manually to resolve.
```

### Auto Mode Error Handling

| Situation | Response |
|-----------|----------|
| dev failed | Exit loop, report error |
| No result_files (schema/convention only) | Skip Phase 3, retry dev |
| All spec updates failed | Warn, continue loop (try next dev) |
| max_iter exceeded | Exit loop, report remaining issues |
