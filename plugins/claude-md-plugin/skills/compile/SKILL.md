---
name: compile
version: 1.0.0
aliases: [gen, generate, build]
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/compile". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Performs TDD workflow (RED→GREEN→REFACTOR) to ensure compiled code passes tests.
  Trigger keywords: 코드 생성, 컴파일, CLAUDE.md에서 코드
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# Compile Skill

## Core Philosophy

**CLAUDE.md + IMPLEMENTS.md → Source Code = Compile**

```
CLAUDE.md (WHAT)  +  IMPLEMENTS.md (HOW)  ─── /compile ──→  Source Code (구현)
```

전통적 컴파일러가 소스코드를 바이너리로 변환하듯,
`/compile`은 CLAUDE.md + IMPLEMENTS.md 명세를 실행 가능한 소스코드로 변환.

## 듀얼 문서 시스템

| 입력 | 역할 | 업데이트 |
|------|------|----------|
| CLAUDE.md | 스펙 (WHAT) | 읽기 전용 |
| IMPLEMENTS.md Planning Section | 구현 방향 (HOW 계획) | 읽기 전용 |
| IMPLEMENTS.md Implementation Section | 구현 상세 | **업데이트** |

## 사용법

```bash
# 기본 사용 (변경된 CLAUDE.md만 처리 — incremental)
/compile

# 전체 CLAUDE.md 처리 (full rebuild)
/compile --all

# 특정 경로만 처리
/compile --path src/auth

# 기존 파일 덮어쓰기
/compile --conflict overwrite
```

## 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리 대상 경로 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 (`skip` \| `overwrite`) |
| `--all` | `false` | 전체 CLAUDE.md compile (incremental 비활성화) |

## 워크플로우

```
/compile
    │
    ├─ --all? ──YES──→ 모든 CLAUDE.md 검색 (기존 full rebuild)
    │
    └─ NO → Bash(diff-compile-targets) → 변경 감지
              │
              ├─ targets = 0 → "All up-to-date" 출력, 종료
              │
              └─ targets > 0
                    │
                    ▼
              IMPLEMENTS.md 존재 확인 (없으면 자동 생성)
                    │
                    ▼
              언어 자동 감지
                    │
                    ▼
              의존성 그래프 기반 실행 순서 결정 (leaf-first)
                    │
                    ▼
              같은 depth의 독립 모듈은 병렬, 의존 관계는 순차 처리
                    │
                    ▼
              결과 수집 및 보고
```

상세 구현은 `references/workflow.md` 참조.

## 언어 및 테스트 프레임워크

프로젝트에서 사용 중인 언어와 테스트 프레임워크를 자동 감지.

- **언어**: 파일 확장자 기반
- **테스트 프레임워크**: 프로젝트 설정 파일 분석 (package.json, pyproject.toml, Cargo.toml 등)

## 파일 충돌 처리

| 모드 | 동작 |
|------|------|
| `skip` (기본) | 기존 파일 유지, 새 파일만 생성 |
| `overwrite` | 기존 파일 덮어쓰기 |

## 출력 예시

### Incremental 모드 (기본)

```
변경된 CLAUDE.md를 감지합니다...

감지된 compile 대상 (3/6):
  ✓ src/auth — staged
  ✓ src/core — modified
  ✓ src/new  — no-source-code

건너뛴 모듈 (3/6): up-to-date

⚠ Dependency warnings:
  - src/auth changed; src/api may need recompilation
  - Use --all for full compilation

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개, 클래스 1개
✓ IMPLEMENTS.md Planning Section 로드
✓ 테스트 생성 (5 test cases)
✓ 구현 생성
✓ 테스트 실행: 5 passed
✓ IMPLEMENTS.md Implementation Section 업데이트

[2/2] src/new/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 1개
✓ IMPLEMENTS.md Planning Section 로드
✓ 테스트 생성 (2 test cases)
✓ 구현 생성
✓ 테스트 실행: 2 passed
✓ IMPLEMENTS.md Implementation Section 업데이트

=== 생성 완료 ===
총 CLAUDE.md: 2개 (변경분)
생성된 파일: 5개
건너뛴 파일: 0개
테스트: 7 passed, 0 failed
업데이트된 IMPLEMENTS.md: 2개
```

### All up-to-date

```
변경된 CLAUDE.md를 감지합니다...

✓ All up-to-date. 변경된 CLAUDE.md가 없습니다.
  Use --all for full compilation.
```

### Full rebuild (--all)

```
프로젝트에서 CLAUDE.md 파일을 검색합니다...

발견된 CLAUDE.md 파일:
1. src/auth/CLAUDE.md + IMPLEMENTS.md
2. src/utils/CLAUDE.md + IMPLEMENTS.md

코드 생성을 시작합니다...
...
```

## 참조 자료

- `examples/generate-result.json`: compiler agent 결과 JSON 예시

## 내부 Skill 목록

| Skill | 역할 | 호출 위치 |
|-------|------|----------|
| `claude-md-parse` | CLAUDE.md JSON 파싱 | compiler Agent |
| `schema-validate` | 스키마 검증 | compiler Agent (REFACTOR 단계) |

내부 Skill은 description에 `(internal)` 표시되어 자동완성에서 숨겨짐.

## DO / DON'T

**DO:**
- Follow TDD workflow (RED→GREEN→REFACTOR)
- Respect file conflict mode (skip/overwrite)
- Generate test files alongside implementation

**DON'T:**
- Delete existing test files
- Overwrite files when conflict mode is "skip"
- Modify CLAUDE.md (read-only during compile)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md 파일을 찾을 수 없음" 메시지 출력 |
| IMPLEMENTS.md 없음 | 기본 템플릿으로 자동 생성 후 진행 |
| 파싱 오류 | 해당 파일 건너뛰고 계속 진행, 오류 로그 |
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| 테스트 실패 | 경고 표시, 수동 수정 필요 안내 |
| 파일 쓰기 실패 | 에러 로그, 해당 파일 건너뛰기 |

## Examples

<example>
<context>
사용자가 프로젝트의 CLAUDE.md를 처리하려고 합니다.
</context>
<user_request>/compile</user_request>
<assistant_response>
프로젝트에서 CLAUDE.md 파일을 검색합니다...

발견된 CLAUDE.md 파일:
1. src/auth/CLAUDE.md
2. src/utils/CLAUDE.md

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개
✓ 테스트 생성
✓ 구현 생성
✓ 테스트 실행: 5 passed

[2/2] src/utils/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 3개
✓ 테스트 생성
✓ 구현 생성
✓ 테스트 실행: 3 passed

=== 생성 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 7개
테스트: 8 passed, 0 failed
</assistant_response>
</example>
