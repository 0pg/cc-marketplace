# Sample: 모호한 요구사항 처리 (Low Completeness)

## 입력

```
사용자 요구사항: "사용자 관리 기능이 필요합니다"
프로젝트 루트: /Users/dev/my-app
```

## Phase 0: Scope Assessment

- **completeness**: low (추상적 기능 설명만, Exports 추론 불가)
- **scope**: single-module (단일 도메인 "사용자 관리")
- **다음 단계**: Phase 2 Tier 1부터 시작

## Phase 1: Requirements Analysis

초기 추출 결과 (불완전):
- Purpose: 사용자 관리 (구체적 범위 불명확)
- Exports: 추론 불가
- Behaviors: 추론 불가

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

### Round 2 — Tier 2 + Tier 3 (인터페이스 + 제약)

AskUserQuestion (3개):

1. **EXPORTS** (Tier 2): "어떤 함수를 export해야 하나요?"
   - 옵션: createUser/getUser/updateUser/deleteUser, 위 4개 + listUsers, 위 5개 + searchUsers
   - 사용자 답변: **위 4개 + listUsers**

2. **BEHAVIOR** (Tier 2): "에러 시나리오는 어떤 것이 있나요?"
   - 옵션: 중복 이메일 에러만, 중복 이메일 + 미존재 사용자 에러, 중복 이메일 + 미존재 + 권한 에러
   - 사용자 답변: **중복 이메일 + 미존재 사용자 에러**

3. **CONTRACT** (Tier 3): "사용자 생성 시 필수 필드는?"
   - 옵션: email + password만, email + password + name, email + password + name + role
   - 사용자 답변: **email + password + name**

## Phase 3~6: 경로 결정 → 문서 생성

- Target path: `src/user` (create 모드)
- CLAUDE.md 생성 (5 Exports, 7 Behaviors)
- IMPLEMENTS.md Planning Section 생성

## Phase 6.5: Plan Preview

```
=== 생성 계획 ===

대상 경로: src/user
액션: created

Purpose: 사용자 CRUD 관리 모듈
Exports: 5개 — createUser, getUser, updateUser, deleteUser, listUsers
Behaviors: 7개 — 사용자 생성 성공, 중복 이메일 에러, 사용자 조회 성공, 미존재 사용자 에러, ...
Dependencies: Internal 0개, External 1개 (bcrypt)
```

AskUserQuestion: "이 계획으로 CLAUDE.md + IMPLEMENTS.md를 생성할까요?"
→ 사용자 선택: **승인**

## Phase 7: 최종 결과

```
---impl-result---
claude_md_file: src/user/CLAUDE.md
implements_md_file: src/user/IMPLEMENTS.md
status: success
action: created
validation: passed
exports_count: 5
behaviors_count: 7
dependencies_count: 1
tech_choices_count: 1
---end-impl-result---
```

## 생성된 CLAUDE.md 예시 (요약)

```markdown
# user

## Purpose

사용자 CRUD 관리 모듈. 사용자 생성, 조회, 수정, 삭제, 목록 조회 기능을 제공합니다.

## Exports

- `createUser(input: CreateUserInput): Promise<User>` — 새 사용자 생성
- `getUser(id: string): Promise<User>` — ID로 사용자 조회
- `updateUser(id: string, input: UpdateUserInput): Promise<User>` — 사용자 정보 수정
- `deleteUser(id: string): Promise<void>` — 사용자 삭제
- `listUsers(filter?: UserFilter): Promise<User[]>` — 사용자 목록 조회
- `CreateUserInput { email: string, password: string, name: string }` — 생성 입력 타입
- `User { id: string, email: string, name: string, createdAt: Date }` — 사용자 엔티티

## Behavior

- success: 유효한 입력 → User 객체 반환
- success: 존재하는 ID → User 조회 성공
- error: 중복 이메일 → DuplicateUserError
- error: 미존재 ID → UserNotFoundError
- success: 필터 적용 → 필터링된 목록 반환
- success: 사용자 삭제 → void 반환
- success: 사용자 수정 → 업데이트된 User 반환

## Contract

- createUser: email은 유효한 이메일 형식, password는 8자 이상
- getUser/updateUser/deleteUser: id는 비어있지 않은 문자열

## Protocol

None

## Domain Context

- 비밀번호는 bcrypt로 해시하여 저장
- 이메일은 unique constraint
```
