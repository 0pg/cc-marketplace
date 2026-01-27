# Default Verification Chain

> spec.md + task.md 기반 기본 검증 체인

```yaml
verification_chain:
  id: default
  name: "기본 검증 체인"

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
        severity: high

    - id: tests
      name: "테스트 실행"
      condition:
        requires: [lint]
      execution:
        type: command
        command: "{project-config.verification.test}"
      result:
        pass_criteria: "모든 테스트 통과"
        on_fail:
          action: delegate_fix
          max_retries: 2
        severity: critical

    - id: test_quality_review
      name: "테스트 품질 리뷰"
      condition:
        requires: [tests]
        when: "spec_exists AND task_exists"
      execution:
        type: agent
        agent: "orchestrator-guide:test-quality-reviewer"
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

## 단계 설명

### Stage 1: Lint 분석

- **목적**: 정적 코드 분석
- **실행**: project-config의 lint 명령어
- **통과 기준**: 경고/에러 0개
- **실패 시**: Implementation 에이전트에 수정 위임 (최대 2회)

### Stage 2: 테스트 실행

- **목적**: 단위/통합 테스트 검증
- **의존성**: lint 통과 후
- **실행**: project-config의 test 명령어
- **통과 기준**: 모든 테스트 통과
- **실패 시**: Implementation 에이전트에 수정 위임 (최대 2회)

### Stage 3: 테스트 품질 리뷰

- **목적**: REQ → VERIFY → Test 매핑 검증
- **의존성**: tests 통과 후
- **조건**: spec.md AND task.md 존재
- **실행**: test-quality-reviewer 에이전트
- **통과 기준**: 모든 REQ가 테스트로 커버됨
- **실패 시**: 체인 중단 (block)

### Stage 4: 코드 리뷰

- **목적**: 코드 품질, 보안, 성능 검토
- **의존성**: tests 통과 후
- **실행**: project-config의 review_role 에이전트
- **통과 기준**: Critical/High 이슈 없음
- **실패 시**: Implementation 에이전트에 수정 위임

## Stage 흐름

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
│3. Quality│ │4. Code      │
│  Review  │ │   Review    │
│(조건부)  │ │             │
└────┬─────┘ └──────┬──────┘
     │              │
     └──────┬───────┘
            ↓
     ┌────────────┐
     │   Report   │
     └────────────┘
```
