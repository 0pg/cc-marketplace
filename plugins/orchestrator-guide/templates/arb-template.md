# Agent Result Block (ARB) Template

> 에이전트 실행 결과를 구조화하여 보고하는 표준 형식

---

## ARB 기본 형식

```yaml
---agent-result---
status: success | partial | blocked | failed
agent: {agent_name}
task_ref: {task_id}

files:
  created:
    - {file_path_1}
  modified:
    - {file_path_2}
  deleted:
    - {file_path_3}

verification:
  tests: pass | fail | skip
  lint: pass | fail | skip

decisions:
  - key: "{decision_key}"
    choice: "{chosen_option}"
    reason: "{reasoning}"

dependencies:
  provides: ["{capability_1}", "{capability_2}"]
  requires: ["{requirement_1}"]
  blocks: ["{blocked_task_1}"]

issues:
  - severity: critical | high | medium | low
    description: "{issue_description}"
    action: "{recommended_action}"

followup:
  - task: "{next_task_description}"
    priority: high | medium | low
---end-agent-result---
```

---

## 필드 상세 설명

### status

에이전트 실행 결과 상태:

| 값 | 의미 | 조건 |
|---|------|------|
| `success` | 완전 성공 | 모든 목표 달성, 검증 통과 |
| `partial` | 부분 성공 | 일부 목표 달성, 나머지 미완료 |
| `blocked` | 차단됨 | 외부 의존성으로 진행 불가 |
| `failed` | 실패 | 목표 달성 실패, 에러 발생 |

### agent

실행한 에이전트 이름 (project-config에서 역할별 매핑 참조):

- Implementation 역할 에이전트
- Exploration 역할 에이전트
- Review 역할 에이전트
- 특수 에이전트 (parallel-coordinator, verification-chain)

### task_ref

task.md의 작업 ID:
- 형식: `TASK-XXX` 또는 `{MODULE}-XXX`
- 예: `TASK-001`, `MOD-005`

### files

변경된 파일 목록:

```yaml
files:
  created:
    - {module}/src/models/entity.rs
    - {module}/src/routes/entity.rs
  modified:
    - {module}/src/routes/mod.rs
    - {module}/src/models/mod.rs
  deleted: []
```

### verification

검증 결과:

```yaml
verification:
  tests: pass      # 테스트 명령어 결과
  lint: pass       # lint 명령어 결과
  build: pass      # build 명령어 결과 (선택)
  review: pass     # 코드 리뷰 결과 (선택)
```

### decisions

작업 중 내린 결정:

```yaml
decisions:
  - key: "{decision_key_1}"
    choice: "{chosen_option}"
    reason: "{decision_rationale}"

  - key: "{decision_key_2}"
    choice: "{chosen_option}"
    reason: "{decision_rationale}"
```

### dependencies

의존성 정보:

```yaml
dependencies:
  provides:        # 이 작업이 제공하는 것
    - "entity_model"
    - "entity_api"
  requires:        # 이 작업이 필요로 한 것
    - "database_connection"
  blocks:          # 이 작업이 블로킹하는 것
    - "entity_integration"
```

### issues

발견된 이슈:

```yaml
issues:
  - severity: medium
    description: "함수가 너무 깁니다 (50줄+)"
    action: "리팩토링 권장, 백로그 추가"

  - severity: low
    description: "문서 주석 누락"
    action: "향후 개선"
```

### followup

후속 작업:

```yaml
followup:
  - task: "Review 역할에 리뷰 요청"
    priority: high

  - task: "통합 테스트 작성"
    priority: medium
```

---

## 상태별 ARB 예시

### Success ARB

```yaml
---agent-result---
status: success
agent: {implementation_agent}
task_ref: TASK-002

files:
  created:
    - {module}/src/models/entity.rs
    - {module}/src/routes/entity.rs
  modified:
    - {module}/src/routes/mod.rs
    - {module}/src/models/mod.rs

verification:
  tests: pass
  lint: pass

decisions:
  - key: "{decision_key}"
    choice: "{chosen_option}"
    reason: "{decision_rationale}"

dependencies:
  provides: ["{provided_capability_1}", "{provided_capability_2}"]
  requires: ["{required_dependency}"]
  blocks: []

issues: []

followup:
  - task: "{next_task}"
    priority: high
---end-agent-result---
```

