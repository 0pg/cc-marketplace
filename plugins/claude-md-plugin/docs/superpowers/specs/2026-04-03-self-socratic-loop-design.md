# Self Socratic Loop Design

## Summary

Add a Self Socratic Loop to `/spec` SKILL that concretizes vague requirements
through project context exploration before decompose/impl.
Redesign `/autodev` as a thin orchestrator calling `Skill(spec) + Skill(dev)`.
Remove validate dependency from all workflows.
Remove `--auto` flag from `/spec` (BREAKING — MAJOR version bump).

## Problem

1. **No requirement refinement before decompose**: When requirements are vague,
   decompose attempts module splitting on vague input, producing meaningless scope
   judgments and passing vague requirement_refs to impl agents.

2. **impl agent skips project exploration**: On ambiguity, impl either asks the user
   immediately (single mode) or silently does best-effort (parallel mode) — never
   tries to find answers in the project itself.

3. **autodev reimplements spec workflow poorly**: Duplicates decompose + impl dispatch
   with `parallel: true` forced, losing brainstorming, Socratic Loop, and AskUserQuestion.

4. **validate used as workflow dependency**: autodev's dev→validate→spec update loop
   treats validate as a quality gate, but validate checks document-code consistency,
   not requirement quality.

## Design Decisions

### Deliberate Trade-offs

| Decision | Trade-off | Rationale |
|----------|-----------|-----------|
| autodev delegates to full spec workflow | Slower than old parallel-forced mode | Spec quality over speed. The old approach was fast but produced poor specs — that was the root problem. |
| max_rounds = 2 (not 3) | Less self-resolution attempts | Round 1 captures domain context + resolves most items. Round 2 addresses reviewer feedback. A third round rarely resolves what the first two could not. Early termination handles genuinely-ambiguous items without wasting rounds. |
| `--no-ask` flag (not `--user-asked`) | Less granular control | Simpler semantic: "never ask the user." Thin orchestrator should not need to track callee's internal AskUserQuestion budget. |

## Design

### New Agents

#### requirement-explorer

**Role:** Concretize vague requirements through project domain context exploration.

**Tools:** Read, Glob, Grep, Bash (git only), Write

**Session file (Round 1):**

```markdown
# Explore Session
type: explore | round: 1 | project_root: {path}

## User Requirement
{original requirement text}

## Existing Modules Index
{scan-claude-md result}

## Project Conventions
{Conventions or "None"}
```

**Session file (Round 2+):**

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

**Workflow:**

Phase 1 — Domain Context Collection:

| Source | Target | Method |
|--------|--------|--------|
| Project root CLAUDE.md | Purpose, Domain Context, Instructions | Read |
| Existing module CLAUDE.md | Purpose, Domain Context (related modules only) | Read (index-based) |
| Conventions | Terms, patterns, structure rules | From session file |
| Source code | Key types/interfaces/DSL definitions | Grep, Read |
| Config files | Tech stack, dependencies | Read |

Phase 1 output is an intermediate artifact used within the agent's context only.
The final externalized form is Phase 4's `## Domain Context Summary`.

Phase 2 — Domain-Context-Based Ambiguity Assessment:

Evaluate each element of the user requirement against the domain context collected in Phase 1.

| Verdict | Criteria | Handling |
|---------|----------|----------|
| **domain-clear** | Single interpretation within domain context | resolved — cite domain definition |
| **explorable** | Multiple interpretations in domain, but code/history may have answer | Phase 3 target |
| **genuinely-ambiguous** | Cannot resolve even with domain context + project exploration | Record as unresolved |

`total` = number of items assessed in Phase 2. `total == 0` means the explorer found
no elements requiring ambiguity assessment — the requirement is already fully clear
within the domain context. Phase 1 and Phase 4 still execute (Domain Context Summary is produced).

Phase 3 — Exploration of explorable items:

| Order | Source | Method |
|-------|--------|--------|
| 1 | Related module DEVELOPERS.md | Constraints, Public API, Decision Log → Read |
| 2 | Source code | Related function signatures, type definitions, error patterns → Grep, Read |
| 3 | git history | Related keyword commits, recent change patterns → `git log` |

Phase 4 — Write concretized requirements:

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

**Result block:**
```
---explore-result---
result_file: ${TMP_DIR}explore-result-{round}.md
total: N
domain_clear: N
resolved: N
unresolved: N
---end-explore-result---
```

