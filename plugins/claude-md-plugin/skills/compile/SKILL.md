---
name: compile
version: 1.1.0
aliases: [gen, generate, build]
trigger:
  - /compile
  - CLAUDE.md에서 코드 생성
  - compile code from spec
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement spec",
  "create source files", or uses "/compile". Processes all CLAUDE.md files in the target path.
  Performs TDD workflow (RED→GREEN→REFACTOR) to ensure compiled code passes tests.
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# Compile Skill

<example>
<context>
사용자가 프로젝트의 CLAUDE.md를 처리하려고 합니다.
</context>
<user>/compile</user>
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

<example>
<context>
사용자가 변경된 스펙만 compile하려고 합니다.
</context>
<user>/compile --incremental</user>
<assistant_response>
증분 compile 모드로 실행합니다...

[변경 분석]
✓ Uncommitted 변경 감지: 1개
  - src/auth/CLAUDE.md (modified)
✓ Outdated 스펙 감지: 1개
  - src/utils (스펙이 소스보다 최신)

Compile 대상: 2개
1. src/auth (uncommitted)
2. src/utils (outdated)

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개
✓ 테스트 생성
✓ 구현 생성
✓ 테스트 실행: 5 passed
✓ Interface 변경 감지: validateToken 시그니처 변경 (breaking)

[2/2] src/utils/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 3개
✓ 테스트 생성
✓ 구현 생성
✓ 테스트 실행: 3 passed

=== 생성 완료 ===
Compile된 모듈: 2개
건너뛴 모듈: 3개 (변경 없음)

[영향 분석]
⚠ Breaking change 감지: src/auth
영향받는 모듈:
  - src/api (validateToken 사용)
  - src/middleware (validateToken 사용)

권장 조치:
  /compile --path src/api
  /compile --path src/middleware
</assistant_response>
</example>

## Core Philosophy

**CLAUDE.md + IMPLEMENTS.md → Source Code = Compile**

```
CLAUDE.md (WHAT)  +  IMPLEMENTS.md (HOW)  ─── /compile ──→  Source Code (구현)
```

전통적 컴파일러가 소스코드를 바이너리로 변환하듯,
`/compile`은 CLAUDE.md + IMPLEMENTS.md 명세를 실행 가능한 소스코드로 변환합니다.

## 듀얼 문서 시스템

| 입력 | 역할 | 업데이트 |
|------|------|----------|
| CLAUDE.md | 스펙 (WHAT) | 읽기 전용 |
| IMPLEMENTS.md Planning Section | 구현 방향 (HOW 계획) | 읽기 전용 |
| IMPLEMENTS.md Implementation Section | 구현 상세 | **업데이트** |

## 사용법

```bash
# 기본 사용 (전체 CLAUDE.md 처리)
/compile

# 특정 경로만 처리
/compile --path src/auth

# 기존 파일 덮어쓰기
/compile --conflict overwrite

# 증분 compile (변경된 스펙만 처리)
/compile --incremental
```

## 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리 대상 경로 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 (`skip` \| `overwrite`) |
| `--incremental` | `false` | 변경된 스펙만 compile (증분 모드) |

## 워크플로우

### 기본 모드 (Full Compile)

```
/compile
    │
    ▼
모든 CLAUDE.md 검색 (root CLAUDE.md 제외)
    │
    ▼
IMPLEMENTS.md 존재 확인 (없으면 자동 생성)
    │
    ▼
언어 자동 감지
    │
    ▼
병렬 처리: compiler Agent 호출 (run_in_background=True)
    │
    ▼
결과 수집 및 보고
```

### 증분 모드 (Incremental Compile)

