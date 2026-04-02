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
      | REQ-1 | 유효한 토큰으로 사용자 인증 가능          |
      | REQ-2 | 만료된 토큰은 거부                       |
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
    And Critical Questions mention "경계값"

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
  세션 파일: ${TMP_DIR}test-writer-session-src-auth.md
  결과는 ${TMP_DIR}에 저장하고 경로만 반환
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

## 입력

```
세션 파일: <path> (test-writer session file, pre-extracted by SKILL)
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

## 임시 디렉토리

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Workflow

### 1. Read Session File

세션 파일에서 추출:
- `mode`: write | revise
- `target`, `language`
- `test_output_dir`, `mapping_output`
- Requirements, Constraints, Data Schemas, Technical Context, Conventions
- Implementation Tasks (있을 때만)
- Existing Test Directory (있을 때만)
- `feedback_file` (revise 모드만)

### 2. Mode 분기

**mode=write:**
- Phase 3(테스트 설계) → Phase 4(테스트 작성) → Phase 5(매핑 생성) → Phase 6(결과)

**mode=revise:**
- feedback_file Read → Critical Questions 추출
- 기존 TMP 테스트 파일 Edit (test_output_dir에서)
- 기존 mapping.json 업데이트
- → Phase 5(매핑 갱신) → Phase 6(결과)

### 3. Test Design (mode=write)

**Constraints → 단위 테스트 설계:**

| Constraints 유형 | 테스트 패턴 |
|-----------------|------------|
| 수치 제한 (`최대 N`) | 경계값: N OK, N+1 실패 |
| 형식 제약 (`UTF-8만`) | 유효 입력 통과, 무효 입력 거부 |
| 보안 제약 (`secure storage`) | 보안 속성 검증 |
| 비즈니스 규칙 | 규칙 준수/위반 시나리오 |
| I/O 계약 (`f(a) → b`) | 입력 a에 대해 출력 b 검증 |

**Requirements → acceptance 테스트 설계:**

각 Requirement에 대해 최소 1개 acceptance-level 테스트:
- happy path (필수)
- error path (Requirement에 에러 시나리오가 있으면)
- 비즈니스 의도를 검증하는 시나리오

**Implementation Tasks가 있는 경우:**
- [ADD]: 새 Constraint/Requirement에 대한 테스트만 생성
- [MODIFY]: 변경된 Constraint에 매칭되는 기존 테스트 수정 + 새 테스트 추가
  - Existing Test Directory의 기존 테스트를 Read하여 참조
- Implementation Tasks 없으면: 전체 Constraints/Requirements에 대해 테스트 생성

### 4. Write Test Files

`test_output_dir` (= `${TMP_DIR}tests/{dir-safe}/`)에 테스트 파일 Write.

**테스트 파일 규칙:**
- import 경로는 **target 기준**으로 작성 (TMP가 아닌 실제 배포 경로)
- 파일 위치와 네이밍은 Conventions 기반 (없으면 언어별 기본 관례)
- 각 테스트는 독립적 — 공유 상태 mutation 금지
- describe/context 구조로 Constraint별 그룹핑

**Incremental 모드 (Existing Test Directory 있을 때):**
- 기존 테스트 파일을 Read하여 기존 구조 파악
- [MODIFY] 대상: 기존 테스트 내용을 TMP에 복사 후 수정
- [ADD] 대상: 새 테스트 파일 생성
- 기존 테스트 중 변경 불필요한 것은 복사하지 않음 (SKILL이 target에서 유지)

### 5. Generate Mapping

`mapping_output` (= `${TMP_DIR}test-mapping-{dir-safe}.json`)에 Write:

```json
{
  "target_path": "{path}",
  "test_files": ["{상대경로1}", "{상대경로2}"],
  "constraints": [
    {
      "id": "CONST-1",
      "text": "{Constraint 원문}",
      "tests": ["{파일}::{테스트명}", ...]
    }
  ],
  "requirements": [
    {
      "id": "REQ-1",
      "text": "{Requirement 원문}",
      "acceptance_tests": ["{파일}::{테스트명}", ...]
    }
  ],
  "unmapped_constraints": [],
  "unmapped_requirements": []
}
```

**자체 검증:** unmapped_*가 비어 있지 않으면 테스트를 추가하여 해소. 해소 불가 시 unmapped에 남기고 결과에 반영.

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

## 핵심 규율

- **모든 Constraint → 최소 1개 테스트**
- **모든 Requirement → 최소 1개 acceptance 테스트**
- **경계값 Constraint → 반드시 경계 테스트 포함** (N OK, N+1 실패)
- **테스트 독립성** — 각 테스트가 다른 테스트에 의존하지 않음

## 병렬 실행 주의

이 Agent는 병렬 배치로 실행될 수 있습니다. **AskUserQuestion 사용 금지.**

## Context 효율성

- 세션 파일에 모든 스펙이 추출되어 있으므로 CLAUDE.md/DEVELOPERS.md 직접 Read 불필요
- 모호한 경우만 Origin 경로로 원본 참조
- 결과는 ${TMP_DIR}에 저장, 경로만 반환
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
  세��� 파일: ${TMP_DIR}test-reviewer-session-src-auth-v1.md
  ��과는 ${TMP_DIR}에 저장하고 경로만 반환
  </user_request>
  <assistant_response>
  1. Session read — round: 1, language: typescript
  2. Mapping loaded — 3 Constraints, 2 Requirements
  3. Test files read — 2 files, 10 tests
  4. Critique:
     - CONST-2: 경계값 테스트 누락 — "최대 7일"에 7일/8일 경계 없음
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

## ���력

```
세션 파일: <path>
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

