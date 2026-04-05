# Self Socratic Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Self Socratic Loop to `/spec` that concretizes vague requirements through project context exploration before decompose/impl, redesign `/autodev` as a thin orchestrator, and remove validate dependency.

**Architecture:** Two new agents (requirement-explorer, requirement-reviewer) form a loop before decompose. Explorer reads project domain context and concretizes ambiguous requirements. Reviewer judges if concretized requirements are sufficient for complete spec definition. Loop runs max 2 rounds, with AskUserQuestion as last resort. autodev becomes `Skill(spec) + Skill(dev)`.

**Tech Stack:** Claude Code plugin (markdown agents/skills/commands), Rust CLI (scan-claude-md, validate-schema)

**Design Spec:** `docs/superpowers/specs/2026-04-03-self-socratic-loop-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `agents/requirement-explorer.md` | Create | Domain-context exploration + requirement concretization agent |
| `agents/requirement-reviewer.md` | Create | 5-criteria evaluation of concretized requirements |
| `skills/spec/SKILL.md` | Modify | Insert Step 2.5 (Self Socratic Loop), add `--no-ask`, remove `--auto` |
| `commands/autodev.md` | Rewrite | Thin orchestrator: Skill(spec) + Skill(dev) |
| `commands/spec-step.md` | Modify | Add explore_status guard |
| `agents/decompose.md` | Modify | Relax "original text only" constraint, consume Domain Context Summary |
| `agents/impl.md` | Modify | Conditional Phase 1.5 skip |
| `CLAUDE.md` | Modify | Agent/skill tables, architecture diagram, descriptions |
| `.claude-plugin/plugin.json` | Modify | MAJOR version bump, register new agents |

---

### Task 1: Create requirement-explorer agent

**Files:**
- Create: `agents/requirement-explorer.md`

- [ ] **Step 1: Write the agent file**

```markdown
---
name: requirement-explorer
description: |
  Use this agent when a large spec needs to be split into individual spec units.
  Analyzes natural language requirements and produces a module decomposition plan:
  target paths, requirement distribution, tree structure, and dependency order.
  Does NOT write CLAUDE.md — that is impl agent's responsibility.
  Returns result as a file to protect SKILL context window.

  <example>
  <context>
  The spec skill calls requirement-explorer to concretize vague requirements
  before decompose/impl.
  </context>
  <user_request>
  Session file: ${TMP_DIR}explore-session-1.md
  Save results to ${TMP_DIR} and return only the path
  </user_request>
  <assistant_response>
  1. Domain context collected — 5 key terms, 3 existing patterns
  2. Ambiguity assessment — 4 items: 2 domain-clear, 1 explorable, 1 genuinely-ambiguous
  3. Exploration — resolved 1 explorable item via src/auth/CLAUDE.md
  4. Result written: ${TMP_DIR}explore-result-1.md

  ---explore-result---
  result_file: ${TMP_DIR}explore-result-1.md
  total: 4
  domain_clear: 2
  resolved: 1
  unresolved: 1
  ---end-explore-result---
  </assistant_response>
  </example>
model: inherit
color: green
tools:
  - Read
  - Glob
  - Grep
  - Bash
  - Write
---

You are a requirements analyst specializing in concretizing vague requirements through
project domain context exploration. You do NOT write CLAUDE.md files or implement code —
you only produce concretized requirements that downstream agents can use for spec definition.

## Input

```
Session file: <path> (explore session file, pre-extracted by spec SKILL)
Save results to ${TMP_DIR} and return only the path
```

## Temporary Directory

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Session File Format

### Round 1:

```markdown
# Explore Session
type: explore | round: 1 | project_root: {path}

## User Requirement
{original requirement text}

## Existing Modules Index
{scan-claude-md result: path, purpose pairs}

## Project Conventions
{Conventions or "None"}
```

### Round 2+:

```markdown
# Explore Session
type: explore | round: {N} | project_root: {path}

## User Requirement
{original requirement text}

## Previous Concretization
previous_result: ${TMP_DIR}explore-result-{N-1}.md

## Reviewer Feedback
feedback_file: ${TMP_DIR}explore-reviewer-result-{N-1}.md

## Existing Modules Index
{scan-claude-md result}

## Project Conventions
{Conventions or "None"}
```

