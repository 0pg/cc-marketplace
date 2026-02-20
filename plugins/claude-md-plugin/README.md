# claude-md-plugin (v2.24.0)

> CLAUDE.md + IMPLEMENTS.md 듀얼 문서 시스템 기반의 문서-코드 동기화 플러그인

## 개요

기존의 "소스코드 → 문서" 접근법을 역전시켜 **CLAUDE.md + IMPLEMENTS.md가 소스코드**이고, **소스코드가 바이너리**가 되는 Compile/Decompile 패러다임을 제공합니다.

```
┌─────────────────────────────────────────────────────────────┐
│                    전통적 소프트웨어                          │
│                                                             │
│   .h (헤더)  +  .c (소스)  ─── compile ──→  Binary (.exe)   │
│   Binary (.exe)  ─── decompile ──→  .h + .c                 │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin (듀얼 문서 시스템)       │
│                                                             │
│   CLAUDE.md (WHAT) + IMPLEMENTS.md (HOW)                    │
│         │                                                   │
│         └──── /compile ──→  Source Code (구현)              │
│                                                             │
│   Source Code (구현)  ─── /decompile ──→                    │
│         └──→ CLAUDE.md (WHAT) + IMPLEMENTS.md (HOW)         │
└─────────────────────────────────────────────────────────────┘
```

| 전통적 개념 | claude-md-plugin | 역할 |
|------------|------------------|------|
| .h (헤더) | CLAUDE.md | **WHAT** - 인터페이스, 스펙 |
| .c (소스) | IMPLEMENTS.md | **HOW** - 구현 명세 |
| Binary | Source Code (.ts, .py, ...) | 실행물 |
| **compile** | CLAUDE.md + IMPLEMENTS.md → Source Code | `/compile` |
| **decompile** | Source Code → CLAUDE.md + IMPLEMENTS.md | `/decompile` |

## Prerequisites

### Rust Toolchain (필수)

플러그인의 `core/` 디렉토리에 포함된 Rust CLI 바이너리 빌드가 필요합니다.

- **rustc** + **cargo** (edition 2021, Rust 1.56+)
- 주요 의존성: clap 4.4, serde, walkdir, regex

```bash
# 빌드
cd plugins/claude-md-plugin/core && cargo build --release
```

빌드 결과물: `claude-md-core` CLI 바이너리

## 설치

프로젝트 플러그인으로 포함되어 있으므로 별도의 설치가 필요하지 않습니다.
`developer-claude-code-plugin` 저장소를 클론하면 자동으로 사용할 수 있습니다.

```bash
git clone <repo-url>
cd developer-claude-code-plugin

# Rust core 빌드
cd plugins/claude-md-plugin/core && cargo build --release
```

## 사용법

### Quick Start

| 상황 | 커맨드 | 결과 |
|------|--------|------|
| 새 모듈 요구사항 정의 | `/impl "요구사항"` | CLAUDE.md + IMPLEMENTS.md |
| 기존 코드 문서화 | `/decompile` | CLAUDE.md + IMPLEMENTS.md |
| 명세 기반 코드 생성 | `/compile` | 소스코드 + 테스트 |
| 문서-코드 일치 확인 | `/validate` | 통합 검증 보고서 |
| 런타임 버그 수정 | `/bugfix --error "에러"` | 3계층 추적 → 문서 수정 |
| 명세 품질 리뷰 | `/impl-review` | 4차원 품질 보고서 |

### 커맨드 상세

#### `/impl` — 요구사항에서 명세 생성

> Aliases: `define`, `requirements`

**언제 사용하나요?**
- 새 기능을 개발하기 전, 요구사항을 CLAUDE.md 명세로 정리하고 싶을 때
- 기존 모듈에 새로운 기능을 추가하고 싶을 때

**사용법:**
```
/impl "JWT 토큰을 검증하는 인증 모듈이 필요합니다"
```

