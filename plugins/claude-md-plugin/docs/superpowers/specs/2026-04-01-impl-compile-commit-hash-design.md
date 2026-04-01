# impl → compile Commit Hash Handoff

## Problem

현재 compile은 **현재 스펙의 전체 상태**만 알고, **무엇이 변경되었는가**는 모른다.

- `diff-compile-targets`는 파일이 변경되었다는 사실만 감지 (git status + timestamp)
- compiler agent는 전체 CLAUDE.md + DEVELOPERS.md를 받아 코드를 생성
- **변경 의도(delta)**가 없으므로 전체 재생성 또는 맹목적 overwrite

특히 **Requirement 삭제**를 감지할 수 없다 — 현재 메커니즘의 가장 큰 결함.

## Core Insight

```
impl = PM/PO (스펙 변경자, producer)
compile = Developer (스펙 적용자, consumer)
```

impl이 문서를 변경하고 커밋하면, compile은 그 커밋의 diff를 통해 **정확히 무엇이 바뀌었는지** 알 수 있다.

**인터페이스는 커밋 메시지 컨벤션.** 별도 상태 파일 없이, git log가 곧 상태.

## Design

### 1. 커밋 메시지 컨벤션

모든 SKILL이 커밋 시 동일한 포맷을 따른다:

```
<command>(<target_path>): [BREAKING] <one-line summary>

<전환 맥락 — 어디서 어디로, 왜 이 변경을 하는가 (1-2문장)>

Changes:
- added: <requirement/constraint 목록>
- modified: <requirement/constraint 목록>
- removed: <requirement/constraint 목록>
```

#### 커밋 메시지 요소별 역할

| 요소 | 역할 | 소비자 |
|------|------|--------|
| `impl(<path>):` | machine discovery (grep) | compile SKILL |
| `[BREAKING]` (선택) | 기존 코드 대량 변경 경고, conflict 전략 결정 신호 | compile SKILL |
| one-line summary | human orientation | 개발자, git log |
| 전환 맥락 | **diff에 없는 유일한 정보** — 변경의 방향과 이유 | compiler agent (Phase 0) |
| Changes | diff의 요약 인덱스 — 파싱 없이 빠른 판단 | compile SKILL (탐색 단계) |

#### 전환 맥락이 필요한 이유

문서(CLAUDE.md)는 **현재 상태**를 기술한다. **전환(transition)**은 기술하지 않는다.

예: Requirements에 "OAuth2 + Session 병행 인증 지원"이라고만 적혀 있으면,
처음부터 이랬는지, Session에서 OAuth2를 추가한 건지, OAuth2에서 Session을 추가한 건지 알 수 없다.

전환 맥락은 compiler agent에게 기존 코드 처리 방식을 알려주는 핵심 정보:

| 시나리오 | 전환 맥락 없이 | 전환 맥락 있으면 |
|---------|-------------|--------------|
| 기능 추가 (순수 신규) | 현재 스펙 보고 생성 → OK | "신규 추가" 확인 → 동일 |
| 기능 수정 (기존 변경) | 전체 재생성 → 기존 코드 불필요 변경 위험 | "X를 Y로 전환, Z 유지" → 정확한 수정 |
| 기능 삭제 | 감지 불가 | "X 제거" → 코드 삭제 |
| 방향 전환 | 현재 스펙만 보면 신규와 구분 불가 | "A에서 B로 전환" → 기존 A 코드 정리 + B 생성 |

#### 커밋 메시지에 포함하지 않는 정보

| 정보 | 이유 | 올바른 위치 |
|------|------|-----------|
| 비즈니스 맥락 상세 | 문서에 있음 | CLAUDE.md Domain Context |
| 구현 가이드 | 문서에 있음 | DEVELOPERS.md Technical Context |
| cross-module 영향 | impl-to-impl 관심사 | impl이 해당 모듈도 함께 수정 |
| 우선순위 | 문서에 있음 | CLAUDE.md Requirements 순서 |

#### 예시

```
impl(src/auth): OAuth2 인증 추가

session 기반 인증에 OAuth2를 추가 경로로 도입.
레거시 클라이언트 지원을 위해 기존 session 인증은 유지.

Changes:
- added: REQ OAuth2 인증 (Google, GitHub), C-7 token refresh 60분 이내
- modified: REQ 인증 방식 → session + OAuth2 병행
```

