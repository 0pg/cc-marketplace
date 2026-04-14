---
name: spec
version: 3.1.0
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

### Step 2 redirect loop (wraps Steps 2.0–2.1e)

At SKILL entry, initialize the visited set and a safety bound:

```bash
visited=()
MAX_REDIRECT_DEPTH=10   # runaway safety net (bug-guard, not convergence criterion)
```

Steps 2.0 through 2.1e run inside this loop. Define `goto_step_2_1` as a bash
function that encapsulates the Step 2.1 dispatch (2.1c prepare sessions) +
2.1d aggregate + 2.1e select sequence — this is the idiomatic bash replacement
for `goto`. After Step 2.1e selects `target_path`, consult the aggregated
verdict for a `redirect_to` field; if present, re-enter Step 2.1 with the
redirect target until authority converges (no redirect) or a cycle is detected.

```bash
visited+=("$target_path")

redirect_to=$(jq -r --arg tp "$target_path" \
              'select(.target==$tp) | .redirect_to // empty' \
              ${TMP_DIR}verdict-aggregate.jsonl)

if [ -n "$redirect_to" ]; then
  # Cycle check (safety net — a loop is a bug, not a convergence signal)
  if printf '%s\n' "${visited[@]}" | grep -qxF "$redirect_to"; then
    trace=$(IFS=$'\n'; printf '%s' "${visited[*]}" | paste -sd ' → ' -)
    emit_halt "redirect cycle: ${trace} → ${redirect_to}"
    exit 0
  fi
  # Existence check
  if [ ! -f "$redirect_to/CLAUDE.md" ]; then
    emit_halt "redirect target does not exist: $redirect_to"
    exit 0
  fi
  target_path="$redirect_to"
  consult_targets=("." "$redirect_to")
  goto_step_2_1      # bash function wrapping Step 2.1 dispatch+aggregate+select
fi

# Runaway safety net (not convergence): labeled as bug-guard
if [ ${#visited[@]} -gt "$MAX_REDIRECT_DEPTH" ]; then
  emit_halt "redirect depth exceeded safety limit (bug guard)"
  exit 0
fi
```

Rationale: verdicts self-describe authority; the SKILL honors `redirect_to`
until the authority chain converges. Cycle and depth guards are bug-guards,
not policy — a well-formed domain never loops and never exceeds a handful of
hops.

### Step 2.0: Candidate Identification (runs before Step 2.1)

When `target_path` is specified and that node has CLAUDE.md → skip this step; consult_targets = ("." "$target_path").

Otherwise dispatch a lightweight explorer pre-pass:

`Task(requirement-explorer, mode=candidate-only, session=<pre-session>)`

The explorer's result file contains `## Candidate Nodes`. Parse its body into `consult_targets`:

```bash
awk '/^## Candidate Nodes$/{f=1;next} /^## /{f=0} f && /^- /{sub(/^- /,""); sub(/[ \t]*#.*$/,""); print}' \
  ${TMP_DIR}explore-candidate-result.md \
  | awk 'NF' | sort -u > ${TMP_DIR}consult-targets.txt
```

Safety net (runaway guard only, not convergence): if explorer emits >10 candidates, log a warning and proceed with all of them; no arbitrary truncation.

### 2.1 Pre-consult (Strategic Conflict Detection)

Detect conflicts with existing constraints/Roadmap and root strategic direction
**before** requirement concretization. Populates context blocks injected into Step 2.5.

#### 2.1a Determine consult targets

Load the explorer-judged candidate set written by Step 2.0 (`${TMP_DIR}consult-targets.txt`):

```bash
mapfile -t consult_targets < ${TMP_DIR}consult-targets.txt
# existing v16 parallel dispatch for "${consult_targets[@]}" continues unchanged
```

The file already contains deduplicated, explorer-judged nodes (project root + any
semantically related modules). No additional filtering here: the explorer's judgment
is authoritative, and Step 2.1d fans out po-consultant across every entry.

#### 2.1b Sibling module relatedness