**실행 결과 예시:**
```
=== /impl 완료 ===

생성/업데이트된 파일:
  ✓ src/auth/CLAUDE.md (WHAT - 스펙)
  ✓ src/auth/IMPLEMENTS.md (HOW - Planning Section)

스펙 요약:
  - Purpose: JWT 토큰 검증 및 사용자 인증
  - Exports: 2개
  - Behaviors: 3개

검증 결과: 스키마 검증 통과

다음 단계:
  - /compile로 코드 구현 가능
  - /validate로 문서-코드 일치 검증 가능
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| 요구사항 불명확 | AskUserQuestion으로 명확화 질문 |
| 대상 경로 모호 | 후보 목록 제시 후 선택 요청 |
| 기존 CLAUDE.md와 충돌 | 병합 전략 제안 |
| 기존 IMPLEMENTS.md와 충돌 | Planning Section만 업데이트 (Implementation Section 유지) |

**다음 단계:** `/compile` → 명세 기반 코드 생성

---

#### `/decompile` — 기존 코드에서 명세 추출

> Aliases: `decom`

**언제 사용하나요?**
- 레거시 코드를 CLAUDE.md 체계로 편입시키고 싶을 때
- 기존 프로젝트를 처음 도입할 때 전체 문서화가 필요할 때

**사용법:**
```
/decompile
```

**실행 결과 예시:**
```
=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

생성된 파일:
  ✓ src/CLAUDE.md + IMPLEMENTS.md
  ✓ src/auth/CLAUDE.md + IMPLEMENTS.md
  ✓ src/api/CLAUDE.md + IMPLEMENTS.md

검증 결과:
  - CLAUDE.md 스키마: 3/3 통과
  - IMPLEMENTS.md 스키마: 3/3 통과

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | 에러 메시지 출력, 실패 반환 |
| tree-parse 실패 | CLI 에러 메시지 전달 |
| decompiler 실패 | 해당 디렉토리 스킵, 경고 표시 후 계속 진행 |

**다음 단계:** `/validate` → 추출된 문서와 코드 일치 확인

---

#### `/compile` — 명세에서 소스코드 생성

> Aliases: `gen`, `generate`, `build`

**언제 사용하나요?**
- `/impl`로 명세를 작성한 뒤, 코드를 자동 생성하고 싶을 때
- CLAUDE.md를 직접 수정한 뒤, 변경 사항을 코드에 반영하고 싶을 때

**사용법:**
```bash
# 기본 사용 (프로젝트 전체)
/compile

# 특정 경로만 처리
/compile --path src/auth

# 기존 파일 덮어쓰기
/compile --conflict overwrite
```

**옵션:**

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리 대상 경로 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 (`skip` \| `overwrite`) |

**실행 결과 예시:**
```
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
✓ 테스트 생성 (3 test cases)
✓ 구현 생성
✓ 테스트 실행: 3 passed

=== 생성 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 7개
테스트: 8 passed, 0 failed
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| IMPLEMENTS.md 없음 | 기본 템플릿으로 자동 생성 후 진행 |
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| 테스트 실패 | 최대 3회 재시도, 이후 경고 표시 |
| 파일 충돌 (skip 모드) | 기존 파일 유지, 새 파일만 생성 |

**다음 단계:** `/validate` → 생성된 코드와 문서 일치 확인

---

#### `/validate` — 문서-코드 일치 검증

> Aliases: `check`, `verify`, `lint`

**언제 사용하나요?**
- `/compile` 후 생성된 코드가 명세와 일치하는지 확인하고 싶을 때
- `/decompile` 후 추출된 문서가 정확한지 검증하고 싶을 때

**사용법:**
```bash
# 기본 사용 (프로젝트 전체)
/validate

