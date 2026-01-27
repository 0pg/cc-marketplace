---
name: tdd-impl
description: |
  TDD/ATDD 기반으로 코드를 구현합니다. tdd-spec.md를 읽어 Red-Green-Refactor 사이클로 구현합니다.
  "TDD로 구현해줘", "테스트 주도 개발", "테스트 먼저 작성", "Red-Green-Refactor" 요청 시 사용됩니다.
---

# TDD Implementation Skill

TDD/ATDD 원칙에 따라 테스트 주도 개발을 수행합니다.

## Overview

이 스킬은 Outside-In TDD (London School) 접근법을 따릅니다.

**핵심 원칙**: 테스트가 인터페이스를 정의한다

구현을 시작하기 전에 테스트를 작성함으로써:
- 컴포넌트가 필요로 하는 의존성 인터페이스가 자연스럽게 드러남
- Mock 객체의 메서드 시그니처가 곧 실제 인터페이스 계약
- 상위 레벨에서 하위 레벨로 인터페이스가 흘러내림

## Prerequisites

이 스킬을 사용하기 전에:

1. **tdd-spec.md 확인**: `.claude/tdd-spec.md` 파일이 존재하는지 확인
2. **없으면 test-design 먼저 실행**: `/test-design` 스킬로 요구사항 분석 및 spec 생성

## Workflow

```
/test-design (선행)
     |
     v
tdd-spec.md 생성
     |
     v
/tdd-impl (이 스킬)
     |
     v
+--------------------+
| Phase 1            |
| 요구사항 검증      |
| (tdd-spec.md 읽기) |
+---------+----------+
          |
          v
+--------------------+
| Phase 2            |
| 코드 구현          |
| (Red-Green-Refactor)|
+---------+----------+
          |
          v
+--------------------+
| Phase 3            |
| 최종 검증          |
+--------------------+
```

## Phase 1: 요구사항 검증

상세 내용은 [requirement-validation.md](references/requirement-validation.md) 참조

1. `.claude/tdd-spec.md` 파일 읽기
2. 요구사항 구조 확인 (REQ-XXX 형식)
3. Verification Criteria 체크리스트 확인

**tdd-spec.md가 없는 경우:**
- 사용자에게 `/test-design` 스킬 실행을 권장
- 또는 AskUserQuestion으로 요구사항 직접 수집

**tdd-spec.md가 있는 경우:**
- 요구사항 목록 확인
- 구현 범위 파악
- Phase 2로 진행

## Phase 2: 코드 구현 (Red-Green-Refactor)

상세 내용은 [code-impl.md](references/code-impl.md) 참조

**언어별 가이드**: Rust 프로젝트는 [rust.md](references/rust.md) 참조

각 요구사항(REQ-XXX)에 대해:

### Red-Green-Refactor Cycle

```
+----------+     +----------+     +------------+
|   RED    | --> |  GREEN   | --> |  REFACTOR  |
|  (실패)  |     |  (통과)  |     |   (개선)   |
+----------+     +----------+     +-----+------+
     ^                                  |
     +----------------------------------+
             다음 테스트 케이스
```

1. **RED**: 테스트 코드 작성, 실행 -> 실패 확인
2. **GREEN**: 테스트 통과하는 최소 코드 작성
3. **REFACTOR**: 테스트 통과 유지하며 코드 개선

### 구현 순서 (Bottom-Up)

테스트 설계는 Top-Down이지만, 구현은 Bottom-Up으로:

```
Acceptance Test (설계 먼저)
       ^
       |
Integration Test
       ^
       |
Unit Test (구현 먼저)
```

1. Unit Test -> Unit 구현
2. Integration Test -> Integration 구현
3. Acceptance Test -> 전체 통합

## Phase 3: 최종 검증

1. 모든 테스트 실행
2. 전체 통과 확인
3. 커버리지 확인 (가능한 경우)
4. tdd-spec.md의 Verification Criteria 업데이트

## Protocol

### Top-Down Interface Design

```
User Story / Requirement
         |
         v
+---------------------------------------------+
|  Acceptance Test (상위 레벨)                |
|  - 전체 기능의 동작을 정의                  |
|  - Mock으로 하위 컴포넌트 인터페이스 발견   |
+-----------------------+---------------------+
                        |
                        v
+---------------------------------------------+
|  Integration Test (중간 레벨)               |
|  - 컴포넌트 간 통신 인터페이스 정의         |
|  - Mock이 실제 구현으로 교체될 계약 명시    |
+-----------------------+---------------------+
                        |
                        v
+---------------------------------------------+
|  Unit Test (하위 레벨)                      |
|  - 개별 객체/함수의 인터페이스 정의         |
|  - 의존성 주입 지점 명확화                  |
+-----------------------+---------------------+
                        |
                        v
                 Implementation
```

## Constraints

- **Protocol vs Implementation**: 이 스킬은 TDD 프로토콜(WHAT)을 정의하며, 구체적인 테스트 프레임워크나 패턴(HOW)은 프로젝트 컨텍스트에 따라 모델이 결정
- **테스트 우선**: 구현 코드 작성 전 반드시 테스트 먼저 작성
- **최소 구현**: GREEN 단계에서는 테스트 통과에 필요한 최소한의 코드만 작성
- **지속적 검증**: 각 단계 후 테스트 실행하여 상태 확인

## Output

- 테스트 코드 (각 레벨별)
- 구현 코드 (테스트 통과하는)
- tdd-spec.md Verification Criteria 업데이트
- (선택) 인터페이스 정의 문서
