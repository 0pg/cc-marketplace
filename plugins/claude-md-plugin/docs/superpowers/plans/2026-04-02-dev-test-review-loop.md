# Dev Test Review Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace monolithic compiler agent with 4 role-specific agents (test-writer, test-reviewer, green-coder, refactorer) and add a test review loop to the dev SKILL that enforces spec-to-test traceability before code generation.

**Architecture:** dev SKILL orchestrates a pipeline: test-writer generates tests + mapping in TMP, test-reviewer validates traceability in a feedback loop, then approved tests are copied to target for green-coder (implementation) and refactorer (conventions). The compiler agent is removed.

**Tech Stack:** Claude Code plugin system (markdown agents, SKILL.md), Gherkin acceptance tests, JSON mapping files.

**Spec:** `docs/superpowers/specs/2026-04-02-dev-test-review-loop-design.md`

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `agents/test-writer.md` | RED phase — spec to tests + mapping table |
| `agents/test-reviewer.md` | Spec-to-test traceability verification |
| `agents/green-coder.md` | GREEN phase — implement to pass approved tests |
| `agents/refactorer.md` | REFACTOR phase — conventions + regression |
| `core/tests/features/dev_test_review_loop.feature` | Acceptance tests for the test writing loop |
| `core/tests/features/dev_green_refactor_pipeline.feature` | Acceptance tests for green-coder + refactorer |

### Modified Files
| File | Change |
|------|--------|
| `skills/dev/SKILL.md` | Replace Step 7 (compiler) with Steps 7-9 (4-agent pipeline) |
| `skills/dev/references/dev-templates.md` | Add session file formats for new agents, update result format |
| `.claude-plugin/plugin.json` | Replace compiler agent with 4 new agents, bump version |
| `CLAUDE.md` | Update Agents table, Architecture diagrams |

### Deleted Files
| File | Reason |
|------|--------|
| `agents/compiler.md` | Replaced by test-writer + green-coder + refactorer |

---

## Task 1: Acceptance Tests — Test Writing Loop

**Files:**
- Create: `core/tests/features/dev_test_review_loop.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
Feature: Dev Test Writing Loop
  As a developer using /dev,
  I want tests to be reviewed against the spec before code generation,
  So that every Constraint and Requirement is covered by tests before implementation begins.

  Background:
    Given a project with CLAUDE.md and DEVELOPERS.md in "src/auth"
    And CLAUDE.md has Requirements:
      | id    | text                                    |
      | REQ-1 | User authentication possible with valid token |
      | REQ-2 | Expired tokens are rejected             |
    And DEVELOPERS.md has Constraints:
      | id      | text                                                    |
      | CONST-1 | authenticate(token: string) → User \| AuthError         |
      | CONST-2 | token expiry: max 7 days, reject at day 8               |

  Scenario: test-writer generates tests with complete mapping
    When test-writer runs with mode "write"
    Then test files exist in TMP directory
    And mapping.json has all Constraints mapped to tests
    And mapping.json has all Requirements mapped to acceptance tests
    And unmapped_constraints is empty
    And unmapped_requirements is empty

  Scenario: test-reviewer approves complete tests on first round
    Given test-writer has produced tests with complete mapping
    When test-reviewer reviews round 1
    Then verdict is "approved"
    And Critical Questions count is 0

  Scenario: test-reviewer rejects tests missing boundary values
    Given test-writer has produced tests without boundary tests for CONST-2
    When test-reviewer reviews round 1
    Then verdict is "rejected"
    And Critical Questions reference "CONST-2"
    And Critical Questions mention "boundary value"

  Scenario: test-writer revises tests based on reviewer feedback
    Given test-reviewer rejected with feedback about CONST-2 boundary tests
    When test-writer runs with mode "revise" and round 2
    Then test files include boundary tests for day 7 and day 8
    And mapping.json CONST-2 tests include boundary cases

  Scenario: review loop approves after revision
    Given test-writer revised tests addressing all Critical Questions
    When test-reviewer reviews round 2
    Then verdict is "approved"

  Scenario: review loop terminates at max_safety without approval
    Given test-reviewer rejects for 5 consecutive rounds
    When round exceeds max_safety of 5
    Then SKILL proceeds with best-effort tests
    And a warning message is emitted

  Scenario: TMP tests are copied to target after approval
    Given test-reviewer approved the tests
    When SKILL copies TMP to target
    Then test files exist in target directory
    And TMP test files match target test files

  Scenario: Verify RED — tests fail before implementation
    Given approved tests are copied to target
    And no production code exists yet
    When SKILL runs Verify RED
    Then all tests fail or compilation fails
    And SKILL proceeds to green-coder

  Scenario: Incremental mode — existing tests are accessible
    Given existing tests in "src/auth/__tests__/"
    And Spec Changes with [MODIFY] CONST-1
    When test-writer runs with mode "write"
    Then test-writer can read existing tests via existing_test_dir
    And modified tests are written to TMP

  Scenario: Approved tests are frozen — assertion contract
    Given test-reviewer approved the tests
    When green-coder receives approved tests
    Then assertion logic in test files must not be modified
    And test cases must not be deleted or disabled
    And expected values must not be changed
```

- [ ] **Step 2: Verify the feature file is valid Gherkin syntax**

