---
name: tdd-orchestration
description: |
  TDD/ATDD 기반 개발 워크플로우.
  "TDD로", "테스트 주도", "Red-Green-Refactor" 요청 시 활성화.
---

# TDD Orchestration

> Red-Green-Refactor 기반 테스트 주도 개발 워크플로우

## Phases

### Phase 1: 테스트 설계

- **호출**: `Skill("tdd-dev:test-design")`
- **산출물**: `.claude/tdd-spec.md`
- **완료 조건**: tdd-spec.md 생성

#### 수행 내용
1. 요구사항 분석
2. 테스트 레벨별 케이스 설계 (Acceptance → Integration → Unit)
3. 인터페이스 발견 (Mock을 통한 의존성 정의)
4. tdd-spec.md 생성

### Phase 2: TDD 구현

- **호출**: `Skill("tdd-dev:tdd-impl")`
- **산출물**: 테스트 코드 + 구현 코드
- **완료 조건**: 모든 테스트 통과

#### 수행 내용
1. tdd-spec.md 읽기
2. 각 요구사항(REQ-XXX)에 대해:
   - RED: 테스트 작성 → 실패 확인
   - GREEN: 최소 구현 → 통과 확인
   - REFACTOR: 코드 개선 → 통과 유지
3. 전체 테스트 실행 → 통과 확인

## Verification

검증 체인: `references/verification-chain.md` 참조

### 검증 단계

1. **Lint**: project-config.verification.lint 실행
2. **Tests**: project-config.verification.test 실행
3. **REQ-Test 매핑**: tdd-dev:test-reviewer 호출
4. **Code Review**: review_role 에이전트 호출

## 트리거

다음 키워드로 이 워크플로우가 선택됩니다:
- "TDD로 구현해줘"
- "테스트 주도 개발"
- "Red-Green-Refactor로"
- "테스트 먼저 작성"

## Phase 흐름

```
사용자 요청 ("TDD로 구현해줘")
        |
        v
+---------------------------+
| Phase 1: 테스트 설계       |
| Skill("tdd-dev:test-design")|
+-------------+-------------+
              |
              v
     .claude/tdd-spec.md
              |
              v
+---------------------------+
| Phase 2: TDD 구현          |
| Skill("tdd-dev:tdd-impl")  |
+-------------+-------------+
              |
              v
     테스트 코드 + 구현 코드
              |
              v
+---------------------------+
| Verification               |
| references/verification-   |
| chain.md 참조              |
+---------------------------+
```

## Convention 플러그인 연동

코드 작성 시 프로젝트에 설치된 Convention 플러그인을 참조합니다:

| 언어 | Convention 플러그인 |
|------|---------------------|
| Rust | `rust-convention` |
| (기타) | 해당 언어의 convention 플러그인 |

Convention 플러그인이 없으면 언어의 표준 스타일 가이드를 따릅니다.