## Workflow

### Phase 1: Domain Context Collection

Explore the project to understand the domain before judging ambiguity.

| Source | Target | Method |
|--------|--------|--------|
| Project root CLAUDE.md | Purpose, Domain Context, Instructions | Read |
| Existing module CLAUDE.md | Purpose, Domain Context (related modules only) | Read (index-based) |
| Conventions | Terms, patterns, structure rules | From session file |
| Source code | Key types/interfaces/DSL definitions | Grep, Read |
| Config files | Tech stack, dependencies | Read |

Phase 1 output is an intermediate artifact used within the agent's context only.
The final externalized form is Phase 4's `## Domain Context Summary`.

### Phase 2: Domain-Context-Based Ambiguity Assessment

Evaluate each element of the user requirement against the domain context collected in Phase 1.

| Verdict | Criteria | Handling |
|---------|----------|----------|
| **domain-clear** | Single interpretation within domain context | resolved — cite domain definition |
| **explorable** | Multiple interpretations in domain, but code/history may have answer | Phase 3 target |
| **genuinely-ambiguous** | Cannot resolve even with domain context + project exploration | Record as unresolved |

`total` = number of items assessed in Phase 2. `total == 0` means the explorer found
no elements requiring ambiguity assessment — the requirement is already fully clear
within the domain context. Phase 1 and Phase 4 still execute (Domain Context Summary is produced).

### Phase 3: Exploration of explorable items

For each `explorable` item, attempt to find the answer in the project:

| Order | Source | Method |
|-------|--------|--------|
| 1 | Related module DEVELOPERS.md | Constraints, Public API, Decision Log → Read |
| 2 | Source code | Related function signatures, type definitions, error patterns → Grep, Read |
| 3 | git history | Related keyword commits, recent change patterns → `git log` |

Each item:
- Answer found → `resolved` + cite source (file:line or commit hash)
- Answer not found → `unresolved`, keep as genuinely-ambiguous

### Phase 4: Write concretized requirements

Save results to `${TMP_DIR}explore-result-{round}.md`:

```markdown
# Explore Result
round: {N}

## Domain Context Summary
{domain context summary — key terms with project-specific meanings,
existing patterns, tech stack. This section propagates to downstream agents.}

## Concretized Requirements
{concretized requirements — preserve original structure, replace ambiguous parts
with domain-context-based specifics}

## Resolution Log
- "{expression}" -> domain-clear: "{domain definition}" (source: {CLAUDE.md or source location})
- "{expression}" -> resolved: "{concretization}" (source: {file:line or commit hash})
- "{expression}" -> unresolved (tried: {exploration details}, judgment: genuinely-ambiguous)

## Remaining Ambiguities
- "{item}": {why not resolvable even with domain context}

## Summary
total: {N}, domain_clear: {N}, resolved: {N}, unresolved: {N}
```

Return result block:

```
---explore-result---
result_file: ${TMP_DIR}explore-result-{round}.md
total: N
domain_clear: N
resolved: N
unresolved: N
---end-explore-result---
```

## Error Handling

| Situation | Response |
|-----------|----------|
| Empty requirement | Report total: 0, return concretized = original |
| No CLAUDE.md files in project | Skip Phase 1 module reads, rely on source code + git |
| git not available | Skip git history exploration, rely on file reads |
| Round 2+ but previous_result not found | Treat as Round 1 |

## Core Constraints

- **AskUserQuestion usage prohibited** — Self-exploration only
- **File modification prohibited** — Read-only exploration + result file Write only
- **Bash restricted to git read commands** — git log, git show, git diff only. No git stash, checkout, or other state-modifying commands.
- **Round 2+: focus on reviewer's Critical Questions** — Address specific items the reviewer flagged, re-explore with deeper investigation
```

- [ ] **Step 2: Verify the file renders correctly**

Run: `head -5 agents/requirement-explorer.md`
Expected: frontmatter with `name: requirement-explorer`

- [ ] **Step 3: Commit**

```bash
git add agents/requirement-explorer.md
git commit -m "feat(claude-md-plugin): add requirement-explorer agent