## 임시 디렉��리

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## Workflow

### Phase 1: Load

세션 파일을 Read하여 추출:
- `round`, `language`, `dir_safe`
- `mapping_file` 경로 → Read → mapping JSON 로드
- `test_dir` 경로 → 내부 테스트 파일들 Read
- `spec_session_file` 경로 → Read → Requirements, Constraints 원문 확인

세션 파일 형식:
```
# Test Review Session
type: test-review | round: N | language: {lang}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
test_dir: ${TMP_DIR}tests/{dir-safe}/
spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
```

### Phase 2: 5-Criteria Review

5개 기준을 순서대로 모든 항목에 적용. 의심스러운 항목은 모두 Critical Question으로 기록.

| 기준 | 검증 내용 |
|------|----------|
| **Constraint 커버리지** | `unmapped_constraints`가 비어 있는가. 매핑된 각 테스트가 해당 Constraint의 입출력 계약을 **실제로** 검증하는가 (테스트 코드 Read하여 assertion 확인). |
| **Requirement 커버리지** | `unmapped_requirements`가 비어 있는가. acceptance 테스트가 Requirement의 비즈니스 의도를 반영하는가. |
| **경계값 충분성** | 수치 제한 Constraint에 경계값 테스트(N OK, N+1 실패)가 있는가. Constraint 원문에서 수치를 추출하여 테스트 코드의 값과 대조. |
| **인터페이스 일관성** | 테스트가 가정하는 함수 시그니처(이름, 파라미터 타입, 반환 타입)가 Constraints의 I/O 계약과 일치하는가. |
| **테스트 독립성** | 각 테스트가 다른 테스트 결과에 의존하지 않는가. 공유 상태 mutation이 없는가. beforeEach/setUp에서 상태가 초기화되는가. |

**비판 원칙:**
- 모든 의심스러운 항목은 Critical Question으로 기록 — 침묵은 승인이 아님
- "충분히 좋다"는 없다 — 모든 항목이 명시적 기준을 통과해야 approve
- Critical Question은 구체적이어야 함: "CONST-2는 7일 경계값 테스트 없음" (O), "테스트 개선 필요" (X)
- mapping JSON의 매핑이 정확한지 테스트 코드를 직접 Read하여 검증 — mapping만 믿지 않음

### Phase 3: Verdict 결정

**approved** — 다음 모두 충족 시:
- 5개 기준 모두 통과
- Critical Questions: 0개

**rejected** — 위 기준 중 하나라도 미충족 시.

### Phase 4: Write Result + Return

결과 파일 경로: `${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md`

`{dir-safe}`: 세션 파일의 `dir_safe` 필드에서 직접 읽기 (경로 파싱 금지)

결과 파일 내용:
```markdown
# Test Review Result
round: {N}
verdict: approved | rejected

## Critical Questions
- {Constraint/Requirement ID}: "{구체적 지적 내용}"

## Approval Rationale (approved 시)
5개 기준 통과 요약.
```

