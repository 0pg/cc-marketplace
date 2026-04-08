---
name: spec
version: 2.0.0
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
| `--no-ask` | No | false | Suppress AskUserQuestion in Self Socratic Loop. When set, max_rounds exhaustion uses best-effort instead of asking the user. |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
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

### 2.4 Collect Node History (if existing node)

If the target path already has a CLAUDE.md, collect recent change context:

```bash
if [ -f "{path}/CLAUDE.md" ]; then
  $CLI_PATH diff-node-history \
    --path {path} --root {project_root} --limit 10 \
    --output "${TMP_DIR}node-history-${dir_safe}.json"
fi
```

If the output file exists and contains commits (`has_history: true`), include its contents
in the explore session file as `## Node History` section. See format below in 2.5a.

### 2.5 Self Socratic Loop

Concretize vague requirements through project domain context exploration before decompose.

```bash
# Preserve original requirement
cat > "${TMP_DIR}original-requirement.md" << 'REQEOF'
{user requirement text}
REQEOF
```

`round = 1`, `max_rounds = 2`

```
loop:
  2.5a. Create ${TMP_DIR}explore-session-{round}.md:

        Round 1:
        ---
        # Explore Session
        type: explore | round: 1 | project_root: {project_root}

        ## User Requirement
        {user requirement text}

        ## Node History (optional — only when existing node has history)
        commits_included: {N} | total_found: {M}
        {for each commit in node-history JSON:}
        ### {short_hash} — {subject}
        timestamp: {timestamp} | breaking: {true|false}
        {for each file_diff:}
        **{file_type} — {section}**: {changes count} added, {changes count} removed
        {end for}
        {end for}

        ## Existing Modules Index
        {scan-claude-md result}

        ## Project Conventions
        {project root Conventions or "None"}
        ---

        Round 2:
        ---
        # Explore Session
        type: explore | round: 2 | project_root: {project_root}

        ## User Requirement
        {user requirement text}

        ## Previous Concretization
        previous_result: ${TMP_DIR}explore-result-1.md

        ## Reviewer Feedback
        feedback_file: ${TMP_DIR}explore-reviewer-result-1.md

        ## Existing Modules Index
        {scan-claude-md result}

        ## Project Conventions
        {project root Conventions or "None"}
        ---

  2.5b. Task(requirement-explorer):
        Session file: ${TMP_DIR}explore-session-{round}.md
        Save results to ${TMP_DIR} and return only the path

        Extract total, domain_clear, resolved, unresolved from result block.

  2.5c. Short-circuit check:
        if total == 0 (no ambiguity assessment needed) OR
           (domain_clear + resolved == total, unresolved == 0):
          concretized_requirement = Read ## Concretized Requirements from explore-result
          domain_context_summary = Read ## Domain Context Summary from explore-result
          explore_status = "short-circuited"
          break (skip reviewer — requirements already clear)

  2.5d. Early termination check:
        if all unresolved items are genuinely-ambiguous AND no explorable items remain:
          → jump to 2.5h (AskUserQuestion) immediately

  2.5e. Create ${TMP_DIR}explore-reviewer-session-{round}.md:
        ---
        # Explore Reviewer Session
        type: explore-reviewer | round: {round}
        explore_result: ${TMP_DIR}explore-result-{round}.md
        original_requirement: ${TMP_DIR}original-requirement.md
        ---

  2.5f. Task(requirement-reviewer):
        Session file: ${TMP_DIR}explore-reviewer-session-{round}.md
        Save results to ${TMP_DIR} and return only the path

        Extract verdict, critical_questions, improvement_notes from result block.

  2.5g. if verdict == "approved":
          concretized_requirement = Read ## Concretized Requirements from explore-result
          domain_context_summary = Read ## Domain Context Summary from explore-result
          if improvement_notes > 0:
            reviewer_notes = Read ## Improvement Notes from ${TMP_DIR}explore-reviewer-result-{round}.md
          else:
            reviewer_notes = ""
          explore_status = "approved"
          break

  2.5h. if round >= max_rounds OR early termination:
          if --no-ask flag is set:
            concretized_requirement = Read ## Concretized Requirements from explore-result
            domain_context_summary = Read ## Domain Context Summary from explore-result
            explore_status = "best-effort"
            break

          Summarize Critical Questions (or Remaining Ambiguities for early termination)
          → AskUserQuestion (last resort):
            "Requirements concretization attempted but these remain unclear:
             - {Critical Question 1}
             - {Critical Question 2}
             Can you provide specifics?"

          Incorporate user answer into a new explore session → 1 more explorer run:
          Create ${TMP_DIR}explore-session-{round+1}.md with user answer appended to
          ## User Requirement section.
          Task(requirement-explorer) → extract result.
          concretized_requirement = result's ## Concretized Requirements
          domain_context_summary = result's ## Domain Context Summary
          explore_status = "user-resolved"
          break

  2.5i. round++ → return to 2.5a
```

### 3. Create Decompose session file

`${TMP_DIR}decompose-session.md`:

```markdown
# Decompose Session
type: decompose | project_root: {project_root}

## User Requirement
{concretized_requirement}

## Original Requirement
{original user requirement text}

## Domain Context Summary
{domain_context_summary}

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

## Domain Context Summary
{domain_context_summary if explore_status in ["approved", "short-circuited"], else omit this section}

## Reviewer Improvement Notes
{reviewer_notes if reviewer_notes != "", else omit this section}

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
  "explore_round": {round from Step 2.5},
  "explore_status": "{explore_status from Step 2.5}",
  "explore_result_file": "${TMP_DIR}explore-result-{round}.md",
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

     ## Domain Context Summary
     {domain_context_summary if explore_status in ["approved", "short-circuited"], else omit this section}

     ## Reviewer Improvement Notes
     {reviewer_notes if reviewer_notes != "", else omit this section}

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

     ## Domain Context Summary
     {domain_context_summary if explore_status in ["approved", "short-circuited"], else omit this section}

     ## Reviewer Improvement Notes
     {reviewer_notes if reviewer_notes != "", else omit this section}

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