# 특정 경로만 검증
/validate src/
```

**검증 항목:**

| 검증기 | 역할 |
|--------|------|
| **validator** | Structure, Exports, Dependencies, Behavior 일치 검증 + Export 커버리지 |

**실행 결과 예시:**
```
CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 개선 권장: 1개
- 개선 필요: 1개

상세 결과
---------
src/auth (양호)
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (개선 권장)
  Drift: 2개 이슈
    - STALE: formatDate export가 코드에 없음
    - MISSING: parseNumber export가 문서에 없음
  Export 커버리지: 78% (14/18 예측 성공)
```

**상태 기준:**

| 상태 | 조건 |
|------|------|
| **양호** | Drift 이슈 0개 AND Export 커버리지 90% 이상 |
| **개선 권장** | Drift 1-2개 OR Export 커버리지 70-89% |
| **개선 필요** | Drift 3개 이상 OR Export 커버리지 70% 미만 |

**다음 단계:** 이슈가 발견되면 CLAUDE.md 또는 소스코드를 수정 후 다시 `/validate`

---

#### `/bugfix` — 런타임 버그 진단 및 수정

> Aliases: `diagnose`, `troubleshoot`, `fix-bug`

**언제 사용하나요?**
- `/compile`로 생성된 코드에서 런타임 에러가 발생했을 때
- 테스트가 실패하여 근본 원인을 추적하고 싶을 때
- 기능이 명세와 다르게 동작할 때

**사용법:**
```bash
# 에러 메시지로 진단
/bugfix --error "TypeError: validateToken is not a function" --path src/auth

# 실패하는 테스트로 진단
/bugfix --test "should return empty array for no results"

# 기능 설명으로 진단
/bugfix --error "로그인 시 토큰 만료되면 자동 갱신이 안 됩니다"
```

**실행 결과 예시:**
```
/bugfix 결과
=========

Root Cause: L1 - SPEC_EXPORT_MISMATCH
요약: CLAUDE.md exports validateToken as standalone but code defines it as class method

수정된 문서: [CLAUDE.md]

⚠ `/compile --path src/auth --conflict overwrite`로 소스 코드를 재생성하세요.

상세 결과: .claude/tmp/debug-src-auth.md
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | `/decompile` 먼저 실행하여 CLAUDE.md 생성 제안 |
| CLAUDE.md 스키마 오류 | `/validate` 먼저 실행 안내 |
| 미컴파일 변경 감지 | `/compile --path <path>` 먼저 실행 안내 |
| 에러 정보 부족 | AskUserQuestion으로 에러 정보 수집 |

**다음 단계:** `/compile --path <dir> --conflict overwrite` → 수정된 문서로 소스코드 재생성

---

#### `/impl-review` — 명세 품질 리뷰

> Aliases: `review-impl`, `impl-quality`, `rate-impl`

**언제 사용하나요?**
- `/impl`로 생성한 CLAUDE.md + IMPLEMENTS.md의 품질을 확인하고 싶을 때
- 요구사항이 문서에 충분히 반영되었는지 검증하고 싶을 때
- `/validate`와 달리 코드가 아닌 **문서 자체의 품질**을 평가하고 싶을 때

**사용법:**
```bash
# 기본 사용 (특정 경로)
/impl-review src/auth

# 프로젝트 전체
/impl-review
```

**실행 결과 예시:**
```
=== /impl-review 완료 ===

D1. 요구사항 커버리지: 85/100 (WARNING 1건)
  - Edge case: 토큰 만료 시 갱신 로직 미정의

D2. CLAUDE.md 품질: 92/100 (INFO 1건)
  - Exports 시그니처 상세도 양호

D3. IMPLEMENTS.md 계획 품질: 88/100 (WARNING 1건)
  - Error handling strategy 미상세

D4. 문서 간 일관성: 95/100 (INFO 1건)
  - Dependencies 섹션과 Planning Section 일치

종합: 89/100 (Good)

수정 제안: 2건 (1 WARNING, 1 INFO)
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | `/impl` 먼저 실행 안내 |
| IMPLEMENTS.md 없음 | `/impl`로 Planning Section 생성 안내 |
| 스키마 검증 실패 | 스키마 오류 수정 후 재실행 |

**다음 단계:** 수정 제안 적용 후 `/compile`로 코드 생성

---

#### `/project-setup` — 프로젝트 Convention 초기 설정

**언제 사용하나요?**
- 새 프로젝트에 Convention 규칙을 설정하고 싶을 때
- 기존 프로젝트의 코딩 스타일을 분석하여 Convention을 자동 추출하고 싶을 때

**사용법:**
```bash
# 자동 탐지
/project-setup