### Partial ARB

```yaml
---agent-result---
status: partial
agent: {implementation_agent}
task_ref: TASK-002

files:
  created:
    - {module}/src/models/entity.rs
  modified:
    - {module}/src/models/mod.rs

verification:
  tests: pass
  lint: pass

decisions:
  - key: "api_implementation"
    choice: "deferred"
    reason: "모델 먼저 완료, API는 다음 단계"

dependencies:
  provides: ["entity_model"]
  requires: ["database_pool"]
  blocks: ["entity_api"]

issues:
  - severity: medium
    description: "API 구현 미완료"
    action: "다음 위임에서 완료"

followup:
  - task: "Entity API 구현"
    priority: high
---end-agent-result---
```

### Blocked ARB

```yaml
---agent-result---
status: blocked
agent: {implementation_agent}
task_ref: TASK-003

files:
  created: []
  modified: []

verification:
  tests: skip
  lint: skip

decisions: []

dependencies:
  provides: []
  requires: ["entity_model"]
  blocks: []

issues:
  - severity: high
    description: "Entity 모델이 아직 구현되지 않음"
    action: "TASK-002 완료 후 재시도"

followup:
  - task: "TASK-002 완료 대기"
    priority: high
---end-agent-result---
```

### Failed ARB

```yaml
---agent-result---
status: failed
agent: {implementation_agent}
task_ref: TASK-002

files:
  created: []
  modified: []

verification:
  tests: fail
  lint: fail

decisions: []

dependencies:
  provides: []
  requires: ["database_pool"]
  blocks: ["entity_api", "entity_integration"]

issues:
  - severity: critical
    description: "컴파일 에러: 매크로 실패"
    action: "환경변수 확인 필요"

  - severity: high
    description: "테스트 DB 연결 실패"
    action: "테스트 환경 설정 확인"

followup:
  - task: "환경 설정 확인"
    priority: high
  - task: "에러 수정 후 재시도"
    priority: high
---end-agent-result---
```

---

## 특수 에이전트 ARB

### parallel-coordinator ARB

```yaml
---agent-result---
status: success
agent: parallel-coordinator
task_ref: PARALLEL-001

parallel_execution:
  total: 3
  success: 3
  failed: 0

agents:
  - name: {implementation_agent_1}
    status: success
    task_ref: TASK-002
  - name: {implementation_agent_2}
    status: success
    task_ref: TASK-003
  - name: {exploration_agent}
    status: success
    task_ref: TASK-001

files:
  created: []
  modified:
    - {module_a}/src/routes/entity.rs
    - {module_b}/src/pages/entity.rs

verification:
  tests: pass
  lint: pass

conflicts: []

dependencies:
  provides: ["entity_api", "entity_ui"]
  requires: []
  blocks: []

followup:
  - task: "통합 리뷰"
    priority: high
---end-agent-result---
```

### verification-chain ARB

```yaml
---agent-result---
status: success
agent: verification-chain
task_ref: VERIFY-001

verification_stages:
  lint:
    status: pass
    warnings: 0
    duration: 5.2s
  tests:
    status: pass
    total: 45
    passed: 45
    failed: 0
  review:
    status: pass
    issues: 2
    critical: 0
    high: 0

files:
  created: []
  modified: []

verification:
  tests: pass
  lint: pass

issues:
  - severity: medium
    category: code_style
    description: "코드 스타일 개선 권장"
    action: "백로그 추가"

followup:
  - task: "코드 스타일 개선"
    priority: low

recommendation: proceed
---end-agent-result---
```

---

## 파싱 가이드

ARB를 파싱할 때:

1. `---agent-result---`와 `---end-agent-result---` 사이 추출
2. YAML로 파싱
3. status 필드로 성공/실패 판단
4. issues 필드로 문제점 확인
5. followup 필드로 다음 작업 결정
