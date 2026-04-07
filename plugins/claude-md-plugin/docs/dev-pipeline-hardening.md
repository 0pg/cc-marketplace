# /dev Pipeline Hardening — Incomplete Implementation Prevention

## Problem Statement

`/autodev` (or `/dev`) completes tasks that appear successful but contain **unimplemented code** —
functions that log TODOs and return default values instead of actual logic.
This is not "implementation quality" issue but genuine **non-implementation** that passes through
the entire TDD pipeline and gets committed.

## Root Cause Analysis

Five structural escape hatches allow non-implementation to reach commit:

| # | Root Cause | Severity | Location |
|---|-----------|----------|----------|
| 1 | No final test execution before commit | Critical | Step 10→12 gap |
| 2 | RED verification doesn't actually run tests | High | Step 7.5b |
| 3 | test-reviewer checks mapping existence, not assertion quality | High | test-reviewer.md |
| 4 | green-coder `partial` status doesn't block pipeline | Critical | Step 8 |
| 5 | max_safety bypasses reviewer rejection after 5 rounds | Medium | Step 7f |

### How TODO Stubs Survive the Pipeline

```
test-writer writes weak tests (toBeDefined)
    ↓ test-reviewer approves (mapping exists, signatures match)
    ↓ RED "verification" doesn't run tests (--collect-only / --no-run)
    ↓ green-coder fails complex logic → returns TODO stubs → status: partial
    ↓ partial proceeds to refactorer → build check (type-only) → commit
    ↓ Result: non-implemented code committed as "success"
```

## Design Principles

1. **Defense in Depth** — Multiple independent gates; no single point of failure
2. **Fail-Open for Recovery, Fail-Closed for Commit** — Allow partial progress through 
   intermediate stages (refactorer may fix issues), but hard-gate before commit
3. **Observability** — Every degraded state must be visible in the result, not silently swallowed
4. **Deterministic Gates** — Final verification uses Bash test execution, not LLM judgment

## Improvement Summary

```
                test-writer
                    │
         ┌──────────┤
         ▼          ▼
   [P2] Step 7f    test-reviewer ← [P1] Criterion 6: Assertion Strength
   Graduated       (reject WEAK assertions like toBeDefined)
   Escalation
   (STUCK/DIVERGE → HALT)
         │
         ▼
   [P1] Step 7.5b ← Actual RED Verification
   (run tests, detect tautological tests)
         │
         ▼
   green-coder
         │
   [P2] Step 8 ← pass_rate tracking + warning
   (soft gate: diagnostic tagging, no rollback)
         │
         ▼
   refactorer → build check
         │
   [P0] Step 10.5 ← Final Test Gate  ★ HARD GATE
   (ALL tests must pass or module is rolled back)
         │
         ▼
   commit (only if gate passed)
```

## P0: Final Test Gate (Step 10.5)

**Files**: `skills/dev/SKILL.md`

### Rationale

The single most impactful fix. Regardless of what happens in earlier stages,
no code with failing tests can be committed. This alone eliminates the TODO-stub problem.

### Specification

Insert between Step 10 (Build verification) and Step 11 (Display changes):

```
### 10.5. Final Test Gate (mandatory)

For each module that passed build verification:

1. Run test suite using mapping.json test_files:
   | Language   | Command                                       |
   | TypeScript | npx jest {test_files from mapping} 2>&1       |
   | Rust       | cargo test 2>&1                               |
   | Python     | python -m pytest {test_files from mapping} -v 2>&1 |
   | Go         | go test ./... -v 2>&1                         |

2. Evaluate:
   - ALL pass → module status = "success", proceed to commit
   - SOME fail:
     a. Cross-reference mapping.json → identify unmet Constraints/Requirements
     b. Report:
        [TEST GATE FAILED] {path}: {N} tests failing
        Unmet Constraints: {CONST-IDs}
        Unmet Requirements: {REQ-IDs}
     c. Rollback module:
        git checkout -- {tracked module files}
        git clean -fd -- {new untracked files created by green-coder}
     d. Module status = "gate_failed", do NOT commit
   - Execution crash/timeout → status = "failed", do NOT commit

3. Cross-module verification (after all per-module gates):
   If multiple modules passed their individual gates, run full test suite once:
   - Pass → proceed to commits
   - Fail → identify interfering modules, block affected commits
```

### Result Format Change

