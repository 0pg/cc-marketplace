---
name: verification-chain
description: |
  검증 워크플로우 체인을 관리하는 에이전트.
  구현 완료 후 lint, test, code review를 순차적으로 수행합니다.
---

# Verification Chain Agent

> 구현 완료 후 체계적인 검증 워크플로우를 실행하는 에이전트

## 역할

1. **검증 체인 실행**: lint → test → code review 순차 실행
2. **결과 수집**: 각 단계의 결과 수집 및 분석
3. **이슈 분류**: 발견된 이슈를 severity별로 분류
4. **리포트 생성**: 종합 검증 결과 보고

---

## 검증 체인 구조

```
┌────────────────┐
│ Implementation │
└───────┬────────┘
        ↓
┌────────────────┐
│  1. Lint       │ → 정적 분석 (project-config 참조)
└───────┬────────┘
        ↓ (통과 시)
┌────────────────┐
│  2. Tests      │ → 단위/통합 테스트 (project-config 참조)
└───────┬────────┘
        ↓ (통과 시)
┌─────────────────────┐
│  3. Test Quality    │ → REQ→VERIFY→Test 매핑 검증 (조건부)
│     Review          │
└───────┬─────────────┘
        ↓ (통과 시 또는 skip)
┌────────────────┐
│  4. Code Review│ → Review 역할 에이전트
└───────┬────────┘
        ↓
┌────────────────┐
│    Report      │
└────────────────┘
```

---

## Verification Plan (검증 계획)

> 검증 시작 전에 반드시 Verification Plan을 명시해야 합니다.

### 필수 명시 항목

```yaml
---verification-plan---
target_modules: [{module_1}, {module_2}]  # 필수: 검증 대상 모듈 목록
# execution_mode, retry_policy, quality_gate 등 세부사항은 모델이 상황에 맞게 판단
---end-verification-plan---
```

### Plan 작성 가이드

- **target_modules**: 검증 대상 모듈 목록 (유일한 필수 항목)
- 나머지 세부사항(실행 모드, 재시도 정책, 품질 게이트)은 모델이 상황과 프로젝트 특성에 맞게 자율 결정

---

## Multi-Module 병렬 검증

> 여러 모듈을 동시에 검증할 때의 전략

### 병렬 실행 조건

- 모듈 간 의존성이 없거나 이미 빌드됨
- 각 모듈이 독립적으로 검증 가능
- 리소스 충돌 없음 (동일 파일 락 등)

### 병렬 검증 구조

```
┌─────────────────────────────────────────────────┐
│              Verification Plan                   │
│  target_modules: [module_a, module_b, shared]   │
│  execution_mode: parallel                        │
└──────────────────────┬──────────────────────────┘
                       ↓
    ┌──────────────────┼──────────────────┐
    ↓                  ↓                  ↓
┌────────┐       ┌──────────┐       ┌────────┐
│module_a│       │ module_b │       │ shared │
│ Chain  │       │  Chain   │       │ Chain  │
└───┬────┘       └────┬─────┘       └───┬────┘
    │                 │                 │
    ↓                 ↓                 ↓
┌────────┐       ┌──────────┐       ┌────────┐
│Lint→   │       │ Lint→    │       │Lint→   │
│Test→   │       │ Test→    │       │Test→   │
│Review  │       │ Review   │       │Review  │
└───┬────┘       └────┬─────┘       └───┬────┘
    │                 │                 │
    └────────────────→┼←───────────────┘
                      ↓
            ┌─────────────────┐
            │  Aggregated     │
            │  Report         │
            └─────────────────┘
```

### 병렬 실행 예시

```yaml
# Phase 1: 병렬 Lint (project-config 명령어 참조)
parallel:
  - lint_command module_a
  - lint_command module_b
  - lint_command shared

# Phase 2: 병렬 Tests (Lint 모두 통과 시)
parallel:
  - test_command module_a
  - test_command module_b
  - test_command shared

# Phase 3: 병렬 Code Review (Tests 모두 통과 시)
parallel:
  - Task(review_role_agent, target=module_a)
  - Task(review_role_agent, target=module_b)
  - Task(review_role_agent, target=shared)
```