```
impl(src/auth): [BREAKING] session 인증 제거

OAuth2 전면 전환 완료. session 인증 코드 및 관련 미들웨어 전체 제거 대상.

Changes:
- removed: REQ session 인증, C-1 session timeout, C-2 cookie 보안
- modified: REQ 인증 방식 → OAuth2 전용
```

#### SKILL별 prefix

| prefix | 생성자 | 예시 |
|--------|--------|------|
| `impl` | impl SKILL | `impl(src/auth): 사용자 인증 요구사항 정의` |
| `compile` | compile SKILL | `compile(src/auth): 인증 모듈 코드 생성` |
| `validate` | validate SKILL | `validate(src/auth): drift 검증 통과` |
| `decompile` | decompile SKILL | `decompile(src/auth): 기존 코드에서 스펙 추출` |

### 2. compile의 impl 커밋 탐색 (default 동작)

```
compile SKILL (대상 디렉토리별)
  │
  ├── Step 1: 마지막 compile 커밋 찾기
  │   git log -1 --format="%H" --grep="^compile({path}):"
  │
  ├── Step 2: 그 이후의 impl 커밋 찾기
  │   git log --format="%H" --grep="^impl({path}):" {last_compile}..HEAD
  │   (last_compile 없으면 전체 히스토리)
  │
  ├── Step 3-a: impl 커밋 발견
  │   │  각 impl 커밋의 diff 합산:
  │   │  git diff {hash}~1..{hash} -- {path}/CLAUDE.md {path}/DEVELOPERS.md
  │   │  + 커밋 메시지에서 전환 맥락 추출
  │   │  → session file에 "## Spec Changes" 섹션 포함
  │   └── incremental compile
  │
  └── Step 3-b: impl 커밋 미발견
      └── 기존 diff-compile-targets fallback (전체 compile)
```

### 3. Spec Changes 섹션 (compile session file)

compile SKILL이 impl 커밋에서 추출하여 session file에 포함하는 섹션:

```markdown
## Spec Changes (since compile(src/auth) @ abc1234)

### Transition Context
session 기반 인증에 OAuth2를 추가 경로로 도입.
레거시 클라이언트 지원을 위해 기존 session 인증은 유지.

### Added
- REQ: OAuth2 인증 (Google, GitHub)
- C-7: token refresh 60분 이내

### Modified
- REQ: 인증 방식 (session 전용 → session + OAuth2 병행)
- C-2: timeout 30s → 60s

### Removed
- REQ: 레거시 CSV 내보내기
```

정보 소스:
- Transition Context: 커밋 메시지 body (전환 맥락)
- Added/Modified/Removed: 커밋 메시지 Changes 섹션 (인덱스) + `parse-claude-md` before/after 비교 (검증)

### 4. compiler agent: 2-Phase 구조

#### Phase 0: Task Definition (신규)

Spec Changes + Transition Context + 현재 소스코드 구조를 분석하여 구현 태스크를 정의.

**TDD 진입 전에 "무엇을 해야 하는가"를 명시적으로 결정하는 단계.**

```
Phase 0: Task Definition
  │
  ├── 입력:
  │   ├── Spec Changes (added/modified/removed)
  │   ├── Transition Context (커밋 메시지)
  │   └── 현재 소스코드 구조 (파일/함수 탐색)
  │
  ├── 출력: Implementation Tasks
  │   ├── [ADD] 새 파일/함수 생성 — target, approach
  │   ├── [MODIFY] 기존 코드 수정 — target (기존 파일 위치), approach
  │   ├── [DELETE] 코드 제거 — target, 참조 정리 범위
  │   └── (없음) → "할 일 없음" → compile 조기 종료
  │
  └── 특수 판단:
      ├── BREAKING → --conflict overwrite 강제
      ├── Constraint만 변경 → "기능 코드 미변경" 명시
      └── 의미적 변경 없음 → compile 건너뛰기
```

Task Definition이 가치를 발휘하는 시나리오:

| 시나리오 | Task Definition 없이 | Task Definition 있으면 |
|---------|---------------------|---------------------|
| 기존 기능 수정 | 전체 재생성 위험 | 기존 코드 위치 특정 후 수정 |
| 기능 삭제 | 삭제 대상 누락 위험 | 삭제 목록 + 참조 정리 명시 |
| 복합 변경 (추가+수정+삭제) | 순서 혼란 | 실행 순서 결정 (삭제→인터페이스→구현→수정) |
| Constraint만 변경 | 불필요한 기능 코드 변경 | "기능 코드 미변경" 명시 |
| 의미적 변경 없는 diff | 불필요한 재생성 | "할 일 없음" → 건너뛰기 |

