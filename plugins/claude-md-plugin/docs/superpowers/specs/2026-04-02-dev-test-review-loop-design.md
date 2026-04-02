# Dev Test Review Loop Design

## Problem

In the current `/dev` workflow, the compiler agent runs monolithically from RED (test generation) through REFACTOR.
The completeness of Constraints → test conversion is left to LLM discretion, so source code consistency against CLAUDE.md (SSOT) is not guaranteed.

validate is a post-hoc verification tool, not a synchronization guarantee mechanism.
**Guarantees must be made at code generation time (compile).**

## Solution

Decompose the compiler agent into 4 role-specific agents, and enforce spec-to-test traceability through an independent reviewer and feedback loop in the RED phase.

## Architecture

### 4-Agent System

| Agent | Role | Input | Output |
|-------|------|-------|--------|
| **test-writer** | RED — spec → tests + mapping | dev session file | Test files in TMP + mapping.json |
| **test-reviewer** | Spec-to-test verification | review session file | verdict (approved/rejected) |
| **green-coder** | GREEN — implementation to pass approved tests | session file + approved tests | Implementation code |
| **refactorer** | REFACTOR — Conventions application + regression tests | session file + implemented code | Refactored code |

Deprecated: existing compiler agent

### dev SKILL Full Flow

```
Step 0: CLI initialization
Step 1: Target resolution (--all or incremental)
Step 2: Language auto-detection
Step 3: dev-context.md check (optional)
Step 4: leaf-first sorting
Step 5: --dry-run handling
Step 6: Session file generation + Spec Changes analysis
  6a. Spec commit search
  6b. CLAUDE.md/DEVELOPERS.md reading
  6c. If Spec Changes exist → derive [ADD]/[MODIFY]/[DELETE] tasks
  6d. Write → dev-session-{dir-safe}.md (including task classification)
  6e. If [DELETE] tasks exist → SKILL executes directly:
      1. Grep to search for imports/references of deletion targets
      2. Collect list of referencing files
      3. Delete target files/functions (Bash rm or Edit)
      4. Remove imports/calls from referencing files (Edit)
      5. Delete related test files
      6. Run regression tests (language-specific test command) → report warning on failure

Step 7: Test Writing Loop (per target, sequential per module)
  7a. Create test-writer session file
  7b. Task(test-writer) → tests in TMP + mapping.json
  7c. Create test-reviewer session file
  7d. Task(test-reviewer) → verdict
  7e. rejected → create revise session → Task(test-writer, mode=revise) → 7c
  7f. approved → copy TMP/tests/{dir-safe}/ → target directory
  7g. Proceed best-effort when max_safety(5) is reached

Step 7.5: Verify RED (SKILL executes directly via Bash)
  7.5a. Run language-specific tests:
        | Language | Command |
        | TypeScript | npx jest --passWithNoTests 2>&1 |
        | Rust | cargo test --no-run 2>&1 (compile only) |
        | Python | python -m pytest --collect-only 2>&1 |
        | Go | go test -run "^$" ./... 2>&1 (compile only) |
  7.5b. Confirm all fail → enter GREEN
  7.5c. Some pass → record as existing implementation coverage, enter GREEN
  7.5d. Compilation itself fails (import errors, etc.) → delegate to green-coder (import fix allowed)

Step 8: Task(green-coder) — implementation based on approved tests
Step 9: Task(refactorer) — Conventions application + regression tests
Step 10: Build verification (cargo check / tsc etc.)
Step 11: git diff --stat
Step 12: dev commit (individual per path)
Step 13: Run /validate if --validate
Step 14: Return results
```

## Agent Details

### test-writer

**Role**: Generate test code + Constraint↔Test mapping table from spec (Requirements + Constraints).

**mode**: `write` | `revise`

**Outputs**:
- Actual test files in `${TMP_DIR}tests/{dir-safe}/` (written with import paths relative to target)
- `${TMP_DIR}test-mapping-{dir-safe}.json`

**Mapping JSON Format**:
```json
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
```

**Core Discipline**:
- Every Constraint maps to at least 1 test
- Every Requirement maps to at least 1 acceptance-level test
- `unmapped_*` must be empty for a self-complete state
- Test file location is determined based on language + Conventions

**mode=revise**: Reads the reviewer feedback file, modifies/adds tests for flagged items. Directly Edits existing TMP test files.

### test-reviewer

**Role**: Spec-to-test traceability + test quality verification. File modification forbidden — returns verdict only.

**Verification Criteria** (5 total, all must pass for approved):

| Criterion | Verification Content |
|-----------|---------------------|
| **Constraint Coverage** | Is `unmapped_constraints` empty? Does each mapped test actually verify the corresponding Constraint's input/output contract? |
| **Requirement Coverage** | Is `unmapped_requirements` empty? Do acceptance tests reflect the Requirement's business intent? |
| **Boundary Value Sufficiency** | Do numerical limit Constraints have boundary value tests (N OK, N+1 fail)? |
| **Interface Consistency** | Do function signatures assumed by tests (name, parameter types, return types) match the Constraints' I/O contracts? |
| **Test Independence** | Does each test not depend on other test results? Is there no shared state mutation? |

