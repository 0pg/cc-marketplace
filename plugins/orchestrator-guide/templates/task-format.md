# task.md 작성 가이드

## task.md 작성 방법 (plan mode)

task 정의는 **plan mode**를 통해 수행합니다:

1. spec.md 확인 (없으면 먼저 spec 정의)
2. **EnterPlanMode**로 plan mode 진입
3. spec 기반 구현 계획 수립
4. TASK-VERIFY 1:1 형식으로 작성
5. **ExitPlanMode**로 승인 요청
6. 승인 후 spec/task.md로 저장

### spec/ 디렉터리 위치

spec/은 모듈/컴포넌트 root에 위치합니다. 자세한 내용은 [spec-format.md](./spec-format.md#spec-디렉터리-위치) 참조.

### plan mode 활용의 장점

- **spec 기반 계획**: spec.md 요구사항을 task로 변환
- **구현 전략 수립**: 접근법, 순서, 의존성 정의
- **승인 워크플로우**: ExitPlanMode로 사용자 확인

---

## 필수 규칙: Task-Verification 1:1

모든 TASK에는 반드시 대응하는 VERIFY가 존재해야 합니다.

### 형식
- TASK-{기능}.{세부}: 구현할 내용
- VERIFY-{기능}.{세부}: 동작 기반 검증 기준

### VERIFY 작성 원칙 (동작 기반)

VERIFY는 **구체적인 동작**을 명시해야 합니다:
- 입력 → 출력 관계
- 정상 케이스 동작
- 에러 케이스 동작
- 상태 변화

### 예시

#### BAD - 추상적
```markdown
- [ ] TASK-001.1: UserService 구현
- [ ] VERIFY-001.1: 테스트 통과
```

#### GOOD - 동작 기반
```markdown
- [ ] TASK-001.1: create_user() 메서드 구현
- [ ] VERIFY-001.1: 유효한 입력 → User 반환, 중복 이메일 → DuplicateError 반환

- [ ] TASK-001.2: get_user_by_id() 메서드 구현
- [ ] VERIFY-001.2: 존재하는 ID → User 반환, 없는 ID → None 반환

- [ ] TASK-002.1: POST /api/users 엔드포인트 구현
- [ ] VERIFY-002.1: 201 Created + Location 헤더 반환, 잘못된 형식 → 400, 중복 → 409
```

---

## task.md 구조 템플릿

```markdown
---
# Tasks - {module_name}

> **범위**: {scope_description}
> **전략**: {implementation_strategy}

---

## {Feature_Name} (TASK-001)

### TASK-001.1: {sub_feature}
- [ ] TASK-001.1.1: {구현 항목}
- [ ] VERIFY-001.1.1: {동작 기반 검증}

- [ ] TASK-001.1.2: {구현 항목}
- [ ] VERIFY-001.1.2: {동작 기반 검증}

---

## 완료 기준

### Phase 1: {phase_name}
- [ ] TASK-001 완료
- [ ] 모든 VERIFY 통과
```