```
---dev-result---
status: success | partial | failed
total: {n}
generated: {n}
gate_passed: {n}
gate_failed: {n}
gate_details:
  - path: {path}, status: success, tests: {passed}/{total}
  - path: {path}, status: gate_failed, tests: {passed}/{total}, unmet: [{IDs}]
tests: {passed} passed, {failed} failed
---end-dev-result---
```

### Rollback Safety

- Tracked files: `git checkout -- {files}`
- New untracked files: `git clean -fd -- {files}` (scoped to module, not recursive)
- Staged files: `git reset HEAD -- {files}` before checkout

## P1-a: RED Verification Fix (Step 7.5b)

**Files**: `skills/dev/SKILL.md`

### Rationale

Current RED verification uses `--collect-only`, `--no-run`, `--passWithNoTests` — 
none of which actually execute test assertions. Tautological tests pass through undetected.

### Specification

Replace Step 7.5b commands:

```
7.5b. Verify RED (SKILL executes directly via Bash):
      | Language   | Command                                       |
      | TypeScript | npx jest --no-cache {test_files} 2>&1         |
      | Rust       | cargo test 2>&1                               |
      | Python     | python -m pytest {test_files} -v 2>&1         |
      | Go         | go test ./... -v 2>&1                         |

7.5c. Interpret results:
      - exit != 0 AND assertion/test failures in output
        → RED confirmed, proceed to Step 8
      - exit != 0 AND only compilation/import errors
        → delegate to green-coder for import fix (existing 7.5e behavior)
      - exit != 0 AND runtime/infrastructure errors (DB connection, network, etc.)
        → WARN: RED unverifiable due to external dependencies, proceed with caution
      - exit == 0 AND ALL tests pass:
        → If ALL mapped tests are Existence-type (STRUCT-XXX): exempt, proceed
        → Else: [RED VIOLATION] — tests are tautological
          red_violation_count++
          if red_violation_count > 2: HALT module
            "[RED FAILED] {path}: tests pass without implementation after {count} rewrites"
          else: return to Step 7 test-writer loop with feedback:
            round++ (reuse existing max_safety counter)
            feedback: "All tests pass without implementation.
              Assertions must verify specific output values, not existence/type/truthiness."
      - exit == 0 AND SOME pass:
        → record as existing implementation coverage, proceed (unchanged)
```

### Interaction with Existing Loop

RED violation feedback reuses the existing test-writer revision loop:
- `round++` increments the same counter as test-reviewer rejections
- `max_safety` cap applies uniformly (no separate unbounded loop)
- Feedback format matches existing `feedback_file` structure (Critical Questions format)

## P1-b: Assertion Strength Criterion (test-reviewer Criterion 6)

**Files**: `agents/test-reviewer.md`

### Rationale

test-reviewer checks 5 structural criteria but never evaluates whether assertions
actually verify the Constraint's specified behavior. A test mapping to CONST-1 with
`expect(result).toBeDefined()` passes all 5 criteria.

### Specification

Add 6th criterion to Phase 2 table:

```
| **Assertion strength** | For each mapped test, does the assertion verify a **specific
  value, error, or behavioral property** from the Constraint — not merely
  existence/type/truthiness? |
```

3-tier classification (include in agent instructions as explicit reference):

```
STRONG (pass):
  toBe(value), toEqual({...}), toThrow(SpecificError), toHaveLength(N),
  toBeLessThan(N), toBeGreaterThan(N), toStrictEqual(value),
  assertEqual(expected, actual), assert result == expected

ACCEPTABLE (pass when Constraint specifies shape/pattern, not exact value):
  toMatch(regex), toHaveProperty('key'), toMatchObject({...}),
  toContain(element), toBeInstanceOf(Type), toBeCloseTo(N, precision),
  assertIn(item, collection), assertIsInstance(obj, cls)

WEAK (reject — must cite specific Constraint and expected behavior):
  toBeDefined(), toBeTruthy(), toBeFalsy(), toBeNull(),
  not.toThrow() when Constraint specifies a return value,
  typeof checks (expect(typeof x).toBe('object')),
  assert result is not None, assertTrue(bool(result))

EXCEPTIONS:
  - STRUCT-XXX Existence tests: toBeDefined() is STRONG (by design)
  - Non-functional Constraints (performance, latency):
    toBeLessThan(N) where N matches Constraint's stated limit is STRONG
```

Critical Question format when criterion 6 fails:
```
CONST-{N}: test '{test_name}' uses weak assertion ({assertion}) —
  must verify {what the Constraint specifies} per Constraint I/O contract
```

