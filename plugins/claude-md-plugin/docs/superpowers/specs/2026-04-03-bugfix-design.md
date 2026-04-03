# `/bugfix` Design

## Problem

When a user reports a bug in a document-driven project, the root cause can exist at three distinct layers:
1. **Requirements Layer** (CLAUDE.md) — the requirement itself is wrong or missing
2. **Constraints Layer** (DEVELOPERS.md) — the constraint/contract is incomplete or incorrect
3. **Implementation Layer** (Source Code) — the code fails to implement a correct spec

Without 3-layer tracing, fixes are applied only at the code level, leaving CLAUDE.md (the SSOT) inconsistent with reality. This violates the document-first invariant and causes spec drift.

## Solution

A `/bugfix` skill that:
1. Collects bug context (expected vs actual behavior)
2. Traces through all 3 layers to determine root cause
3. Makes autonomous fixes only when judgment is **unambiguous**
4. Escalates to the user with structured context when **any ambiguity** exists
5. Fixes at the highest affected layer, letting lower layers be derived

## Core Invariant

```
INV-bugfix-1: Conflict Resolution
  Code always defers to CLAUDE.md.
  - CLAUDE.md correct → fix code (Layer 3)
  - CLAUDE.md incorrect → fix CLAUDE.md first (Layer 1), then regenerate code
  Never patch code while leaving CLAUDE.md inconsistent.

INV-bugfix-2: Ambiguity Escalation
  Layer 3 (code): autonomous fix when judgment is unambiguous (evidence-based).
  Layer 1/2 (SSOT documents): always require user approval before modification,
    even when judgment is unambiguous. Modifying the SSOT has broader impact than a code fix.
  Any degree of ambiguity (any layer) → escalate to user with structured context.
```

## Architecture

```
User: /bugfix "description" [--path] [--error] [--file]
       │
       ▼
┌─────────────────────────────────────────────┐
│ bugfix SKILL                                │
│                                             │
│ 1. Bug context 수집                          │
│ 2. CLAUDE.md 선정 + DEVELOPERS.md +          │
│    소스 파일 + diff-spec-range 수집            │
│    (선정 로직은 "SKILL Step 2" 참고)            │
│ 3. bugfix-session.md 생성                    │
│ 4. Task(bugfixer agent) dispatch            │
│ 5. Result 기반 fix 실행:                      │
│    not_a_bug → 사용자에게 알림 후 종료           │
│    unambiguous L1 → AskUserQuestion 승인     │
│                   → CLAUDE.md 직접 수정      │
│                   → spec commit → /dev      │
│    unambiguous L2 → AskUserQuestion 승인     │
│                   → DEVELOPERS.md 수정 → /dev│
│    unambiguous L3 → result block의           │
│                   test_result 확인 (agent    │
│                   가 이미 fix 완결)            │
│    multi → L1→L2→L3 순서로 위 분기 순차 실행   │
│    ambiguous → AskUserQuestion (escalation) │
│               → 사용자 선택 기반 fix            │
│ 6. bugfix commit (L3 fix 포함 시에만)          │
└─────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────┐
│ bugfixer AGENT                              │
│ ⚡ Skill("superpowers:systematic-debugging") │
│                                             │
│ 1. Bug report 파싱: E (expected), A (actual) │
│ 2. Layer 1 분석: CLAUDE.md Requirements 탐색 │
│ 3. Layer 2 분석: DEVELOPERS.md Constraints   │
│ 4. Layer 3 분석: 코드 root cause 추적         │
│ 5. git evidence: diff-spec-range, log 분석   │
│ 6. 자명/모호 판단 → result block 반환          │
│    (L3 자명: 실패 테스트 작성 → 코드 수정 → 검증) │
└─────────────────────────────────────────────┘
```

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `description` | Yes | — | 버그 설명 (expected vs actual) |
| `--path` | No | `.` | 대상 경로 |
| `--error` | No | — | 에러 메시지 / 스택 트레이스 |
| `--file` | No | — | 버그가 있는 특정 파일 |

## 3-Layer Tracing

### Layer 1: Requirements (CLAUDE.md)
CLAUDE.md의 Requirements 섹션에서 버그 관련 Requirement를 탐색한다.
- E와 일치하는 Requirement가 있는가?
- 해당 Requirement가 구체적인가 (모호하지 않은가)?

### Layer 2: Constraints (DEVELOPERS.md)
DEVELOPERS.md의 Constraints 섹션에서 관련 constraint를 탐색한다.
- 이 동작에 대한 constraint가 존재하는가?
- constraint가 E를 보장하는가?

