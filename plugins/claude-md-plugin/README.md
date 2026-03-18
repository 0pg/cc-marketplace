# claude-md-plugin (v4.1.0)

> Code-First + Spec-as-Contract: 소스코드가 Source of Truth, CLAUDE.md는 코드가 만족해야 할 계약

## 개요

**소스코드가 유일한 Source of Truth**이며, CLAUDE.md는 코드가 만족해야 할 **계약(Contract)**을 정의합니다. 계약 기반으로 코드를 생성하고, 코드가 계약을 위반하는지 검증합니다.

```
┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                          │
│                                                             │
│   CLAUDE.md (Contract)                                      │
│         │                                                   │
│         ├──── /compile ──→  계약을 만족하는 코드 생성       │
│         ├──── /validate ──→ 코드가 계약 위반? → 보고        │
│         │                                                   │
│   Source Code (Source of Truth)                              │
│         │                                                   │
│         └──── /decompile ──→  코드에서 계약 추출            │
└─────────────────────────────────────────────────────────────┘
```

| 개념 | claude-md-plugin | 역할 |
|------|------------------|------|
| Contract | CLAUDE.md | 코드가 만족해야 할 계약 (**WHAT**) |
| Source of Truth | Source Code (.ts, .py, ...) | 실제 구현 |
| **compile** | 계약 → 코드 생성 | `/compile` |
| **decompile** | 코드 → 계약 추출 | `/decompile` |
| **validate** | 코드가 계약 위반? | `/validate` |

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
| 자연어로 작업 요청 | `/dev "요청"` | 적절한 스킬로 라우팅 |
| 새 모듈 요구사항 정의 | `/impl "요구사항"` | CLAUDE.md + compile-context |
| 기존 코드 문서화 | `/decompile` | CLAUDE.md |
| 명세 기반 코드 생성 | `/compile` | 소스코드 + 테스트 |
| 계약-코드 일치 확인 | `/validate` | 위반 보고서 (계약 수정 안 함) |
| 런타임 버그 수정 | `/bugfix --error "에러"` | 3계층 추적 → 코드 재생성 |
| 명세 품질 리뷰 | `/impl-review` | 3차원 품질 보고서 |
| 계약 변경 영향 분석 | `/impact src/auth` | 영향받는 모듈 보고서 |
| 계약 버전 비교 | `/diff-spec src/auth` | 시맨틱 diff 보고서 |
| 프로젝트 건강도 확인 | `/status` | 건강도 대시보드 |
| 모듈 분할/병합 | `/refactor src/auth --mode split` | 계약 수준 리팩토링 |

### 커맨드 상세

#### `/dev` — 자연어 → 스킬 라우팅

**언제 사용하나요?**
- 어떤 스킬을 사용해야 할지 모를 때
- 자연어로 작업을 요청하고 싶을 때

**사용법:**
```bash
# 기능 추가 요청 → /impl로 라우팅
/dev "로그인 기능 추가"

# 버그 수정 요청 → /bugfix로 라우팅
/dev "토큰 검증 에러"

# 경로 지정
/dev "인증 모듈 검증" --path src/auth
```

**분류 기준:**

| 카테고리 | 키워드 | 대상 스킬 |
|----------|--------|-----------|
| FEATURE | add, create, new, 추가, 생성, 기능 | `/impl` |
| BUGFIX | fix, bug, error, 버그, 에러, 실패 | `/bugfix` |
| COMPILE | compile, generate, build, 컴파일 | `/compile` |
| VALIDATE | validate, check, verify, 검증 | `/validate` |
| AMBIGUOUS | (매칭 없음) | 사용자에게 질문 |

---

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
  ✓ .claude/tmp/compile-context-{hash}.md (compile-context)

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
=== CLAUDE.md 추출 완료 ===

생성된 파일:
  ✓ src/CLAUDE.md
  ✓ src/auth/CLAUDE.md
  ✓ src/api/CLAUDE.md

검증 결과:
  - CLAUDE.md 스키마: 3/3 통과

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

# 크로스 모듈 통합 테스트 포함
/compile --integration

# 미리보기 (파일 생성 안 함)
/compile --dry-run
```

**옵션:**

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리 대상 경로 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 (`skip` \| `overwrite`) |
| `--integration` | `false` | 크로스 모듈 계약 검증 통합 테스트 포함 |
| `--dry-run` | `false` | 생성될 코드 미리보기 (파일 생성 안 함) |
| `--parallel` | `3` | 같은 depth 병렬 실행 최대 수 |

**실행 결과 예시:**
```
발견된 CLAUDE.md 파일:
1. src/auth/CLAUDE.md
2. src/utils/CLAUDE.md

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개, 클래스 1개
✓ compile-context 로드 (있는 경우)
✓ 테스트 생성 (5 test cases)
✓ 구현 생성
✓ 테스트 실행: 5 passed

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
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| 테스트 실패 | 최대 3회 재시도, 이후 경고 표시 |
| 파일 충돌 (skip 모드) | 기존 파일 유지, 새 파일만 생성 |

