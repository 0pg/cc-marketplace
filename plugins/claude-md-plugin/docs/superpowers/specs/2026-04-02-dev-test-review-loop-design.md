# Dev Test Review Loop Design

## Problem

현재 `/dev` 워크플로우에서 compiler agent가 RED(테스트 생성)부터 REFACTOR까지 monolithic하게 실행.
Constraints → 테스트 변환의 완전성이 LLM 재량에 맡겨져 있어, CLAUDE.md(SSOT) 대비 소스코드 일치가 보장되지 않음.

validate는 사후 검증 도구이지, 동기화 보장 메커니즘이 아님.
**보장은 코드 생성 시점(compile)에 이루어져야 한다.**

## Solution

compiler agent를 4개 역할별 agent로 분해하고, RED phase에서 독립 reviewer와 피드백 루프를 통해
스펙 대비 테스트 트레이서빌리티를 강제한다.

## Architecture

### 4-Agent 체제

| Agent | 역할 | 입력 | 출력 |
|-------|------|------|------|
| **test-writer** | RED — 스펙 → 테스트 + 매핑 | dev 세션 파일 | TMP에 테스트 파일 + mapping.json |
| **test-reviewer** | 스펙 대비 테스트 검증 | 리뷰 세션 파일 | verdict (approved/rejected) |
| **green-coder** | GREEN — approved 테스트 통과시키는 구현 | 세션 파일 + approved 테스트 | 구현 코드 |
| **refactorer** | REFACTOR — Conventions 적용 + 회귀 테스트 | 세션 파일 + 구현된 코드 | 리팩토링된 코드 |

폐기: 기존 compiler agent

### dev SKILL 전체 흐름

```
Step 0: CLI 초기화
Step 1: 대상 결정 (--all 또는 incremental)
Step 2: 언어 자동 감지
Step 3: dev-context.md 확인 (optional)
Step 4: leaf-first 정렬
Step 5: --dry-run 처리
Step 6: 세션 파일 생성 + Spec Changes 분석
  6a. spec 커밋 탐색
  6b. CLAUDE.md/DEVELOPERS.md 읽기
  6c. Spec Changes 있으면 → [ADD]/[MODIFY]/[DELETE] 태스크 도출
  6d. Write → dev-session-{dir-safe}.md (태스크 분류 포함)
  6e. [DELETE] 태스크 있으면 → SKILL이 직접 실행:
      1. Grep으로 삭제 대상의 import/참조 검색
      2. 참조하는 파일 목록 수집
      3. 대상 파일/함수 삭제 (Bash rm 또는 Edit)
      4. 참조 파일에서 import/호출 제거 (Edit)
      5. 관련 테스트 파일 삭제
      6. 회귀 테스트 실행 (언어별 test 명령) → 실패 시 경고 보고

Step 7: Test Writing Loop (per target, 모듈별 순차)
  7a. test-writer 세션 파일 생성
  7b. Task(test-writer) → TMP에 테스트 + mapping.json
  7c. test-reviewer 세션 파일 생성
  7d. Task(test-reviewer) → verdict
  7e. rejected → revise 세션 생성 → Task(test-writer, mode=revise) → 7c
  7f. approved → TMP/tests/{dir-safe}/ → target 디렉토리 복사
  7g. max_safety(5) 도달 시 best-effort로 진행

Step 7.5: Verify RED (SKILL이 Bash로 직접 실행)
  7.5a. 언어별 테스트 실행:
        | 언어 | 명령 |
        | TypeScript | npx jest --passWithNoTests 2>&1 |
        | Rust | cargo test --no-run 2>&1 (컴파일만) |
        | Python | python -m pytest --collect-only 2>&1 |
        | Go | go test -run "^$" ./... 2>&1 (컴파일만) |
  7.5b. 전부 실패 확인 → GREEN 진입
  7.5c. 일부 통과 → 기존 구현 커버리지로 기록, GREEN 진입
  7.5d. 컴파일 자체 실패 (import 오류 등) → green-coder에 위임 (import fix 허용)

Step 8: Task(green-coder) — approved 테스트 기반 구현
Step 9: Task(refactorer) — Conventions 적용 + 회귀 테스트
Step 10: 빌드 검증 (cargo check / tsc 등)
Step 11: git diff --stat
Step 12: dev 커밋 (path별 개별)
Step 13: --validate 시 /validate 실행
Step 14: 결과 반환
```