Domain-context exploration agent for Self Socratic Loop.
Concretizes vague requirements by reading project code, docs, and git history."
```

---

### Task 2: Create requirement-reviewer agent

**Files:**
- Create: `agents/requirement-reviewer.md`

- [ ] **Step 1: Write the agent file**

```markdown
---
name: requirement-reviewer
description: |
  Use this agent when critically reviewing concretized requirements before spec definition.
  Evaluates whether requirements are sufficiently concrete for complete CLAUDE.md/DEVELOPERS.md
  generation, judged against the project's domain context.
  Called by spec SKILL in the Self Socratic Loop, after requirement-explorer produces
  concretized requirements.
  Returns verdict: approved | rejected with specific Critical Questions.

  <example>
  <context>
  spec SKILL calls requirement-reviewer after explorer concretizes requirements.
  </context>
  <user_request>
  Session file: ${TMP_DIR}explore-reviewer-session-1.md
  Save results to ${TMP_DIR} and return only the path
  </user_request>
  <assistant_response>
  1. Session read — round: 1, explore_result loaded
  2. Original requirement loaded — faithfulness check ready
  3. Evaluation:
     - Purpose identifiability: pass
     - Requirements derivability: fail — "notification behavior" not pass/fail determinable
     - Constraints derivability: pass
     - Domain Context sufficiency: pass
     - Resolution soundness: pass — spot-checked src/auth/CLAUDE.md:3 confirmed
  4. Verdict: rejected (1 Critical Question)
  5. Result written: ${TMP_DIR}explore-reviewer-result-1.md

  ---explore-reviewer-result---
  result_file: ${TMP_DIR}explore-reviewer-result-1.md
  verdict: rejected
  round: 1
  critical_questions: 1
  ---end-explore-reviewer-result---
  </assistant_response>
  </example>
model: inherit
color: magenta
tools:
  - Read
  - Glob
  - Grep
  - Write
---

You are a critical reviewer specializing in evaluating whether concretized requirements
are sufficient for complete spec definition. Your role is to judge quality against the
project's domain context — not in a vacuum.
You do NOT generate CLAUDE.md or code — you only review and return a verdict.

## Input

```
Session file: <path>
Save results to ${TMP_DIR} and return only the path
```

## Temporary Directory

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Session File Format

```markdown
# Explore Reviewer Session
type: explore-reviewer | round: {N}
explore_result: ${TMP_DIR}explore-result-{N}.md
original_requirement: ${TMP_DIR}original-requirement.md
```

## Workflow

### Phase 1: Load

Read the session file to extract file paths.

1. Read `explore_result` → load Concretized Requirements, Domain Context Summary, Resolution Log, Remaining Ambiguities
2. Read `original_requirement` → load original requirement text (for faithfulness verification)

### Phase 2: 5-Criteria Domain-Context-Based Evaluation

Evaluate all criteria against the domain context from the explore result.

| Criterion | Question | Rejected when |
|-----------|----------|---------------|
| **Purpose identifiability** | Can the module's reason for existence (business value) be derived from concretized requirements? | Cannot write Purpose in one sentence |
| **Requirements derivability** | Can a verifiable Requirements list be written? (pass/fail determinable) | Key behaviors cannot be expressed as pass/fail |
| **Constraints derivability** | Can input/output/error types be specified? | Core interface signatures cannot be inferred |
| **Domain Context sufficiency** | Do domain terms/rules converge to single interpretation? | Domain terms have 2+ possible interpretations |
| **Resolution soundness** | Are Resolution Log sources grounded in actual project artifacts? Spot-check at least 2 cited sources via Grep/Read. Also verify concretized requirements are faithful to the original — no hallucinated/drifted requirements. | Cited file:line does not contain claimed content, or concretized requirements introduce items not traceable to original |

**Evaluation principles:**
- Record all suspicious items as Critical Questions — silence is not approval
- "Good enough" does not exist — all criteria must pass explicitly
- Critical Questions must be specific: "notification behavior is not pass/fail testable" (O), "requirements need improvement" (X)

### Phase 3: Verdict Decision