**Constraints:**
- AskUserQuestion prohibited — self-exploration only
- File modification prohibited — read-only exploration + result file Write only
- Bash restricted to git read commands (git log, git show, git diff) — no git stash, checkout, or other state-modifying commands
- Round 2+: focus on reviewer's Critical Questions

---

#### requirement-reviewer

**Role:** Judge whether concretized requirements are sufficient for complete spec definition,
evaluated against the project's domain context.

**Tools:** Read, Glob, Grep, Write

Reviewer has Glob/Grep to **spot-check** explorer's Resolution Log citations against actual
project artifacts. It does not re-run full exploration — it verifies a sample of claimed sources.

**Session file:**

```markdown
# Explore Reviewer Session
type: explore-reviewer | round: {N}
explore_result: ${TMP_DIR}explore-result-{N}.md
original_requirement: ${TMP_DIR}original-requirement.md
```

**Workflow:**

Phase 1 — Load:
- `explore_result` → Concretized Requirements, Domain Context Summary, Resolution Log, Remaining Ambiguities
- `original_requirement` → original requirement text (for faithfulness verification)

Phase 2 — 5-Criteria domain-context-based evaluation:

| Criterion | Question | Rejected when |
|-----------|----------|---------------|
| **Purpose identifiability** | Can the module's reason for existence (business value) be derived from concretized requirements? | Cannot write Purpose in one sentence |
| **Requirements derivability** | Can a verifiable Requirements list be written? (pass/fail determinable) | Key behaviors cannot be expressed as pass/fail |
| **Constraints derivability** | Can input/output/error types be specified? | Core interface signatures cannot be inferred |
| **Domain Context sufficiency** | Do domain terms/rules converge to single interpretation? | Domain terms have 2+ possible interpretations |
| **Resolution soundness** | Are Resolution Log sources grounded in actual project artifacts? Spot-check at least 2 cited sources via Grep/Read. Also verify concretized requirements are faithful to the original — no hallucinated/drifted requirements. | Cited file:line does not contain claimed content, or concretized requirements introduce items not traceable to original |

Phase 3 — Verdict:
- **approved**: All 5 criteria pass, Critical Questions = 0
- **rejected**: Any criterion fails → specific Critical Questions

**Result file:**

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

**Result block:**
```
---explore-reviewer-result---
result_file: ${TMP_DIR}explore-reviewer-result-{round}.md
verdict: approved | rejected
round: {N}
critical_questions: {N}
---end-explore-reviewer-result---
```

**Constraints:**
- AskUserQuestion prohibited
- File modification prohibited — result file Write only
- Spot-check explorer's Resolution Log — reject if cited source does not match
- Verify faithfulness to original requirement — reject if explorer drifted

---

### spec SKILL Changes

#### Insert Step 2.5: Self Socratic Loop

Between existing Step 2 (Read conventions/language) and Step 3 (Create decompose session).

```
round = 1, max_rounds = 2

Write original requirement to ${TMP_DIR}original-requirement.md

loop:
  2.5a. Create ${TMP_DIR}explore-session-{round}.md
        (Round 1: original requirement + index + conventions)
        (Round 2+: + previous result + reviewer feedback)

  2.5b. Task(requirement-explorer):
        Session file: ${TMP_DIR}explore-session-{round}.md
        Save results to ${TMP_DIR} and return only the path

  2.5c. Short-circuit check:
        if explorer reports total == 0 (no ambiguity assessment needed) OR
           (domain_clear + resolved == total, unresolved == 0):
          concretized_requirement = ## Concretized Requirements from explore-result
          domain_context_summary = ## Domain Context Summary from explore-result
          explore_status = "short-circuited"
          break (skip reviewer — requirements already clear)

  2.5d. Early termination check:
        if all unresolved items are genuinely-ambiguous AND no explorable items remain:
          → jump to 2.5h (AskUserQuestion) immediately, skip remaining rounds

  2.5e. Create ${TMP_DIR}explore-reviewer-session-{round}.md

  2.5f. Task(requirement-reviewer):
        Session file: ${TMP_DIR}explore-reviewer-session-{round}.md
        Save results to ${TMP_DIR} and return only the path

  2.5g. if verdict == "approved":
          concretized_requirement = ## Concretized Requirements from explore-result
          domain_context_summary = ## Domain Context Summary from explore-result
          explore_status = "approved"
          break

  2.5h. if round >= max_rounds OR early termination:
          if --no-ask flag is set:
            → use current concretized_requirement as best-effort, explore_status = "best-effort"
            → break

          Summarize reviewer's Critical Questions (or Remaining Ambiguities for early termination)
          → AskUserQuestion (last resort):
            "Requirements concretization attempted but these remain unclear:
             - {Critical Question 1}
             - {Critical Question 2}
             Can you provide specifics?"

          Incorporate user answer → create new explore session → 1 more explorer run
          concretized_requirement = result (no reviewer re-evaluation — user-provided answers are authoritative)
          domain_context_summary = result's ## Domain Context Summary
          explore_status = "user-resolved"
          break

  2.5i. round++ → return to 2.5a

Update state.json:
  explore_round: {final round}
  explore_status: approved | user-resolved | short-circuited | best-effort
```