```
/compile --incremental
    │
    ▼
┌─────────────────────────────────────────────┐
│ Step 1: 변경 분석                            │
│                                             │
│ [1] Skill("git-status-analyzer")            │
│     → uncommitted_dirs 추출                 │
│                                             │
│ [2] Skill("commit-comparator")              │
│     ← uncommitted_dirs 입력                 │
│     → outdated_dirs, no_source_dirs 추출    │
│                                             │
│ 데이터 흐름:                                  │
│   git-status-analyzer                       │
│         │                                   │
│         └─── uncommitted_dirs ──→           │
│                                   │         │
│                    commit-comparator        │
│                           │                 │
│                           └─→ outdated_dirs │
│                           └─→ no_source_dirs│
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ Step 2: 대상 결정                            │
│                                             │
│ compile_targets = A ∪ B ∪ C                  │
│   A: Uncommitted 변경이 있는 디렉토리         │
│   B: 스펙이 소스보다 최신인 디렉토리 (outdated)│
│   C: 소스 파일이 없는 신규 모듈 (no_source)   │
│                                             │
│ if compile_targets.empty():                  │
│   → "모든 스펙이 최신 상태입니다" 출력        │
│   → 종료                                    │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ Step 3: Compile 실행                         │
│                                             │
│ 각 compile_target에 대해:                    │
│   Task(compiler) 호출                        │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ Step 4: 사후 분석                            │
│                                             │
│ 각 compiled 모듈에 대해:                     │
│   Skill("interface-diff")                    │
│   → Exports 시그니처 변경 감지               │
│   → Breaking change 판정                    │
│                                             │
│ Breaking change 있으면:                      │
│   Skill("dependency-tracker")                │
│   → 영향받는 모듈 분석                       │
│   → 재컴파일 권장 명령 출력                   │
└─────────────────────────────────────────────┘
    │
    ▼
결과 보고
```

상세 구현은 `references/workflow.md` 참조.

## 증분 Compile 대상 결정

```
compile_targets = A ∪ B ∪ C

A: Uncommitted 변경
   - CLAUDE.md 또는 IMPLEMENTS.md에 uncommitted 변경이 있음
   - git status --porcelain으로 감지

B: 커밋 기준 outdated
   - max(CLAUDE.md 커밋, IMPLEMENTS.md 커밋) > max(소스파일들 커밋)
   - 스펙이 소스보다 최신 = compile 필요

C: no_source (신규 모듈)
   - CLAUDE.md는 있으나 소스 파일이 없음
   - 새로 작성된 스펙 = compile 필요
```

### no_source 케이스 처리

소스 파일이 없는 신규 모듈(no_source)은 특별 처리됩니다:

```
if module in no_source:
    1. IMPLEMENTS.md 없으면 자동 생성
    2. compiler Agent 호출 (신규 생성 모드)
    3. interface-diff 건너뜀 (before 상태 없음)
    4. 생성 결과 보고
```

## 언어 및 테스트 프레임워크

프로젝트에서 사용 중인 언어와 테스트 프레임워크를 자동 감지합니다.

- **언어**: 파일 확장자 기반
- **테스트 프레임워크**: 프로젝트 설정 파일 분석 (package.json, pyproject.toml, Cargo.toml 등)

## 파일 충돌 처리

| 모드 | 동작 |
|------|------|
| `skip` (기본) | 기존 파일 유지, 새 파일만 생성 |
| `overwrite` | 기존 파일 덮어쓰기 |

## 출력 예시

### 기본 모드

```
프로젝트에서 CLAUDE.md 파일을 검색합니다...

발견된 CLAUDE.md 파일:
1. src/auth/CLAUDE.md + IMPLEMENTS.md
2. src/utils/CLAUDE.md + IMPLEMENTS.md

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개, 클래스 1개
✓ IMPLEMENTS.md Planning Section 로드
✓ 테스트 생성 (5 test cases)
✓ 구현 생성
✓ 테스트 실행: 5 passed
✓ IMPLEMENTS.md Implementation Section 업데이트

[2/2] src/utils/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 3개
✓ IMPLEMENTS.md Planning Section 로드
✓ 테스트 생성 (3 test cases)
✓ 구현 생성
✓ 테스트 실행: 3 passed
✓ IMPLEMENTS.md Implementation Section 업데이트

=== 생성 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 7개
건너뛴 파일: 0개
테스트: 8 passed, 0 failed
업데이트된 IMPLEMENTS.md: 2개
```

### 증분 모드