#### Phase 1: Task별 TDD (기존 + 개선)

Task Definition에서 도출된 태스크를 순서대로 TDD 사이클 실행.

```
Phase 1: Task별 TDD
  │
  ├── [ADD] 태스크:
  │   ├── RED: 새 Constraint → 테스트 생성
  │   ├── GREEN: 구현
  │   └── REFACTOR: Conventions 적용
  │
  ├── [MODIFY] 태스크:
  │   ├── RED: 변경된 Constraint → 기존 테스트 수정/추가
  │   ├── GREEN: 기존 코드 수정
  │   └── REFACTOR: 회귀 테스트 확인
  │
  └── [DELETE] 태스크:
      ├── 대상 코드 제거
      ├── 참조 정리 (import, 호출부)
      └── 관련 테스트 제거/수정
```

#### Phase 간 안전망

Task Definition(Phase 0)이 부정확해도 TDD(Phase 1)가 안전망 역할:
- 잘못된 태스크 → 테스트 실패 → 재조정
- 누락된 태스크 → 컴파일 에러 또는 미충족 Constraint → 추가 발견
- 잘못된 순서 → 의존성 에러 → 순서 재조정

### 5. Superpowers 조합

#### 현재 (v10)

| Agent | Superpowers | 역할 |
|-------|------------|------|
| impl | brainstorming (single 모드) | 요구사항 탐색/설계 |
| compiler | test-driven-development | Constraints → TDD |
| validator | verification-before-completion | 증거 기반 검증 |
| decompiler | (없음) | 추출 작업 |
| decompose | (없음) | 모듈 분해 |

#### 변경 후

| Agent | Phase | Superpowers | 역할 |
|-------|-------|------------|------|
| impl | - | brainstorming (single 모드) | 요구사항 탐색/설계. 커밋 메시지에 전환 맥락 + Changes 포함 |
| compiler | Phase 0 | writing-plans | Spec Changes + diff → Implementation Tasks 도출 |
| compiler | Phase 1 | test-driven-development | Task 단위 RED-GREEN-REFACTOR. **변경점: 전체 스펙 대상 → Task 단위 대상** |
| validator | - | verification-before-completion | 증거 기반 검증 (변경 없음) |
| decompiler | - | (없음) | 추출 작업 (변경 없음) |
| decompose | - | (없음) | 모듈 분해 (변경 없음) |

#### Phase 0에 writing-plans를 사용하는 이유

Phase 0(Task Definition)은 "스펙 변경(diff)을 받아 코드 작성 전에 구현 계획을 수립하는 것":
- 입력: Spec Changes (added/modified/removed) + Transition Context + 현재 소스코드 구조
- 출력: Implementation Tasks (순서 있는 구현 계획)
- writing-plans의 핵심 역할과 정확히 일치: **스펙이 있고, 코드 작성 전에 구현 전략을 세운다**

#### Phase 1에서 TDD 적용 범위 변경

현재:
```
TDD 대상 = 전체 Constraints → 전체 테스트 → 전체 구현
```

변경 후:
```
TDD 대상 = Task별 관련 Constraints → Task별 테스트 → Task별 구현
```

이것은 superpowers:test-driven-development 자체의 변경이 아니라,
**compiler agent가 TDD에 넘기는 scope가 달라지는 것.**

### 6. 전체 흐름 (변경 후)