**approved** — when all of the following are met:
- All 5 criteria pass
- Critical Questions: 0

**rejected** — when any criterion fails.

### Phase 4: Write Result + Return

Result file path: `${TMP_DIR}explore-reviewer-result-{round}.md`

Result file content:
```markdown
# Explore Review Result
round: {N}
verdict: approved | rejected

## Evaluation
- Purpose identifiability: pass | fail — {rationale}
- Requirements derivability: pass | fail — {rationale}
- Constraints derivability: pass | fail — {rationale}
- Domain Context sufficiency: pass | fail — {rationale}
- Resolution soundness: pass | fail — {rationale}
  - Spot-checked: {file:line → confirmed | not found}
  - Faithfulness: {confirmed | drifted — details}

## Critical Questions (when rejected)
- {criterion}: "{specific problem — which item is insufficient and why}"
```

Return result block:
```
---explore-reviewer-result---
result_file: ${TMP_DIR}explore-reviewer-result-{round}.md
verdict: approved | rejected
round: {N}
critical_questions: {N}
---end-explore-reviewer-result---
```

## Error Handling

| Situation | Response |
|-----------|----------|
| explore_result not found | verdict: rejected, Critical Question: "explore result file not found at {path}" |
| original_requirement not found | verdict: rejected, Critical Question: "original requirement file not found" |
| Resolution Log empty | Criterion 5 passes only if there were 0 explorable items (total == domain_clear) |
| round field missing | Assume round: 1 |

## Core Constraints

- **AskUserQuestion usage prohibited** — All judgments based solely on explore result and project artifacts
- **File modification prohibited** — Only result file Write allowed
- **Do not blindly trust explorer's Resolution Log** — Spot-check at least 2 cited sources. Reject if evidence is weak.
- **Verify faithfulness to original requirement** — Concretized requirements must be traceable to the original. Reject if explorer hallucinated or drifted.
```

- [ ] **Step 2: Verify the file renders correctly**

Run: `head -5 agents/requirement-reviewer.md`
Expected: frontmatter with `name: requirement-reviewer`

- [ ] **Step 3: Commit**

```bash
git add agents/requirement-reviewer.md
git commit -m "feat(claude-md-plugin): add requirement-reviewer agent