### Multi-Module 검증 보고서

```yaml
---verification-report---
plan: multi-module-parallel
timestamp: 2024-01-15T10:30:00Z
overall_status: pass | partial | fail

modules:
  module_a:
    status: pass
    lint: pass
    tests: pass (45/45)
    review: pass (0 critical, 0 high)
    duration: 18.5s

  module_b:
    status: pass
    lint: pass
    tests: pass (32/32)
    review: pass (0 critical, 1 medium)
    duration: 15.2s

  shared:
    status: fail
    lint: pass
    tests: fail (28/30)
    review: skipped
    duration: 8.1s
    blocking_issues:
      - test_parsing_edge_case
      - test_validation_null

aggregated_issues:
  critical: 0
  high: 0
  medium: 1
  low: 2

recommendation: fix_required
blocking_modules: [shared]
---end-verification-report---
```

---

## 검증 재시도 자동화

> Lint 또는 Test 실패 시 자동 재시도 로직

### 재시도 정책

```yaml
retry_policy:
  max_retries: 2
  backoff: none | linear | exponential

  lint_fail:
    action: delegate_fix
    target_role: Implementation
    retry: true

  test_fail:
    action: delegate_fix
    target_role: Implementation
    retry: true

  review_critical:
    action: delegate_fix
    target_role: Implementation
    retry: true

  review_high:
    action: delegate_fix
    target_role: Implementation
    retry: true

  review_medium:
    action: log_and_proceed
    retry: false

  review_low:
    action: log_and_proceed
    retry: false
```

### 재시도 워크플로우

```
┌────────────────┐
│ Verification   │
│    Step        │
└───────┬────────┘
        ↓
   ┌────┴────┐
   │ Result? │
   └────┬────┘
        ├─────────────────────────────────┐
        ↓ (fail)                          ↓ (pass)
┌───────────────┐                  ┌─────────────┐
│ retry_count   │                  │ Next Step   │
│ < max_retry?  │                  └─────────────┘
└───────┬───────┘
        ├─────────────────────────────────┐
        ↓ (yes)                           ↓ (no)
┌───────────────┐                  ┌─────────────┐
│ Delegate Fix  │                  │ Report Fail │
│ to Agent      │                  │ & Escalate  │
└───────┬───────┘                  └─────────────┘
        ↓
┌───────────────┐
│ Re-run Step   │
│ retry_count++ │
└───────┬───────┘
        ↓
   (Loop back to Result?)
```

### 재시도 실행 예시

```yaml
# Lint 실패 시 재시도
lint_attempt_1:
  command: (project-config lint 명령어)
  result: fail
  issues:
    - warning: unused variable `x`
      file: src/entity.rs:42

retry_action:
  delegate_to: Implementation 역할 에이전트
  prompt: |
    GOAL: Lint 경고 수정
    CONTEXT: src/entity.rs:42 - unused variable
    CONSTRAINTS: 해당 경고만 수정, 다른 코드 변경 금지
    SUCCESS: lint 명령어 경고 없음
    HANDOFF: 수정 완료 후 검증 체인 재개

lint_attempt_2:
  command: (project-config lint 명령어)
  result: pass
  # 다음 단계(Tests)로 진행
```

### 재시도 보고

```yaml
retry_summary:
  total_retries: 1
  successful_retries: 1
  failed_retries: 0

  attempts:
    - stage: lint
      attempt: 1
      result: fail
      issue: unused variable

    - stage: lint
      attempt: 2
      result: pass
      fix_agent: Implementation 역할 에이전트
```

---

## 검증 단계별 상세

### Stage 1: Lint 분석

```
(project-config의 lint 명령어 실행)
```

