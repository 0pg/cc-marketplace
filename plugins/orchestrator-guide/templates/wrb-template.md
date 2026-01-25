# WRB (Worker Result Block) Template

> Worker가 Phase 완료 후 Orchestrator에 보고하는 표준 형식

---

## WRB 기본 형식

```yaml
---worker-result---
status: success | partial | blocked | failed
worker: worker
phase_ref: {phase_id}

execution_summary:
  total_tasks: {number}
  completed: {number}
  failed: {number}
  strategy: parallel | sequential | hybrid

tasks_executed:
  - agent: {agent_name}
    task_ref: {task_id}
    status: success | partial | blocked | failed
    arb_summary: "{summary}"

files:
  created:
    - {file_path_1}
  modified:
    - {file_path_2}

verification:
  lint: pass | fail | skip
  tests: pass | fail | skip

context_notes: |
  # 다음 Phase에 전달할 맥락

issues:
  - severity: critical | high | medium | low
    description: "{description}"
    action: "{action}"

recommendations: |
  # Orchestrator에 대한 권장
---end-worker-result---
```

---

## 필드 상세 설명

### status

Phase 실행 결과 상태:

| 값 | 의미 | 조건 |
|---|------|------|
| `success` | 완전 성공 | 모든 task 성공, 검증 통과 |
| `partial` | 부분 성공 | 일부 task 성공, 나머지 미완료 |
| `blocked` | 차단됨 | 외부 의존성으로 진행 불가 |
| `failed` | 실패 | 핵심 task 실패, 목표 미달성 |

### worker

항상 "worker"

### phase_ref

OWI에서 받은 phase_ref 그대로 반환:

```yaml
phase_ref: PHASE-001
```

### execution_summary

실행 요약:

```yaml
execution_summary:
  total_tasks: 5        # 분해된 총 task 수
  completed: 5          # 완료된 task 수
  failed: 0             # 실패한 task 수
  strategy: hybrid      # 실행 전략
```

**strategy 값:**
- `parallel`: 모든 task 병렬 실행
- `sequential`: 모든 task 순차 실행
- `hybrid`: 병렬과 순차 혼합

### tasks_executed

실행된 각 task의 결과:

```yaml
tasks_executed:
  - agent: backend-impl
    task_ref: TASK-001
    status: success
    arb_summary: "User 모델 구현 완료, 테스트 3개 통과"

  - agent: backend-impl
    task_ref: TASK-002
    status: success
    arb_summary: "User API CRUD 구현 완료"

  - agent: rust-code-reviewer
    task_ref: TASK-003
    status: success
    arb_summary: "코드 리뷰 완료, 이슈 없음"
```

### files

변경된 파일 목록:

```yaml
files:
  created:
    - backend/src/models/user.rs
    - backend/src/routes/user.rs
    - backend/tests/user_test.rs
  modified:
    - backend/src/models/mod.rs
    - backend/src/routes/mod.rs
```

### verification

검증 결과:

```yaml
verification:
  lint: pass           # lint 명령어 결과
  tests: pass          # test 명령어 결과
  build: pass          # build 명령어 결과 (선택)
```

### context_notes

다음 Phase에 전달할 맥락:

```yaml
context_notes: |
  ## 완료 사항
  - User 엔티티 CRUD 완전 구현
  - 기본 유효성 검증 포함

  ## 발견된 이슈
  - 이메일 중복 체크 추가 권장 (다음 Phase)

  ## 결정 사항
  - UUID 대신 자동 증가 ID 사용 결정
  - 이유: 기존 패턴과 일관성

  ## 다음 Phase 참고
  - 인증 로직 추가 시 user.rs 수정 필요
  - 관련 파일: backend/src/routes/user.rs:45-60
```

### issues

발견된 이슈:

```yaml
issues:
  - severity: medium
    description: "이메일 형식 검증이 기본적임"
    action: "향후 강화된 검증 추가 권장"

  - severity: low
    description: "일부 함수 문서화 누락"
    action: "백로그 추가"
```

### recommendations

Orchestrator에 대한 권장 사항:

```yaml
recommendations: |
  ## 다음 Phase 권장
  - Phase 2에서 인증/인가 구현 권장
  - 이메일 중복 체크 로직 추가

  ## 주의 사항
  - DB 마이그레이션 필요 (user 테이블)
  - 배포 전 마이그레이션 스크립트 실행
```

---

## 상태별 WRB 예시

### Success WRB

```yaml
---worker-result---
status: success
worker: worker
phase_ref: PHASE-001

execution_summary:
  total_tasks: 4
  completed: 4
  failed: 0
  strategy: hybrid

tasks_executed:
  - agent: backend-impl
    task_ref: TASK-001
    status: success
    arb_summary: "User 모델 구현 완료"

  - agent: backend-impl
    task_ref: TASK-002
    status: success
    arb_summary: "User API 구현 완료"

  - agent: backend-impl
    task_ref: TASK-003
    status: success
    arb_summary: "User 테스트 작성 완료"

  - agent: rust-code-reviewer
    task_ref: TASK-004
    status: success
    arb_summary: "코드 리뷰 통과"

files:
  created:
    - backend/src/models/user.rs
    - backend/src/routes/user.rs
    - backend/tests/user_test.rs
  modified:
    - backend/src/models/mod.rs
    - backend/src/routes/mod.rs

verification:
  lint: pass
  tests: pass

context_notes: |
  ## 완료 사항
  - User CRUD 완전 구현
  - 테스트 커버리지 85%

  ## 다음 Phase 참고
  - 인증 미들웨어 연동 지점: routes/user.rs:12

issues: []

recommendations: |
  다음 Phase로 진행 권장
  - Phase 2: 인증/인가 구현
---end-worker-result---
```

