# TDD Implementation Skill

TDD/ATDD 원칙에 따라 테스트 주도 개발을 수행합니다.

---
description: TDD/ATDD 기반으로 코드를 구현합니다. Outside-In TDD로 테스트가 컴포넌트 인터페이스를 정의합니다.
trigger: /tdd-impl
use_when:
  - TDD로 구현해줘
  - 테스트 주도 개발
  - 테스트 먼저 작성
  - Red-Green-Refactor
---

## Overview

이 스킬은 Outside-In TDD (London School) 접근법을 따릅니다.

**핵심 원칙**: 테스트가 인터페이스를 정의한다

구현을 시작하기 전에 테스트를 작성함으로써:
- 컴포넌트가 필요로 하는 의존성 인터페이스가 자연스럽게 드러남
- Mock 객체의 메서드 시그니처가 곧 실제 인터페이스 계약
- 상위 레벨에서 하위 레벨로 인터페이스가 흘러내림

## Workflow

### Phase 1: 요구사항 검증

상세 내용은 [requirement-validation.md](references/requirement-validation.md) 참조

1. 제공된 요구사항 분석
2. 테스트 케이스 도출 가능 여부 검증
3. 인터페이스 발견 가능 여부 검증
4. 검증 기준 명확성 확인

**불충분시:**
- AskUserQuestion으로 누락된 정보 요청
- 구체적으로 어떤 정보가 필요한지 명시

**검증 완료시:**
- `.claude/tdd-spec.md` 파일 생성/업데이트
- 형식은 [spec-format.md](references/spec-format.md) 참조
- 요구사항을 REQ-XXX 형식으로 구조화하여 저장

### Phase 2: 테스트 설계 (Top-Down)

상세 내용은 [test-design.md](references/test-design.md) 참조

1. 요구사항에서 Acceptance Test 케이스 도출
2. Mock으로 의존성 인터페이스 발견
3. Integration Test 케이스 설계
4. Unit Test 케이스 설계
5. 테스트 명세 문서화

**인터페이스 발견 흐름:**
```
Acceptance Test → Integration Test → Unit Test
     ↓                  ↓                ↓
 상위 인터페이스    중간 인터페이스    하위 인터페이스
```

### Phase 3: 코드 구현 (Red-Green-Refactor)

상세 내용은 [code-impl.md](references/code-impl.md) 참조

각 테스트 케이스에 대해:

1. **RED**: 테스트 코드 작성, 실행 → 실패 확인
2. **GREEN**: 테스트 통과하는 최소 코드 작성
3. **REFACTOR**: 테스트 통과 유지하며 코드 개선

### Phase 4: 최종 검증

1. 모든 테스트 실행
2. 전체 통과 확인
3. 커버리지 확인 (가능한 경우)

## Protocol

### Top-Down Interface Design

```
User Story / Requirement
         ↓
┌─────────────────────────────────────────────┐
│  Acceptance Test (상위 레벨)                │
│  - 전체 기능의 동작을 정의                  │
│  - Mock으로 하위 컴포넌트 인터페이스 발견   │
└────────────────────┬────────────────────────┘
                     ↓
┌─────────────────────────────────────────────┐
│  Integration Test (중간 레벨)               │
│  - 컴포넌트 간 통신 인터페이스 정의         │
│  - Mock이 실제 구현으로 교체될 계약 명시    │
└────────────────────┬────────────────────────┘
                     ↓
┌─────────────────────────────────────────────┐
│  Unit Test (하위 레벨)                      │
│  - 개별 객체/함수의 인터페이스 정의         │
│  - 의존성 주입 지점 명확화                  │
└────────────────────┬────────────────────────┘
                     ↓
              Implementation
```

### Red-Green-Refactor Cycle

```
┌─────────┐     ┌─────────┐     ┌──────────┐
│   RED   │ ──→ │  GREEN  │ ──→ │ REFACTOR │
│ (실패)  │     │ (통과)  │     │  (개선)  │
└─────────┘     └─────────┘     └────┬─────┘
     ↑                               │
     └───────────────────────────────┘
            다음 테스트 케이스
```

## Constraints

- **Protocol vs Implementation**: 이 스킬은 TDD 프로토콜(WHAT)을 정의하며, 구체적인 테스트 프레임워크나 패턴(HOW)은 프로젝트 컨텍스트에 따라 모델이 결정
- **테스트 우선**: 구현 코드 작성 전 반드시 테스트 먼저 작성
- **최소 구현**: GREEN 단계에서는 테스트 통과에 필요한 최소한의 코드만 작성
- **지속적 검증**: 각 단계 후 테스트 실행하여 상태 확인

## Output

- 테스트 코드 (각 레벨별)
- 구현 코드 (테스트 통과하는)
- (선택) 인터페이스 정의 문서