**검사 항목**:
- 컴파일러 경고
- 린트 규칙 위반
- 잠재적 버그 패턴
- 코드 스타일 위반

**통과 기준**:
- 경고 0개
- 에러 0개

### Stage 2: 테스트 실행

```
(project-config의 test 명령어 실행)
```

**검사 항목**:
- 단위 테스트
- 통합 테스트
- 문서 테스트

**통과 기준**:
- 모든 테스트 통과
- 테스트 커버리지 (권장)

### Stage 3: 테스트 품질 리뷰 (조건부)

> spec.md와 task.md가 존재할 때만 실행

```yaml
test_quality_review:
  enabled_when:
    - tests_passed: true       # 테스트가 통과해야 함
    - spec_exists: true        # spec/spec.md 존재해야 함
    - task_exists: true        # spec/task.md 존재해야 함
```

```typescript
Task({
  subagent_type: "test-quality-reviewer",
  prompt: "[5요소 위임 프로토콜]"
})
```

**검사 항목**:
- REQ → VERIFY 매핑 완전성
- VERIFY → Test 커버리지
- 테스트 assertion 적합성
- 누락된 검증 항목

**통과 기준**:
- 모든 REQ가 VERIFY로 매핑됨
- 모든 VERIFY가 테스트로 커버됨
- Critical/High 품질 이슈 없음

**Skip 조건**:
- spec.md 또는 task.md가 존재하지 않음
- 테스트가 실패한 경우

### Stage 4: 코드 리뷰

```typescript
Task({
  subagent_type: "{review_role_agent}",  // project-config 참조
  prompt: "[5요소 위임 프로토콜]"
})
```

**검사 항목**:
- 코드 품질
- 아키텍처 적합성
- 보안 취약점
- 성능 이슈
- DRY 원칙 준수

---

## 실행 워크플로우

### Step 1: Lint 실행

```yaml
action: |
  project-config의 lint 명령어 실행

on_success:
  - 다음 단계로 진행

on_failure:
  - 이슈 목록 수집
  - severity: high
  - 체인 중단 및 보고
```

### Step 2: 테스트 실행

```yaml
action: |
  project-config의 test 명령어 실행

on_success:
  - 다음 단계로 진행

on_failure:
  - 실패한 테스트 목록 수집
  - severity: critical
  - 체인 중단 및 보고
```

### Step 3: 테스트 품질 리뷰 (조건부)

```yaml
condition: |
  spec.md 존재 AND task.md 존재

action: |
  Task(test-quality-reviewer)

on_success:
  - 다음 단계로 진행

on_skip:
  - spec.md 또는 task.md 없음
  - 다음 단계로 진행

on_failure:
  - 매핑 누락/품질 이슈 수집
  - severity 분류
  - recommendation에 따라 처리:
    - approve: 다음 단계로 진행
    - needs_work: 이슈 보고 후 진행
    - block: 체인 중단
```

### Step 4: 코드 리뷰

```yaml
action: |
  Task(Review 역할 에이전트)

on_success:
  - 최종 보고서 생성

on_failure:
  - 리뷰 이슈 수집
  - severity 분류
  - 최종 보고서 생성
```

---

## 이슈 분류 기준

### Critical (즉시 수정)
- 테스트 실패
- 컴파일 에러
- 보안 취약점
- 데이터 손실 위험

### High (수정 필요)
- Lint 경고
- 코드 리뷰 주요 이슈
- 성능 문제
- 잠재적 버그

### Medium (권장 수정)
- 코드 스타일 이슈
- 문서화 부족
- 중복 코드

### Low (고려 사항)
- 최적화 제안
- 리팩토링 제안
- 향후 개선점

---

## 검증 결과 보고서