# 프로젝트 루트 지정
/project-setup /path/to/project
```

**실행 결과:**
- `project_root/CLAUDE.md`에 `## Project Convention` 섹션 추가
- 각 `module_root/CLAUDE.md`에 `## Code Convention` 섹션 추가
- `validate-convention` CLI로 검증 수행

---

#### `/convention-update` — Convention 섹션 업데이트

**언제 사용하나요?**
- 기존 Convention 규칙을 수정하고 싶을 때

**사용법:**
```bash
# 대화형
/convention-update

# 직접 지시
/convention-update "들여쓰기를 4 spaces로 변경"
```

**실행 결과:** Convention 섹션 업데이트 후 `validate-convention` CLI로 검증 수행

---

### 워크플로우 예시

#### A. 신규 모듈 개발 (처음부터)

```
/impl "요구사항" → /compile → /validate
```

1. `/impl "JWT 인증 모듈이 필요합니다"` — 요구사항을 CLAUDE.md + IMPLEMENTS.md로 변환
2. `/compile` — 명세 기반 코드 + 테스트 자동 생성
3. `/validate` — 생성된 코드와 문서 일치 확인

#### B. 레거시 코드 문서화

```
/decompile → /validate → (드리프트 수정) → /validate
```

1. `/decompile` — 기존 코드에서 CLAUDE.md + IMPLEMENTS.md 추출
2. `/validate` — 추출된 문서와 코드 일치 확인
3. 이슈가 있으면 CLAUDE.md 또는 코드 수정
4. `/validate` — 수정 후 재검증

#### C. 명세 변경 후 재구현

```
/impl "변경된 요구사항" → /compile --conflict overwrite → /validate
```

1. `/impl "기존 인증에 OAuth2 지원 추가"` — 명세 업데이트
2. `/compile --conflict overwrite` — 변경된 명세로 코드 재생성 (기존 파일 덮어쓰기)
3. `/validate` — 변경 사항 검증

#### D. 런타임 버그 수정

```
/bugfix --error "에러" → /compile --conflict overwrite → /validate
```

1. `/bugfix --error "TypeError: ..."` — 3계층 추적으로 근본 원인 진단 및 문서 수정
2. `/compile --conflict overwrite` — 수정된 문서로 소스코드 재생성
3. `/validate` — 수정 후 문서-코드 일치 확인

#### E. 명세 품질 리뷰

```
/impl-review → (수정 적용) → /compile
```

1. `/impl-review src/auth` — 4차원 품질 리뷰 수행
2. 수정 제안 적용 — 대화형으로 CLAUDE.md / IMPLEMENTS.md 수정
3. `/compile` — 수정된 명세로 코드 재생성

## 핵심 개념

### 듀얼 문서 시스템 (CLAUDE.md + IMPLEMENTS.md)

각 디렉토리에 CLAUDE.md와 IMPLEMENTS.md가 1:1로 매핑됩니다.

