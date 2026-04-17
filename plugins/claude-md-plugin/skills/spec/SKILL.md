---
name: spec
version: 4.0.0
aliases: [define, requirements, impl]
description: |
  This skill should be used when the user asks to "define requirements", "write spec",
  "create CLAUDE.md from requirements", "define behavior before coding", or uses "/spec".
  Analyzes natural language requirements and generates CLAUDE.md + DEVELOPERS.md in a
  single-pass workflow (cross-node authority resolution → impl generation → optional gate).
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
| `requirement` | Yes* | - | Requirement text. \*Omit when `--resync` is set. |
| `--path` | No | `.` | Target path |
| `--resync` | No | false | Regenerate DEVELOPERS.md to match the current CLAUDE.md (which the user has manually edited). Skips pre-consult, skips new-requirement processing. Requires the target's CLAUDE.md to already exist. Replaces the obsolete `/sync` skill. |
| `--no-ask` | No | false | Suppress AskUserQuestion. When set, impl agent skips Tiered Clarification and Plan Preview; cross-node ambiguity halts verbatim with authority reasons preserved. |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

#### Snapshot helper

Any session file that feeds an agent performing snapshot judgment (impl Phase 7
Remove/Keep/Merge, impl-reviewer Snapshot integrity / Identifier coherence) MUST
surface the prior-state snapshot verbatim:

```
## Current CLAUDE.md
{verbatim contents of target_path/CLAUDE.md, or "absent" when the file does not exist}

## Current DEVELOPERS.md
{verbatim contents of target_path/DEVELOPERS.md, or "absent" when the file does not exist}
```

Injection is **unconditional**: if the target path is not yet resolved, use the
`--path` argument as the snapshot source; if the files do not exist, render the
body as the literal token `absent`. Do not summarize or truncate the body.

Bash helper template (inline into each session-creation site below):

```bash
snapshot_target="${target_path:-${arg_path:-.}}"
if [ -f "$snapshot_target/CLAUDE.md" ]; then
  current_claude_md=$(cat "$snapshot_target/CLAUDE.md")
else
  current_claude_md="absent"
fi
if [ -f "$snapshot_target/DEVELOPERS.md" ]; then
  current_developers_md=$(cat "$snapshot_target/DEVELOPERS.md")
else
  current_developers_md="absent"
fi
```

### --resync short-path

When `--resync` is set:

1. Verify `{target_path}/CLAUDE.md` exists. If not, halt with
   `⚠ --resync requires existing CLAUDE.md at {target_path}. Use /spec without --resync to create.`
2. Skip Steps 2.0–2.1 (pre-consult unnecessary; no new concretization).
3. Skip Step 2.4 (node history unused).
4. Proceed directly to Step 3 with:
   - `resync: true` injected into the spec session
   - `## User Requirement` body set to
     `"(resync: no new requirement; regenerate DEVELOPERS.md from current CLAUDE.md)"`
   - `action: update` forced

All other Step 3 semantics (impl → impl-reviewer gate → optional 1 revision →
commit) apply unchanged. The reviewer still runs; impl's self-critique catches
stale Constraints under resync conditions.

### 1. Generate existing CLAUDE.md index

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index.json"
```

### 2. Read project conventions and document language

Read the `## Conventions` section from project root CLAUDE.md if present.

Read the `## Instructions` section from project root CLAUDE.md and extract the `Document language` value.
If not found, set `document_language` to empty (the impl agent will ask the user, or infer in scope=parallel).

### Step 2 redirect loop (wraps Steps 2.0–2.1e)

At SKILL entry, initialize the visited set and a safety bound:

```bash
visited=()
MAX_REDIRECT_DEPTH=10   # runaway safety net (bug-guard, not convergence criterion)
```

Steps 2.0 through 2.1e run inside this loop. After Step 2.1e selects
`target_path`, read the selected target's `consult-result-*.md` file. If its
`## Redirect To` body names another path, re-enter Step 2.1 at that path and
append the current `target_path` to `visited`. Continue until the selected
target has no redirect (authority converged).