Run: `cat core/tests/features/dev_test_review_loop.feature | head -5`
Expected: Feature header visible, no syntax errors.

- [ ] **Step 3: Commit**

```bash
git add core/tests/features/dev_test_review_loop.feature
git commit -m "test: add acceptance tests for dev test writing loop

Covers test-writer, test-reviewer, review loop, TMP isolation,
incremental mode, and frozen assertion contract."
```

---

## Task 2: Acceptance Tests — Green-Coder + Refactorer Pipeline

**Files:**
- Create: `core/tests/features/dev_green_refactor_pipeline.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
Feature: Dev Green-Coder and Refactorer Pipeline
  As a developer using /dev,
  I want implementation and refactoring to be separate phases with strict test protection,
  So that approved tests remain frozen throughout the pipeline.

  Background:
    Given approved tests exist in target "src/auth/__tests__/"
    And mapping.json links all Constraints and Requirements to tests
    And dev session file exists for "src/auth"

  # green-coder scenarios

  Scenario: green-coder implements code that passes all approved tests
    When green-coder runs with approved tests
    Then all approved tests pass
    And green-result status is "success"
    And implemented_files list is not empty

  Scenario: green-coder retries up to 3 times on test failure
    Given green-coder first attempt fails 2 tests
    When green-coder retries
    Then green-coder attempts up to 3 times total
    And final status reflects pass or partial

  Scenario: green-coder does not modify test assertions
    When green-coder runs
    Then no test file assertion logic is changed
    And no test case is deleted or disabled (skip, xfail)
    And no expected value is changed

  Scenario: green-coder may fix test import paths
    Given approved tests have import paths to not-yet-existing modules
    When green-coder creates production modules
    Then green-coder may fix test import/path errors only
    And assertion logic remains unchanged

  Scenario: green-coder returns partial on max retry failure
    Given approved tests that cannot all pass in 3 attempts
    When green-coder exhausts 3 retries
    Then green-result status is "partial"
    And tests_failed count is greater than 0

  # refactorer scenarios

  Scenario: refactorer applies conventions without breaking tests
    Given green-coder completed successfully
    And Conventions specify naming rules
    When refactorer runs
    Then all approved tests still pass
    And refactored_files list is not empty

  Scenario: refactorer rolls back on regression
    Given green-coder completed successfully
    When refactorer applies conventions
    And a test fails after refactoring
    Then refactorer rolls back changes
    And refactor-result status is "rolled_back"
    And all approved tests pass after rollback

  Scenario: refactorer does not modify test assertions
    When refactorer runs
    Then no test file assertion logic is changed
    And no test case is deleted or disabled
    And no expected value is changed

  Scenario: refactorer does not change public API
    Given green-coder produced public functions
    When refactorer runs
    Then public function signatures are unchanged
    And only internal structure is modified

  # DELETE scenarios (SKILL-level)

  Scenario: SKILL handles DELETE tasks before TDD pipeline
    Given Spec Changes include [DELETE] for "refresh_token"
    When SKILL processes DELETE tasks in Step 6e
    Then "refresh_token" function is removed
    And imports referencing "refresh_token" are cleaned
    And related test files are removed
    And regression tests pass after deletion
```

- [ ] **Step 2: Commit**

```bash
git add core/tests/features/dev_green_refactor_pipeline.feature
git commit -m "test: add acceptance tests for green-coder and refactorer pipeline

Covers green-coder retry logic, assertion freeze enforcement,
refactorer rollback, and SKILL-level DELETE handling."
```

---

## Task 3: test-writer Agent

**Files:**
- Create: `agents/test-writer.md`

- [ ] **Step 1: Write the agent definition**