#### New argument: --no-ask

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `--no-ask` | No | false | Suppress AskUserQuestion in Self Socratic Loop. When set, max_rounds exhaustion uses best-effort instead of asking the user. |

#### Modify Step 3: Pass concretized requirements to decompose

```markdown
## User Requirement
{concretized_requirement}     <- Self Socratic Loop result

## Original Requirement
{original requirement text}   <- preserved for traceability

## Domain Context Summary
{domain_context_summary}      <- propagated from explorer for downstream agents
```

Decompose uses `## User Requirement` (concretized) for scope judgment and module identification.
For `requirement_refs` field in decompose-result.json, decompose extracts from concretized text
(not restricted to original text excerpts — the concretization IS the refined requirement).

#### Step 6 multi-scope: Domain Context Summary propagation

For multi-scope, each module's session file includes the same `## Domain Context Summary`:

```markdown
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
{domain_context_summary}      <- same for all modules, from explorer

## Existing Modules Index
{latest scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

#### impl agent Phase 1.5 conditional skip

When `## Domain Context Summary` is present in the session file AND the explore loop
completed with status `approved` or `short-circuited`, impl agent skips Phase 1.5
(Dependency Exploration). Impl proceeds directly from Phase 1 to Phase 2 or Phase P.

When Domain Context Summary is present but explore_status is `best-effort` or `user-resolved`,
impl runs Phase 1.5 normally — the explorer's context may be incomplete.

Note: explore_status is not directly available in the impl session file. The SKILL encodes
the skip decision: include `## Domain Context Summary` section only when skip is appropriate.
When the SKILL determines Phase 1.5 should run, it omits `## Domain Context Summary` from
the session file.

#### Remove --auto flag and Auto Loop

Delete from spec SKILL:
- `--auto` argument
- `--auto-max-iter` argument
- Phase 0 / Auto Loop / Auto Phase 1-4 sections
- Auto Mode Error Handling table

This is a **BREAKING CHANGE** — requires MAJOR version bump.
Migration path: `/spec --auto "req"` → `/autodev "req"`.

#### state.json additions for /spec-step resume

Add fields to state.json for Self Socratic Loop state:

```json
{
  "explore_round": 0,
  "explore_status": "pending | in-progress | approved | user-resolved | short-circuited | best-effort",
  "explore_result_file": ""
}
```

On session resume via `/spec-step`:
- `explore_status == "pending" | "in-progress"` → Self Socratic Loop not completed.
  spec-step prints: `"Self Socratic Loop was interrupted. Run /spec again to restart."`
  Exit without proceeding. (spec-step does not re-run the explore loop — it only handles
  the plan/review/execute phases.)
- `explore_status == "approved" | "user-resolved" | "short-circuited" | "best-effort"` →
  skip loop, proceed to existing status branching logic.

---

### decompose agent changes

#### Constraint relaxation

**Before:**
```
## Core Constraints
- Original text rewriting prohibited — Only original text excerpts are allowed for requirement_refs
```

**After:**
```
## Core Constraints
- requirement_refs must be direct excerpts from ## User Requirement (which may be
  concretized text from the Self Socratic Loop, not necessarily the user's original words)
```

Similarly in Phase 3 principles:

**Before:**
```
- Direct excerpts from the original text (no rewriting — rewriting is the impl agent's responsibility)
```

**After:**
```
- Direct excerpts from ## User Requirement section (no further rewriting by decompose —
  concretization is the explorer agent's responsibility)
```

#### New session file section

Decompose consumes `## Domain Context Summary` to inform module identification
(Phase 2 step 4: "Determine paths"). Domain terms help identify natural module boundaries.