Halt conditions (bug-guards, not convergence criteria):
- **Cycle** — the redirect target already appears in `visited`. Halt with
  the visited trace preserved verbatim.
- **Missing target** — `{redirect_to}/CLAUDE.md` does not exist. Halt with the
  redirect target path preserved.
- **Runaway depth** — `|visited| > MAX_REDIRECT_DEPTH` (10). Halt; this
  indicates a bug in the domain model, not a normal termination.

Rationale: verdicts self-describe authority; the SKILL honors `redirect_to`
verbatim until the chain converges. A well-formed domain never loops and never
exceeds a handful of hops.

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
**before** impl dispatch. Populates context blocks injected into Step 3.

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
is already injected into the explorer session (Step 2.0); the requirement-explorer
judges semantic relatedness itself (domain overlap, data flow, shared concepts —
not limited to string matching).

#### 2.1c Prepare consult session files (one per target)

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

Failure handling (non-blocking): if any result file is missing or unreadable,
emit warning `⚠ Pre-consult failed for {failed_targets}. Proceeding with partial results.`

#### 2.1e Target Selection from Verdicts

Read each consult-result file. Per INV-15, the SKILL executes each consultant's
verdict verbatim — no aggregation schema, no re-interpretation. The outcome to
achieve:

- Identify non-root candidates (`target ≠ "."`) whose `## Execution` is
  `auto_executable`.
- **Exactly one** such candidate → that candidate's path is the target.
- **Zero** such candidates → halt (when `NO_ASK=true`) with each candidate's
  verdict reason preserved verbatim, or `AskUserQuestion` with the same reasons
  (interactive). No automatic tiebreak.
- **Multiple** such candidates → halt (when `NO_ASK=true`) with each candidate's
  reason preserved verbatim, or `AskUserQuestion` (interactive). Cross-node
  ownership is not decided by the SKILL.

Preserve each `## Reason` body as written by the consultant; do not collapse or
summarize. INV-15 forbids the SKILL from synthesizing a substitute decision.

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
                     "replaces existing behavior, impl MAY note the override explicitly in Rationale.\n" \
                     "This exception is available in interactive mode only; under --no-ask the SKILL\n" \
                     "defers to the authority's halt verdict verbatim.)\n"
        pre_fetched_conflicts += entry

    elif result.verdict == "feasible" and result.roadmap_fit == "aligned":
        pre_fetched_strategic += f"[{target}] Roadmap aligned: {result.roadmap_fit_explanation}\n"

# Partially / not-feasible early warnings
partially_feasible_targets = [t for t, r in consult_results if r.verdict == "partially_feasible"]
if partially_feasible_targets:
    Output: "ℹ Pre-consult: partially_feasible constraints in {partially_feasible_targets}. Spec will be adjusted."

not_feasible_targets = [t for t, r in consult_results if r.verdict == "not_feasible"]
if not_feasible_targets:
    Output: "⚠ Pre-consult: not_feasible conflict in {not_feasible_targets}. (⚠ halt verdict: default = surface to caller. Exception = if the user intentionally replaces existing behavior, impl MAY note the override explicitly in Rationale. This exception is available in interactive mode only; under --no-ask the SKILL defers to the authority's halt verdict verbatim.)"
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

The node-history JSON feeds into the spec session as `## Node History` (Step 3a).

### 3. Spec execution (single-pass with optional gate)

The impl agent runs as a single pass: extract → draft → self-critique → snapshot-
judge → generate CLAUDE.md + DEVELOPERS.md + rationale sidecar. The impl-reviewer
acts as a single optional gate on the generated documents. On rejection, the SKILL
re-dispatches impl once with reviewer feedback injected; on a second rejection
the SKILL halts with the verdict surfaced.

Compute `dir_safe`: replace slashes with hyphens in target_path (root `.` → `root`).