```
auth/
├── CLAUDE.md       ← WHAT (스펙)
│   ├── Exports: validateToken(token: string): Claims
│   └── Domain Context: 토큰 만료 7일 (PCI-DSS)
│
└── IMPLEMENTS.md   ← HOW (구현 명세)
    ├── [Planning Section] - /impl이 업데이트
    │   ├── Dependencies Direction
    │   ├── Implementation Approach
    │   └── Technology Choices
    │
    └── [Implementation Section] - /compile이 업데이트
        ├── Algorithm
        ├── Key Constants
        ├── Error Handling
        ├── State Management
        └── Implementation Guide
```

**명령어별 업데이트 범위:**

| 명령어 | CLAUDE.md | IMPLEMENTS.md |
|--------|-----------|---------------|
| `/impl` | 생성/업데이트 | Planning Section 업데이트 |
| `/compile` | 읽기 전용 | Implementation Section 업데이트 |
| `/decompile` | 생성 (전체) | 생성 (전체 - Planning + Implementation) |
| `/bugfix` | 수정 (L1 fix) | 수정 (L2 fix) |

### Exports = Interface Catalog

Exports 섹션은 다른 모듈이 코드 탐색 없이 인터페이스를 파악할 수 있는 카탈로그입니다.

```
의존 모듈 참조 시 탐색 순서:
1. 의존 모듈 CLAUDE.md Exports ← 여기서 인터페이스 확인
2. 의존 모듈 CLAUDE.md Behavior ← 동작 이해 필요 시
3. 실제 소스코드 ← 최후 수단 (Exports로 불충분할 때만)
```

### CLAUDE.md 배치 규칙

다음 조건 중 하나라도 충족하는 디렉토리에 CLAUDE.md가 존재해야 합니다:
- 1개 이상의 소스코드 파일이 존재
- 2개 이상의 하위 디렉토리가 존재

### 트리 구조 의존성

```
project/CLAUDE.md
    │
    ├──► src/CLAUDE.md
    │        │
    │        └──► src/auth/CLAUDE.md
    │
    └──► tests/CLAUDE.md
```

- **부모 → 자식**: 참조 가능
- **자식 → 부모**: 참조 불가
- **형제 ↔ 형제**: 참조 불가

### Convention 섹션

프로젝트 수준 컨벤션을 CLAUDE.md 내 섹션으로 관리합니다:

- **`## Project Convention`** (project_root CLAUDE.md): 아키텍처/모듈 구조 규칙
  - 필수: Project Structure, Module Boundaries, Naming Conventions
- **`## Code Convention`** (module_root CLAUDE.md): 소스코드 수준 규칙
  - 필수: Language & Runtime, Code Style, Naming Rules

멀티 모듈 프로젝트에서는 module_root가 project_root의 Convention을 override할 수 있습니다.

**컨벤션 우선순위:**
1. module_root `## Code Convention` → 코드 스타일
2. module_root `## Project Convention` (optional override)
3. project_root `## Code Convention` (fallback)
4. project_root `## Project Convention`

### CLAUDE.md 스키마

```markdown
# {디렉토리명}

## Purpose
이 디렉토리의 책임 (1-2문장)

## Domain Context
코드에서 읽을 수 없는 "왜?" - 비즈니스 맥락, 결정 이유

## Structure
- subdir/: 설명 (상세는 subdir/CLAUDE.md 참조)
- file.ext: 역할

## Exports
### Functions
- `FunctionName(params) ReturnType`

### Types
- `TypeName { fields }`

## Dependencies
- external: package v1.2.3

## Behavior
- 정상 케이스: input → expected output
- 에러 케이스: invalid input → specific error

## Constraints
- 제약사항

## Project Convention (project/module root만)
### Project Structure
### Module Boundaries
### Naming Conventions

## Code Convention (module root만)
### Language & Runtime
### Code Style
### Naming Rules
```

## 아키텍처

### Agents