---

### autodev Redesign — Thin Orchestrator

```markdown
# /autodev

Autonomously executes requirements from start to finish.
Orchestrates spec (spec definition) and dev (code generation) as a pipeline.

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| requirement | Yes* | - | Requirement text |
| --path | No | . | Target path |

* If missing, collected once via AskUserQuestion.

## AskUserQuestion Budget

autodev permits at most 1 AskUserQuestion total across the entire workflow:
- Either in Step 1 (requirement collection when missing)
- Or in spec's Self Socratic Loop last-resort (when max_rounds exhausted)
- NOT both.

Implementation: when Step 1 uses AskUserQuestion, autodev passes `--no-ask` to spec.
When Step 1 is skipped (requirement provided), spec runs without `--no-ask` and may
use its last-resort AskUserQuestion if needed.

## Workflow

### Step 1: Requirement Collection
If no requirement provided, AskUserQuestion once:
  "What feature would you like to implement? Briefly describe core behavior and target path."
  Set no_ask = true.

### Step 2: Init
CLI_PATH, TMP_DIR setup.

### Step 3: Spec
Skill("claude-md-plugin:spec", args: "{requirement} --path {impl_path} {--no-ask if no_ask}")

spec internally runs: Self Socratic Loop -> decompose -> Socratic Loop -> execute.

Check spec-result status. If failed or cancelled → exit with error.

### Step 4: Dev
Skill("claude-md-plugin:dev", args: "--conflict overwrite --path {impl_path}")

Check dev-result status. If failed → exit with warning.

### Step 5: Result Report

Success:
  checkmark autodev complete
    spec: CLAUDE.md + DEVELOPERS.md generated
    dev:  Code generation complete

  git diff --stat

Failure (spec or dev failed):
  warning autodev terminated (reason: {reason})
    Resolve manually with /spec or /dev.

## Error Handling

| Situation | Response |
|-----------|----------|
| No requirement | AskUserQuestion once in Step 1 |
| spec failed | Report error, exit |
| spec cancelled by user | Report cancellation, exit |
| dev failed | Report warning, show partial results |
```

**Removed:**
- Direct decompose invocation (spec handles it)
- Forced parallel mode (spec handles scope-appropriate mode)
- validate dependency loop (entire Auto Phase 1/2/3/4)
- `--max-iter` argument

---

### spec --auto Migration

| Before | After |
|--------|-------|
| `/spec --auto "req"` | `/autodev "req"` |
| `/spec --auto --auto-max-iter 5 "req"` | `/autodev "req"` (no max-iter needed, no validate loop) |

This is a **BREAKING CHANGE**. Users of `--auto` must switch to `/autodev`.

---

## Affected Files

| File | Change | Details |
|------|--------|---------|
| `agents/requirement-explorer.md` | **New** | Domain-context exploration agent |
| `agents/requirement-reviewer.md` | **New** | Requirement concreteness reviewer agent (with Glob/Grep for spot-check) |
| `skills/spec/SKILL.md` | **Modify** | Insert Step 2.5 (Self Socratic Loop), add `--no-ask` flag, remove `--auto`/`--auto-max-iter`, add state.json fields for explore state |
| `commands/autodev.md` | **Rewrite** | Thin orchestrator with `--no-ask` delegation |
| `commands/spec-step.md` | **Modify** | Add guard for `explore_status == "pending" | "in-progress"` → print message + exit. No new status branching logic needed. |
| `agents/decompose.md` | **Modify** | (1) Relax Core Constraint: `requirement_refs` excerpts from `## User Requirement` (may be concretized) (2) Phase 3 principle: "Direct excerpts from ## User Requirement" (3) Consume `## Domain Context Summary` in Phase 2 |
| `agents/impl.md` | **Modify** | Conditional skip of Phase 1.5 when `## Domain Context Summary` present (SKILL controls inclusion based on explore_status) |
| `CLAUDE.md` | **Update** | (1) Agent table: add requirement-explorer, requirement-reviewer (2) Skills table: remove `--auto` from `/spec` description, add `--no-ask` (3) Architecture: update /spec diagram — add Self Socratic Loop before decompose (4) Commands: update `/autodev` description — "Skill(spec) + Skill(dev) orchestrator", remove validate loop description (5) Remove all validate-dependency references from autodev/spec descriptions |
| `.claude-plugin/plugin.json` | **Update** | MAJOR version bump (--auto removal is breaking), register 2 new agents |

