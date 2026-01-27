# TDD Verification Chain

> Red-Green-Refactor 단계별 검증

## 검증 단계

### Stage 1: Lint 분석

- **실행**: `{project-config.verification.lint}`
- **통과 기준**: 경고 0개
- **실패 시**: delegate_fix (Implementation 에이전트에 위임)
- **최대 재시도**: 2회

### Stage 2: TDD 테스트 실행

- **의존성**: Stage 1 통과
- **실행**: `{project-config.verification.test}`
- **통과 기준**: 모든 테스트 통과
- **실패 시**: delegate_fix (최대 3회)
- **심각도**: critical

### Stage 3: REQ-Test 매핑 검증

- **의존성**: Stage 2 통과
- **조건**: tdd-spec.md 존재 시
- **실행**: `Task("tdd-dev:test-reviewer")`
- **통과 기준**: 모든 REQ가 테스트로 커버됨
- **실패 시**: block (체인 중단)
- **심각도**: high

### Stage 4: 코드 리뷰

- **의존성**: Stage 2 통과
- **실행**: `Task(review_role)`
- **통과 기준**: Critical/High 이슈 없음
- **실패 시**: delegate_fix
- **심각도**: high

## 검증 흐름

```
┌────────────────┐
│  1. Lint       │
└───────┬────────┘
        ↓ (통과 시)
┌────────────────┐
│  2. Tests      │
└───────┬────────┘
        ↓ (통과 시)
   ┌────┴────┐
   │         │
   ↓         ↓
┌──────────┐ ┌─────────────┐
│3. REQ-   │ │4. Code      │
│   Test   │ │   Review    │
│  매핑    │ │             │
└────┬─────┘ └──────┬──────┘
     │              │
     └──────┬───────┘
            ↓
     ┌────────────┐
     │   Report   │
     └────────────┘
```

## TDD 전용 검증 특징

### tdd-spec.md 기반 검증

Default workflow의 spec.md + task.md 대신:
- `.claude/tdd-spec.md` 사용
- REQ-XXX → 테스트 직접 매핑
- 중간 VERIFY 단계 생략

### 리뷰어 선택

- **Default workflow**: `test-quality-reviewer` (REQ → VERIFY → Test)
- **TDD workflow**: `test-reviewer` (REQ → Test 직접)

## Chain 스키마

```yaml
verification_chain:
  id: tdd-chain
  name: "TDD 검증 체인"

  stages:
    - id: lint
      name: "Lint 분석"
      execution:
        type: command
        command: "{project-config.verification.lint}"
      result:
        pass_criteria: "경고 0개"
        on_fail:
          action: delegate_fix
          max_retries: 2

    - id: tests
      name: "TDD 테스트 실행"
      condition:
        requires: [lint]
      execution:
        type: command
        command: "{project-config.verification.test}"
      result:
        pass_criteria: "모든 테스트 통과"
        on_fail:
          action: delegate_fix
          max_retries: 3
        severity: critical

    - id: req_test_mapping
      name: "REQ-Test 매핑 검증"
      condition:
        requires: [tests]
        when: "tdd_spec_exists"
      execution:
        type: agent
        agent: "tdd-dev:test-reviewer"
      result:
        pass_criteria: "모든 REQ 커버"
        on_fail:
          action: block
        severity: high

    - id: code_review
      name: "코드 리뷰"
      condition:
        requires: [tests]
      execution:
        type: agent
        agent: "review_role"
      result:
        pass_criteria: "Critical/High 이슈 없음"
        on_fail:
          action: delegate_fix
        severity: high
```
