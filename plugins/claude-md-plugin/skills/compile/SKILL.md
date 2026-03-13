---
name: compile
version: 1.0.0
aliases: [gen, generate, build]
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/compile". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Performs 2-agent TDD workflow: test-designer (RED) → compiler (GREEN+REFACTOR) to ensure compiled code passes tests.
  Trigger keywords: 코드 생성, 컴파일, CLAUDE.md에서 코드
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, AskUserQuestion]
---

# Compile Skill

## Core Philosophy

**CLAUDE.md → Source Code = Compile**

```
CLAUDE.md (WHAT)  ─── /compile ──→  Source Code (구현)
```

전통적 컴파일러가 소스코드를 바이너리로 변환하듯,
`/compile`은 CLAUDE.md 명세를 실행 가능한 소스코드로 변환.
compile-context (session temp)가 있으면 구현 방향 힌트로 활용.

## 2-Agent 아키텍처

```
/compile
    │
    ├─ Step 1: Task(test-designer) → 테스트 설계 (RED)
    │   INV-EXPORT: Exports 시그니처를 불변 테스트로 변환
    │
    └─ Step 2: Task(compiler) → 코드 생성 (GREEN + REFACTOR)
        INV-EXPORT: 테스트 수정 금지, 구현만 수정
```

**왜 2-Agent인가?**
1. **설계와 구현의 명확한 분리**: RED(설계)가 별도 agent이므로 GREEN이 설계를 변경할 수 없음
2. **Context 격리**: GREEN agent는 "테스트를 통과시키는 코드"만 생성
3. **INV-EXPORT 자연 보장**: compiler에게 테스트 파일은 Read-only → 시그니처 변경 원천 차단

## 입력 문서

| 입력 | 역할 | 업데이트 |
|------|------|----------|
| CLAUDE.md | 스펙 (WHAT) | 읽기 전용 |
| compile-context (optional) | 구현 방향 (session temp) | 읽기 전용 |

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
              compile-context 존재 확인 (optional)
                    │
                    ▼
              언어 자동 감지
                    │
                    ▼
              의존성 그래프 기반 실행 순서 결정 (leaf-first)
                    │
                    ▼
              각 대상에 대해 2-Agent 실행:
              │
              ├─ Step 1: Task(test-designer) → 테스트 설계
              │
              ├─ Step 2: Task(compiler) → 코드 생성
              │   │
              │   ├─ 성공 → 완료
              │   │
              │   └─ 실패 (3회 재시도 후)
              │       │
              │       ├─ Step 3: Task(test-designer) + 에러 컨텍스트
              │       │   → 테스트 인프라 수정 (assertion 변경 금지)
              │       │
              │       └─ Step 4: Task(compiler) → 재시도
              │           ├─ 성공 → 완료
              │           └─ 실패 → 사용자에게 보고
              │
              └─ 결과 수집 및 보고
```

상세 구현은 `references/workflow.md` 참조.

## 언어 및 테스트 프레임워크

프로젝트에서 사용 중인 언어와 테스트 프레임워크를 자동 감지.

- **언어**: 파일 확장자 기반
- **테스트 프레임워크**: 프로젝트 설정 파일 분석 (package.json, pyproject.toml, Cargo.toml 등)
- **Test Convention**: 프로젝트 CLAUDE.md `### Test Convention` 서브섹션 (있으면 우선)

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
✓ compile-context 로드 (optional)
✓ [RED] test-designer: 5 export tests + 3 behavior tests
✓ [GREEN] 구현 생성 → 테스트 통과 (attempt 1/3)
✓ [REFACTOR] Convention 적용 → 회귀 테스트 통과

[2/2] src/new/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 1개
✓ compile-context 로드 (optional)
✓ [RED] test-designer: 1 export test + 2 behavior tests
✓ [GREEN] 구현 생성 → 테스트 통과 (attempt 1/3)
✓ [REFACTOR] Convention 적용 → 회귀 테스트 통과

=== 생성 완료 ===
총 CLAUDE.md: 2개 (변경분)
생성된 파일: 5개
건너뛴 파일: 0개
테스트: 11 passed, 0 failed
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
1. src/auth/CLAUDE.md
2. src/utils/CLAUDE.md

코드 생성을 시작합니다...
...
```

## 참조 자료

- `references/compiler-workflow.md`: compiler agent 워크플로우 상세
- `references/test-designer-reference.md`: test-designer agent 방법론 + 언어별 패턴
- `references/workflow.md`: compile skill 워크플로우 상세
- `examples/generate-result.json`: compiler agent 결과 JSON 예시

## DO / DON'T

**DO:**
- Follow 2-agent TDD workflow (test-designer → compiler)
- Respect INV-EXPORT: test files are read-only for compiler
- Respect file conflict mode (skip/overwrite)
- Run feedback loop (max 1 round) when compiler fails

**DON'T:**
- Delete existing test files
- Let compiler modify test-designer's test files
- Overwrite files when conflict mode is "skip"
- Modify CLAUDE.md (read-only during compile)
- Run more than 1 feedback loop (infinite loop prevention)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md 파일을 찾을 수 없음" 메시지 출력 |
| compile-context 없음 | CLAUDE.md만으로 진행 (optional) |
| 파싱 오류 | 해당 파일 건너뛰고 계속 진행, 오류 로그 |
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| test-designer 실패 | 해당 대상 건너뛰고 오류 로그 |
| compiler 실패 (피드백 루프 포함) | 경고 표시, 수동 수정 필요 안내 |
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
✓ [RED] test-designer: 4 export tests + 3 behavior tests
✓ [GREEN] 구현 생성 → 테스트 통과
✓ [REFACTOR] Convention 적용

[2/2] src/utils/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 3개
✓ [RED] test-designer: 3 export tests + 2 behavior tests
✓ [GREEN] 구현 생성 → 테스트 통과
✓ [REFACTOR] Convention 적용

=== 생성 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 7개
테스트: 12 passed, 0 failed
</assistant_response>
</example>