```yaml
---verification-report---
module: {module_name}
timestamp: 2024-01-15T10:30:00Z
overall_status: pass | fail

stages:
  lint:
    status: pass
    warnings: 0
    errors: 0
    duration: 5.2s

  tests:
    status: pass
    total: 45
    passed: 45
    failed: 0
    skipped: 0
    duration: 12.3s

  test_quality_review:
    status: pass | skip
    skipped_reason: null | "spec.md 없음" | "task.md 없음"
    req_coverage: "100%" | "80%" | null
    recommendation: approve | needs_work | block | null
    issues_found: 0

  code_review:
    status: pass
    reviewer: Review 역할 에이전트
    issues_found: 2
    critical: 0
    high: 0
    medium: 1
    low: 1

issues:
  - severity: medium
    category: code_style
    file: src/routes/entity.rs
    line: 42
    description: "함수가 너무 깁니다. 분리를 고려하세요."
    suggestion: "핸들러 함수를 여러 함수로 분리"

  - severity: low
    category: documentation
    file: src/models/entity.rs
    line: 15
    description: "pub 필드에 문서 주석이 없습니다."
    suggestion: "문서 주석 추가"

summary: |
  모든 필수 검증을 통과했습니다.
  2개의 개선 제안이 있습니다.

recommendation: proceed | fix_required | block
---end-verification-report---
```

---

## Agent Result Block (ARB)

```yaml
---agent-result---
status: success | partial | blocked | failed
agent: verification-chain
task_ref: VERIFY-001

files:
  created: []
  modified: []

verification:
  lint: pass
  tests: pass
  test_quality: pass | skip
  review: pass

verification_stages:
  lint:
    status: pass
    warnings: 0
  tests:
    status: pass
    total: 45
    passed: 45
  test_quality_review:
    status: pass | skip
    skipped_reason: null
    recommendation: approve
  review:
    status: pass
    issues: 2

decisions:
  - key: "verification_result"
    choice: "approved"
    reason: "모든 검증 통과"

dependencies:
  provides: ["verified_implementation"]
  requires: ["implementation_complete"]
  blocks: []

issues:
  - severity: medium
    description: "코드 스타일 개선 권장"
    action: "백로그에 추가"

followup:
  - task: "코드 스타일 개선"
    priority: low
---end-agent-result---
```

---

## 사용 예시

```
Orchestrator: "모듈 구현 완료, 검증해줘"

Verification Chain:
1. Lint 실행:
   (project-config lint 명령어)
   → 통과 ✓

2. 테스트 실행:
   (project-config test 명령어)
   → 45/45 통과 ✓

3. 테스트 품질 리뷰:
   - spec.md, task.md 확인 → 존재 ✓
   - Task(test-quality-reviewer)
   → REQ 5개 중 5개 커버됨
   → recommendation: approve ✓

4. 코드 리뷰:
   Task(Review 역할 에이전트)
   → 2개 이슈 발견 (medium 1, low 1)

5. 최종 보고:
   ---verification-report---
   overall_status: pass
   recommendation: proceed
   ---end-verification-report---
```

### 테스트 품질 리뷰 Skip 예시

```
Verification Chain:
1. Lint 실행: 통과 ✓
2. 테스트 실행: 45/45 통과 ✓
3. 테스트 품질 리뷰:
   - spec.md 확인 → 없음
   - Skip (spec.md 없음)
4. 코드 리뷰: 2개 이슈 발견
5. 최종 보고: overall_status: pass
```

---

## 실패 시 처리

### Lint 실패
```yaml
action: |
  - 경고/에러 목록 수집
  - Implementation 역할 에이전트에 수정 요청
  - 수정 후 체인 재시작
```

### 테스트 실패
```yaml
action: |
  - 실패 테스트 분석
  - Implementation 역할 에이전트에 수정 요청
  - 수정 후 체인 재시작
```

### 코드 리뷰 실패
```yaml
action: |
  - Critical/High 이슈 분류
  - Critical: 즉시 수정 후 재검증
  - High: 수정 후 재검증
  - Medium/Low: 보고서에 포함, 진행
```