## Invariants

No new invariants introduced. Existing invariants preserved:
- INV-1 (Tree Structure) — unchanged, decompose still validates
- INV-4 (Update Responsibility) — /spec still owns CLAUDE.md + DEVELOPERS.md
- Session File Pattern — explorer and reviewer follow the same SKILL→session→Agent pattern

Changes to existing invariant interpretations:
- decompose's `requirement_refs` now excerpts from concretized text (not original).
  This is consistent with the intent — concretized text IS the refined requirement.

## Review Feedback Addressed

### Round 1

| ID | Issue | Resolution |
|----|-------|------------|
| C1 | Concretized text conflicts with decompose's "original text only" constraint | Relaxed decompose constraint — concretized text is the refined requirement. Added decompose to Affected Files. |
| C2 | CLAUDE.md still describes validate dependency | Expanded CLAUDE.md change details — 5 specific areas to update. |
| C3 | Reviewer cannot verify Resolution Log citations | Added Glob, Grep to reviewer tools. Criterion 5 now requires spot-checking at least 2 cited sources. |
| C4 | CLAUDE.md --auto reference not in Affected Files | Explicitly listed in CLAUDE.md change details. |
| M1 | Explorer and impl do overlapping dependency exploration | impl skips Phase 1.5 when Domain Context Summary is present. |
| M2 | Decompose session format conflict | Same as C1. Also specified Domain Context Summary propagation. |
| M3 | genuinely-ambiguous items waste retry rounds | Added early termination at Step 2.5d — skip to AskUserQuestion when only genuinely-ambiguous items remain. |
| M4 | autodev single mode allows AskUserQuestion in spec | Added AskUserQuestion budget (max 1 total). `--no-ask` flag prevents double-asking. |
| M5 | Reviewer lacks access to original requirement | Added `original_requirement` path to reviewer session file. Criterion 5 now checks faithfulness. |
| M6 | decompose not in Affected Files | Added to Affected Files table. |
| Q1 | Clear requirements still require reviewer round | Added short-circuit at Step 2.5c — skip reviewer when 0 ambiguities. |
| Q3 | /spec-step resume lacks explore state | Added explore_round, explore_status, explore_result_file to state.json. |
| Q5 | --auto removal is breaking change | Marked as BREAKING CHANGE, specified MAJOR version bump. |
| m1 | Naming inconsistency (-{N} vs -v{N}) | Unified to `-{round}` (no v prefix). |
| m2 | Phase 1 output format unclear | Clarified as intermediate artifact; Phase 4's Domain Context Summary is the externalized form. |
| m4 | Domain Context Summary not propagated | Added propagation to decompose and impl session files. |

### Round 2

| ID | Issue | Resolution |
|----|-------|------------|
| NC1 | decompose constraint replacement wording not specified | Added concrete before/after text for both Core Constraint and Phase 3 principles. |
| NC2 | `--user-asked` flag not implementable / violates thin orchestrator | Replaced with `--no-ask` flag — simpler semantic, no internal budget tracking needed. |
| NC3 | spec-step.md not in Affected Files | Added to Affected Files. spec-step gets a guard that prints "Run /spec again" for incomplete explore states. |
| NM1 | Domain Context Summary multi-scope propagation not detailed | Added explicit multi-scope session file format showing `## Domain Context Summary` inclusion. |
| NM2/NM5 | Explorer Bash "git only" not enforceable at tool level | Accepted as instruction-level constraint (consistent with 7 existing agents using Bash). Added explicit Bash restriction wording: "git log, git show, git diff only". |
| NM3 | Short-circuit `total == 0` semantics ambiguous | Clarified in Phase 2 description: "total == 0 means no elements requiring ambiguity assessment." |
| NM4 | autodev speed regression vs old parallel mode | Added Design Decisions section with explicit trade-off rationale. |
| DC2 | max_rounds = 3 excessive | Reduced to max_rounds = 2 with rationale. |
| DC3 | `--user-asked` creates implicit coupling | Replaced with `--no-ask` (see NC2). |
| DC4 | Poor Domain Context Summary suppresses impl re-exploration | SKILL controls Domain Context Summary inclusion based on explore_status: only approved/short-circuited skip Phase 1.5. best-effort/user-resolved do not skip. |
| Nm4 | Version bump location: `core/plugin.json` → `.claude-plugin/plugin.json` | Fixed in Affected Files table. |
