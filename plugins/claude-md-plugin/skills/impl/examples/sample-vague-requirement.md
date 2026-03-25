# Sample: 모호한 요구사항 처리 (Low Completeness)

## 입력

```
사용자 요구사항: "사용자 관리 기능이 필요합니다"
프로젝트 루트: /Users/dev/my-app
```

## Phase 0: Scope Assessment

```
---scope-assessment---
completeness: low
scope: single-module
evidence:
  D1_purpose: 추론 가능 — "사용자 관리 기능" (CRUD/인증/권한 등 구체적 범위 불명)
  D2_constraints: 없음 — 제약/규칙 미언급
  D3_domain_context: 없음 — 결정 근거/배경 미언급
next_phase: Phase 2 Tier 1
---end-scope-assessment---
```

## Phase 1: Requirements Analysis

```
---extraction-summary---
format: natural-language
purpose: 사용자 관리 [inferred]
constraints: gap
domain_context: gap
location: unknown [gap]
gaps: [PURPOSE 구체화, CONSTRAINTS, DOMAIN_CONTEXT, LOCATION]
---end-extraction-summary---
```

## Phase 1.5: dep-explorer

- Internal deps: 0개
- External deps: 1개 existing (bcrypt — package.json에서 발견)

## Phase 2: Tiered Clarification

### Round 1 — Tier 1 (범위)

AskUserQuestion (2개):

1. **PURPOSE**: "사용자 관리의 핵심 책임은 무엇인가요?"
   - 옵션: CRUD (생성/조회/수정/삭제), 인증 (로그인/로그아웃), 권한 관리, 프로필 관리
   - 사용자 답변: **CRUD**

2. **LOCATION**: "어디에 위치해야 하나요?"
   - 옵션: src/user, src/users, src/account
   - 사용자 답변: **src/user**

### Round 2 — Tier 2 + Tier 3 (제약 + 맥락)

AskUserQuestion (3개):

1. **CONSTRAINTS** (Tier 2): "어떤 제약/규칙이 있나요?"
   - 옵션: 중복 이메일 금지만, 중복 이메일 + 필수 필드(email/password/name), 중복 이메일 + 필수 필드 + 비밀번호 8자 이상
   - 사용자 답변: **중복 이메일 + 필수 필드 + 비밀번호 8자 이상**

2. **CONSTRAINTS 에러** (Tier 2): "에러 시나리오는 어떤 것이 있나요?"
   - 옵션: 중복 이메일 에러만, 중복 이메일 + 미존재 사용자 에러, 중복 이메일 + 미존재 + 권한 에러
   - 사용자 답변: **중복 이메일 + 미존재 사용자 에러**

3. **DOMAIN_CONTEXT** (Tier 3): "비밀번호 해싱이나 보안 관련 배경이 있나요?"
   - 사용자 답변: **bcrypt 해싱, 이메일 unique constraint**

## Phase 3~6: 경로 결정 → 문서 생성

- Target path: `src/user` (create 모드)
- CLAUDE.md 생성 (Constraints 8개, Domain Context 있음)

## Phase 6.5: Plan Preview

```
=== 생성 계획 ===

대상 경로: src/user
액션: created

Purpose: 사용자 CRUD 관리 모듈
Constraints: 8개 — 중복 이메일 금지, 필수 필드(email/password/name), 비밀번호 8자 이상, ...
Domain Context: 있음
Dependencies: Internal 0개, External 1개 (bcrypt)
```

AskUserQuestion: "이 계획으로 CLAUDE.md를 생성할까요?"
→ 사용자 선택: **승인**

## Phase 7: 최종 결과

```
---impl-result---
claude_md_file: src/user/CLAUDE.md
developers_md_file: src/user/DEVELOPERS.md
compile_context_file: .claude/tmp/compile-context-src-user.md
status: success
action: created
validation: passed
constraints_count: 8
domain_context: present
dependencies_count: 1
tech_choices_count: 1
---end-impl-result---
```

## 생성된 CLAUDE.md 예시 (요약)

```markdown
# user

## Purpose

사용자 CRUD 관리 모듈. 사용자 생성, 조회, 수정, 삭제, 목록 조회 기능을 제공합니다.

## Constraints

- 유효한 입력 → User 객체 반환
- 존재하는 ID → User 조회 성공
- 중복 이메일 → DuplicateUserError
- 미존재 ID → UserNotFoundError
- 필터 적용 → 필터링된 목록 반환
- email은 유효한 이메일 형식, password는 8자 이상
- 생성/수정/삭제 시 id는 비어있지 않은 문자열
- 이메일은 unique constraint

## Domain Context

- 비밀번호는 bcrypt로 해시하여 저장
- 이메일 중복은 DB 레벨 unique constraint로 보장
```
