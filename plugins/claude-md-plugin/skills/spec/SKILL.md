---
name: spec
version: 3.0.0
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

### 2.1 Pre-consult (Strategic Conflict Detection)

Detect conflicts with existing constraints/Roadmap and root strategic direction
**before** requirement concretization. Populates context blocks injected into Step 2.5.

#### 2.1a Determine consult targets

```bash
if [ -f "{path}/CLAUDE.md" ]; then
  # Existing node — consult root + target
  consult_targets=("." "{path}")
else
  # New node — root strategic context only (partial-skip: target has no spec yet)
  consult_targets=(".")
fi
# Deduplicate (handles case where path == project_root)
consult_targets=($(printf '%s\n' "${consult_targets[@]}" | sort -u))
```

#### 2.1b Sibling module candidate hints (no LLM)

From the scan-claude-md index (Step 1), find modules not already in `consult_targets`
whose `purpose` field shares at least one word (≥ 3 characters) with the requirement text:

```python
import re
req_words = set(re.findall(r'\w{3,}', requirement_text.lower()))
related_module_hints = []
for module in claude_md_index["modules"]:
    if module["path"] in consult_targets:
        continue
    purpose_words = set(re.findall(r'\w{3,}', module.get("purpose", "").lower()))
    if req_words & purpose_words:
        related_module_hints.append(module["path"])
related_module_hints = related_module_hints[:3]  # top 3 by insertion order
```

#### 2.1c Execute consult for each target (sequential)

For each `target` in `consult_targets`:

1. Derive `dir_safe_target`: if target is `.` → `"root"`, else replace `/` with `-`
2. `Skill(/consult, args: "{target} \"{requirement_text}\"")`
3. Read `${TMP_DIR}consult-result-${dir_safe_target}.md`
4. Extract: `verdict`, `roadmap_fit`, `## Constraints` block, `## History` block, `## Suggested Path` block

#### 2.1d Build pre-fetched context blocks

```
pre_fetched_conflicts = ""    # filled when verdict ∈ {partially_feasible, not_feasible}
pre_fetched_strategic = ""    # filled when verdict == feasible AND roadmap_fit == aligned

for target, result in consult_results:
    if result.verdict in ["partially_feasible", "not_feasible"]:
        entry = f"""[{target}] Verdict: {result.verdict}

Constraints:
{result.constraints}

History:
{result.history}

Suggested Path:
{result.suggested_path}
"""
        if result.verdict == "not_feasible":
            entry += "(⚠ not_feasible: if this requirement intentionally replaces existing behavior, " \
                     "the explorer must note that explicitly in Concretized Requirements.)\n"
        pre_fetched_conflicts += entry

    elif result.verdict == "feasible" and result.roadmap_fit == "aligned":
        pre_fetched_strategic += f"[{target}] Roadmap aligned: {result.roadmap_fit_explanation}\n"

# Sibling modules not covered by pre-consult (explorer judges relevance)
unconsulted_hints = [p for p in related_module_hints if p not in consult_targets]
```

### 2.4 Collect Node History (if existing node AND not pre-consulted)

If the target path has a CLAUDE.md **and** the target was NOT included in `consult_targets` (Step 2.1a):

```bash
$CLI_PATH diff-node-history \
  --path {path} --root {project_root} --limit 10 \
  --output "${TMP_DIR}node-history-${dir_safe}.json"
```

If the target was pre-consulted (target ∈ `consult_targets`): **skip this step**.
The `po-consultant` agent already captured recent node history in the consult result.

If the output file exists and contains commits (`has_history: true`), include its contents
in the explore session file as `## Node History` section. See format below in 2.5a.

### 2.5 Self Socratic Loop

Concretize vague requirements through project domain context exploration before spec execution.

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

        ## Node History (optional — only when existing node has history AND not pre-consulted)
        commits_included: {N} | total_found: {M}
        {for each commit in node-history JSON:}
        ### {short_hash} — {subject}
        timestamp: {timestamp} | breaking: {true|false}
        {for each file_diff:}
        **{file_type} — {section}**: {changes count} added, {changes count} removed
        {end for}
        {end for}

        ## Pre-fetched Conflicts (from pre-consult — omit section if empty)
        {pre_fetched_conflicts}

        ## Pre-fetched Strategic Context (from pre-consult — omit section if empty)
        {pre_fetched_strategic}

        ## Related Module Candidates (keyword match — explorer judges relevance — omit if empty)
        {one path per line from unconsulted_hints}

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

        ## Pre-fetched Conflicts (from pre-consult — omit section if empty)
        {pre_fetched_conflicts}

        ## Pre-fetched Strategic Context (from pre-consult — omit section if empty)
        {pre_fetched_strategic}

        ## Related Module Candidates (keyword match — explorer judges relevance — omit if empty)
        {one path per line from unconsulted_hints}

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

### 3. Spec execution (plan → review → execute)

**3a. Create Plan session file**

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

**3b. Dispatch Task(impl, mode=plan)**

```
Task(impl):
  Session file: ${TMP_DIR}spec-plan-session-{dir-safe}.md
  Project root: {project_root}

  Read the session file and generate an execution plan (plan.md) in mode=plan.
```

Extract `plan_file`, `target_path`, `action`, `dir-safe` from the result block.

> `dir_safe`: replace slashes with hyphens in target_path (e.g., `src/auth` → `src-auth`)

**3b-1. Initialize workflow state**

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

**3c. Socratic Loop**

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

**3d. Create Execute session file**

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

**3e. Dispatch Task(impl, mode=execute)**

```
Task(impl):
  Session file: ${TMP_DIR}spec-execute-session-{dir-safe}.md
  Project root: {project_root}

  Read the session file and generate CLAUDE.md + DEVELOPERS.md in mode=execute.
```

**3e-1. Update state + auto-commit after Execute completion**

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

### 4. Display changes

```bash
git diff --stat
```

### 5. Result

```
---spec-result---
modules:
  - {path}: {status} ({action})
---end-spec-result---
```

## DO / DON'T

**DO:**
- Run Self Socratic Loop before spec execution
- Complete Socratic review loop (impl-reviewer) before execute
- Notify user about any warnings from impl agent

**DON'T:**
- Skip Self Socratic Loop
- Skip Socratic review loop (impl-reviewer)
- Dispatch impl without plan mode first

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| No requirement argument | Collect requirements via AskUserQuestion |
| impl agent failure | warn, report error and exit |