```
증분 compile 모드로 실행합니다...

[변경 분석]
✓ Uncommitted 변경 감지: 1개
  - src/auth/CLAUDE.md (modified)
  - src/auth/IMPLEMENTS.md (modified)
✓ Outdated 스펙 감지: 1개
  - src/utils (스펙: 2024-01-20, 소스: 2024-01-15)

Compile 대상: 2개 (전체 5개 중)
1. src/auth (uncommitted)
2. src/utils (outdated)

건너뛰는 모듈: 3개
- src/api (up-to-date)
- src/config (up-to-date)
- src/middleware (up-to-date)

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개
✓ 테스트 생성
✓ 구현 생성
✓ 테스트 실행: 5 passed

[2/2] src/utils/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 3개
✓ 테스트 생성
✓ 구현 생성
✓ 테스트 실행: 3 passed

=== 생성 완료 ===
Compile된 모듈: 2개
건너뛴 모듈: 3개

[Interface 변경 분석]
src/auth:
  - 추가: refreshToken(token: string): Claims
  - 변경: validateToken
    Before: validateToken(token: string): boolean
    After:  validateToken(token: string, options?: Options): Claims
  ⚠ Breaking change 감지

src/utils:
  - 변경 없음

[영향 분석]
⚠ Breaking change가 있는 모듈: src/auth

영향받는 모듈:
  - src/api (validateToken, Claims 사용)
  - src/middleware (validateToken 사용)

권장 조치:
  /compile --path src/api
  /compile --path src/middleware
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md 파일을 찾을 수 없습니다" 메시지 출력 |
| IMPLEMENTS.md 없음 | 기본 템플릿으로 자동 생성 후 진행 |
| 파싱 오류 | 해당 파일 건너뛰고 계속 진행, 오류 로그 |
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| 테스트 실패 | 경고 표시, 수동 수정 필요 안내 |
| 파일 쓰기 실패 | 에러 로그, 해당 파일 건너뛰기 |
| Git 저장소 아님 (증분 모드) | 경고 출력, 전체 compile로 fallback |

## Post-Compile 검증 + Self-Healing

compile 완료 후 자동으로 검증 및 self-healing을 수행합니다.

### 검증 실행

각 compiled node에 대해 병렬로 drift-validator, export-validator를 호출합니다.

### 상태 판정

| 상태 | 조건 | Healing |
|------|------|---------|
| **양호** | Drift 0개 AND Export = 100% | 불필요 (SUCCESS) |
| **개선 권장** | Drift 1-2개 OR Export 90-99% | 필요 |
| **개선 필요** | Drift ≥ 3개 OR Export < 90% | 필요 |

**"양호"가 아니면 모두 healing 대상입니다.**

### Self-Healing

이슈 유형에 따라 자동 또는 수동으로 처리합니다.

| 이슈 유형 | 처리 |
|----------|------|
| **compile_related** (이번 compile에서 발생) | 자동 healing → CLAUDE.md 맥락 추가 → 재컴파일 |
| **unrelated** (기존 이슈) | AskUserQuestion으로 사용자 확인 |

**AskUserQuestion 옵션:**
- CLAUDE.md 수정 (코드에 맞춤) - Recommended
- 코드 수정 (CLAUDE.md에 맞춤) → **compile skill 재실행**
- 무시하고 진행

최대 3회 재시도합니다. 상세 흐름은 `references/workflow.md` 참조.

### 출력 예시

```
=== 생성 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 7개
테스트: 8 passed, 0 failed

=== Post-Compile 검증 ===
검증 대상: 2개

[검증 결과 - 1차]
| 디렉토리 | Drift | Export | 상태 |
|----------|-------|--------|------|
| src/auth | 0 | 100% | ✓ 양호 |
| src/utils | 2 | 72% | ✗ 개선 필요 |

⚠ src/utils에서 이슈 발견:
  - MISSING: parseNumber export (이번 compile에서 생성됨)

[자동 Healing - compile 관련 이슈]
✓ src/utils/CLAUDE.md 맥락 추가
✓ 재컴파일 실행

[검증 결과 - 2차]
| 디렉토리 | Drift | Export | 상태 |
|----------|-------|--------|------|
| src/utils | 0 | 100% | ✓ 양호 |

=== 최종 결과 ===
모든 모듈 검증 통과 (2/2)
자동 healing: 1건 처리됨
```

## 관련 Internal Skills

| Skill | 용도 |
|-------|------|
| `git-status-analyzer` | Uncommitted 스펙 파일 찾기 |
| `commit-comparator` | 스펙 vs 소스 커밋 시점 비교 |
| `interface-diff` | Exports 시그니처 변경 감지 |
| `dependency-tracker` | 의존 모듈 영향 분석 |
