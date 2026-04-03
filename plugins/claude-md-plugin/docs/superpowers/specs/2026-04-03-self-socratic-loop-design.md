# Self Socratic Loop Design

## Summary

Add a Self Socratic Loop to `/spec` SKILL that concretizes vague requirements
through project context exploration before decompose/impl.
Redesign `/autodev` as a thin orchestrator calling `Skill(spec) + Skill(dev)`.
Remove validate dependency from all workflows.

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
feedback_file: ${TMP_DIR}explore-reviewer-result-v{N-1}.md

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

Output:
```
---domain-context---
domain: {project domain summary}
key_terms:
  - "{term}": "{specific meaning in this project}"
existing_patterns:
  - "{pattern}": "{how it is used in this project}"
tech_stack: {language, framework, key libraries}
---end-domain-context---
```

Phase 2 — Domain-Context-Based Ambiguity Assessment:

| Verdict | Criteria | Handling |
|---------|----------|----------|
| **domain-clear** | Single interpretation within domain context | resolved — cite domain definition |
| **explorable** | Multiple interpretations in domain, but code/history may have answer | Phase 3 target |
| **genuinely-ambiguous** | Cannot resolve even with domain context + project exploration | Record as unresolved |

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
{domain context summary from Phase 1}

## Concretized Requirements
{concretized requirements — preserve original structure, replace ambiguous parts with domain-context-based specifics}

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
- Round 2+: focus on reviewer's Critical Questions

---

#### requirement-reviewer

**Role:** Judge whether concretized requirements are sufficient for complete spec definition,
evaluated against the project's domain context.

**Tools:** Read, Write

**Session file:**

```markdown
# Explore Reviewer Session
type: explore-reviewer | round: {N}
explore_result: ${TMP_DIR}explore-result-{N}.md
```

**Workflow:**

Phase 1 — Load explore result (Concretized Requirements, Domain Context Summary,
Resolution Log, Remaining Ambiguities).

Phase 2 — 5-Criteria domain-context-based evaluation:

| Criterion | Question | Rejected when |
|-----------|----------|---------------|
| **Purpose identifiability** | Can the module's reason for existence (business value) be derived from concretized requirements? | Cannot write Purpose in one sentence |
| **Requirements derivability** | Can a verifiable Requirements list be written? (pass/fail determinable) | Key behaviors cannot be expressed as pass/fail |
| **Constraints derivability** | Can input/output/error types be specified? | Core interface signatures cannot be inferred |
| **Domain Context sufficiency** | Do domain terms/rules converge to single interpretation? | Domain terms have 2+ possible interpretations |
| **Resolution soundness** | Are Resolution Log sources grounded in actual project artifacts? | Relies on unsupported inference or assumptions |

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

## Critical Questions (when rejected)
- {criterion}: "{specific problem — which item is insufficient and why}"
```

**Result block:**
```
---explore-reviewer-result---
result_file: ${TMP_DIR}explore-reviewer-result-v{round}.md
verdict: approved | rejected
round: {N}
critical_questions: {N}
---end-explore-reviewer-result---
```

**Constraints:**
- AskUserQuestion prohibited
- File modification prohibited — result file Write only
- Do not blindly trust explorer's Resolution Log — reject if evidence is weak

---

### spec SKILL Changes

#### Insert Step 2.5: Self Socratic Loop

Between existing Step 2 (Read conventions/language) and Step 3 (Create decompose session).

```
round = 1, max_rounds = 3

Write original requirement to ${TMP_DIR}original-requirement.md

loop:
  2.5a. Create ${TMP_DIR}explore-session-{round}.md
        (Round 1: original requirement + index + conventions)
        (Round 2+: + previous result + reviewer feedback)

  2.5b. Task(requirement-explorer):
        Session file: ${TMP_DIR}explore-session-{round}.md
        Save results to ${TMP_DIR} and return only the path

  2.5c. Create ${TMP_DIR}explore-reviewer-session-v{round}.md

  2.5d. Task(requirement-reviewer):
        Session file: ${TMP_DIR}explore-reviewer-session-v{round}.md
        Save results to ${TMP_DIR} and return only the path

  2.5e. if verdict == "approved":
          concretized_requirement = ## Concretized Requirements from explore-result
          break

  2.5f. if round >= max_rounds:
          Summarize reviewer's Critical Questions → AskUserQuestion (last resort):
            "Requirements concretization attempted but these remain unclear:
             - {Critical Question 1}
             - {Critical Question 2}
             Can you provide specifics?"

          Incorporate user answer → create new explore session → 1 more explorer run
          concretized_requirement = result (no reviewer re-evaluation — user-provided answers are authoritative)
          break

  2.5g. round++ → return to 2.5a
```

#### Modify Step 3: Pass concretized requirements to decompose

```markdown
## User Requirement
{concretized_requirement}     <- Self Socratic Loop result

## Original Requirement
{original requirement text}   <- preserved for traceability
```

Same change propagated to Step 6 impl session files.

#### Remove --auto flag and Auto Loop

Delete from spec SKILL:
- `--auto` argument
- `--auto-max-iter` argument
- Phase 0 / Auto Loop / Auto Phase 1-4 sections
- Auto Mode Error Handling table

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

## Workflow

### Step 1: Requirement Collection
If no requirement provided, AskUserQuestion once:
  "What feature would you like to implement? Briefly describe core behavior and target path."

### Step 2: Init
CLI_PATH, TMP_DIR setup.

### Step 3: Spec
Skill("claude-md-plugin:spec", args: "{requirement} --path {impl_path}")

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

---

## Affected Files

| File | Change |
|------|--------|
| `agents/requirement-explorer.md` | **New** — domain-context exploration agent |
| `agents/requirement-reviewer.md` | **New** — requirement concreteness reviewer agent |
| `skills/spec/SKILL.md` | **Modify** — insert Step 2.5, remove --auto |
| `commands/autodev.md` | **Rewrite** — thin orchestrator |
| `CLAUDE.md` | **Update** — agent table, skill table, architecture diagram |
| `core/plugin.json` | **Update** — version bump, register new agents |

## Invariants

No new invariants introduced. Existing invariants preserved:
- INV-1 (Tree Structure) — unchanged, decompose still validates
- INV-4 (Update Responsibility) — /spec still owns CLAUDE.md + DEVELOPERS.md
- Session File Pattern — explorer and reviewer follow the same SKILL→session→Agent pattern