### Layer 3: Implementation (Source Code)
systematic-debugging의 Phase 1–3을 적용해 코드 레벨 root cause를 추적한다.
- 에러 메시지 / 스택 트레이스 분석
- 관련 코드 경로 추적
- 테스트 커버리지 확인

## Judgment Algorithm

```
입력:
  E = 사용자가 기대하는 동작
  A = 현재 코드의 실제 동작
  S = CLAUDE.md가 명시한 동작 (없으면 null)

diff-spec-range CLI 결과 필드 매핑:
  changed_requirements not empty          → spec이 변경됨 (last dev commit 이후)
  source_changed=true + changed_requirements empty → 소스 변경, spec 변경 없음
  all_requirements=true                   → git 미사용 또는 첫 커밋 (전체 Requirements 검증)

자명한 케이스:
  ✓ E == A
      → not_a_bug (버그 없음 또는 이미 수정됨)
  ✓ E == S AND A != S
      → 코드가 틀림 → Layer 3 fix (autonomous)
  ✓ changed_requirements not empty AND source_changed=false
      → 코드 스테일 (spec 변경이 dev에 반영 안 됨) → /dev 재실행
  ✓ source_changed=true AND changed_requirements empty AND A != S
      → 코드가 CLAUDE.md에서 이탈 → Layer 3 fix (autonomous)

모호한 케이스 (escalate to user):
  - E 자체가 불명확
  - S == null (CLAUDE.md 누락)
  - E != S AND S가 명시적으로 존재
  - git 증거 불충분 (all_requirements=true)
  - 여러 Requirement가 충돌
  - E != S AND A == S (코드는 스펙 준수, 사용자 기대가 스펙과 다름)
```

## Escalation Format

모호한 경우, bugfixer agent가 SKILL에 ambiguous 결과를 반환하고,
SKILL이 다음 포맷으로 AskUserQuestion을 통해 사용자에게 판단을 요청한다:

```
판단이 필요합니다.

## 현재 상황
- 사용자 기대 (E): "{expected}"
- 현재 동작 (A): "{actual}"
- CLAUDE.md REQ-N: "{spec text}"  ← 관련 Requirement 직접 인용
  (또는: "이 동작에 대한 Requirement 없음")

## 판단 근거가 모호한 이유
"{구체적 이유}"

## 선택지
A) 스펙과 코드 모두 E에 맞게 수정한다
   → 실행 순서: CLAUDE.md REQ-N 먼저 수정 → spec commit → /dev로 코드 재생성
   (Fix-Highest-Layer-First: 코드는 SSOT 수정 이후 derived됨)
B) 스펙을 수정한다 (E를 요구사항으로 추가/변경)
   → CLAUDE.md에 신규 Requirement 추가 → spec commit → /dev 재생성
C) 현재 동작(A)이 올바름 (버그 아님)
   → 버그 리포트 종료

어떻게 처리할까요?
```

## Fix-Highest-Layer-First 원칙

Layer 1이 틀렸으면 Layer 1을 먼저 수정한다.
Layer 1 수정 → /dev → 코드가 자동으로 수렴된다.
코드만 수정하고 CLAUDE.md를 inconsistent 상태로 남기지 않는다.

```
Fix path per layer:
  L1 (CLAUDE.md 잘못됨)    → CLAUDE.md 직접 수정 → spec commit → /dev 재생성
  L2 (DEVELOPERS.md 누락)  → DEVELOPERS.md 직접 수정 → /dev 재생성
  L3 (코드만 잘못됨)        → 실패 테스트 작성 → 코드 수정 → 검증
  multi-layer              → L1 → L2 → L3 순서로 처리
                             L1 fix 후 /dev 재생성 시 L3도 함께 수렴될 수 있음
                             /dev 재생성 후에도 L3 이슈가 남으면 별도 Layer 3 fix
```

Layer 2 fix에 /spec 대신 직접 수정을 사용하는 이유:
DEVELOPERS.md는 개발자가 직접 작성하는 Derived Spec이다.
Constraints 갭을 채우는 targeted patch가 /spec full workflow보다 bugfix 맥락에서 더 정확하다.

## SKILL Step 2: Context 수집 로직

### CLAUDE.md 선정

