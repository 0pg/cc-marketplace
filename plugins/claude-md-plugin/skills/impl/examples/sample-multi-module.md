# Sample: 멀티 모듈 요구사항 분해

## 입력

```
사용자 요구사항: "결제 시스템이 필요합니다: 카드 결제, 정산, 환불"
프로젝트 루트: /Users/dev/my-app
```

## Phase 0: Scope Assessment

- **completeness**: medium (명확한 목적 + 모호한 인터페이스)
- **scope**: **multi-module** (3개 독립 도메인 감지)
  - 감지 신호: 나열형 "카드 결제, 정산, 환불" — 각각 독립된 Exports 보유 가능

### AskUserQuestion: 멀티 모듈 처리 방법

질문: "여러 도메인이 감지되었습니다 (카드 결제, 정산, 환불). 어떻게 진행할까요?"

옵션:
1. **모듈별 분해 (권장)** — 첫 모듈만 생성, 나머지는 /impl 가이드 제공
2. **도메인 그룹 생성** — Structure로 하위 모듈을 참조하는 상위 CLAUDE.md 생성
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
- Exports: processPayment, getPaymentStatus 등 추론

### Phase 1.5: dep-explorer
- Internal: 0개
- External: 1개 existing (payments-sdk)

### Phase 2: Tiered Clarification (completeness=medium → Round 1 건너뛰기)

#### Round 2 — Tier 2 + Tier 3

AskUserQuestion (3개):

1. **EXPORTS** (Tier 2): "카드 결제에서 어떤 함수를 export해야 하나요?"
   - 사용자 답변: processPayment, cancelPayment, getPaymentStatus

2. **BEHAVIOR** (Tier 2): "결제 실패 시나리오는?"
   - 사용자 답변: 잔액부족, 카드 만료, 한도 초과

3. **DOMAIN_CONTEXT** (Tier 3): "결제 타임아웃 기준이 있나요?"
   - 사용자 답변: 30초 타임아웃, PG사 API 호출

### Phase 3~6
- Target path: `src/payment`
- CLAUDE.md + IMPLEMENTS.md 생성

## Phase 6.5: Plan Preview

```
=== 생성 계획 ===

대상 경로: src/payment
액션: created

Purpose: 카드 결제 처리 모듈
Exports: 3개 — processPayment, cancelPayment, getPaymentStatus
Behaviors: 6개 — 결제 성공, 잔액부족 에러, 카드 만료 에러, ...
Dependencies: Internal 0개, External 1개 (payments-sdk)
```

사용자 선택: **승인**

## Phase 7: 최종 결과

```
---impl-result---
claude_md_file: src/payment/CLAUDE.md
implements_md_file: src/payment/IMPLEMENTS.md
status: success
action: created
validation: passed
exports_count: 3
behaviors_count: 6
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

참고: 정산/환불 모듈은 src/payment/CLAUDE.md의 Exports를 참조할 수 있습니다.
```