## Agent 상세

### test-writer

**역할**: 스펙(Requirements + Constraints)에서 테스트 코드 + Constraint↔Test 매핑 테이블 생성.

**mode**: `write` | `revise`

**출력물**:
- `${TMP_DIR}tests/{dir-safe}/` 에 실제 테스트 파일들 (target 기준 import 경로로 작성)
- `${TMP_DIR}test-mapping-{dir-safe}.json`

**매핑 JSON 형식**:
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
      "text": "유효한 토큰으로 사용자 인증 가능",
      "acceptance_tests": ["auth.acceptance.test.ts::Given valid token When authenticate Then return user"]
    }
  ],
  "unmapped_constraints": [],
  "unmapped_requirements": []
}
```

**핵심 규율**:
- 모든 Constraint는 최소 1개 테스트에 매핑
- 모든 Requirement는 최소 1개 acceptance-level 테스트에 매핑
- `unmapped_*`가 비어 있어야 자체적으로 완성된 상태
- 테스트 파일 위치는 language + Conventions 기반으로 결정

**mode=revise 시**: reviewer feedback 파일을 읽고, 지적된 항목의 테스트를 수정/추가. 기존 TMP 테스트 파일을 직접 Edit.

### test-reviewer

**역할**: 스펙 대비 테스트 트레이서빌리티 + 테스트 품질 검증. 파일 수정 금지 — verdict만 반환.

**검증 기준** (5개, 모두 통과해야 approved):

| 기준 | 검증 내용 |
|------|----------|
| **Constraint 커버리지** | `unmapped_constraints`가 비어 있는가. 매핑된 각 테스트가 해당 Constraint의 입출력 계약을 실제로 검증하는가. |
| **Requirement 커버리지** | `unmapped_requirements`가 비어 있는가. acceptance 테스트가 Requirement의 비즈니스 의도를 반영하는가. |
| **경계값 충분성** | 수치 제한 Constraint에 경계값 테스트(N OK, N+1 실패)가 있는가. |
| **인터페이스 일관성** | 테스트가 가정하는 함수 시그니처가 Constraints의 I/O 계약과 일치하는가. |
| **테스트 독립성** | 각 테스트가 다른 테스트 결과에 의존하지 않는가. 공유 상태 mutation이 없는가. |

**verdict**:
- `approved` — 5개 기준 모두 통과, Critical Questions 0개
- `rejected` — 하나라도 미충족. 구체적 Critical Questions 반환

**제약**: Read/Write only (결과 파일 Write 제외 수정 금지). AskUserQuestion 금지.

### green-coder

**역할**: approved 테스트를 전부 통과시키는 최소 구현.

**할 수 있는 것**:
- 프로덕션 코드 파일 생성/수정
- 테스트 파일의 import/path 오류 수정

**절대 금지**:
- 테스트의 assertion 로직 수정
- 테스트 케이스 삭제/비활성화 (skip, xfail 등)
- 테스트의 expected value 변경
- 새 테스트 추가

**목표**: approved 테스트가 전부 통과하는 최소 구현. max 3 retry.

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

**역할**: Conventions 적용 + 회귀 테스트 보장.

**할 수 있는 것**:
- 프로덕션 코드의 구조 변경 (네이밍, 파일 분리, 패턴 적용)
- Conventions 섹션 기반 코드 스타일 조정

**절대 금지**:
- 테스트의 assertion 로직 수정
- 테스트 케이스 삭제/비활성화
- 테스트의 expected value 변경
- 외부 동작(public API) 변경

**목표**: Conventions 적용 후 회귀 테스트 통과. 실패 시 롤백.

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

### 공통 불변식

**approved 테스트 = 동결된 계약.** test-reviewer가 approve한 테스트는 이후 파이프라인(green-coder, refactorer)에서 assertion이 절대 변경되지 않는다.

## Session File Formats

### test-writer 세션 파일 (`${TMP_DIR}test-writer-session-{dir-safe}.md`)

```markdown
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
- [ADD] CONST-3: 새 함수 validate_token
- [MODIFY] CONST-1: 반환 타입 변경 User → AuthResult