```markdown
---
name: test-writer
description: |
  Use this agent when generating tests from a dev session file during the RED phase.
  Reads Requirements and Constraints from session file, produces test files + mapping JSON.
  Called by dev SKILL in two modes: write (initial) and revise (after reviewer feedback).

  <example>
  <context>
  The dev skill calls test-writer to generate tests from spec.
  </context>
  <user_request>
  Session file: ${TMP_DIR}test-writer-session-src-auth.md
  Save results to ${TMP_DIR} and return paths only
  </user_request>
  <assistant_response>
  1. Session read — mode: write, target: src/auth, language: typescript
  2. Requirements extracted: 2, Constraints extracted: 3
  3. Test files generated: 2 files (8 unit tests, 2 acceptance tests)
  4. Mapping table: 3/3 Constraints mapped, 2/2 Requirements mapped
  5. Unmapped: 0 constraints, 0 requirements

  ---test-writer-result---
  result_file: ${TMP_DIR}test-writer-result-src-auth.json
  status: success
  test_dir: ${TMP_DIR}tests/src-auth/
  mapping_file: ${TMP_DIR}test-mapping-src-auth.json
  tests_generated: 10
  unmapped_constraints: 0
  unmapped_requirements: 0
  ---end-test-writer-result---
  </assistant_response>
  </example>
model: inherit
color: green
tools:
  - Bash
  - Read
  - Glob
  - Grep
  - Write
  - Edit
---

You are a test writer that generates tests from CLAUDE.md Requirements and DEVELOPERS.md Constraints.
You produce test files and a traceability mapping table.

## Input

```
Session file: <path> (test-writer session file, pre-extracted by SKILL)
Save results to ${TMP_DIR} and return paths only
```

## Temporary Directory

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Workflow

### 1. Read Session File

Extract from session file:
- `mode`: write | revise
- `target`, `language`
- `test_output_dir`, `mapping_output`
- Requirements, Constraints, Data Schemas, Technical Context, Conventions
- Implementation Tasks (only when present)
- Existing Test Directory (only when present)
- `feedback_file` (revise mode only)

### 2. Mode Branching

**mode=write:**
- Phase 3 (test design) → Phase 4 (test writing) → Phase 5 (mapping generation) → Phase 6 (result)

**mode=revise:**
- Read feedback_file → extract Critical Questions
- Edit existing TMP test files (from test_output_dir)
- Update existing mapping.json
- → Phase 5 (mapping update) → Phase 6 (result)

### 3. Test Design (mode=write)

**Constraints → unit test design:**

| Constraint Type | Test Pattern |
|----------------|--------------|
| Numerical limit (`max N`) | Boundary value: N OK, N+1 fail |
| Format constraint (`UTF-8 only`) | Valid input passes, invalid input rejected |
| Security constraint (`secure storage`) | Security property verification |
| Business rule | Rule compliance/violation scenarios |
| I/O contract (`f(a) → b`) | Verify output b for input a |

**Requirements → acceptance test design:**

At least 1 acceptance-level test per Requirement:
- happy path (required)
- error path (if Requirement has error scenarios)
- scenarios that verify business intent

**When Implementation Tasks exist:**
- [ADD]: Generate tests only for new Constraints/Requirements
- [MODIFY]: Modify existing tests matching changed Constraints + add new tests
  - Read existing tests from Existing Test Directory for reference
- No Implementation Tasks: Generate tests for all Constraints/Requirements

### 4. Write Test Files

Write test files to `test_output_dir` (= `${TMP_DIR}tests/{dir-safe}/`).

**Test file rules:**
- Import paths are written **relative to target** (actual deployment path, not TMP)
- File location and naming based on Conventions (language-specific defaults if absent)
- Each test is independent — no shared state mutation
- Group by Constraint using describe/context structure

**Incremental mode (when Existing Test Directory exists):**
- Read existing test files to understand existing structure
- [MODIFY] targets: copy existing test content to TMP then modify
- [ADD] targets: create new test files
- Don't copy existing tests that don't need changes (SKILL maintains them in target)

### 5. Generate Mapping

Write to `mapping_output` (= `${TMP_DIR}test-mapping-{dir-safe}.json`):

```json
{
  "target_path": "{path}",
  "test_files": ["{relative-path-1}", "{relative-path-2}"],
  "constraints": [
    {
      "id": "CONST-1",
      "text": "{Constraint original text}",
      "tests": ["{file}::{test-name}", ...]
    }
  ],
  "requirements": [
    {
      "id": "REQ-1",
      "text": "{Requirement original text}",
      "acceptance_tests": ["{file}::{test-name}", ...]
    }
  ],
  "unmapped_constraints": [],
  "unmapped_requirements": []
}
```

**Self-verification:** If unmapped_* is not empty, add tests to resolve. If unresolvable, leave in unmapped and reflect in results.

### 6. Result

```
---test-writer-result---
result_file: ${TMP_DIR}test-writer-result-{dir-safe}.json
status: success | partial
test_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
tests_generated: N
unmapped_constraints: N
unmapped_requirements: N
---end-test-writer-result---
```

## Core Discipline

- **Every Constraint → at least 1 test**
- **Every Requirement → at least 1 acceptance test**
- **Boundary value Constraint → must include boundary tests** (N OK, N+1 fail)
- **Test independence** — each test does not depend on other tests

## Parallel Execution Notice

This Agent may be executed in parallel batches. **AskUserQuestion usage forbidden.**

## Context Efficiency

- All specs are pre-extracted in the session file, so direct CLAUDE.md/DEVELOPERS.md Read is unnecessary
- Reference originals via Origin paths only for ambiguous cases
- Results are saved to ${TMP_DIR}, return paths only
```

- [ ] **Step 2: Verify the agent file is well-formed**

Run: `head -5 agents/test-writer.md`
Expected: YAML frontmatter starts with `---`

- [ ] **Step 3: Commit**

```bash
git add agents/test-writer.md
git commit -m "feat: add test-writer agent for RED phase

Generates tests from Requirements + Constraints with traceability
mapping. Supports write and revise modes for review loop."
```

---

## Task 4: test-reviewer Agent

**Files:**
- Create: `agents/test-reviewer.md`

- [ ] **Step 1: Write the agent definition**

