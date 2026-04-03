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
  Autonomous fix only when judgment is unambiguous (evidence-based).
  Any degree of ambiguity → escalate to user with structured context.
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
│ 2. 관련 CLAUDE.md + DEVELOPERS.md +          │
│    소스 파일 + diff-spec-range 수집            │
│ 3. bugfix-session.md 생성                    │
│ 4. Task(bugfixer agent) dispatch            │
│ 5. Result 기반 fix 실행:                      │
│    unambiguous L1 → AskUserQuestion 승인     │
│                   → CLAUDE.md 직접 수정      │
│                   → spec commit → /dev      │
│    unambiguous L2 → AskUserQuestion 승인     │
│                   → DEVELOPERS.md 수정 → /dev│
│    unambiguous L3 → 테스트 통과 확인           │
│    ambiguous     → AskUserQuestion (escalation) │
│                   → 사용자 선택 기반 fix        │
│ 6. bugfix commit                            │
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

자명한 케이스 (autonomous fix):
  ✓ E == S AND A != S
      → 코드가 틀림 → Layer 3 fix
  ✓ spec commit > last dev commit (diff-spec-range)
      → 코드 스테일 → /dev 재실행
  ✓ last dev commit 이후 소스 변경 + spec commit 없음 AND A != S
      → 코드가 CLAUDE.md에서 이탈 → Layer 3 fix

모호한 케이스 (escalate to user):
  - E 자체가 불명확
  - S == null (CLAUDE.md 누락)
  - E != S AND S가 명시적으로 존재
  - git 증거 불충분 (최근 변경 없음)
  - 여러 Requirement가 충돌
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
A) 코드를 수정한다 (E에 맞게)
   → CLAUDE.md REQ-N도 E를 반영하도록 업데이트 필요
B) 스펙을 수정한다 (E를 요구사항으로 추가/변경)
   → /spec → /dev 재생성
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

```
bugfix({path}): {one-line summary}

Root cause: Layer {N} — {brief description}

Changes:
- {list of changed files with description}
```

Layer 1/2 fix가 포함된 경우 spec commit과 dev commit이 분리된다:
```
spec({path}): fix requirement — {summary}
dev({path}): regenerate after spec fix
bugfix({path}): {summary}
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

## Relationship to Other Skills

| Skill | Difference |
|-------|------------|
| `/validate` | 전체 drift 탐지 (사후 검증). bugfix는 특정 버그의 root cause 추적 + fix |
| `/dev` | bugfix의 L1/L2 fix 후 코드 재생성에 내부적으로 활용 |
| `/spec` | bugfix는 /spec을 호출하지 않음. L1 fix는 CLAUDE.md를 직접 수정. /spec의 full workflow(brainstorming, AskUserQuestion)는 bugfix 맥락에서 오버킬. |