```
1. --file 제공:
   --file 경로에서 상위로 탐색하며 첫 번째 CLAUDE.md 발견 → 해당 파일 선택
   예) --file=src/auth/login.ts → src/auth/CLAUDE.md 탐색 → 없으면 src/CLAUDE.md → ...

2. --file 미제공, --path 제공:
   scan-claude-md --root {path} 결과에서 target 디렉토리의 CLAUDE.md 선택
   (path 내 최상위 CLAUDE.md)

3. Conventions: 프로젝트 루트 CLAUDE.md에서 Conventions 섹션을 추가로 포함 (계층 상속)
```

### 소스 파일 선정

```
1. --file 제공: 해당 파일 + 같은 디렉토리의 소스 파일 목록
2. --file 미제공:
   선정된 CLAUDE.md의 디렉토리 내 소스 파일 목록 (확장자 기반 언어 감지)
   파일 수가 많으면 (>10) 목록만 포함, 내용은 agent가 필요 시 직접 Read
```

### diff-spec-range 실행

```bash
$CLI_PATH diff-spec-range --file {selected_CLAUDE.md_path} --root {project_root} \
  --output "${TMP_DIR}spec-diff-{dir-safe}.json"
```

## Session File Schema

```markdown
# Bugfix Session
type: bugfix | path: {path}

## Bug Description
{user description — expected vs actual}

## Error Message
{stack trace or error, if provided}

## Target File
{specific file, if provided}

## Layer 1: Requirements (CLAUDE.md)
{Purpose, Requirements, Domain Context}

## Layer 2: Constraints (DEVELOPERS.md)
{Constraints, Technical Context}

## Layer 3: Source Files
{관련 소스 파일 목록 및 내용}

## Recent Spec Changes
{diff-spec-range 출력}

## Conventions
{계층 resolved Conventions}
```

## Result Block

```
---bugfix-result---
status: fixed | escalated | not_a_bug | failed
root_cause_layer: 1 | 2 | 3 | multi | unknown
judgment: unambiguous | ambiguous
fix_type: spec_update | constraints_update | code_fix | none
fix_description: {what was fixed or proposed}
test_result: passed | skipped | failed (Layer 3 only)
---end-bugfix-result---
```

## Commit Message

**L3-only fix** (코드만 수정):
```
bugfix({path}): {one-line summary}

Root cause: Layer 3 — {brief description}

Changes:
- {list of changed files}
```

**L1 fix** (CLAUDE.md 수정 + /dev 재생성):
```
spec({path}): fix requirement — {summary}
dev({path}): regenerate after spec fix — {summary}
```
별도 bugfix commit 없음. spec + dev commit으로 변경이 완결된다.

**L2 fix** (DEVELOPERS.md 수정 + /dev 재생성):
```
dev({path}): fix constraint and regenerate — {summary}
```
별도 bugfix commit 없음.

**multi-layer fix** (예: L1 + L3):
```
spec({path}): fix requirement — {summary}
dev({path}): regenerate after spec fix — {summary}
```
/dev 재생성 후에도 L3 이슈가 남을 경우:
```
bugfix({path}): fix residual implementation issue — {summary}
```

## Error Handling

| Situation | Response |
|-----------|----------|
| E가 버그 리포트에서 불명확 | AskUserQuestion: expected 동작 구체화 요청 |
| bugfixer agent 실패 | warn, escalate to user |
| /dev 재생성 실패 | report, exit with status=failed |
| Layer 3 테스트 작성 불가 | systematic-debugging Phase 4 설계 피드백 신호로 처리 |
| 3회 이상 fix 시도 실패 | systematic-debugging: 아키텍처 문제 의심, 사용자 상담 |

## Agent Composition

| Agent | Superpowers | Role |
|-------|-------------|------|
| `bugfixer` | systematic-debugging | 3-layer root cause 분석 + Layer 3 코드 fix |

### bugfixer agent 필요 tools

```
tools: [Bash, Read, Glob, Grep, Edit, Write]

- Bash: git log, diff-spec-range, 테스트 실행
- Read/Glob/Grep: 소스 파일 탐색 및 읽기
- Edit: Layer 3 코드 수정
- Write: 실패 테스트 파일 작성
```

## Relationship to Other Skills

| Skill | Difference |
|-------|------------|
| `/validate` | 전체 drift 탐지 (사후 검증). bugfix는 특정 버그의 root cause 추적 + fix |
| `/dev` | bugfix의 L1/L2 fix 후 코드 재생성에 내부적으로 활용 |
| `/spec` | bugfix는 /spec을 호출하지 않음. L1 fix는 CLAUDE.md를 직접 수정. /spec의 full workflow(brainstorming, AskUserQuestion)는 bugfix 맥락에서 오버킬. |