**다음 단계:** `/validate` → 생성된 코드와 문서 일치 확인

---

#### `/validate` — 계약-코드 일치 검증

> Aliases: `check`, `verify`, `lint`

**언제 사용하나요?**
- 코드가 계약(CLAUDE.md)을 만족하는지 확인하고 싶을 때
- `/compile` 후 생성된 코드가 계약과 일치하는지 확인하고 싶을 때

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
| **validator** | Structure, Exports, Dependencies, Behavior 계약 위반 검증 + Export 커버리지 |

**실행 결과 예시:**
```
CLAUDE.md 계약 검증 보고서
========================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 위반 발견: 1개

상세 결과
---------
src/auth (양호)
  위반: 0개
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (위반 발견)
  위반: 2개
    - HIGH Exports STALE: formatDate — 계약에 있으나 코드에 없음
    - MEDIUM Structure UNCOVERED: helper.ts — 코드에 존재하나 계약에 미등록
  추천: `/compile --path src/utils --conflict overwrite`
```

**상태 기준:**

| 상태 | 조건 |
|------|------|
| **양호** | 위반 0개 AND Export 커버리지 90% 이상 |
| **위반 발견** | 확인된 위반이 1개 이상 |
| **개선 권장** | Export 커버리지 70-89% AND 위반 없음 |
| **개선 필요** | 스키마 FAIL OR Export 커버리지 70% 미만 |

**다음 단계:** 위반이 발견되면 `/compile` 재실행으로 코드 재생성, 또는 계약 업데이트가 필요하면 수동 편집

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

Root Cause: L3 - CODE_SPEC_DIVERGENCE
요약: Code returns null instead of empty array as specified in CLAUDE.md Behavior

수정: 계약 기준 코드 재생성 (/compile 자동 실행)
Compile: PASS
검증: PASS

상세 결과: .claude/tmp/debug-src-utils.md
```

**L1 root cause (계약 자체 오류) 시:**
```
Root cause가 L1 (계약 자체 오류)으로 진단되었습니다.
선택지:
A) 계약(CLAUDE.md) 수정 → /compile 재실행
B) 코드 직접 재생성 — 현재 계약 기준으로 /compile 재실행
C) 추가 분석 요청
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | `/decompile` 먼저 실행하여 계약 추출 제안 |
| CLAUDE.md 스키마 오류 | `/validate` 먼저 실행 안내 |
| 미컴파일 변경 감지 | `/compile --path <path>` 먼저 실행 안내 |
| 에러 정보 부족 | AskUserQuestion으로 에러 정보 수집 |

**다음 단계:** `/compile --path <dir> --conflict overwrite` → 계약 기반 코드 재생성

---

#### `/impl-review` — 명세 품질 리뷰

> Aliases: `review-impl`, `impl-quality`, `rate-impl`

**언제 사용하나요?**
- `/impl`로 생성한 CLAUDE.md의 품질을 확인하고 싶을 때
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

D3. 문서 간 일관성: 95/100 (INFO 1건)
  - Dependencies 섹션 일치

종합: 91/100 (Good)

수정 제안: 2건 (1 WARNING, 1 INFO)
```

**에러 시 대응:**

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | `/impl` 먼저 실행 안내 |
| 스키마 검증 실패 | 스키마 오류 수정 후 재실행 |

**다음 단계:** 수정 제안 적용 후 `/compile`로 코드 생성

---

#### `/impact` — 계약 변경 영향 분석

> Aliases: `impact-analysis`, `affected`

**언제 사용하나요?**
- CLAUDE.md를 수정한 후, 어떤 모듈이 영향을 받는지 확인하고 싶을 때
- Exports 시그니처를 변경/삭제하기 전에 영향 범위를 파악하고 싶을 때

**사용법:**
```bash
# 특정 모듈의 계약 변경 영향 분석
/impact src/auth
```

**실행 결과 예시:**
```
계약 변경 영향 분석: src/auth
==============================

변경 요약
---------
| Export          | 변경 유형          | 영향 수준   |
|-----------------|-------------------|------------|
| validateToken   | SIGNATURE_CHANGED | BREAKING   |
| revokeToken     | ADDED             | COMPATIBLE |

영향받는 모듈
-----------
BREAKING (1개 모듈):
  src/api
    - validateToken 시그니처 변경 → 호출 코드 수정 필요
    - 추천: /compile --path src/api --conflict overwrite

추천 액션:
  /compile --path src/api --conflict overwrite
  /validate