## Existing Test Directory (Incremental 모드, 기존 테스트 있을 때만)
existing_test_dir: {path}/{detected_test_dir}/

## Dependencies
{dev-context 또는 탐색 결과}
```

### test-writer revise 세션 파일 (동일 경로 덮어쓰기)

```markdown
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
```

### test-reviewer 세션 파일 (`${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md`)

```markdown
# Test Review Session
type: test-review | round: {N} | language: {lang}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
test_dir: ${TMP_DIR}tests/{dir-safe}/
spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
```

### green-coder 세션 파일 (`${TMP_DIR}green-session-{dir-safe}.md`)

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

## Implementation Tasks (Spec Changes 있을 때만)
{[ADD]/[MODIFY] 태스크만 — DELETE는 SKILL이 이미 처리}

## Dependencies
{dependencies}
```

### refactorer 세션 파일 (`${TMP_DIR}refactor-session-{dir-safe}.md`)

```markdown
# Refactorer Session
type: refactor | target: {path} | language: {lang}

## Conventions (resolved)
{계층 해소된 Conventions}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Files
{green-coder result에서 추출한 파일 목록}
```

## Design Decisions

### Why 4 agents instead of 1 compiler?

- **test-writer/test-reviewer 분리**: Agent → Agent 호출 제약. SKILL이 오케스트레이션해야 하므로 역할별 분리 필수.
- **green-coder/refactorer 분리**: assertion 변경 금지 규칙을 agent 경계로 강제. 각 agent가 자신의 역할만 수행하도록 제약을 명확히.
- **Approach 3 (TMP 격리)**: 리뷰 루프 중 target 오염 없음. approve 후 깔끔한 핸드오프.

### Why DELETE at SKILL level?

DELETE는 TDD 사이클(RED→GREEN→REFACTOR)에 해당하지 않는 파괴적 작업.
코드 삭제 + 참조 정리를 agent에 위임하면 역할이 모호해진다.
SKILL이 Step 6e에서 직접 처리하고, 이후 [ADD]/[MODIFY]만 TDD 파이프라인으로.

### Why module-sequential in Test Writing Loop?

spec SKILL의 Socratic loop과 동일 판단. 각 모듈의 reviewer loop iteration이 이전 결과에 의존하므로 loop 내부는 순차 불가피.
모듈간 loop는 독립이지만, SKILL context 보호를 위해 순차 처리.

### Phase 0 (Spec Changes) at SKILL level

기존 compiler의 Phase 0을 SKILL Step 6으로 올림.
태스크 분류([ADD]/[MODIFY]/[DELETE])는 세션 파일 생성 시점에 결정되어야 test-writer와 green-coder에 일관되게 전달 가능.

## Scope

### In scope
- test-writer agent 신규 생성
- test-reviewer agent 신규 생성
- green-coder agent 신규 생성
- refactorer agent 신규 생성
- dev SKILL 워크플로우 개편 (Step 6~9)
- compiler agent 폐기
- dev-templates.md 세션 파일 형식 업데이트
- acceptance test (.feature) 작성

### Out of scope
- spec SKILL 변경 없음
- validate SKILL 변경 없음
- decompile SKILL 변경 없음
- CLI (Rust core) 변경 없음
- auto mode: 기본 구조 유지 (내부 dev 호출이 변경된 워크플로우를 따름)