| Agent | 역할 |
|-------|------|
| `impl` | 요구사항 분석 및 CLAUDE.md + IMPLEMENTS.md 생성 |
| `dep-explorer` | 요구사항 의존성 탐색 (internal + external) |
| `decompiler` | 소스코드에서 CLAUDE.md + IMPLEMENTS.md 추출 |
| `compiler` | CLAUDE.md + IMPLEMENTS.md에서 소스코드 생성 (TDD) |
| `debug-layer-analyzer` | 단일 계층(L3/L1/L2) 진단 분석 (debugger의 sub-agent) |
| `debugger` | 소스코드 런타임 버그 → 3계층 추적 → 수정 (orchestrator) |
| `impl-reviewer` | CLAUDE.md + IMPLEMENTS.md 품질 리뷰 및 요구사항 커버리지 검증 |
| `validator` | CLAUDE.md-코드 일치 검증 (Structure, Exports, Dependencies, Behavior) + Export 커버리지 |

### Skills

**Entry Point (사용자 진입점):**

| Skill | 역할 |
|-------|------|
| `/impl` | 요구사항 → CLAUDE.md + IMPLEMENTS.md |
| `/decompile` | 소스코드 → CLAUDE.md + IMPLEMENTS.md |
| `/compile` | CLAUDE.md + IMPLEMENTS.md → 소스코드 |
| `/validate` | 문서-코드 일치 검증 |
| `/bugfix` | 소스코드 런타임 버그 → 3계층 추적 → 수정 |
| `/impl-review` | CLAUDE.md + IMPLEMENTS.md 품질 리뷰 |

**Internal (Agent가 호출):**

| Skill | 역할 |
|-------|------|
| `tree-parse` | 디렉토리 구조 분석 |

### 설계 원칙

```
User → Entry Point Skill → Agent → Internal Skill(s)
```

| 컴포넌트 | 역할 | 오케스트레이션 |
|----------|------|---------------|
| **Entry Point Skill** | 사용자 진입점 | 간단 (파일 검색, 반복, Agent 호출) |
| **Internal Skill** | 단일 기능 (SRP) | 없음, Stateless |
| **Agent** | 비즈니스 로직 | 복잡 (N개 Skill, 재시도, 상태) |

## CLI 도구

플러그인에 포함된 Rust CLI 도구 (`core/`):

```bash
# 트리 파싱 - CLAUDE.md가 필요한 디렉토리 식별
claude-md-core parse-tree --root . --output tree.json

# 바운더리 결정 - 디렉토리의 책임 범위 분석
claude-md-core resolve-boundary --path src/auth --output boundary.json

# 코드 분석 - exports, dependencies, behaviors 추출
claude-md-core analyze-code --path src/auth --output analysis.json

# CLAUDE.md 파싱 - JSON 출력
claude-md-core parse-claude-md --file src/auth/CLAUDE.md

# 스키마 검증 - CLAUDE.md 형식 검증
claude-md-core validate-schema --file CLAUDE.md --output validation.json

# Convention 검증 - Convention 섹션 존재 및 필수 서브섹션 확인
claude-md-core validate-convention --project-root .
claude-md-core validate-convention --project-root . --module-roots packages/api,packages/web

# CLAUDE.md 인덱스 생성 - 프로젝트 전체 CLAUDE.md 스캔
claude-md-core scan-claude-md --root . --output index.json

# 변경 감지 - incremental compile 대상 식별
claude-md-core diff-compile-targets --root .

# Exports 마크다운 생성 - analyze-code JSON → Exports 섹션
claude-md-core format-exports --input analysis.json --output exports.md

# 전체 분석 마크다운 생성 - analyze-code JSON → 분석 요약
claude-md-core format-analysis --input analysis.json --output summary.md

# 프로젝트 전체 인덱싱 - tree-parse + code analysis
claude-md-core index-project --root . --output index-results/
```

## 언어 지원

**프로젝트에서 사용하는 언어와 테스트 프레임워크를 자동 감지합니다.**

- 언어 감지: 파일 확장자 기반
- 테스트 프레임워크 감지: 프로젝트 설정 파일 분석

## 라이선스

MIT