If the Constraint itself is too abstract for a strong assertion, flag differently:
```
CONST-{N}: Constraint text is under-specified for testable assertion —
  cannot write strong test without more specific I/O contract
```

## P2-a: Partial Status Tracking (Step 8)

**Files**: `skills/dev/SKILL.md`

### Rationale

green-coder's `partial` status currently proceeds silently. Combined with P0 (Final Test Gate),
Step 8 becomes a diagnostic/tracking point rather than a gate — the hard gate is at Step 10.5.

### Specification

Replace Step 8 result handling:

```
Check green-result status:
- success: proceed to Step 9
- partial:
    1. Extract tests_passed, tests_failed from green-result
    2. Calculate pass_rate = tests_passed / (tests_passed + tests_failed)
    3. Log: "⚠ [GREEN PARTIAL] {path}: {tests_passed}/{total} tests passing ({pass_rate}%)"
    4. Tag module as "gate_required" (Step 10.5 will hard-gate)
    5. Proceed to Step 9 (refactorer gets a chance to fix remaining issues)
- failed: report error, move to next module
```

### Design Decision: Why Not Rollback Here?

Refactorer may fix remaining issues. Rolling back at Step 8 wastes green-coder's partial progress
and eliminates the refactorer's recovery opportunity. The hard gate at Step 10.5 catches anything
that remains broken after all agents have had their chance.

## P2-b: max_safety Graduated Escalation (Step 7f)

**Files**: `skills/dev/SKILL.md`

### Rationale

Current max_safety silently proceeds with "best-effort" tests, discarding reviewer's rejection.
This creates a path where known-bad tests enter the pipeline.

### Specification

Replace Step 7f:

```
7f. if round >= max_safety:
    1. Read last 2 rounds' test-reviewer result files
    2. Extract Critical Question IDs (CONST-N / REQ-N) from each

    3. Compare:
       CONVERGING — last round's issue IDs ⊂ previous round's issue IDs
         (issues are being resolved, some remain):
         → Proceed with best-effort tests
         → Append unreviewed_gaps to green-coder session file:
           "unreviewed_gaps: [{last round's Critical Questions with IDs}]"
         → Module status includes "review_incomplete" flag
         → Log: "⚠ [REVIEW INCOMPLETE] {path}: proceeding with {N} known gaps: {IDs}"

       STUCK — last round's issue IDs == previous round's issue IDs
         (identical issues, test-writer cannot resolve):
         → HALT module
         → Log: "[TEST LOOP STUCK] {path}: unresolvable after {max_safety} rounds
            Stuck on: {issue IDs and summaries}
            Action: Review DEVELOPERS.md Constraints for testability"
         → Module status = "skipped"

       DIVERGING — |last round's issue IDs| > |previous round's issue IDs|
         (issues growing, not converging):
         → HALT module
         → Log: "[TEST LOOP DIVERGING] {path}: issues growing after {max_safety} rounds
            Action: Review Constraints for ambiguity or test-reviewer criteria"
         → Module status = "skipped"
```

### Classification Reliability

- Uses **Constraint/Requirement IDs** (CONST-N, REQ-N) for comparison, not free text
- Compares only last 2 rounds (deterministic set comparison), not full 5-round trend analysis
- test-reviewer already required to include IDs in Critical Questions — no format change needed

## Implementation Order

| Priority | Change | Files Modified |
|----------|--------|---------------|
| P0 | Final Test Gate | `skills/dev/SKILL.md` |
| P1-a | RED Verification | `skills/dev/SKILL.md` |
| P1-b | Assertion Strength | `agents/test-reviewer.md` |
| P2-a | Partial Status Tracking | `skills/dev/SKILL.md` |
| P2-b | max_safety Escalation | `skills/dev/SKILL.md` |

## Verification

After all changes, the defense-in-depth chain ensures TODO stubs must pass ALL layers to be committed:

```
Layer 1 (Test Quality):  Criterion 6 rejects weak assertions
Layer 2 (RED Integrity):  Tautological tests detected and sent back
Layer 3 (Progress Tracking):  partial status visible, not silent
Layer 4 (Hard Gate):  Failing tests → rollback → no commit
```

A TODO stub committed requires simultaneous failure of all 4 layers — structurally impossible
when the Final Test Gate (Layer 4) is a deterministic Bash execution with zero LLM judgment.