No lexical pre-filter. The full scan-claude-md index (each module's `path` + `purpose`)
is already injected into the explorer session (Step 2.5a → `## Existing Modules Index`);
the requirement-explorer judges semantic relatedness itself (domain overlap, data flow,
shared concepts — not limited to string matching).

```python
related_module_hints = []   # retained as empty for downstream compatibility; explorer reads the full index
```

**Rationale**: lexical matching (shared ≥3-char words) produced both false negatives
(synonyms like "login"/"auth") and false positives (common words like "data"/"user").
Delegating the judgment to the explorer — which already has the full index — is strictly
more expressive and costs no additional tokens.

#### 2.1c Prepare consult session files (one per target)

# NOTE: mirrors /consult SKILL Steps 1-3. Update both on format changes.

For each `target` in `consult_targets`:

1. Derive `dir_safe_target`: if target is `.` → `"root"`, else replace `/` with `-`
2. Read `{target}/CLAUDE.md` (full content) and `{target}/DEVELOPERS.md` (or "absent")
3. ```bash
   $CLI_PATH diff-node-history --path {target} --root {project_root} --limit 5 \
     --output "${TMP_DIR}preconsult-history-${dir_safe_target}.json"
   ```
4. Extract from DEVELOPERS.md `## Agent Observations` (if present):
   Filter types: `structural`, `decision`, `improvement` only (skip `tactical`, `preference`).
5. Extract `## Roadmap` section from DEVELOPERS.md (or "Roadmap not defined").
6. Write `${TMP_DIR}consult-session-${dir_safe_target}.md`:
   ```markdown
   # Consult Session
   type: consult | target: {target} | project_root: {project_root}
   dir_safe: {dir_safe_target}

   ## Request
   "{requirement_text}"

   ## [1] Current Spec

   ### CLAUDE.md
   {full CLAUDE.md content}

   ### DEVELOPERS.md
   {full DEVELOPERS.md content, or "absent"}

   ## [2] Decision History

   ### diff-node-history (limit 5)
   {parsed JSON from preconsult-history-{dir_safe_target}.json — commits with section changes}

   ### Agent Observations (structural, decision, improvement only)
   {filtered entries from ## Agent Observations, or "None"}

   ## [3] Strategic Direction

   ### Roadmap
   {roadmap_content}
   ```

#### 2.1d Dispatch po-consultant in parallel

Dispatch `Task(po-consultant)` for **all** prepared session files simultaneously
(single parallel batch):

  For each `target` in `consult_targets`:
    Task(po-consultant):
      Session file: ${TMP_DIR}consult-session-${dir_safe_target}.md
      Save result to ${TMP_DIR} and return only the result block path

Wait for all tasks to complete. Then read each `${TMP_DIR}consult-result-${dir_safe_target}.md`.
Extract: `verdict`, `roadmap_fit`, `## Constraints` block, `## History` block,
`## Suggested Path` block, roadmap_fit explanation sentence.

```python
# Failure handling (non-blocking):
failed_targets = [t for t in consult_targets if result file missing or unreadable]
if failed_targets:
    Output warning: "⚠ Pre-consult failed for {failed_targets}. Proceeding with partial results."
```

Then aggregate each target's verdict-level fields into a single JSONL file so
downstream steps (target selection, redirect handling) can query with `jq`:

```bash
extract_section() {
  # $1 = file, $2 = heading (e.g., "## Reason")
  awk -v h="$2" '
    $0 == h { capture=1; next }
    /^## / { capture=0 }
    capture { print }
  ' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' \
        | awk 'NF' | paste -sd ' ' -
}

: > ${TMP_DIR}verdict-aggregate.jsonl
for result in ${TMP_DIR}consult-result-*.md; do
  target=$(basename "$result" .md | sed 's/^consult-result-//' | tr '-' '/')
  jq -cn \
    --arg t   "$target" \
    --arg v   "$(extract_section "$result" '## Verdict')" \
    --arg e   "$(extract_section "$result" '## Execution')" \
    --arg rn  "$(extract_section "$result" '## Reason')" \
    --arg rf  "$(extract_section "$result" '## Roadmap Fit')" \
    --arg rd  "$(extract_section "$result" '## Redirect To')" \
    '{target:$t, verdict:$v, execution:$e, reason:$rn, roadmap_fit:$rf}
     + ({redirect_to:$rd} | if .redirect_to == "" then del(.redirect_to) else . end)' \
    >> ${TMP_DIR}verdict-aggregate.jsonl
done
```

#### 2.1e Target Selection from Verdicts

Read `${TMP_DIR}verdict-aggregate.jsonl`. Filter candidates (excluding root `.`) by
`execution=="auto_executable"`. Let the verdict tell us what to do — do not re-judge.

```bash
auto_ok=$(jq -c 'select(.execution=="auto_executable" and .target != ".")' \
           ${TMP_DIR}verdict-aggregate.jsonl)
count=$(echo "$auto_ok" | awk 'NF' | wc -l | tr -d ' ')

case "$count" in
  1)
    target_path=$(echo "$auto_ok" | jq -r '.target')
    ;;
  0)
    reasons=$(jq -r 'select(.target != ".") | "- \(.target): [\(.execution)] \(.reason)"' \
               ${TMP_DIR}verdict-aggregate.jsonl)
    if [ "$NO_ASK" = "true" ]; then
      emit_halt "no auto-executable target; PM/PO verdicts:\n$reasons"
    else
      ask_user_with_reasons "$reasons"
    fi
    ;;
  *)
    conflicts=$(echo "$auto_ok" | jq -r '"- \(.target): \(.reason)"')
    if [ "$NO_ASK" = "true" ]; then
      emit_halt "multiple nodes claim ownership:\n$conflicts"
    else
      ask_user_with_reasons "$conflicts"
    fi
    ;;
esac
```

Rationale: SKILL executes the authorities' verbatim judgment. Single auto_executable →
proceed; zero or multiple → surface state (no automatic tiebreak).

#### 2.1f Build pre-fetched context blocks

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
            entry += "(⚠ halt verdict: default = surface to caller. Exception = if the user intentionally\n" \
                     "replaces existing behavior, the explorer MAY note the override explicitly in\n" \
                     "Concretized Requirements. This exception is available in interactive mode only;\n" \
                     "under --no-ask the SKILL defers to the authority's halt verdict verbatim.)\n"
        pre_fetched_conflicts += entry

    elif result.verdict == "feasible" and result.roadmap_fit == "aligned":
        pre_fetched_strategic += f"[{target}] Roadmap aligned: {result.roadmap_fit_explanation}\n"

# Sibling modules not covered by pre-consult (explorer judges relevance)
unconsulted_hints = [p for p in related_module_hints if p not in consult_targets]

# Partially feasible early warning
partially_feasible_targets = [t for t, r in consult_results if r.verdict == "partially_feasible"]
if partially_feasible_targets:
    Output: "ℹ Pre-consult: partially_feasible constraints in {partially_feasible_targets}. Spec will be adjusted."

# Not feasible early warning
not_feasible_targets = [t for t, r in consult_results if r.verdict == "not_feasible"]
if not_feasible_targets:
    Output: "⚠ Pre-consult: not_feasible conflict in {not_feasible_targets}. (⚠ halt verdict: default = surface to caller. Exception = if the user intentionally replaces existing behavior, the explorer MAY note the override explicitly in Concretized Requirements. This exception is available in interactive mode only; under --no-ask the SKILL defers to the authority's halt verdict verbatim.)"
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

`round = 1`, `max_safety = 10` (runaway safety net; not a convergence criterion)

Termination is **reviewer-driven** via the `progress` field: the loop exits when the reviewer signals it has no new concerns (`progress: no`) or approves.

```
loop:
  2.5a. Create ${TMP_DIR}explore-session-{round}.md:

        Round 1:
        ---
        # Explore Session
        type: explore | round: 1 | project_root: {project_root} | target_path: {path}

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

        ## Related Module Candidates (omitted — explorer judges relatedness from Existing Modules Index below)

        ## Existing Modules Index
        {scan-claude-md result}

        ## Project Conventions
        {project root Conventions or "None"}
        ---

        Round 2:
        ---
        # Explore Session
        type: explore | round: 2 | project_root: {project_root} | target_path: {path}

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

        ## Related Module Candidates (omitted — explorer judges relatedness from Existing Modules Index below)

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
        {if round > 1:}
        prev_result_file: ${TMP_DIR}explore-reviewer-result-{round-1}.md
        ---

  2.5f. Task(requirement-reviewer):
        Session file: ${TMP_DIR}explore-reviewer-session-{round}.md
        Save results to ${TMP_DIR} and return only the path

        Extract verdict, progress, critical_questions, improvement_notes from result block.

  2.5g. if verdict == "approved":
          concretized_requirement = Read ## Concretized Requirements from explore-result
          domain_context_summary = Read ## Domain Context Summary from explore-result
          if improvement_notes > 0:
            reviewer_notes = Read ## Improvement Notes from ${TMP_DIR}explore-reviewer-result-{round}.md
          else:
            reviewer_notes = ""
          explore_status = "approved"
          break

  2.5h. if (round > 1 AND progress == "no") OR round >= max_safety OR early termination:
          # progress=="no": reviewer has no new concerns — stuck; surface to user.
          # max_safety: runaway guard, expected to never trigger in normal operation.
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

`round = 1`, `max_safety = 10` (runaway safety net; not a convergence criterion)

Termination is **reviewer-driven** via the `progress` field: the loop exits when the reviewer signals it has no new concerns (`progress: no`) or approves.

```
loop:
  1. Create Reviewer session file:
     ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md:
       # Spec Reviewer Session
       type: spec-reviewer | round: {round}
       plan_file: {plan_file}
       dir_safe: {dir-safe}
       {if round > 1:}
       prev_result_file: ${TMP_DIR}spec-reviewer-result-{dir-safe}-v{round-1}.md

  2. Dispatch Task(impl-reviewer):
       Session file: ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md
       Save results to ${TMP_DIR} and return only the path

     Extract verdict and progress from result block.

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

  3b. if round > 1 AND progress == "no":
       ⚠ Reviewer reports no progress — revise cycle is stuck.
         Surfacing current plan with unresolved Critical Questions.
     ```bash
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'stuck-no-progress'
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```
       break

  4. if round >= max_safety:
       ⚠ Socratic loop hit runaway safety net ({max_safety} iterations).
         This indicates a bug or pathological input; proceeding with the best available plan.
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

### Step 4.5: Post-Spec Impact Surface

After Execute writes CLAUDE.md + DEVELOPERS.md and the commit is made, surface
downstream consumers *if and only if* the target's `## Data Schemas` section
changed between the previous commit and the new on-disk content. Schema change
is the only trigger because it is the single exported surface that can break
consumers — Constraint-only changes stay internal to the node.

The deterministic CLIs `detect-schema-change` and `impact-scan` do the work; the
SKILL merely appends an `## Affected Consumers` block to `${TMP_DIR}result-block.md`
so Step 5 can echo it back to the user alongside the recommendation to run
`/sync` per consumer (or `/autodev --auto-sync` to delegate).

```bash
# Step 4.5: Surface affected consumers on schema change
before=$(git show HEAD:$target_path/DEVELOPERS.md 2>/dev/null || echo "")
after=$(cat $target_path/DEVELOPERS.md 2>/dev/null || echo "")
changed=$(core detect-schema-change \
  --before <(printf '%s' "$before") --after <(printf '%s' "$after") \
  | jq -r '.changed')

if [ "$changed" = "true" ]; then
  core impact-scan --target "$target_path" --format list > ${TMP_DIR}affected-consumers.txt
  if [ -s ${TMP_DIR}affected-consumers.txt ]; then
    {
      echo ""
      echo "## Affected Consumers"
      while IFS= read -r c; do [ -n "$c" ] && echo "- $c"; done \
        < ${TMP_DIR}affected-consumers.txt
      echo ""
      echo "> Recommend \`/sync\` each consumer, or \`/autodev --auto-sync\` to delegate."
    } >> ${TMP_DIR}result-block.md
  fi
fi
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

