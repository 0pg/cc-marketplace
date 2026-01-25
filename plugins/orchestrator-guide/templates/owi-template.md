# OWI (Orchestrator-Worker Instruction) Template

> Orchestrator가 Worker에게 Phase를 위임할 때 사용하는 표준 형식

---

## OWI 기본 형식

```yaml
---orchestrator-instruction---
phase: {phase_name}
phase_ref: {phase_id}

context_scope: |
  # 맥락 범위
  - spec/task.md 참조 항목
  - 관련 모듈/파일 범위
  - notes.md 관련 내용

objective: |
  # 이 Phase에서 달성할 목표
  - 고수준 설명
  - 완료 조건

constraints: |
  # 제약 사항
  - 범위 제한
  - 금지 사항

expected_agents: |
  # 예상 에이전트 역할 (선택)
  - Implementation: {target_modules}
  - Review: {review_scope}
---end-orchestrator-instruction---
```

---

## 필드 상세 설명

### phase

Phase 이름 (사람이 읽기 쉬운 형태):

```yaml
phase: "Phase 1: 구현"
phase: "Phase 2: 검증"
phase: "Phase 3: 통합"
```

### phase_ref

Phase 고유 식별자:

```yaml
phase_ref: PHASE-001
phase_ref: PHASE-002
```

### context_scope

Worker가 참조해야 할 맥락 범위:

```yaml
context_scope: |
  ## 참조 문서
  - spec/spec.md: 전체 요구사항
  - spec/task.md: TASK-001 ~ TASK-005

  ## 관련 모듈
  - module_a/src/: 데이터 모델 구현 대상
  - module_b/src/: API 구현 대상

  ## notes.md 관련 내용
  - 데이터베이스 연결 패턴
  - 에러 처리 컨벤션
```

### objective

Phase에서 달성해야 할 목표:

```yaml
objective: |
  ## 목표
  module_a와 module_b의 기본 구조 구현

  ## 완료 조건
  - 데이터 모델 정의 완료
  - CRUD API 엔드포인트 구현
  - 기본 테스트 통과
  - lint 경고 없음
```

### constraints

제약 사항:

```yaml
constraints: |
  ## 범위 제한
  - module_a, module_b 범위 내에서만 작업
  - 기존 공통 모듈 수정 금지

  ## 금지 사항
  - 기존 API 시그니처 변경 금지
  - 새로운 외부 의존성 추가 금지

  ## 주의 사항
  - 기존 패턴 준수
  - 에러 처리 표준 준수
```

### expected_agents

예상되는 에이전트 역할 (선택적 가이드):

```yaml
expected_agents: |
  ## Implementation
  - module_a 구현 (데이터 모델 + API)
  - module_b 구현 (데이터 모델 + API)

  ## Review
  - 통합 코드 리뷰

  ## 참고
  - 병렬 실행 권장: module_a, module_b 독립적
```

**참고:** expected_agents는 권장 사항이며, Worker가 상황에 맞게 조정 가능

---

## 사용 예시

### 구현 Phase OWI

```yaml
---orchestrator-instruction---
phase: "Phase 1: 핵심 모듈 구현"
phase_ref: PHASE-001

context_scope: |
  ## 참조 문서
  - spec/spec.md: REQ-001 ~ REQ-005
  - spec/task.md: TASK-001 ~ TASK-010

  ## 관련 모듈
  - backend/src/models/: 데이터 모델
  - backend/src/routes/: API 엔드포인트
  - backend/src/services/: 비즈니스 로직

  ## 패턴 참조
  - backend/src/routes/existing_entity.rs: 기존 CRUD 패턴

objective: |
  ## 목표
  User 엔티티의 CRUD 기능 구현

  ## 완료 조건
  - User 모델 정의 (id, name, email, created_at)
  - CRUD API 엔드포인트 (/api/users)
  - 기본 유효성 검증
  - 단위 테스트 작성

constraints: |
  ## 범위 제한
  - backend/src/ 범위 내 작업
  - 데이터베이스 스키마 변경 포함

  ## 금지 사항
  - 인증/인가 로직 포함 금지 (다음 Phase)
  - 기존 엔티티 수정 금지

expected_agents: |
  ## Implementation
  - User 모델 구현
  - User API 구현
  - User 서비스 로직

  ## Review
  - 구현 완료 후 코드 리뷰
---end-orchestrator-instruction---
```

### 검증 Phase OWI

```yaml
---orchestrator-instruction---
phase: "Phase 2: 검증 및 통합"
phase_ref: PHASE-002

context_scope: |
  ## 이전 Phase 결과
  - PHASE-001 완료: User CRUD 구현
  - 수정된 파일:
    - backend/src/models/user.rs
    - backend/src/routes/user.rs
    - backend/src/services/user.rs

  ## 검증 대상
  - 위 파일들의 통합 테스트
  - 성능 기준 확인

objective: |
  ## 목표
  User 모듈의 통합 검증 및 품질 확인

  ## 완료 조건
  - 통합 테스트 작성 및 통과
  - 성능 벤치마크 통과
  - 코드 품질 리뷰 완료

constraints: |
  ## 범위 제한
  - 검증 및 테스트만 수행
  - 기능 추가 금지

  ## 주의 사항
  - 테스트 격리 확인
  - 기존 테스트 영향 없음 확인

expected_agents: |
  ## Exploration
  - 기존 테스트 패턴 분석

  ## Implementation
  - 통합 테스트 작성

  ## Review
  - 테스트 품질 리뷰
---end-orchestrator-instruction---
```

### 리팩토링 Phase OWI

```yaml
---orchestrator-instruction---
phase: "Phase 3: 리팩토링"
phase_ref: PHASE-003

context_scope: |
  ## 리팩토링 대상
  - backend/src/services/legacy_service.rs
  - 현재 500줄, 분할 필요

  ## 참조
  - notes.md: 리팩토링 방향 결정사항
  - 기존 패턴: backend/src/services/modern_service.rs

objective: |
  ## 목표
  legacy_service.rs 모듈 분할

  ## 완료 조건
  - 3개 이하 모듈로 분할
  - 각 모듈 200줄 이하
  - 기존 API 호환성 유지
  - 모든 기존 테스트 통과

constraints: |
  ## 범위 제한
  - legacy_service.rs만 리팩토링
  - 호출하는 코드 수정 최소화

  ## 금지 사항
  - 공개 API 시그니처 변경 금지
  - 새로운 기능 추가 금지

expected_agents: |
  ## Exploration
  - 현재 구조 분석
  - 분할 지점 식별

  ## Implementation
  - 모듈 분할 실행

  ## Review
  - 분할 결과 리뷰
  - 호환성 확인
---end-orchestrator-instruction---
```

---

## Orchestrator 사용 방법

Orchestrator가 Worker에게 Phase를 위임할 때:

```typescript
Task({
  subagent_type: "worker",
  prompt: `
---orchestrator-instruction---
phase: "Phase 1: 구현"
phase_ref: PHASE-001

context_scope: |
  ...

objective: |
  ...

constraints: |
  ...

expected_agents: |
  ...
---end-orchestrator-instruction---
`
})
```

---

## 검증

OWI 작성 시 체크리스트:

- [ ] phase와 phase_ref가 명확하게 정의됨
- [ ] context_scope에 필요한 모든 참조 포함
- [ ] objective에 측정 가능한 완료 조건 포함
- [ ] constraints에 범위와 금지 사항 명시
- [ ] expected_agents는 선택적이지만 권장됨