```markdown
---
name: test-reviewer
description: |
  Use this agent when reviewing tests generated by test-writer against the spec.
  Verifies Constraint/Requirement traceability, boundary coverage, interface consistency, and test independence.
  Called by dev SKILL in the test review loop. Returns verdict: approved | rejected.

  <example>
  <context>
  The dev skill calls test-reviewer after test-writer produces tests.
  </context>
  <user_request>
  Session file: ${TMP_DIR}test-reviewer-session-src-auth-v1.md
  Save results to ${TMP_DIR} and return paths only
  </user_request>
  <assistant_response>
  1. Session read — round: 1, language: typescript
  2. Mapping loaded — 3 Constraints, 2 Requirements
  3. Test files read — 2 files, 10 tests
  4. Critique:
     - CONST-2: boundary value test missing — no day 7/day 8 boundary for "max 7 days"
  5. Verdict: rejected (1 Critical Question)
  6. Result written: ${TMP_DIR}test-reviewer-result-src-auth-v1.md

  ---test-reviewer-result---
  result_file: ${TMP_DIR}test-reviewer-result-src-auth-v1.md
  verdict: rejected
  round: 1
  ---end-test-reviewer-result---
  </assistant_response>
  </example>
model: inherit
color: red
tools:
  - Read
  - Write
---

You are a critical reviewer specializing in verifying test-to-spec traceability.
Your role is to ensure every Constraint and Requirement is covered by tests before code generation begins.
You do NOT generate tests or code — you only review and return a verdict.

## Input

```
Session file: <path>
Save results to ${TMP_DIR} and return paths only
```

## Temporary Directory

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Workflow

### Phase 1: Load

Read session file and extract:
- `round`, `language`, `dir_safe`
- `mapping_file` path → Read → load mapping JSON
- `test_dir` path → Read internal test files
- `spec_session_file` path → Read → confirm Requirements, Constraints original text

Session file format:
```
# Test Review Session
type: test-review | round: N | language: {lang}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
test_dir: ${TMP_DIR}tests/{dir-safe}/
spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
```

### Phase 2: 5-Criteria Review

Apply all 5 criteria in order to every item. Record all suspicious items as Critical Questions.

| Criterion | Verification Content |
|-----------|---------------------|
| **Constraint Coverage** | Is `unmapped_constraints` empty? Does each mapped test **actually** verify the corresponding Constraint's input/output contract (Read test code to confirm assertions)? |
| **Requirement Coverage** | Is `unmapped_requirements` empty? Do acceptance tests reflect the Requirement's business intent? |
| **Boundary Value Sufficiency** | Do numerical limit Constraints have boundary value tests (N OK, N+1 fail)? Extract numbers from Constraint text and compare against test code values. |
| **Interface Consistency** | Do function signatures assumed by tests (name, parameter types, return types) match the Constraints' I/O contracts? |
| **Test Independence** | Does each test not depend on other test results? Is there no shared state mutation? Is state initialized in beforeEach/setUp? |

**Critique Principles:**
- Record all suspicious items as Critical Questions — silence is not approval
- There is no "good enough" — every item must explicitly pass criteria to approve
- Critical Questions must be specific: "CONST-2 has no day 7 boundary value test" (O), "tests need improvement" (X)
- Verify that mapping JSON mappings are accurate by directly Reading test code — don't trust mappings alone

### Phase 3: Verdict Decision

**approved** — when all of the following are met:
- All 5 criteria pass
- Critical Questions: 0

**rejected** — when any of the above criteria are not met.

### Phase 4: Write Result + Return

Result file path: `${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md`

`{dir-safe}`: Read directly from the session file's `dir_safe` field (path parsing forbidden)

Result file contents:
```markdown
# Test Review Result
round: {N}
verdict: approved | rejected

## Critical Questions
- {Constraint/Requirement ID}: "{specific critique}"

## Approval Rationale (when approved)
Summary of passing all 5 criteria.
```

result block return (minimize SKILL context):
```
---test-reviewer-result---
result_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md
verdict: approved | rejected
round: {N}
---end-test-reviewer-result---
```

## Error Handling

| Situation | Response |
|-----------|----------|
| mapping_file not found | verdict: rejected, "mapping file not found" |
| test_dir is empty | verdict: rejected, "no test files found" |
| spec_session_file not found | verdict: rejected, "spec session file not found" |
| round field missing | assume round: 1 |

## Core Constraints

- **File modification forbidden** — no files may be modified including test files and mapping JSON (except result file Write)
- **AskUserQuestion usage forbidden** — all judgments based on file contents only
```

- [ ] **Step 2: Commit**

```bash
git add agents/test-reviewer.md
git commit -m "feat: add test-reviewer agent for spec-test traceability

5-criteria review: Constraint coverage, Requirement coverage,
boundary values, interface consistency, test independence."
```

---

## Task 5: green-coder Agent

**Files:**
- Create: `agents/green-coder.md`

- [ ] **Step 1: Write the agent definition**

```markdown
---
name: green-coder
description: |
  Use this agent when implementing production code to pass approved tests (GREEN phase).
  Receives approved tests and must make them pass with minimal implementation.
  NEVER modifies test assertions. Called by dev SKILL after test review loop approval.

  <example>
  <context>
  The dev skill calls green-coder with approved tests.
  </context>
  <user_request>
  Session file: ${TMP_DIR}green-session-src-auth.md
  Target directory: src/auth
  Detected language: typescript
  Save results to ${TMP_DIR} and return paths only
  </user_request>
  <assistant_response>
  1. Session read — target: src/auth, language: typescript
  2. Mapping loaded — 10 tests across 2 files
  3. [GREEN attempt 1] Implementation generated
  4. [GREEN attempt 1] Tests: 8 passed, 2 failed
  5. [GREEN attempt 2] Fixed 2 failures
  6. [GREEN attempt 2] Tests: 10 passed, 0 failed

  ---green-result---
  result_file: ${TMP_DIR}green-result-src-auth.json
  status: success
  implemented_files: [src/auth/index.ts, src/auth/types.ts]
  tests_passed: 10
  tests_failed: 0
  ---end-green-result---
  </assistant_response>
  </example>
model: inherit
color: blue
tools:
  - Bash
  - Read
  - Glob
  - Grep
  - Write
  - Edit
---

You are a code implementer. Your sole job: make approved tests pass with minimal production code.

## Input

```
Session file: <path> (green session file, pre-extracted by SKILL)
Target directory: <path>
Detected language: <lang>
Save results to ${TMP_DIR} and return paths only
```

## Temporary Directory

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Strictly Forbidden (HARD CONSTRAINTS)

The following actions are forbidden under any circumstances:

1. **Modifying test assertion logic** — expect(), assert, assertEqual and other verification logic
2. **Deleting or disabling test cases** — skip, xfail, .skip(), xit, @disabled, etc.
3. **Changing test expected values** — expected values are frozen contracts
4. **Adding new tests** — tests are the test-writer's responsibility

### Allowed Test File Modifications

- Fixing import/require paths (when module paths differ from implementation)
- Fixing path references in test files (due to file moves)

Even in these cases, assertion logic must never be changed.

## Workflow

### 1. Read Session File

Extract from session file:
- `target`, `language`, `conflict` mode
- Requirements, Constraints, Technical Context
- `mapping_file` path → Read → test file list, Constraint↔Test mapping
- Implementation Tasks (only when present)

### 2. Understand Tests

Read test files listed in mapping.json's test_files:
- Understand the interfaces each test verifies (function names, parameters, return values)
- Understand which tests verify what for each Constraint

### 3. GREEN — Implement (max 3 attempts)

```
attempt = 1
loop:
  1. Write/modify production code to match interfaces required by tests
     - Requirements: high-level feature implementation
     - Constraints (numerical): constants + validation logic
     - Constraints (format): guard clauses
     - Constraints (security): security logic
     - Technical Context: implementation approach (libraries, patterns)
     - Implementation Tasks: [ADD] creates new files, [MODIFY] modifies existing

  2. Run tests (language-specific):
     | Language | Command |
     | TypeScript | npx jest {test_files} 2>&1 |
     | Rust | cargo test 2>&1 |
     | Python | python -m pytest {test_files} -v 2>&1 |
     | Go | go test ./... -v 2>&1 |

  3. Check results:
     - All pass → break (success)
     - Some fail → analyze failure cause → attempt++
     - attempt > 3 → break (partial)
```

### 4. File Conflicts

Handle according to session file's conflict mode:
- `skip`: keep existing files
- `overwrite`: overwrite

### 5. Result

```
---green-result---
result_file: ${TMP_DIR}green-result-{dir-safe}.json
status: success | partial | failed
implemented_files: [...]
tests_passed: N
tests_failed: N
---end-green-result---
```

## Parallel Execution Notice

This Agent may be executed in parallel batches. **AskUserQuestion usage forbidden.**
```

- [ ] **Step 2: Commit**

```bash
git add agents/green-coder.md
git commit -m "feat: add green-coder agent for GREEN phase

Implements minimal production code to pass approved tests.
Assertion modification strictly forbidden. Max 3 retry attempts."
```

---

## Task 6: refactorer Agent

**Files:**
- Create: `agents/refactorer.md`

- [ ] **Step 1: Write the agent definition**

```markdown
---
name: refactorer
description: |
  Use this agent when applying coding conventions to implemented code (REFACTOR phase).
  Receives production code from green-coder and applies Conventions while ensuring regression tests pass.
  NEVER modifies test assertions. Rolls back on regression failure.

  <example>
  <context>
  The dev skill calls refactorer after green-coder completes.
  </context>
  <user_request>
  Session file: ${TMP_DIR}refactor-session-src-auth.md
  Target directory: src/auth
  Detected language: typescript
  Save results to ${TMP_DIR} and return paths only
  </user_request>
  <assistant_response>
  1. Session read — target: src/auth, language: typescript
  2. Conventions loaded — 6 subsections
  3. Implementation files: 2 files
  4. Refactoring: naming rules applied, coding rules applied
  5. Regression test: 10 passed, 0 failed

  ---refactor-result---
  result_file: ${TMP_DIR}refactor-result-src-auth.json
  status: success
  refactored_files: [src/auth/index.ts, src/auth/types.ts]
  tests_passed: 10
  tests_failed: 0
  ---end-refactor-result---
  </assistant_response>
  </example>
model: inherit
color: yellow
tools:
  - Bash
  - Read
  - Glob
  - Grep
  - Write
  - Edit
---

You are a code refactorer. Your sole job: apply coding conventions while keeping all tests green.

## Input

```
Session file: <path> (refactor session file, pre-extracted by SKILL)
Target directory: <path>
Detected language: <lang>
Save results to ${TMP_DIR} and return paths only
```

## Temporary Directory

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Strictly Forbidden (HARD CONSTRAINTS)

The following actions are forbidden under any circumstances:

1. **Modifying test assertion logic** — expect(), assert, assertEqual and other verification logic
2. **Deleting or disabling test cases** — skip, xfail, .skip(), xit, @disabled, etc.
3. **Changing test expected values** — expected values are frozen contracts
4. **Changing external behavior (public API)** — function signatures, return types, export lists

### Allowed Changes

- Internal structural changes to production code (variable names, function splitting, file splitting)
- Code style adjustments based on Conventions (naming rules, coding rules, project structure)
- Import path adjustments (due to file moves)

## Workflow

### 1. Read Session File

Extract from session file:
- `target`, `language`
- Conventions (resolved) — 6 subsections
- `mapping_file` path → Read → test file list
- Implementation Files list

### 2. Snapshot

Record pre-refactoring state (for rollback):
```bash
git stash push -m "refactor-snapshot-{dir-safe}" -- {target}/*
git stash pop
```
Or record current state via git diff.

Actual rollback performed via `git checkout -- {files}`.

### 3. Apply Conventions

Apply per Conventions subsection:

| Convention | Application Target |
|-----------|-------------------|
| Naming Rules | Variable/function/class/constant names |
| Coding Rules | Patterns, structure, error handling approach |
| Project Structure | File location, directory structure |
| Module Boundaries | Import direction, dependency rules |
| Naming Conventions | Module/directory/package names |
| Language & Runtime | Language/runtime conventions |

Only files in the Implementation Files list are targets.
Test files are not modification targets.

### 4. Regression Test

```bash
# Language-specific test execution
# TypeScript: npx jest {test_files} 2>&1
# Rust: cargo test 2>&1
# Python: python -m pytest {test_files} -v 2>&1
# Go: go test ./... -v 2>&1
```

**All pass:** → success
**Any failure:** → rollback

### 5. Rollback (on failure)

```bash
git checkout -- {refactored_files}
```

Re-run tests after rollback to confirm original state restoration.

### 6. Result

```
---refactor-result---
result_file: ${TMP_DIR}refactor-result-{dir-safe}.json
status: success | rolled_back | skipped
refactored_files: [...]
tests_passed: N
tests_failed: N
---end-refactor-result---
```

`skipped`: When Conventions are absent or there are no changes to apply.

## Parallel Execution Notice

This Agent may be executed in parallel batches. **AskUserQuestion usage forbidden.**
```

- [ ] **Step 2: Commit**

```bash
git add agents/refactorer.md
git commit -m "feat: add refactorer agent for REFACTOR phase

Applies Conventions to production code with regression protection.
Assertion modification strictly forbidden. Rolls back on test failure."
```

---

## Task 7: Update dev-templates.md

**Files:**
- Modify: `skills/dev/references/dev-templates.md`

- [ ] **Step 1: Read the current file**

Run: `cat skills/dev/references/dev-templates.md`

- [ ] **Step 2: Rewrite dev-templates.md with new session file formats**

Replace the entire contents with the updated session file formats from the design spec. The file should contain:

1. **Dev Session File Format** (existing, add Implementation Tasks section)
2. **Test Writer Session File Format** (new — write and revise modes)
3. **Test Reviewer Session File Format** (new)
4. **Green Coder Session File Format** (new)
5. **Refactorer Session File Format** (new)
6. **Mapping JSON Format** (new)
7. **Result Formats** (updated — test-writer-result, test-reviewer-result, green-result, refactor-result)
8. **Error Handling** (updated for 4-agent pipeline)

The complete file content:

```markdown
# Dev Templates

## Dev Session File Format

```markdown
# Dev Task: {path}
type: dev | target: {path} | language: {lang} | conflict: {mode}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md
project_conventions: {project_root}/CLAUDE.md#Conventions

## Requirements (from CLAUDE.md)
{Entire Requirements section}

## Constraints (from DEVELOPERS.md)
{Entire Constraints section — test generation source}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas section — for type reference, not a test generation source}

## Technical Context
{Entire Technical Context section}

## Conventions (resolved)
{Hierarchy-resolved Conventions}

## Dependencies
{dev-context or exploration results}

## Implementation Tasks (only when Spec Changes exist)
- [ADD] CONST-N: {description}
- [MODIFY] CONST-N: {change description}
- [DELETE] CONST-N: {deletion target}

## Spec Changes (optional — included only when spec commit found)
breaking: {true|false}

### Transition Context
{Transition context — from where, to where, why}

### Added
{Added Requirements/Constraints}

### Modified
{Modified Requirements/Constraints}

### Removed
{Removed Requirements/Constraints}

## Verification Contract
- All Constraints → corresponding tests exist
- All Requirements → corresponding acceptance tests exist
- All tests pass
- /validate --strict {path}
`` `

## Test Writer Session File Format

### mode=write

`` `markdown
# Test Writer Session
type: test-writer | mode: write | target: {path} | language: {lang}
test_output_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Requirements (from CLAUDE.md)
{Entire Requirements section}

## Constraints (from DEVELOPERS.md)
{Entire Constraints section}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas section}

## Technical Context
{Technical Context}

## Conventions (resolved)
{Hierarchy-resolved Conventions}

## Implementation Tasks (only when Spec Changes exist)
- [ADD] CONST-N: {description}
- [MODIFY] CONST-N: {change description}

## Existing Test Directory (Incremental mode, only when existing tests exist)
existing_test_dir: {path}/{detected_test_dir}/

## Dependencies
{dev-context or exploration results}
`` `

### mode=revise

`` `markdown
# Test Writer Session
type: test-writer | mode: revise | round: {N} | target: {path} | language: {lang}
test_output_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json
feedback_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{N-1}.md

## Origin
(same)

## Requirements (from CLAUDE.md)
(same)

## Constraints (from DEVELOPERS.md)
(same)

## Data Schemas (from DEVELOPERS.md, reference only)
(same)

## Technical Context
(same)

## Conventions (resolved)
(same)

## Implementation Tasks
(same)

## Existing Test Directory
(same)

## Dependencies
(same)
`` `

## Test Reviewer Session File Format

`` `markdown
# Test Review Session
type: test-review | round: {N} | language: {lang}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
test_dir: ${TMP_DIR}tests/{dir-safe}/
spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
`` `

## Green Coder Session File Format

`` `markdown
# Green Coder Session
type: green | target: {path} | language: {lang} | conflict: {mode}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Requirements (from CLAUDE.md)
{Requirements}

## Constraints (from DEVELOPERS.md)
{Constraints}

## Technical Context
{Technical Context}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Tasks (only when Spec Changes exist)
{[ADD]/[MODIFY] tasks only — DELETE already handled by SKILL}

## Dependencies
{dependencies}
`` `

## Refactorer Session File Format

`` `markdown
# Refactorer Session
type: refactor | target: {path} | language: {lang}

## Conventions (resolved)
{Hierarchy-resolved Conventions}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Files
{File list extracted from green-coder result}
`` `

## Mapping JSON Format

`` `json
{
  "target_path": "src/auth",
  "test_files": ["src/auth/__tests__/auth.test.ts", "src/auth/__tests__/auth.acceptance.test.ts"],
  "constraints": [
    {
      "id": "CONST-1",
      "text": "authenticate(token: string) → User | AuthError",
      "tests": ["auth.test.ts::should return User for valid token", "auth.test.ts::should throw AuthError for expired token"]
    }
  ],
  "requirements": [
    {
      "id": "REQ-1",
      "text": "User authentication possible with valid token",
      "acceptance_tests": ["auth.acceptance.test.ts::Given valid token When authenticate Then return user"]
    }
  ],
  "unmapped_constraints": [],
  "unmapped_requirements": []
}
`` `

## Result Formats

### test-writer-result

`` `
---test-writer-result---
result_file: ${TMP_DIR}test-writer-result-{dir-safe}.json
status: success | partial
test_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
tests_generated: N
unmapped_constraints: N
unmapped_requirements: N
---end-test-writer-result---
`` `

### test-reviewer-result

`` `
---test-reviewer-result---
result_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md
verdict: approved | rejected
round: {N}
---end-test-reviewer-result---
`` `

### green-result

`` `
---green-result---
result_file: ${TMP_DIR}green-result-{dir-safe}.json
status: success | partial | failed
implemented_files: [...]
tests_passed: N
tests_failed: N
---end-green-result---
`` `

### refactor-result

`` `
---refactor-result---
result_file: ${TMP_DIR}refactor-result-{dir-safe}.json
status: success | rolled_back | skipped
refactored_files: [...]
tests_passed: N
tests_failed: N
---end-refactor-result---
`` `

## Error Handling

| Situation | Response |
|-----------|----------|
| Session file parsing failure | Return agent failure |
| test-writer unmapped > 0 | Return partial status |
| test-reviewer max_safety reached | Proceed best-effort, warning |
| Verify RED compilation failure | Delegate to green-coder (import fix allowed) |
| GREEN 3 failures | Return partial status |
| REFACTOR regression failure | Rollback, return rolled_back status |
| File write failure | Skip that file |
```

- [ ] **Step 3: Commit**

```bash
git add skills/dev/references/dev-templates.md
git commit -m "feat: update dev-templates with 4-agent session file formats

Add test-writer, test-reviewer, green-coder, refactorer session formats.
Add mapping JSON format and result block definitions."
```

---

## Task 8: Rewrite dev SKILL.md

**Files:**
- Modify: `skills/dev/SKILL.md`

- [ ] **Step 1: Read the current SKILL.md**

Run: `cat skills/dev/SKILL.md`

- [ ] **Step 2: Rewrite SKILL.md with 4-agent pipeline**

Key changes:
1. Steps 0-5: unchanged (CLI init, target resolution, language detection, dev-context, leaf-first, dry-run)
2. Step 6: Add Phase 0 (Spec Changes → task classification including DELETE execution)
3. Step 7: Replace single Task(compiler) with Test Writing Loop (test-writer → test-reviewer feedback loop)
4. Step 7.5: Add Verify RED (TMP→target copy + test execution)
5. Step 8: Add Task(green-coder)
6. Step 9: Add Task(refactorer)
7. Steps 10-14: Renumber from old 7.5-10 (build verify, diff, commit, validate, result)
8. Update DO/DON'T and error handling sections

The complete rewritten SKILL.md should follow the design spec's "dev SKILL full flow" section exactly, with:
- Step 7 loop using `round = 1`, `max_safety = 5`
- Session file generation for each agent using templates from dev-templates.md
- DELETE handling at Step 6e with Grep→collect→delete→clean→test sequence
- Verify RED with language-specific test commands
- green-coder and refactorer session file generation from prior results

- [ ] **Step 3: Verify the SKILL structure**

Run: `grep "^### " skills/dev/SKILL.md | head -20`
Expected: Step headers 0 through 14 visible.

- [ ] **Step 4: Commit**

```bash
git add skills/dev/SKILL.md
git commit -m "feat: rewrite dev SKILL with 4-agent pipeline

Replace monolithic compiler with test-writer → test-reviewer loop →
green-coder → refactorer pipeline. Add Spec Changes analysis at SKILL
level and DELETE task handling."
```

---

## Task 9: Update plugin.json

**Files:**
- Modify: `.claude-plugin/plugin.json`

- [ ] **Step 1: Read plugin.json**

Run: `cat .claude-plugin/plugin.json`

- [ ] **Step 2: Replace compiler agent with 4 new agents and bump version**

In the `agents` array, replace:
```json
"./agents/compiler.md"
```

With:
```json
"./agents/test-writer.md",
"./agents/test-reviewer.md",
"./agents/green-coder.md",
"./agents/refactorer.md"
```

Bump version from `10.8.0` to `10.9.0` (MINOR — new agents, compiler removed).

- [ ] **Step 3: Verify JSON is valid**

Run: `python3 -c "import json; json.load(open('.claude-plugin/plugin.json'))"`
Expected: No error output.

- [ ] **Step 4: Commit**

```bash
git add .claude-plugin/plugin.json
git commit -m "feat: register 4 new agents, remove compiler, bump to v10.9.0"
```

---

## Task 10: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Read CLAUDE.md**

Run: `cat CLAUDE.md`

- [ ] **Step 2: Update Agents table**

Replace the current Agents table:
```markdown
| Agent | Superpowers Combination | Role |
|-------|------------------------|------|
| `decompose` | (none) | Large-scale spec → module decomposition plan |
| `impl` | brainstorming | Requirements analysis + CLAUDE.md/DEVELOPERS.md generation |
| `test-writer` | (none) | RED — spec → tests + Constraint↔Test mapping |
| `test-reviewer` | (none) | Spec-to-test traceability verification |
| `green-coder` | (none) | GREEN — minimal implementation to pass approved tests |
| `refactorer` | (none) | REFACTOR — Conventions application + regression tests |
| `validator` | verification-before-completion | Semantic drift detection |
| `decompiler` | (none) | Source code → CLAUDE.md/DEVELOPERS.md extraction |
```

- [ ] **Step 3: Update /dev architecture diagram**

Replace the `/dev` diagram in the Architecture section with:

```markdown
#### /dev (CLAUDE.md → Source Code)

`` `
User: /dev [--all] [--conflict skip|overwrite] [--dry-run] [--validate]
        │
        ▼
┌─────────────────────────────────────────────┐
│ dev SKILL                                   │
│                                             │
│ 1. Target resolution (--all or incremental) │
│ 2. Language detection + Spec Changes analysis│
│ 3. [DELETE] tasks executed directly by SKILL │
│ 4. Test Writing Loop (per target):          │
│    Task(test-writer) → Task(test-reviewer)  │
│    → feedback loop (max 5)                  │
│ 5. TMP → target copy + Verify RED           │
│ 6. Task(green-coder) per target             │
│ 7. Task(refactorer) per target              │
│ 8. Build verify + git diff + dev commit     │
└─────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────┐  ┌──────────────────────┐
│ test-writer AGENT     │  │ test-reviewer AGENT   │
│                       │  │                       │
│ Constraints → tests   │◄►│ 5-criteria review     │
│ Requirements → accept │  │ verdict: approved     │
│ mapping.json created  │  │         | rejected    │
└───────────────────────┘  └──────────────────────┘
        │ approved
        ▼
┌───────────────────────┐  ┌──────────────────────┐
│ green-coder AGENT     │  │ refactorer AGENT      │
│                       │  │                       │
│ Approved test-based   │─►│ Conventions applied   │
│ minimal impl (max 3)  │  │ Rollback on failure   │
└───────────────────────┘  └──────────────────────┘
`` `
```

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for 4-agent dev pipeline

Replace compiler in Agents table with test-writer, test-reviewer,
green-coder, refactorer. Update /dev architecture diagram."
```

---

## Task 11: Delete compiler agent

**Files:**
- Delete: `agents/compiler.md`

- [ ] **Step 1: Verify compiler is no longer referenced**

Run: `grep -r "compiler" skills/ agents/ .claude-plugin/ --include="*.md" --include="*.json" -l`
Expected: No references (after Tasks 8-10 are complete).

- [ ] **Step 2: Delete the file**

```bash
rm agents/compiler.md
```

- [ ] **Step 3: Commit**

```bash
git add -A agents/compiler.md
git commit -m "chore: remove compiler agent, replaced by 4 role-specific agents"
```

---

## Task 12: Update marketplace.json version

**Files:**
- Modify: `../../.claude-plugin/marketplace.json`

- [ ] **Step 1: Read and update marketplace version**

Find the claude-md-plugin entry and update version to `10.9.0` to match plugin.json.

- [ ] **Step 2: Verify JSON is valid**

Run: `python3 -c "import json; json.load(open('../../.claude-plugin/marketplace.json'))"`
Expected: No error output.

- [ ] **Step 3: Commit**

```bash
git add ../../.claude-plugin/marketplace.json
git commit -m "chore: sync marketplace.json version to 10.9.0"
```
