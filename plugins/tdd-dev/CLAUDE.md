# TDD-Dev Plugin Development Guide

## Overview

TDD/ATDD 원칙을 가이드하는 플러그인입니다. Outside-In TDD (London School) 접근법을 따릅니다.

## 핵심 원칙

### 1. Protocol vs Implementation

- **Plugin (WHAT)**: TDD/ATDD 프로토콜 정의
- **Model (HOW)**: 구체적 테스트 프레임워크, 패턴 선택

플러그인은 "무엇을 해야 하는지"를 정의하고, "어떻게 할지"는 프로젝트 컨텍스트에 따라 모델이 결정합니다.

### 2. 테스트가 인터페이스를 정의한다

Outside-In TDD의 핵심:
- 상위 테스트 작성 시 → 필요한 협력 객체 발견
- Mock으로 협력 객체 대체 → 인터페이스 계약 정의
- Mock의 메서드 시그니처 = 실제 인터페이스

### 3. Red-Green-Refactor

모든 구현은 이 사이클을 따릅니다:
1. RED: 실패하는 테스트 작성
2. GREEN: 최소한의 코드로 통과
3. REFACTOR: 동작 유지하며 개선

## Directory Structure

```
plugins/tdd-dev/
├── .claude-plugin/
│   └── plugin.json              # 플러그인 메타데이터
├── CLAUDE.md                    # 이 파일 (개발 가이드)
├── README.md                    # 사용자 문서
├── skills/
│   └── tdd-impl/
│       ├── SKILL.md             # TDD 워크플로우 스킬 (메인 진입점)
│       └── references/
│           ├── requirement-validation.md  # 요구사항 검증 가이드
│           ├── test-design.md   # 테스트 설계/인터페이스 발견 가이드
│           ├── code-impl.md     # Red-Green-Refactor 구현 가이드
│           └── (언어별 가이드: rust.md, typescript.md 등 추후 추가)
```

## Workflow

```
요구사항 입력
     ↓
┌─────────────────┐
│ 요구사항 검증   │ ← requirement-validation.md
│ (충분성 확인)   │
└────────┬────────┘
         ↓
┌─────────────────┐
│ 테스트 설계     │ ← test-design.md
│ (Top-Down)      │
│ - 인터페이스 발견│
│ - 케이스 도출   │
└────────┬────────┘
         ↓
┌─────────────────┐
│ 코드 구현       │ ← code-impl.md
│ (R-G-R Cycle)   │
│ - Bottom-Up 구현│
└────────┬────────┘
         ↓
    최종 검증
```

## 확장 가이드

### 언어별 가이드 추가

`skills/tdd-impl/references/` 디렉터리에 언어별 가이드를 추가할 수 있습니다:

```
skills/tdd-impl/references/
├── rust.md        # Rust 테스트 패턴
├── typescript.md  # TypeScript/Jest 패턴
├── python.md      # Python/pytest 패턴
└── go.md          # Go testing 패턴
```

가이드 작성 시 포함할 내용:
- 테스트 프레임워크 사용법
- Mocking 라이브러리
- 언어별 테스트 패턴
- 디렉터리 구조 권장사항