**verdict**:
- `approved` — All 5 criteria pass, 0 Critical Questions
- `rejected` — Any criterion not met. Returns specific Critical Questions

**Constraints**: Read/Write only (no modifications except result file Write). AskUserQuestion forbidden.

### green-coder

**Role**: Minimal implementation that passes all approved tests.

**Allowed**:
- Create/modify production code files
- Fix import/path errors in test files

**Strictly Forbidden**:
- Modifying test assertion logic
- Deleting/disabling test cases (skip, xfail, etc.)
- Changing test expected values
- Adding new tests

**Goal**: Minimal implementation that passes all approved tests. Max 3 retries.

**result block**:
```
---green-result---
result_file: ${TMP_DIR}green-result-{dir-safe}.json
status: success | partial | failed
implemented_files: [...]
tests_passed: N
tests_failed: N
---end-green-result---
```

### refactorer

**Role**: Conventions application + regression test guarantee.

**Allowed**:
- Structural changes to production code (naming, file splitting, pattern application)
- Code style adjustments based on Conventions section

**Strictly Forbidden**:
- Modifying test assertion logic
- Deleting/disabling test cases
- Changing test expected values
- Changing external behavior (public API)

**Goal**: Pass regression tests after Conventions application. Roll back on failure.

**result block**:
```
---refactor-result---
result_file: ${TMP_DIR}refactor-result-{dir-safe}.json
status: success | rolled_back | skipped
refactored_files: [...]
tests_passed: N
tests_failed: N
---end-refactor-result---
```

### Common Invariants

**Approved tests = frozen contracts.** Tests approved by test-reviewer must never have their assertions changed in subsequent pipeline stages (green-coder, refactorer).

## Session File Formats

### test-writer session file (`${TMP_DIR}test-writer-session-{dir-safe}.md`)

```markdown
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
- [ADD] CONST-3: new function validate_token
- [MODIFY] CONST-1: return type changed User → AuthResult

## Existing Test Directory (Incremental mode, only when existing tests exist)
existing_test_dir: {path}/{detected_test_dir}/

## Dependencies
{dev-context or exploration results}
```

### test-writer revise session file (overwrites same path)

```markdown
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
```

### test-reviewer session file (`${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md`)

```markdown
# Test Review Session
type: test-review | round: {N} | language: {lang}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
test_dir: ${TMP_DIR}tests/{dir-safe}/
spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
```

### green-coder session file (`${TMP_DIR}green-session-{dir-safe}.md`)

```markdown
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
```

### refactorer session file (`${TMP_DIR}refactor-session-{dir-safe}.md`)

```markdown
# Refactorer Session
type: refactor | target: {path} | language: {lang}

## Conventions (resolved)
{Hierarchy-resolved Conventions}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Files
{File list extracted from green-coder result}
```

## Design Decisions

### Why 4 agents instead of 1 compiler?

- **test-writer/test-reviewer separation**: Agent → Agent call constraint. SKILL must orchestrate, so role-based separation is essential.
- **green-coder/refactorer separation**: Enforce the assertion modification prohibition at the agent boundary. Clear constraints so each agent performs only its role.
- **Approach 3 (TMP isolation)**: No target contamination during review loop. Clean handoff after approval.

### Why DELETE at SKILL level?

DELETE is a destructive operation that does not fit the TDD cycle (RED→GREEN→REFACTOR).
Delegating code deletion + reference cleanup to an agent makes the role ambiguous.
SKILL handles it directly at Step 6e, then only [ADD]/[MODIFY] go through the TDD pipeline.

### Why module-sequential in Test Writing Loop?

Same rationale as spec SKILL's Socratic loop. Each module's reviewer loop iteration depends on previous results, so sequential processing within the loop is inevitable.
Inter-module loops are independent, but sequential processing is used to protect SKILL context.

### Phase 0 (Spec Changes) at SKILL level

Moved the existing compiler's Phase 0 to SKILL Step 6.
Task classification ([ADD]/[MODIFY]/[DELETE]) must be determined at session file creation time to be consistently passed to both test-writer and green-coder.

## Scope

### In scope
- Create new test-writer agent
- Create new test-reviewer agent
- Create new green-coder agent
- Create new refactorer agent
- Restructure dev SKILL workflow (Steps 6-9)
- Deprecate compiler agent
- Update dev-templates.md session file formats
- Write acceptance tests (.feature)

### Out of scope
- No changes to spec SKILL
- No changes to validate SKILL
- No changes to decompile SKILL
- No changes to CLI (Rust core)
- auto mode: maintain base structure (internal dev calls follow the changed workflow)