**3a. Create spec session file**

Prepare `${TMP_DIR}spec-session-{dir-safe}.md`:

```markdown
# Spec Session
type: spec | project_root: {project_root}
target_path: {target_path}
action: create | update   # derived from whether target_path/CLAUDE.md exists; --resync forces update
scope: single
resync: true | false   # true when --resync flag was passed
no_ask: true | false   # true when --no-ask flag was passed; impl skips Tiered Clarification + Plan Preview
document_language: {document_language or ""}

## User Requirement
{full user requirement text, or "(resync: no new requirement; regenerate DEVELOPERS.md from current CLAUDE.md)" when resync=true}

## Pre-fetched Conflicts
{pre_fetched_conflicts or omit section if empty}

## Pre-fetched Strategic Context
{pre_fetched_strategic or omit section if empty}

## Node History
{parsed node-history JSON or omit section if absent}

## Existing Modules Index
{scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}

## Current CLAUDE.md
{current_claude_md}

## Current DEVELOPERS.md
{current_developers_md}
```

**3b. First dispatch: Task(impl)**

```
Task(impl):
  Session file: ${TMP_DIR}spec-session-{dir-safe}.md
  Project root: {project_root}

  Read the session file and generate CLAUDE.md + DEVELOPERS.md.
```

Extract from the result block: `claude_md_file`, `developers_md_file`,
`rationale_file`, `status`, `action`, `target_path`, `warnings`.

Handle terminal statuses:
- `status: cancelled_by_user` → exit; no files saved
- `status: failed` → exit with warnings surfaced

On `status: success`, proceed to 3c.

**3c. Review gate: Task(impl-reviewer)**

**Preservation audit (action=update only).** Before dispatching the reviewer,
when `action=update` and the rationale sidecar contains a `## Preserved
Sections` subsection, run `diff-preservation` to deterministically verify that
every declared section is byte-identical between the prior and new
DEVELOPERS.md. The resulting JSON is injected into the reviewer session so the
reviewer can treat any drift as an unconditional rejection.

```bash
preserved_sections=$(awk '/^## Preserved Sections$/{f=1;next} /^## /{f=0} f && /^- /{sub(/^- /,""); print}' "${rationale_file}" | paste -sd ',' -)
audit_file=""
if [ -n "$preserved_sections" ]; then
  prior_tmp="${TMP_DIR}prior-developers-${dir_safe}.md"
  printf '%s' "$current_developers_md" > "$prior_tmp"
  audit_file="${TMP_DIR}preservation-audit-${dir_safe}.json"
  $CLI_PATH diff-preservation \
    --prior "$prior_tmp" \
    --new "${target_path}/DEVELOPERS.md" \
    --sections "$preserved_sections" \
    > "$audit_file"
fi
```

Prepare `${TMP_DIR}spec-reviewer-session-{dir-safe}.md`:

```markdown
# Spec Reviewer Session
type: spec-reviewer | round: 1
target_path: {target_path}
dir_safe: {dir-safe}
rationale_file: {rationale_file from impl result}
action: {action}

## Prior CLAUDE.md
{current_claude_md when action=update, omit section when action=create}

## Prior DEVELOPERS.md
{current_developers_md when action=update, omit section when action=create}

## Preservation Audit
{contents of audit_file when present, omit section otherwise}
```

Dispatch:

```
Task(impl-reviewer):
  Session file: ${TMP_DIR}spec-reviewer-session-{dir-safe}.md
  Save results to ${TMP_DIR} and return only the path
```

Extract `verdict` and `result_file` from the reviewer result block.

- `verdict: approved` → proceed to 3e (commit).
- `verdict: rejected` → proceed to 3d (one revision).

**3d. Revision (at most once)**

Overwrite `${TMP_DIR}spec-session-{dir-safe}.md` with the prior session content
plus a `## Reviewer Feedback` section appended:

```markdown
## Reviewer Feedback
feedback_file: {result_file from reviewer round 1}
```