```

**다음 단계:** 영향받는 모듈에 대해 `/compile` 재실행

---

#### `/diff-spec` — 계약 버전 비교

> Aliases: `spec-diff`, `contract-diff`

**언제 사용하나요?**
- CLAUDE.md를 수정한 후, 어떤 계약 조항이 변경되었는지 구조적으로 확인하고 싶을 때
- 특정 버전과 현재 계약의 차이를 파악하고 싶을 때

**사용법:**
```bash
# HEAD와 현재 비교
/diff-spec src/auth

# 특정 ref와 비교
/diff-spec src/auth --ref v2.0.0
```

**실행 결과 예시:**
```
계약 시맨틱 Diff: src/auth
===========================
비교: HEAD → 현재

요약
----
| 섹션    | 추가 | 제거 | 변경 | 상태     |
|---------|------|------|------|----------|
| Exports | 1    | 0    | 1    | BREAKING |

Exports 변경
-----------
+ revokeToken(tokenId: string): Promise<void>
~ validateToken: 시그니처 변경 [BREAKING]

다음 단계:
  /impact src/auth — 영향받는 모듈 분석
```

**다음 단계:** `/impact` → 영향받는 모듈 분석

---

#### `/status` — 프로젝트 건강도 대시보드

> Aliases: `health`, `dashboard`, `overview`

**언제 사용하나요?**
- 프로젝트 전체의 계약 상태를 빠르게 파악하고 싶을 때
- `/validate`보다 가볍고 빠른 전체 현황 확인이 필요할 때

**사용법:**
```bash
/status
```

**실행 결과 예시:**
```
프로젝트 계약 건강도: GOOD

| 지표 | 값 | 상태 |
|------|-----|------|
| CLAUDE.md 수 | 5 | - |
| 스키마 유효 | 4/5 (80%) | WARNING |
| Compile 신선도 | 3/5 FRESH | WARNING |
| DEVELOPERS.md 쌍 | 5/5 (100%) | OK |

추천: /compile 으로 stale 모듈 재컴파일
```

---

#### `/refactor` — 모듈 분할/병합

> Aliases: `split`, `merge`, `restructure`

**언제 사용하나요?**
- 모듈이 너무 커져서 분할하고 싶을 때
- 여러 작은 모듈을 하나로 병합하고 싶을 때
- 코드가 아닌 계약(CLAUDE.md) 수준에서 구조를 변경하고 싶을 때

**사용법:**
```bash
# 모듈 분할
/refactor src/auth --mode split

# 모듈 병합
/refactor src/auth/token --mode merge
```

**실행 결과 예시:**
```
src/auth/CLAUDE.md를 분석합니다...
Exports: 6개

분할 제안:
  src/auth/token/CLAUDE.md: validateToken, revokeToken, Claims
  src/auth/session/CLAUDE.md: createSession, destroySession, SessionConfig

영향 분석: src/api — 경로 변경 필요

리팩토링 완료. 다음 단계:
  1. 영향 모듈 Dependencies 업데이트
  2. /compile --all --conflict overwrite
  3. /validate
```

**다음 단계:** `/compile --all --conflict overwrite` → 전체 재컴파일

---

#### `/migrate` — 버전 업그레이드 마이그레이션

**언제 사용하나요?**
- claude-md-plugin 버전을 업그레이드한 후, 기존 CLAUDE.md를 새 스키마에 맞게 수정해야 할 때
- 특히 v4.0 (MAJOR) 업그레이드 시 필수 — 3개 신규 섹션 자동 추가

**사용법:**
```bash
/migrate
```

**실행 흐름:**
1. CLI로 스키마 진단 → 마이그레이션 필요 파일 식별
2. 사용자 승인 → `fix-schema`로 누락 섹션 자동 추가
3. 재검증 → (선택) /validate → (선택) /compile --all

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

1. `/impl "JWT 인증 모듈이 필요합니다"` — 요구사항을 CLAUDE.md로 변환
2. `/compile` — 명세 기반 코드 + 테스트 자동 생성
3. `/validate` — 생성된 코드와 문서 일치 확인

#### B. 레거시 코드 문서화

```
/decompile → /validate → (위반 수정) → /validate
```

1. `/decompile` — 기존 코드에서 계약(CLAUDE.md) 추출
2. `/validate` — 코드가 추출된 계약을 만족하는지 확인
3. 위반이 있으면 `/compile` 재실행 또는 계약 수동 수정
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
/bugfix --error "에러" → (자동 /compile) → /validate
```

1. `/bugfix --error "TypeError: ..."` — 3계층 추적으로 근본 원인 진단
   - L3 root cause (대부분) → 자동으로 `/compile` 재실행하여 코드 재생성
   - L1 root cause → 사용자 승인 후 계약 수정 → `/compile` 재실행
2. `/validate` — 수정 후 계약-코드 일치 확인

#### E. 계약 변경 영향 분석

