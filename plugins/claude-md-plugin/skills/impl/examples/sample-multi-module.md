# Sample: 멀티 모듈 요구사항 분해

## 입력

```
사용자 요구사항: "결제 시스템이 필요합니다: 카드 결제, 정산, 환불"
프로젝트 루트: /Users/dev/my-app
```

## Phase 0: Scope Assessment

```
---scope-assessment---
completeness: medium
scope: multi-module
evidence:
  D1_purpose: 있음 — "결제 시스템" (명확한 도메인)
  D2_requirements: 추론 가능 — "카드 결제, 정산, 환불" (구체적 요구사항 없음)
  D3_domain_context: 없음 — 결정 근거/배경 미언급
next_phase: Phase 2 Tier 2
---end-scope-assessment---
```

- **감지 신호**: 나열형 "카드 결제, 정산, 환불" — 각각 독립된 책임 보유 가능

### AskUserQuestion: 멀티 모듈 처리 방법

질문: "여러 도메인이 감지되었습니다 (카드 결제, 정산, 환불). 어떻게 진행할까요?"

옵션:
1. **모듈별 분해 (권장)** — 첫 모듈만 생성, 나머지는 /impl 가이드 제공
2. **도메인 그룹 생성** — Purpose로 하위 모듈을 참조하는 상위 CLAUDE.md 생성
3. 단일 모듈 유지 — 모든 기능을 하나의 CLAUDE.md에

사용자 선택: **모듈별 분해**

### AskUserQuestion: 첫 모듈 선택

질문: "어떤 모듈을 먼저 생성할까요?"

옵션:
1. 카드 결제 (payment)
2. 정산 (settlement)
3. 환불 (refund)

사용자 선택: **카드 결제 (payment)**

## Phase 1~6: 카드 결제 모듈 처리

### Phase 1: Requirements Analysis
- Purpose: 카드 결제 처리
- Requirements: 추론 가능 (구체적 요구사항 미언급)

### Phase 1.5: dep-explorer
- Internal: 0개
- External: 1개 existing (payments-sdk)

### Phase 2: Tiered Clarification (completeness=medium → Round 1 건너뛰기)

#### Round 2 — Tier 2 + Tier 3

AskUserQuestion (3개):

1. **REQUIREMENTS** (Tier 2): "카드 결제에서 어떤 요구사항/규칙이 있나요?"
   - 사용자 답변: 결제 성공 시 트랜잭션 ID 반환, 잔액부족/카드만료/한도초과 시 에러

2. **REQUIREMENTS 추가** (Tier 2): "결제 취소/상태 조회 기능도 필요한가요?"
   - 사용자 답변: 예, cancelPayment + getPaymentStatus 필요

3. **DOMAIN_CONTEXT** (Tier 3): "결제 타임아웃 기준이 있나요?"
   - 사용자 답변: 30초 타임아웃, PG사 API 호출

### Phase 3~6
- Target path: `src/payment`
- CLAUDE.md 생성

## Phase 6.5: Plan Preview

```
=== 생성 계획 ===

대상 경로: src/payment
액션: created

Purpose: 카드 결제 처리 모듈
Requirements: 6개 — 결제 성공 시 트랜잭션 ID 반환, 잔액부족 에러, 카드만료 에러, ...
Domain Context: 있음 (30초 타임아웃, PG사 API)
Dependencies: Internal 0개, External 1개 (payments-sdk)
```

사용자 선택: **승인**

## Phase 7: 최종 결과

```
---impl-result---
claude_md_file: src/payment/CLAUDE.md
developers_md_file: src/payment/DEVELOPERS.md
compile_context_file: .claude/tmp/compile-context-src-payment.md
status: success
action: created
validation: passed
requirements_count: 6
domain_context: present
dependencies_count: 1
tech_choices_count: 1
---end-impl-result---
```

## 나머지 모듈 /impl 가이드

최종 결과에 포함되는 안내:

```
=== 나머지 모듈 생성 가이드 ===

다음 명령어로 나머지 모듈을 생성할 수 있습니다:

1. /impl "정산 모듈: 카드 결제 내역을 기반으로 가맹점 정산 처리"
2. /impl "환불 모듈: 결제 건에 대한 전액/부분 환불 처리"

참고: 정산/환불 모듈은 src/payment/CLAUDE.md를 의존성으로 참조할 수 있습니다.
```