Leave all other sections unchanged (the impl agent already has the requirement,
pre-fetched context, modules index, conventions, and prior-state snapshot).

Re-dispatch:

```
Task(impl):
  Session file: ${TMP_DIR}spec-session-{dir-safe}.md
  Project root: {project_root}

  Read the session file. The ## Reviewer Feedback section is present; apply
  feedback to revise CLAUDE.md + DEVELOPERS.md.
```

Extract the new result block. Handle `cancelled_by_user` / `failed` terminals as in 3b.

Prepare `${TMP_DIR}spec-reviewer-session-{dir-safe}.md` (overwrite) with `round: 2`
and re-dispatch `Task(impl-reviewer)`.

- Second-round `verdict: approved` → proceed to 3e.
- Second-round `verdict: rejected` → halt. Surface the reviewer's Critical
  Questions verbatim to the user and exit with status `spec rejected after 1 revision`.
  Generated CLAUDE.md + DEVELOPERS.md remain on disk (not committed) so the user
  can inspect and either re-run `/spec` with refined input or edit manually. Do
  not delete the files; do not commit.

**3e. Auto-commit**

Construct the commit message after approval:

1. **summary**: one-line summary based on Purpose and Requirements from the generated CLAUDE.md
2. **[BREAKING]** (optional): include only when Requirements are deleted or there is a major direction change
3. **Transition context**: 1-2 sentences
   - `create` action: "New module creation" + Purpose summary
   - `update` action: describe transition direction based on Requirements changes from `git diff HEAD -- {target_path}/CLAUDE.md`
   - Good example: "Introducing OAuth2 as an additional authentication path alongside session-based auth. Maintaining sessions for legacy client support."
   - Bad example: "Update authentication system" (no directionality)
4. **Changes**: derived from `git diff HEAD -- {target_path}/CLAUDE.md {target_path}/DEVELOPERS.md`
   - added / modified / removed items (omit categories that don't apply)

```bash
# Commit only CLAUDE.md + DEVELOPERS.md (exclude TMP files)
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
git diff --stat HEAD~1
```

### Step 4.5: Post-Spec Impact Surface

After the commit is made, surface downstream consumers *if and only if* the target's
`## Data Schemas` section changed between the previous commit and the new on-disk
content. Schema change is the only trigger because it is the single exported
surface that can break consumers — Constraint-only changes stay internal to the
node.

The deterministic CLIs `detect-schema-change` and `impact-scan` do the work; the
SKILL appends an `## Affected Consumers` block to `${TMP_DIR}result-block.md`
so Step 5 can echo it back to the user alongside the recommendation to run
`/autodev --auto-sync` (or re-run `/spec` per consumer).

```bash
# Step 4.5: Surface affected consumers on schema change
before=$(git show HEAD~1:$target_path/DEVELOPERS.md 2>/dev/null || echo "")
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
      echo "> Recommend \`/autodev --auto-sync\` to propagate, or run \`/spec --resync --path <consumer>\` per consumer."
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

Followed by the `## Affected Consumers` block from Step 4.5 when present.

## DO / DON'T

**DO:**
- Complete cross-node authority resolution (Steps 2.0–2.1e) before dispatching impl
- Run the impl-reviewer gate after impl completes
- Surface reviewer rejection verbatim when revision fails a second time

**DON'T:**
- Re-interpret po-consultant verdicts (INV-15)
- Loop impl-reviewer beyond 2 rounds total (impl does its own self-critique internally)
- Commit files when reviewer rejected after revision (surface to user for manual resolution)

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| No requirement argument | Collect requirement via AskUserQuestion |
| impl status: failed | Exit with warnings surfaced |
| impl status: cancelled_by_user | Exit cleanly; no files saved |
| impl-reviewer rejected after 1 revision | Halt, surface Critical Questions, leave generated files uncommitted for user inspection |
| Redirect loop cycle / missing target / runaway depth | Halt with visited trace |
