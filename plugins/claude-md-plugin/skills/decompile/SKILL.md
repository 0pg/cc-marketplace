---
name: decompile
version: 2.0.0
aliases: [decom]
trigger:
  - /decompile
  - 소스코드에서 CLAUDE.md 추출
  - extract spec from code
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", or uses "/decompile" or "/decom".
  Analyzes existing source code (binary) and creates CLAUDE.md (source) documentation for each directory.
allowed-tools: [Bash, Read, Write, Task, AskUserQuestion]
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
| IMPLEMENTS.md Implementation | HOW 상세 | Algorithm, Key Constants, Error Handling, State Management, Implementation Guide |

## Incremental Mode (기본값: true)

**Git 기반 증분 decompile 지원:**

| 조건 | 판단 | 동작 |
|------|------|------|
| CLAUDE.md 없음 | 신규 | decompile 필요 |
| 소스 uncommitted 변경 | 미확정 | decompile 필요 |
| 소스 commit > CLAUDE.md commit | outdated | decompile 필요 |
| CLAUDE.md commit >= 소스 commit | up-to-date | **skip** |

### 옵션

```
/decompile                     # incremental mode (기본값)
/decompile --full              # 전체 모드 (모든 디렉토리 처리)
/decompile --incremental=false # 전체 모드
/decompile src                 # 특정 경로만 처리
```

## 워크플로우

```
/decompile [path] [options]
    │
    ▼
옵션 파싱 (incremental mode 결정)
    │
    ▼
Task(recursive-decompiler, root_path)
    │
    ├─ boundary-resolve → subdirs
    ├─ 각 subdir에 대해 재귀 호출
    │      Task(recursive-decompiler, subdir)
    ├─ incremental 판단 (git 기반)
    └─ needs_decompile=true → Task(decompiler)
    │
    ▼
최종 보고
```

**핵심:** 재귀적으로 자식 디렉토리를 먼저 처리하여 자연스러운 leaf-first 순서를 보장합니다.

상세 실행 순서는 `references/execution-order.md` 참조.

## 실행 방법

### Step 1: 옵션 파싱

사용자 입력에서 옵션을 추출합니다:
- `target_path`: 경로가 지정되지 않으면 현재 디렉토리(`.`)를 사용합니다.
- `incremental_mode`: 기본값은 `true`입니다.
  - `--full` 또는 `--incremental=false` 옵션이 있으면 `false`로 설정합니다.

### Step 2: 재귀 decompiler 호출

`recursive-decompiler` agent를 호출합니다. 이 agent가 모든 하위 디렉토리를 재귀적으로 처리합니다.

```
Task(
    subagent_type="claude-md-plugin:recursive-decompiler",
    prompt="""
target_path: {target_path}
current_depth: 0
max_depth: 100
incremental_mode: {incremental_mode}
visited_paths: []

이 디렉토리와 모든 하위 디렉토리에 대해 재귀적으로 decompile을 수행해주세요.
결과는 .claude/tmp/{session-id}-{prefix}-{target} 형태로 저장하고 통계만 반환
""",
    description="Recursive decompile {target_path}"
)
```

### Step 3: 최종 보고

`recursive-decompiler` 결과에서 통계를 추출하여 최종 보고서를 출력합니다:
- `processed`: 처리된 디렉토리 수
- `skipped`: 스킵된 디렉토리 수 (incremental mode)
- `child_claude_mds`: 생성된 CLAUDE.md 경로 목록

```
=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

생성된 파일: {processed}개
스킵된 파일: {skipped}개 (incremental)

생성된 문서:
{생성된 CLAUDE.md 목록}

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
```

## 대상 디렉토리 확인 예시

```
=== Decompile 시작 ===

모드: incremental (변경된 파일만 처리)
대상: src/

제외 디렉토리: node_modules, target, dist, __pycache__, .git 등

계속하시겠습니까?
```

## 내부 컴포넌트

| 컴포넌트 | 타입 | 역할 | 호출 위치 |
|----------|------|------|----------|
| `recursive-decompiler` | Agent | 재귀 탐색, incremental 판단, 오케스트레이션 | decompile Skill |
| `decompiler` | Agent | 단일 모듈 CLAUDE.md + IMPLEMENTS.md 생성 | recursive-decompiler Agent |
| `boundary-resolve` | Skill | 바운더리 분석 | recursive-decompiler, decompiler |
| `code-analyze` | Skill | 코드 분석 | decompiler Agent |
| `schema-validate` | Skill | 스키마 검증 | decompiler Agent |

**tree-parse는 더 이상 사용되지 않습니다.** 재귀적 구조로 대체되었습니다.

## 최종 보고 예시

### Incremental Mode

```
=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

모드: incremental
처리 결과:
  ✓ src/auth/CLAUDE.md + IMPLEMENTS.md (신규)
  ✓ src/utils/CLAUDE.md + IMPLEMENTS.md (소스 변경됨)
  - src/api/CLAUDE.md (스킵: up-to-date)

통계:
  - 처리됨: 2
  - 스킵됨: 1 (incremental)

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
```

### Full Mode

```
=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

모드: full
처리 결과:
  ✓ src/auth/CLAUDE.md + IMPLEMENTS.md
  ✓ src/utils/CLAUDE.md + IMPLEMENTS.md
  ✓ src/api/CLAUDE.md + IMPLEMENTS.md

통계:
  - 처리됨: 3
  - 스킵됨: 0

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
```

## 파일 기반 결과 전달

Agent는 결과를 `.claude/tmp/{session-id}-{prefix}-{target}` 형태로 저장하고 경로만 반환합니다.
이로써 context 폭발을 방지합니다.

## 오류 처리

| 상황 | 대응 |
|------|------|
| Git 저장소 아님 | 경고 메시지 출력, incremental 비활성화 |
| recursive-decompiler 실패 | 에러 메시지 전달 |
| decompiler 실패 | 해당 디렉토리 스킵, 경고 표시 |
| 스키마 검증 실패 | Agent가 경고와 함께 진행 |
| 순환 참조 (symlink) | visited_paths 기반 감지, 즉시 반환 |
| max_depth 초과 | 경고 로그, 재귀 중단 (2차 안전장치) |

## Ignored Directories

다음 디렉토리는 자동으로 제외됩니다:

- `node_modules` - npm/yarn 의존성
- `target` - Rust/Maven 빌드 결과
- `dist` - 빌드 출력
- `build` - 빌드 출력
- `__pycache__` - Python 캐시
- `.git` - Git 메타데이터
- `vendor` - Go/PHP 의존성
- `.next` - Next.js 빌드
- `.nuxt` - Nuxt.js 빌드
- `coverage` - 테스트 커버리지
- `.venv`, `venv`, `env` - Python 가상환경
- 숨김 디렉토리 (`.` 시작)