### Partial WRB

```yaml
---worker-result---
status: partial
worker: worker
phase_ref: PHASE-001

execution_summary:
  total_tasks: 4
  completed: 2
  failed: 1
  strategy: sequential

tasks_executed:
  - agent: backend-impl
    task_ref: TASK-001
    status: success
    arb_summary: "User 모델 구현 완료"

  - agent: backend-impl
    task_ref: TASK-002
    status: failed
    arb_summary: "API 구현 중 DB 연결 오류"

  - agent: backend-impl
    task_ref: TASK-003
    status: blocked
    arb_summary: "TASK-002 완료 대기"

  - agent: rust-code-reviewer
    task_ref: TASK-004
    status: blocked
    arb_summary: "구현 완료 대기"

files:
  created:
    - backend/src/models/user.rs
  modified:
    - backend/src/models/mod.rs

verification:
  lint: pass
  tests: skip

context_notes: |
  ## 완료 사항
  - User 모델 정의 완료

  ## 미완료 사항
  - API 구현 (DB 연결 문제)
  - 테스트 작성 (API 완료 후)

  ## 블로커
  - DB 연결 설정 확인 필요

issues:
  - severity: high
    description: "DB 연결 실패: connection refused"
    action: "환경 설정 확인 필요"

recommendations: |
  ## 조치 필요
  1. DB 연결 설정 확인
  2. 환경변수 DATABASE_URL 검증
  3. 해결 후 Phase 재실행
---end-worker-result---
```

### Blocked WRB

```yaml
---worker-result---
status: blocked
worker: worker
phase_ref: PHASE-002

execution_summary:
  total_tasks: 3
  completed: 0
  failed: 0
  strategy: sequential

tasks_executed:
  - agent: backend-impl
    task_ref: TASK-005
    status: blocked
    arb_summary: "PHASE-001 완료 대기"

files:
  created: []
  modified: []

verification:
  lint: skip
  tests: skip

context_notes: |
  ## 블로커
  - PHASE-001 미완료
  - User 엔티티 필요

issues:
  - severity: critical
    description: "이전 Phase 미완료"
    action: "PHASE-001 완료 후 재시도"

recommendations: |
  PHASE-001 완료 후 이 Phase 재실행 필요
---end-worker-result---
```

### Failed WRB

```yaml
---worker-result---
status: failed
worker: worker
phase_ref: PHASE-001

execution_summary:
  total_tasks: 4
  completed: 1
  failed: 2
  strategy: sequential

tasks_executed:
  - agent: backend-impl
    task_ref: TASK-001
    status: success
    arb_summary: "User 모델 구현 완료"

  - agent: backend-impl
    task_ref: TASK-002
    status: failed
    arb_summary: "컴파일 에러: 매크로 오류"

  - agent: backend-impl
    task_ref: TASK-003
    status: failed
    arb_summary: "TASK-002 실패로 인한 연쇄 실패"

files:
  created:
    - backend/src/models/user.rs
  modified: []

verification:
  lint: fail
  tests: fail

context_notes: |
  ## 실패 원인
  - 매크로 의존성 버전 불일치
  - sqlx 버전과 호환성 문제

issues:
  - severity: critical
    description: "컴파일 에러: sqlx 매크로 실패"
    action: "의존성 버전 확인 및 업데이트"

  - severity: high
    description: "테스트 실행 불가"
    action: "컴파일 문제 해결 후 재시도"

recommendations: |
  ## 에스컬레이션
  1. Cargo.toml 의존성 검토 필요
  2. sqlx 버전 호환성 확인
  3. 수동 개입 후 Phase 재실행
---end-worker-result---
```

---

## Orchestrator 처리 가이드

### status별 처리

```
success:
  - 다음 Phase 진행
  - context_notes 참조

partial:
  - issues 분석
  - 완료된 task 유지
  - 미완료 task 재위임 또는 에스컬레이션

blocked:
  - 블로커 해결
  - 해결 후 Phase 재실행

failed:
  - 원인 분석
  - 수동 개입 또는 재시도
  - 심각하면 전체 계획 재검토
```

### context_notes 활용

- 다음 Phase OWI의 context_scope에 포함
- 의사결정 기록으로 활용
- 발견된 이슈는 백로그로 관리

---

## 파싱 가이드

WRB를 파싱할 때:

1. `---worker-result---`와 `---end-worker-result---` 사이 추출
2. YAML로 파싱
3. status 필드로 성공/실패 판단
4. tasks_executed로 개별 task 결과 확인
5. issues 필드로 문제점 파악
6. recommendations로 다음 행동 결정