5-criteria domain-context-based evaluator for Self Socratic Loop.
Spot-checks explorer citations, verifies faithfulness to original requirement."
```

---

### Task 3: Modify decompose agent — relax constraints

**Files:**
- Modify: `agents/decompose.md:115-118` (Phase 3 principles)
- Modify: `agents/decompose.md:176-178` (Core Constraints)

- [ ] **Step 1: Update Phase 3 principles**

In `agents/decompose.md`, replace lines 117-118:

```
Before:
- Direct excerpts from the original text (no rewriting — rewriting is the impl agent's responsibility)

After:
- Direct excerpts from ## User Requirement section (no further rewriting by decompose — concretization is the explorer agent's responsibility)
```

- [ ] **Step 2: Update Core Constraints**

In `agents/decompose.md`, replace line 178:

```
Before:
- **Original text rewriting prohibited** — Only original text excerpts are allowed for requirement_refs

After:
- **requirement_refs must be direct excerpts from ## User Requirement** — which may be concretized text from the Self Socratic Loop, not necessarily the user's original words. Decompose does not further rewrite.
```

- [ ] **Step 3: Add Domain Context Summary consumption note**

After the existing session file format section (around line 71), add a note:

```markdown
When `## Domain Context Summary` is present in the session file, use it to inform
module identification (Phase 2 step 4: "Determine paths"). Domain terms help identify
natural module boundaries.
```

- [ ] **Step 4: Verify changes**

Run: `grep -n "requirement_refs\|original text\|Domain Context" agents/decompose.md`
Expected: Updated constraint text visible, no references to "original text rewriting prohibited"

- [ ] **Step 5: Commit**

```bash
git add agents/decompose.md
git commit -m "fix(claude-md-plugin): relax decompose constraint for concretized text

requirement_refs now excerpts from ## User Requirement which may be concretized.
Add Domain Context Summary consumption for module boundary identification."
```

---

### Task 4: Modify impl agent — conditional Phase 1.5 skip

**Files:**
- Modify: `agents/impl.md:346-368` (Phase 1.5 section)

- [ ] **Step 1: Add conditional skip logic at the top of Phase 1.5**

In `agents/impl.md`, before the Phase 1.5 content (line 346), add:

```markdown
**Conditional skip:** When `## Domain Context Summary` is present in the session file,
skip Phase 1.5 entirely — the requirement-explorer has already performed domain context
collection and dependency exploration. Proceed directly to Phase 2 (Tiered Clarification)
or Phase P (Write plan.md).

When `## Domain Context Summary` is absent, execute Phase 1.5 as below.
```

- [ ] **Step 2: Verify changes**

Run: `grep -n "Domain Context Summary\|Conditional skip" agents/impl.md`
Expected: New conditional skip text at Phase 1.5

- [ ] **Step 3: Commit**

```bash
git add agents/impl.md
git commit -m "feat(claude-md-plugin): add conditional Phase 1.5 skip in impl agent

When Domain Context Summary is present (from explorer), skip dependency
exploration to avoid duplicate work. SKILL controls inclusion."
```

---

### Task 5: Modify spec SKILL — insert Self Socratic Loop + remove --auto

**Files:**
- Modify: `skills/spec/SKILL.md`

This is the largest change. We need to:
1. Add `--no-ask` argument
2. Remove `--auto` and `--auto-max-iter` arguments
3. Insert Step 2.5 (Self Socratic Loop) between Step 2 and Step 3
4. Modify Step 3 (decompose session) to pass concretized requirements
5. Modify Step 6 (multi-scope session files) to include Domain Context Summary
6. Add state.json explore fields
7. Remove entire Auto Mode section (lines 607-739)

- [ ] **Step 1: Update Arguments table**

Replace the Arguments table (lines 28-33):

```markdown
## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `requirement` | Yes | - | Requirement text |
| `--path` | No | `.` | Target path |
| `--no-ask` | No | false | Suppress AskUserQuestion in Self Socratic Loop. When set, max_rounds exhaustion uses best-effort instead of asking the user. |
```

- [ ] **Step 2: Insert Step 2.5 after Step 2**

After Step 2 ("Read project conventions and document language", ending around line 56), insert:

```markdown
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

        ## Existing Modules Index
        {scan-claude-md result}

        ## Project Conventions
        {project root Conventions or "None"}
        ---

        Round 2+:
        ---
        # Explore Session
        type: explore | round: {N} | project_root: {project_root}

        ## User Requirement
        {user requirement text}

        ## Previous Concretization
        previous_result: ${TMP_DIR}explore-result-{N-1}.md

        ## Reviewer Feedback
        feedback_file: ${TMP_DIR}explore-reviewer-result-{N-1}.md

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
        if all unresolved items are genuinely-ambiguous AND no explorable items remain
           (i.e., unresolved == total - domain_clear - resolved, and round > 1 with no progress):
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

        Extract verdict, critical_questions from result block.

  2.5g. if verdict == "approved":
          concretized_requirement = Read ## Concretized Requirements from explore-result
          domain_context_summary = Read ## Domain Context Summary from explore-result
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

- [ ] **Step 3: Modify Step 3 — decompose session with concretized requirements**

Replace the decompose session file format (lines 60-74):

```markdown
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
```

- [ ] **Step 4: Modify Step 6 single-scope — add Domain Context Summary to plan session file**

In the single-scope plan session file (line 103-110), add after `document_language`:

```markdown
## Domain Context Summary
{domain_context_summary}
```

SKILL logic for Domain Context Summary inclusion:
- `explore_status == "approved" | "short-circuited"` → include `## Domain Context Summary` (impl skips Phase 1.5)
- `explore_status == "best-effort" | "user-resolved"` → omit `## Domain Context Summary` (impl runs Phase 1.5)

- [ ] **Step 5: Modify Step 6 multi-scope — add Domain Context Summary to parallel session files**

In the multi-scope session file templates (around lines 425-450), add `## Domain Context Summary` section with the same inclusion logic as Step 4:

```markdown
## Domain Context Summary
{domain_context_summary}      <- same for all modules, from explorer
```

- [ ] **Step 6: Add state.json explore fields**

In the state.json initialization block (Step 6b-1, around line 143), add fields:

```json
"explore_round": {final round from Step 2.5},
"explore_status": "{explore_status}",
"explore_result_file": "${TMP_DIR}explore-result-{final round}.md"
```

- [ ] **Step 7: Remove Auto Mode section**

Delete the entire Auto Mode section from "## Auto Mode (--auto)" (line 609) through the end of the file (line 739). This includes:
- `## Auto Mode (--auto)` header and note
- Phase 0
- Auto Loop (auto_iter, Auto Phase 1/2/3/4)
- Auto Phase 4: Exit report
- Auto Mode Error Handling table

- [ ] **Step 8: Bump SKILL version**

Update frontmatter version from `1.2.0` to `2.0.0` (MAJOR — breaking change: --auto removed).

- [ ] **Step 9: Verify changes**

Run: `grep -n "auto\|no-ask\|Self Socratic\|Domain Context Summary\|explore_status\|concretized_requirement" skills/spec/SKILL.md | head -20`
Expected:
- `--no-ask` in Arguments table
- `Self Socratic Loop` as Step 2.5
- `Domain Context Summary` in decompose and impl session files
- No `--auto` references
- No `Auto Mode` section

- [ ] **Step 10: Commit**

```bash
git add skills/spec/SKILL.md
git commit -m "feat(claude-md-plugin)!: add Self Socratic Loop to /spec, remove --auto

BREAKING CHANGE: --auto flag removed. Use /autodev for autonomous execution.

Insert Step 2.5 Self Socratic Loop (explorer + reviewer, max 2 rounds).
Add --no-ask flag for suppressing AskUserQuestion.
Pass concretized requirements + Domain Context Summary to decompose/impl.
Add explore state fields to state.json."
```

---

### Task 6: Rewrite autodev command — thin orchestrator

**Files:**
- Rewrite: `commands/autodev.md`

- [ ] **Step 1: Replace entire file content**

```markdown
---
name: autodev
description: |
  Use when the user wants to autonomously develop a feature end-to-end without manual steps.
  Runs requirements → CLAUDE.md → code generation as a pipeline.
  Autonomous execution from start to finish given only requirements, without step-by-step commands.
  Trigger keywords: auto develop, end-to-end, autonomous implementation
argument-hint: '"requirement" [--path path]'
allowed-tools: [Read, Write, Bash, Skill, AskUserQuestion]
---

# /autodev

Autonomously executes requirements from start to finish.
Orchestrates spec (spec definition) and dev (code generation) as a pipeline.

**Thin orchestrator — delegates all spec logic to /spec.**

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

\* If no requirement is provided, it will be collected once via AskUserQuestion.

## AskUserQuestion Budget

autodev permits at most **1 AskUserQuestion** total across the entire workflow:
- Either in Step 1 (requirement collection when missing)
- Or in spec's Self Socratic Loop last-resort (when max_rounds exhausted)
- NOT both.

When Step 1 uses AskUserQuestion, autodev passes `--no-ask` to spec.
When Step 1 is skipped (requirement provided), spec runs without `--no-ask`.

## Workflow

### Step 1: Requirement Collection

If no requirement provided, AskUserQuestion once:
- "What feature would you like to implement? Briefly describe core behavior and target path."
- Set `no_ask = true`.

If requirement is provided: `no_ask = false`.

After this step, **AskUserQuestion is prohibited for all remaining steps.**

### Step 2: Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### Step 3: Spec

```
Skill("claude-md-plugin:spec", args: "{requirement} --path {impl_path} {--no-ask if no_ask}")
```

spec internally runs: Self Socratic Loop → decompose → Socratic Loop → execute.

Check spec-result:
- `status: success` → proceed to Step 4
- `status: failed | cancelled_by_user` → exit with error report

### Step 4: Dev

```
Skill("claude-md-plugin:dev", args: "--conflict overwrite --path {impl_path}")
```

Check dev-result:
- `status: success | partial` → proceed to Step 5
- `status: failed` → exit with warning

### Step 5: Result Report

**Success:**

```
✓ autodev complete
  spec: CLAUDE.md + DEVELOPERS.md generated
  dev:  Code generation complete
```

```bash
git diff --stat
```

**Failure (spec or dev failed):**

```
⚠ autodev terminated (reason: {reason})
  Resolve manually with /spec or /dev.
```

## Error Handling

| Situation | Response |
|-----------|----------|
| No requirement | AskUserQuestion once in Step 1 |
| spec failed | Report error, exit |
| spec cancelled by user | Report cancellation, exit |
| dev failed | Report warning, show partial results |
```

- [ ] **Step 2: Verify the rewrite**

Run: `grep -c "validate\|Auto Phase\|auto_iter\|max-iter" commands/autodev.md`
Expected: `0` (no validate references, no auto loop)

Run: `grep "Skill(" commands/autodev.md`
Expected: Two lines — `Skill("claude-md-plugin:spec"` and `Skill("claude-md-plugin:dev"`

- [ ] **Step 3: Commit**

```bash
git add commands/autodev.md
git commit -m "feat(claude-md-plugin)!: rewrite autodev as thin orchestrator

BREAKING CHANGE: autodev no longer reimplements spec workflow.
Delegates to Skill(spec) + Skill(dev). Removes validate dependency loop.
Removes --max-iter argument. AskUserQuestion budget: max 1 total."
```

---

### Task 7: Modify spec-step command — add explore guard

**Files:**
- Modify: `commands/spec-step.md:39-41` (state fields extraction)
- Modify: `commands/spec-step.md:50-52` (after state.json read, before status branching)

- [ ] **Step 1: Add explore_status to extracted fields**

In line 40-41, add `explore_status` to the extraction list:

```markdown
Read state.json and extract the following fields:
- `status`, `round`, `plan_file`, `last_reviewer_result`
- `target_path`, `action`, `project_root`, `user_requirement`
- `explore_status`
```

- [ ] **Step 2: Add explore guard before status branching**

After the "state.json not found" check (line 48), before "### 3. Status Branching" (line 50), insert:

```markdown
**Self Socratic Loop guard:**

If `explore_status` is `"pending"` or `"in-progress"` or is missing from state.json:
```
⚠ Self Socratic Loop was not completed for this workflow.
  Run /spec --path {target_path} again to restart requirement concretization.
```
Exit without proceeding.

If `explore_status` is `"approved"`, `"user-resolved"`, `"short-circuited"`, or `"best-effort"`:
proceed to Status Branching below.
```

- [ ] **Step 3: Verify changes**

Run: `grep -n "explore_status" commands/spec-step.md`
Expected: References in field extraction and guard section

- [ ] **Step 4: Commit**

```bash
git add commands/spec-step.md
git commit -m "feat(claude-md-plugin): add explore_status guard to spec-step

Prevent spec-step from resuming when Self Socratic Loop is incomplete.
Guides user to run /spec again to restart."
```

---

### Task 8: Update CLAUDE.md — architecture and tables

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add new agents to Agent table**

In the Agents table (find `| Agent | Superpowers Composition | Role |`), add two rows:

```markdown
| `requirement-explorer` | (none) | Domain-context exploration → requirement concretization |
| `requirement-reviewer` | (none) | 5-criteria evaluation of concretized requirements |
```

- [ ] **Step 2: Update /spec architecture diagram**

Replace the spec SKILL diagram (lines 122-133) to include Self Socratic Loop:

```
┌─────────────────────────────────────────────┐
│ spec SKILL                                  │
│                                             │
│ 1. Bash(scan-claude-md) → Build index       │
│ 2. Self Socratic Loop:                      │
│    Task(requirement-explorer) →             │
│    Task(requirement-reviewer) →             │
│    approved | last-resort AskUserQuestion   │
│ 3. Create decompose session file            │
│ 4. Task(decompose agent) → Decompose plan   │
│ 5. Scope branching:                         │
│    single → 1 Task(impl agent)              │
│    multi  → Approve → Task(impl agent) × N  │
│             root-first, max 3 parallel       │
│ 6. Show git diff                            │
└─────────────────────────────────────────────┘
```

- [ ] **Step 3: Update Skills table**

In the Core Skills table, update `/spec` description to remove `--auto`:

```markdown
| `/spec` | Requirements → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints). Self Socratic Loop for requirement concretization before decompose. |
```

- [ ] **Step 4: Update Commands table**

Update `/autodev` description:

```markdown
| `/autodev` | Thin orchestrator: Skill(spec) + Skill(dev). Autonomous end-to-end execution. |
```

- [ ] **Step 5: Remove validate dependency references**

Search for and update any text that describes autodev or /spec as depending on validate loop. Key locations:
- The `/autodev` description in the Commands table (already updated in Step 4)
- Any "spec→dev→validate loop" references

- [ ] **Step 6: Verify changes**

Run: `grep -n "requirement-explorer\|requirement-reviewer\|Self Socratic\|--auto" CLAUDE.md`
Expected: New agents in table, Self Socratic in diagram, no `--auto` references

- [ ] **Step 7: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude-md-plugin): update architecture for Self Socratic Loop

