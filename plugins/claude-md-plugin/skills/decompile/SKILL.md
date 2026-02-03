---
name: decompile
version: 1.0.0
aliases: [decom]
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", or uses "/decompile" or "/decom".
  Analyzes existing source code (binary) and creates CLAUDE.md (source) documentation for each directory.
allowed-tools: [Bash, Read, Write, Task, Skill, AskUserQuestion]
---

# Decompile Skill

## Core Philosophy

**Source Code(바이너리) → CLAUDE.md + IMPLEMENTS.md(소스) = Decompile**

```
Source Code (구현)  ─── /decompile ──→  CLAUDE.md (WHAT) + IMPLEMENTS.md (HOW)
```

전통적 디컴파일러가 바이너리에서 소스코드를 추출하듯,
`/decompile`은 기존 소스코드에서 CLAUDE.md + IMPLEMENTS.md 명세를 추출합니다.

## 듀얼 문서 시스템

| 출력 | 역할 | 내용 |
|------|------|------|
| CLAUDE.md | WHAT (스펙) | Purpose, Exports, Behavior, Contract, Protocol, Domain Context |
| IMPLEMENTS.md Planning | HOW 계획 | Dependencies Direction, Implementation Approach, Technology Choices |
| IMPLEMENTS.md Implementation | HOW 상세 | Algorithm, Key Constants, Error Handling, State Management, Session Notes |

## 워크플로우

```
/decompile
    │
    ▼
Skill("tree-parse") → 대상 디렉토리 목록
    │
    ▼
depth 내림차순 정렬 (leaf-first)
    │
    ▼
For each directory (순차 실행):
    Task(decompiler) → CLAUDE.md + IMPLEMENTS.md 생성
    │
    └─→ 검증 후 즉시 배치 (다음 Agent가 읽을 수 있도록)
    │
    ▼
최종 보고
```

**핵심:** 자식 디렉토리 CLAUDE.md가 먼저 생성되어야 부모가 자식의 Purpose를 읽을 수 있습니다.

상세 실행 순서는 `references/execution-order.md` 참조.

## 대상 디렉토리 확인 예시

```
=== CLAUDE.md 생성 대상 ===

다음 디렉토리에 CLAUDE.md를 생성합니다 (leaf-first 순서):
  1. [depth=2] src/auth/ (4 source files)
  2. [depth=2] src/api/ (5 source files)
  3. [depth=1] src/ (2 source files, 3 subdirectories)

제외된 디렉토리: node_modules, target, dist

계속하시겠습니까?
```

## 내부 Skill 목록

| Skill | 역할 | 호출 위치 |
|-------|------|----------|
| `tree-parse` | 디렉토리 트리 파싱 | decompile Skill |
| `boundary-resolve` | 바운더리 분석 | decompiler Agent |
| `code-analyze` | 코드 분석 | decompiler Agent |
| `schema-validate` | 스키마 검증 | decompiler Agent |

내부 Skill은 description에 `(internal)` 표시되어 자동완성에서 숨겨집니다.

## 최종 보고 예시

```
=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

생성된 파일:
  ✓ src/CLAUDE.md + IMPLEMENTS.md
  ✓ src/auth/CLAUDE.md + IMPLEMENTS.md
  ✓ src/api/CLAUDE.md + IMPLEMENTS.md

검증 결과:
  - CLAUDE.md 스키마: 3/3 통과
  - IMPLEMENTS.md 스키마: 3/3 통과
  - 참조 규칙: 3/3 통과

사용자 질문: 5개 응답됨
  - Domain Context 관련: 3개
  - Implementation 배경: 2개

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
```

## 파일 기반 결과 전달

Agent는 결과를 scratchpad에 저장하고 경로만 반환합니다.
이로써 Skill context 폭발을 방지합니다.

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | 에러 메시지 출력, 실패 반환 |
| tree-parse 실패 | CLI 에러 메시지 전달 |
| decompiler 실패 | 해당 디렉토리 스킵, 경고 표시 |
| 스키마 검증 실패 | Agent가 최대 5회 재시도 후 경고와 함께 진행 |
