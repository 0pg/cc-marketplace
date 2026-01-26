# TDD-Dev Plugin

TDD/ATDD 원칙을 가이드하는 Claude Code 플러그인입니다.

## Features

- **Outside-In TDD**: 테스트가 컴포넌트 인터페이스를 정의하는 Top-Down 접근법
- **Red-Green-Refactor**: 검증된 TDD 사이클 프로토콜
- **요구사항 검증**: TDD 시작 전 요구사항 충분성 확인

## Installation

Claude Code 설정에서 이 플러그인을 추가하세요.

## Usage

### /tdd-impl

TDD 방식으로 코드를 구현합니다.

```
/tdd-impl
```

**자동 트리거:**
- "TDD로 구현해줘"
- "테스트 주도 개발"
- "테스트 먼저 작성"

**워크플로우:**
1. 요구사항 검증 - 테스트 케이스 도출 가능 여부 확인
2. 테스트 설계 - Top-Down으로 인터페이스 발견
3. 코드 구현 - Red-Green-Refactor 사이클 실행
4. 최종 검증 - 모든 테스트 통과 확인

## Core Principles

### 테스트가 인터페이스를 정의한다

```
User Story / Requirement
         ↓
    Acceptance Test  ──→ 상위 인터페이스 발견
         ↓
    Integration Test ──→ 중간 인터페이스 발견
         ↓
    Unit Test        ──→ 하위 인터페이스 발견
         ↓
    Implementation
```

### Red-Green-Refactor

```
┌─────────┐     ┌─────────┐     ┌──────────┐
│   RED   │ ──→ │  GREEN  │ ──→ │ REFACTOR │
│ (실패)  │     │ (통과)  │     │  (개선)  │
└─────────┘     └─────────┘     └────┬─────┘
     ↑                               │
     └───────────────────────────────┘
```

## Example

```
User: 사용자 로그인 기능을 TDD로 구현해줘