```
impl SKILL
  │
  ├── 요구사항 분석 + 문서 생성 (기존)
  ├── ⚡ superpowers:brainstorming (single 모드)
  ├── CLAUDE.md + DEVELOPERS.md 커밋
  │   └── 커밋 메시지: impl(path): summary + 전환 맥락 + Changes
  │
  ▼
compile SKILL
  │
  ├── Step 1: git log --grep → 마지막 compile 커밋
  ├── Step 2: git log --grep → 이후 impl 커밋 탐색
  ├── Step 3: impl 커밋 diff 추출 + 커밋 메시지 파싱
  ├── Session file 생성:
  │   ├── 전체 스펙 (기존)
  │   ├── + ## Spec Changes (diff 기반)
  │   └── + ## Transition Context (커밋 메시지 기반)
  │
  └── compiler AGENT
        │
        ├── Phase 0: Task Definition
        │   ├── ⚡ superpowers:writing-plans
        │   ├── Spec Changes + diff 분석
        │   ├── 현재 소스코드 탐색
        │   └── Implementation Tasks 도출
        │       └── "할 일 없음" → 조기 종료
        │
        ├── Phase 1: Task별 TDD
        │   ├── ⚡ superpowers:test-driven-development
        │   ├── Task 순서대로 RED-GREEN-REFACTOR
        │   └── DELETE 태스크: 코드 제거 + 참조 정리
        │
        └── 결과 반환
  │
  ├── compile(path): summary 커밋
  └── git diff --stat
  │
  ▼
validate SKILL (선택)
  │
  ├── ⚡ superpowers:verification-before-completion
  └── 문서-코드 일치 검증 (변경 없음)
```

### 7. Priority 및 Override

```
1. --since {hash} 명시 → discovery 생략, 직접 사용
2. impl 커밋 자동 탐색 → 발견되면 incremental (Phase 0 + Phase 1)
3. fallback → 기존 diff-compile-targets (전체 compile, Phase 0 생략)
```

### 8. 수동 수정 처리

impl 커밋만 grep하므로 수동 수정(PR 리뷰, 직접 편집)은 무시됨.
이것이 올바른 동작 — compile은 impl의 변경만 반영하는 것이 역할.
수동 수정은 `diff-compile-targets`의 영역.

```
compile(src/auth)  ← 이 시점 이후의 impl만 탐색
    │
    ├── PR 리뷰 수정 (수동) ← impl 아님, grep에 안 잡힘
    ├── impl(src/auth)      ← 잡힘 ✓
    └── impl(src/auth)      ← 잡힘 ✓
```

## Edge Cases

| 케이스 | 동작 |
|--------|------|
| 첫 compile (compile 커밋 없음) | 전체 히스토리에서 impl 탐색, 없으면 전체 compile |
| impl 3회 → compile | 3개 impl 커밋의 diff 합산 + 전환 맥락 병합 |
| git rebase로 history 변경 | grep 결과 없음 → fallback |
| 다른 브랜치에서 impl | 현재 브랜치 git log에 없음 → fallback |
| --all 플래그 | hash 무시, 전체 compile (Phase 0 생략) |
| Phase 0에서 "할 일 없음" 판단 | compile 조기 종료, 불필요한 코드 생성 방지 |
| Phase 0 태스크 부정확 | Phase 1 TDD가 안전망 (테스트 실패 → 재조정) |

## 기존 메커니즘과의 관계

| 기존 메커니즘 | 역할 | commit hash와의 관계 |
|-------------|------|-------------------|
| `diff-compile-targets` | **어느 디렉토리**를 컴파일할지 | 보완: hash는 **무엇이 바뀌었는지** |
| `contract-hash` | 변경 여부 (skip 최적화) | 보완: hash가 더 정밀한 판단 |
| `spec_diff.rs` | validate에서 drift 검출 | diff 파싱 로직 재사용 가능 |

## 구현 범위

| 대상 | 변경 내용 |
|------|----------|
| **impl SKILL** | 커밋 메시지를 컨벤션 포맷으로 생성 (전환 맥락 + Changes) |
| **compile SKILL** | Step 1-3 탐색 로직 추가, session file에 Spec Changes + Transition Context 포함 |
| **compiler AGENT** | Phase 0 (Task Definition) 추가, Phase 1을 Task 단위 TDD로 전환 |
| **기타 SKILLs** | compile, validate, decompile도 동일 커밋 메시지 컨벤션 |
| **CLI (선택)** | `spec-diff` subcommand (parse-claude-md 기반 구조적 비교) |

## 설계 원칙

- **git이 곧 상태**: 별도 상태 파일 없음
- **컨벤션이 곧 프로토콜**: 커밋 메시지 포맷으로만 연결, 커플링 제로
- **문서는 상태, 커밋은 전환**: 각자의 역할이 다름
- **분석과 실행의 분리**: Phase 0 (무엇을) → Phase 1 (어떻게)
- **fallback 안전성**: 모든 edge case가 기존 동작으로 수렴
- **하위 호환**: 기존 워크플로우 영향 없음