result block 반환 (SKILL context 최소화):
```
---test-reviewer-result---
result_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md
verdict: approved | rejected
round: {N}
---end-test-reviewer-result---
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| mapping_file 없음 | verdict: rejected, "mapping file not found" |
| test_dir 비어 있음 | verdict: rejected, "no test files found" |
| spec_session_file 없음 | verdict: rejected, "spec session file not found" |
| round 필드 없음 | round: 1로 가정 |

## 핵심 제약

- **파일 수정 금지** — 테스트 파일, mapping JSON 포함 어떤 파일도 수정 금지 (결과 파일 Write 제외)
- **AskUserQuestion 사용 금지** — 모든 판단은 파일 내용만으로
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
  세션 파일: ${TMP_DIR}green-session-src-auth.md
  대상 디렉토리: src/auth
  감지된 언어: typescript
  결과는 ${TMP_DIR}에 ��장하고 경로만 반환
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

## 입력

```
세션 파일: <path> (green session file, pre-extracted by SKILL)
대�� 디렉토리: <path>
감지된 언어: <lang>
결과는 ${TMP_DIR}에 저장하고 경��만 반환
```

## 임시 디렉토리

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## 절대 금지 (HARD CONSTRAINTS)

다음 행위는 어떤 상황에서도 금지됩니다:

1. **테스트의 assertion 로직 수정** — expect(), assert, assertEqual 등의 검증 로직
2. **테스트 케이스 삭제 또는 비활성화** — skip, xfail, .skip(), xit, @disabled 등
3. **테스트의 expected value 변경** — 기대값은 동결된 계약
4. **새 테스트 추가** — 테스트는 test-writer의 책임

### 허용되는 테스트 파일 수정

- import/require 경로 수정 (모듈 경로가 구현과 달라졌을 때)
- 테스트 파일의 경로 참조 수정 (파일 이동으로 인한)

이 경우에도 assertion 로직은 절대 변경 금지.

## Workflow

### 1. Read Session File

세션 파일에서 추출:
- `target`, `language`, `conflict` 모드
- Requirements, Constraints, Technical Context
- `mapping_file` 경로 → Read → 테스트 파일 목록, Constraint↔Test 매핑
- Implementation Tasks (있을 때만)

### 2. Understand Tests

mapping.json의 test_files에 나열된 테스트 파일을 Read:
- 각 테스트가 검증하는 인터페이스(함수명, 파라미터, 반환값) 파악
- Constraint별로 어떤 테스트가 무엇을 검증하는지 이해

### 3. GREEN — Implement (max 3 attempts)

```
attempt = 1
loop:
  1. 테스트에서 요구하는 인터페이스에 맞춰 프로덕션 코드 작성/수정
     - Requirements: 고수준 기능 구현
     - Constraints (수치): 상수 + 검증 로직
     - Constraints (형식): guard clause
     - Constraints (보안): 보안 로직
     - Technical Context: 구현 방식 (라이브러리, 패턴)
     - Implementation Tasks: [ADD]는 새 파일, [MODIFY]는 기존 수정

  2. 테스트 실행 (언어별):
     | 언어 | 명령 |
     | TypeScript | npx jest {test_files} 2>&1 |
     | Rust | cargo test 2>&1 |
     | Python | python -m pytest {test_files} -v 2>&1 |
     | Go | go test ./... -v 2>&1 |

  3. 결과 확인:
     - 전부 통과 → break (success)
     - 일부 실패 → 실패 원인 분석 → attempt++
     - attempt > 3 → break (partial)
```

### 4. File Conflicts

세션 파일의 conflict 모드에 따라 처리:
- `skip`: 기존 파일 유지
- `overwrite`: 덮어쓰기

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

## 병렬 실행 주의

이 Agent는 병렬 배치로 실행될 수 있습니다. **AskUserQuestion 사용 금지.**
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
  세션 파일: ${TMP_DIR}refactor-session-src-auth.md
  대상 디렉토��: src/auth
  ���지된 언어: typescript
  ���과는 ${TMP_DIR}에 저�����고 경로만 반환
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

## 입력

```
세션 파일: <path> (refactor session file, pre-extracted by SKILL)
대�� 디렉토리: <path>
감지된 언어: <lang>
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