```
(CLAUDE.md 수정) → /diff-spec → /impact → /compile (영향 모듈) → /validate
```

1. CLAUDE.md를 직접 수정하거나 `/impl`로 업데이트
2. `/diff-spec src/auth` — 어떤 계약 조항이 변경되었는지 확인
3. `/impact src/auth` — 영향받는 모듈 식별
4. `/compile --path src/api --conflict overwrite` — 영향받는 모듈 재컴파일
5. `/validate` — 전체 검증

#### F. 명세 품질 리뷰

```
/impl-review → (수정 적용) → /compile
```

1. `/impl-review src/auth` — 3차원 품질 리뷰 수행
2. 수정 제안 적용 — 대화형으로 CLAUDE.md 수정
3. `/compile` — 수정된 명세로 코드 재생성

## 핵심 개념

### 문서 체계

소스코드가 유일한 Source of Truth이며, CLAUDE.md는 코드가 만족해야 할 계약입니다.

```
auth/
├── CLAUDE.md              ← Contract (코드가 만족해야 할 계약)
│   ├── Exports: validateToken(token: string): Claims
│   └── Domain Context: 토큰 만료 7일 (PCI-DSS)
│
├── DEVELOPERS.md          ← WHY (파일 관계, 결정 근거, 운영 정보)
│
└── .claude/tmp/
    └── compile-context-{hash}.md  ← /impl → /compile 핸드오프용 세션 임시 파일 (optional)
```

**명령어별 업데이트 범위:**

| 명령어 | CLAUDE.md | Source Code | compile-context |
|--------|-----------|-------------|-----------------|
| `/impl` | 생성/업데이트 | - | 생성 (세션 스코프) |
| `/compile` | 읽기 전용 | 생성 | 읽기 전용 (있으면 참조) |
| `/decompile` | 생성 (전체) | 읽기 전용 | - |
| `/validate` | 읽기 전용 | 읽기 전용 | - |
| `/bugfix` | 사용자 승인 후 수정 | 재생성 (/compile) | - |

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
  - 필수: Language & Runtime, Coding Rules, Naming Rules

멀티 모듈 프로젝트에서는 module_root가 project_root의 Convention을 override할 수 있습니다.

**컨벤션 우선순위:**
1. module_root `## Code Convention` → 코딩 규칙
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

## Contract
- Preconditions, Postconditions, Throws, Invariants

## Async Contract
- Execution Order, Cancellation, Backpressure, Timeout

## Error Taxonomy
- Error Hierarchy, Recovery Strategy, Propagation

## Concurrency Model
- Thread Safety, Shared State, Synchronization

## Constraints
- 제약사항

## Project Convention (project/module root만)
### Project Structure
### Module Boundaries
### Naming Conventions

## Code Convention (module root만)
### Language & Runtime
### Coding Rules
### Naming Rules
```

## 아키텍처

### Agents

| Agent | 역할 |
|-------|------|
| `impl` | 요구사항 분석 및 CLAUDE.md 생성 |
| `dep-explorer` | 의존성 탐색 (requirement 모드: 새 모듈 의존성, module 모드: 기존 모듈 의존자) |
| `decompiler` | 소스코드에서 CLAUDE.md 추출 |
| `compiler` | CLAUDE.md에서 소스코드 생성 (TDD) |
| `debug-layer-analyzer` | 단일 계층(L1/L2/L3) 진단 분석 (debugger의 sub-agent) |
| `debugger` | 소스코드 런타임 버그 → 3계층 추적 → 수정 (orchestrator) |
| `impl-reviewer` | CLAUDE.md 품질 리뷰 및 요구사항 커버리지 검증 |
| `validator` | CLAUDE.md-코드 일치 검증 (Structure, Exports, Dependencies, Behavior) + Export 커버리지 |
| `issue-verifier` | 검증 이슈 재검증 (false positive 필터링) |
| `violation-reporter` | 확인된 이슈 기반 계약 위반 보고 (CLAUDE.md 수정 안 함) |

### Skills

**Entry Point (사용자 진입점):**

| Skill | 역할 |
|-------|------|
| `/impl` | 요구사항 → CLAUDE.md |
| `/decompile` | 소스코드 → CLAUDE.md |
| `/compile` | CLAUDE.md → 소스코드 |
| `/validate` | 문서-코드 일치 검증 |
| `/bugfix` | 소스코드 런타임 버그 → 3계층 추적 → 수정 |
| `/impl-review` | CLAUDE.md 품질 리뷰 |
| `/impact` | 계약 변경 → 영향받는 모듈 분석 |
| `/diff-spec` | 계약 버전 간 시맨틱 diff |
| `/status` | 프로젝트 건강도 대시보드 |
| `/refactor` | 모듈 분할/병합 (계약 수준 리팩토링) |

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