Add requirement-explorer/reviewer to agent table.
Update /spec diagram with Self Socratic Loop step.
Update /spec and /autodev descriptions.
Remove validate dependency and --auto references."
```

---

### Task 9: Update plugin.json — version bump and agent registration

**Files:**
- Modify: `.claude-plugin/plugin.json`

- [ ] **Step 1: Bump version to 11.0.0**

Change `"version": "10.12.1"` to `"version": "11.0.0"`.

MAJOR bump because `--auto` removal from `/spec` is a breaking change.

- [ ] **Step 2: Register new agents**

Add to the `agents` array:

```json
"./agents/requirement-explorer.md",
"./agents/requirement-reviewer.md"
```

- [ ] **Step 3: Verify JSON validity**

Run: `python3 -c "import json; json.load(open('.claude-plugin/plugin.json')); print('valid')"`
Expected: `valid`

- [ ] **Step 4: Commit**

```bash
git add .claude-plugin/plugin.json
git commit -m "chore(claude-md-plugin): bump to v11.0.0, register explorer/reviewer agents

BREAKING: --auto removed from /spec. Use /autodev instead.
New agents: requirement-explorer, requirement-reviewer."
```

---

### Task 10: Update marketplace.json version

**Files:**
- Modify: `../../.claude-plugin/marketplace.json`

- [ ] **Step 1: Update version**

Find the `claude-md-plugin` entry and update `"version"` from `"10.12.1"` to `"11.0.0"`.

- [ ] **Step 2: Verify JSON validity**

Run: `python3 -c "import json; json.load(open('../../.claude-plugin/marketplace.json')); print('valid')"`
Expected: `valid`

- [ ] **Step 3: Commit**

```bash
git add ../../.claude-plugin/marketplace.json
git commit -m "chore: sync marketplace.json version for claude-md-plugin v11.0.0"
```

---

### Task 11: Integration verification

- [ ] **Step 1: Verify all affected files are committed**

Run: `git status`
Expected: Clean working tree.

- [ ] **Step 2: Verify no orphan references to --auto**

Run: `grep -rn "\-\-auto" skills/spec/ commands/ agents/ CLAUDE.md`
Expected: No matches (or only migration notes in design spec docs/).

- [ ] **Step 3: Verify no validate dependency in autodev/spec**

Run: `grep -rn "validate" commands/autodev.md skills/spec/SKILL.md`
Expected: No references to validate as a workflow step (validate-schema for CLI is OK).

- [ ] **Step 4: Verify new agents are registered**

Run: `python3 -c "import json; d=json.load(open('.claude-plugin/plugin.json')); print([a for a in d['agents'] if 'requirement' in a])"`
Expected: `['./agents/requirement-explorer.md', './agents/requirement-reviewer.md']`

- [ ] **Step 5: Verify file count and structure**

Run: `ls agents/requirement-*.md`
Expected: `agents/requirement-explorer.md  agents/requirement-reviewer.md`

- [ ] **Step 6: Final commit (if any fixups)**

Only if previous steps revealed issues that needed fixing.