## 임시 디렉토리

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
```

## 절대 금지 (HARD CONSTRAINTS)

다음 행위는 어떤 상황에서도 금지됩니다:

1. **테스트의 assertion 로직 수정** — expect(), assert, assertEqual 등의 검증 로직
2. **테스트 케이스 삭제 또는 비활성화** — skip, xfail, .skip(), xit, @disabled 등
3. **테스트의 expected value 변경** — 기대값은 동결된 계약
4. **외부 동작(public API) 변경** — 함수 시그니처, 반환 타입, export 목록

### 허용되는 변경

- 프로덕션 코드의 내부 구조 변경 (변수명, 함수 분리, 파일 분리)
- Conventions 기반 코드 스타일 조정 (네이밍 규칙, 코딩 규칙, 프로젝트 구조)
- import 경로 조정 (파일 이동에 따른)

## Workflow

### 1. Read Session File

세션 파일에서 추출:
- `target`, `language`
- Conventions (resolved) — 6개 서브섹션
- `mapping_file` 경로 → Read → 테스트 파일 목록
- Implementation Files 목록

### 2. Snapshot

리팩토링 전 상태를 기록 (롤백용):
```bash
git stash push -m "refactor-snapshot-{dir-safe}" -- {target}/*
git stash pop
```
또는 git diff로 현재 상태 기록.

실제 롤백은 `git checkout -- {파일들}`로 수행.

### 3. Apply Conventions

Conventions 서브섹션별 적용:

| Convention | 적용 대상 |
|-----------|----------|
| Naming Rules | 변수/함수/클래스/상수명 |
| Coding Rules | 패턴, 구조, 에러 처리 방식 |
| Project Structure | 파일 위치, 디렉토리 구조 |
| Module Boundaries | import 방향, 의존성 규칙 |
| Naming Conventions | 모듈/디렉토리/패키지명 |
| Language & Runtime | 언어/런타임 관례 |

Implementation Files 목록의 파일만 대상으로 함.
테스트 파일은 수정 대상이 아님.

### 4. Regression Test

```bash
# 언어별 테스트 실행
# TypeScript: npx jest {test_files} 2>&1
# Rust: cargo test 2>&1
# Python: python -m pytest {test_files} -v 2>&1
# Go: go test ./... -v 2>&1
```

**전부 통과:** → success
**하나라도 실패:** → 롤백

### 5. Rollback (실패 시)

```bash
git checkout -- {refactored_files}
```

롤백 후 테스트 재실행하여 원래 상태 복원 확인.

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

`skipped`: Conventions가 없거나 적용할 변경이 없는 경우.

## 병렬 실행 주의

이 Agent는 병렬 배치로 실행될 수 있습니다. **AskUserQuestion 사용 금지.**
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
{Requirements 섹션 전체}

## Constraints (from DEVELOPERS.md)
{Constraints 섹션 전체 — 테스트 생성 원천}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션 — 타입 참조용, 테스트 생성 원천 아님}

## Technical Context
{Technical Context 섹션 전체}

## Conventions (resolved)
{계층 해소된 Conventions}

## Dependencies
{dev-context 또는 탐색 결과}

## Implementation Tasks (Spec Changes 있을 때만)
- [ADD] CONST-N: {설명}
- [MODIFY] CONST-N: {변경 내용}
- [DELETE] CONST-N: {삭제 대상}

## Spec Changes (optional — spec 커밋 발견 시에만 포함)
breaking: {true|false}

### Transition Context
{전환 맥락 — 어디서 어디로, 왜}

### Added
{추가된 Requirements/Constraints}

### Modified
{변경된 Requirements/Constraints}

### Removed
{���제된 Requirements/Constraints}

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
{Requirements 섹션 전체}

## Constraints (from DEVELOPERS.md)
{Constraints 섹션 전체}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션}

## Technical Context
{Technical Context}

## Conventions (resolved)
{계층 해소된 Conventions}

## Implementation Tasks (Spec Changes 있을 때만)
- [ADD] CONST-N: {설명}
- [MODIFY] CONST-N: {변경 내용}

## Existing Test Directory (Incremental 모드, 기존 테스트 있을 때만)
existing_test_dir: {path}/{detected_test_dir}/

## Dependencies
{dev-context ��는 탐색 결과}
`` `

### mode=revise

`` `markdown
# Test Writer Session
type: test-writer | mode: revise | round: {N} | target: {path} | language: {lang}
test_output_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json
feedback_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{N-1}.md

## Origin
(동일)

## Requirements (from CLAUDE.md)
(동일)

## Constraints (from DEVELOPERS.md)
(동일)

## Data Schemas (from DEVELOPERS.md, reference only)
(동일)

## Technical Context
(동일)

## Conventions (resolved)
(동일)

## Implementation Tasks
(동일)

## Existing Test Directory
(동일)

## Dependencies
(동일)
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

## Implementation Tasks (Spec Changes 있을 때만)
{[ADD]/[MODIFY] 태스크만 — DELETE는 SKILL이 이미 처리}

## Dependencies
{dependencies}
`` `

## Refactorer Session File Format

`` `markdown
# Refactorer Session
type: refactor | target: {path} | language: {lang}

## Conventions (resolved)
{계층 해���된 Conventions}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Files
{green-coder result에서 추출한 파일 목록}
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
      "text": "유효한 토큰으로 사용자 인증 가능",
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

| 상황 | 대응 |
|------|------|
| 세션 파일 파싱 실패 | Agent 실패 반환 |
| test-writer unmapped > 0 | partial 상태 반환 |
| test-reviewer max_safety 도달 | best-effort 진행, 경고 |
| Verify RED 컴파일 실패 | green-coder에 위임 (import fix 허용) |
| GREEN 3회 실패 | partial 상태 반환 |
| REFACTOR 회귀 실패 | 롤백, rolled_back 상태 반환 |
| 파일 쓰기 실패 | 해당 파일 건너뛰기 |
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

The complete rewritten SKILL.md should follow the design spec's "dev SKILL 전체 흐름" section exactly, with:
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
| Agent | Superpowers 조합 | 역할 |
|-------|-----------------|------|
| `decompose` | (없음) | 대규모 스펙 → 모듈 분해 계획 |
| `impl` | brainstorming | 요구사항 분석 + CLAUDE.md/DEVELOPERS.md 생�� |
| `test-writer` | (없음) | RED — 스펙 → 테스트 + Constraint↔Test 매핑 |
| `test-reviewer` | (없음) | 스펙 대비 테스트 트레이서빌리티 검증 |
| `green-coder` | (없음) | GREEN — approved 테스트 통과시키는 최소 구현 |
| `refactorer` | (없음) | REFACTOR — Conventions 적용 + 회귀 테스트 |
| `validator` | verification-before-completion | semantic drift 검출 |
| `decompiler` | (없음) | 소스코드 → CLAUDE.md/DEVELOPERS.md 추출 |
```

- [ ] **Step 3: Update /dev architecture diagram**

Replace the `/dev` diagram in the Architecture section with:

```markdown
#### /dev (CLAUDE.md → 소스코드)

`` `
User: /dev [--all] [--conflict skip|overwrite] [--dry-run] [--validate]
        │
        ▼
┌─────────────────────────────────────────────┐
│ dev SKILL                                   │
│                                             │
│ 1. 대상 결정 (--all 또는 incremental)       │
│ 2. 언어 감지 + Spec Changes 분석            │
│ 3. [DELETE] 태스크 SKILL이 직접 실행         │
│ 4. Test Writing Loop (per target):          │
│    Task(test-writer) → Task(test-reviewer)  │
│    → feedback loop (max 5)                  │
│ 5. TMP → target 복사 + Verify RED           │
│ 6. Task(green-coder) per target             │
│ 7. Task(refactorer) per target              │
│ 8. 빌드 검증 + git diff + dev 커밋          │
└─────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────┐  ┌──────────────────────┐
│ test-writer AGENT     │  │ test-reviewer AGENT   │
│                       │  │                       │
│ Constraints → 테스트  │◄►│ 5-criteria 검증       │
│ Requirements → accept │  │ verdict: approved     │
│ mapping.json 생성     │  │         | rejected    │
└───────────────────────┘  └──────────────────────┘
        │ approved
        ▼
┌───────────────────────┐  ┌──────────────────────┐
│ green-coder AGENT     │  │ refactorer AGENT      │
│                       │  │                       │
│ approved 테스트 기반  │─►│ Conventions 적용      │
│ 최소 구현 (max 3)    │  │ 회귀 실패 시 롤백     │
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
