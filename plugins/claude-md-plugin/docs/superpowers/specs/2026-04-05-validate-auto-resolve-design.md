# Validate Auto-resolve Design

## Problem

현재 `/validate`의 Phase 4(Interactive Auto-fix)는 모든 시맨틱 이슈에 대해 유저가 개별적으로 resolution을 선택해야 한다.
이슈가 많을수록 유저 피로도가 높고, validator agent의 보고가 false positive일 수 있음에도 검증 없이 유저에게 전달된다.

## Solution

validator가 보고한 시맨틱 이슈를 **validate-reviewer agent**가 Socratic 방식으로 타당성 검증 + resolution direction 결정하고,
**validate-resolver agent**가 simulation-based Socratic verification 후 자동 수정한다.

## Design Decisions

| 항목 | 결정 |
|------|------|
| 리뷰 대상 범위 | 시맨틱 이슈만 (CLI 결정론적 이슈 제외) |
| Auto-resolve 범위 | reviewer가 resolution direction까지 결정, resolver가 자동 실행 |
| 아키텍처 패턴 | 단발 리뷰 (루프 없음) |
| 유저 피드백 | resolution direction이 ambiguous할 때만 요청 |
| 실행 분리 | reviewer는 판정만, resolver agent가 수행 |
| resolver 검증 | simulation-based Socratic loop로 evidence + side-effect 사전 검증 |
| 기본 동작 | auto-resolve가 기본. `--report-only`는 Phase 3까지만 (기존 유지) |
| 병렬 구조 | per-target 파이프라인 (validator → reviewer → resolver, target 간 병렬) |

## Workflow: AS-IS vs TO-BE

```
┌─────────────────────────────────┐    ┌─────────────────────────────────────┐
│         AS-IS (현재)             │    │           TO-BE (변경 후)            │
├─────────────────────────────────┤    ├─────────────────────────────────────┤
│                                 │    │                                     │
│ Phase 1: Initialization         │    │ Phase 1: Initialization (동일)      │
│                                 │    │                                     │
│ Phase 2: CLI Verification       │    │ Phase 2: CLI Verification (동일)    │
│  2a: Schema + auto-fix          │    │  2a: Schema + auto-fix              │
│  2b: Convention                 │    │  2b: Convention                     │
│  2c: Boundary                   │    │  2c: Boundary                       │
│  2d: DEVELOPERS.md (INV-3)      │    │  2d: DEVELOPERS.md (INV-3)          │
│  2e: Language                   │    │  2e: Language                       │
│                                 │    │                                     │
│ Phase 2.5: Test Coverage        │    │ Phase 2.5: Test Coverage (동일)     │
│                                 │    │                                     │
│ Phase 3: Semantic Verification  │    │ Phase 3: Semantic Verification (동일)│
│  Session file creation          │    │  Session file creation              │
│  Task(validator) × N (max 3)    │    │  Task(validator) × N (max 3)        │
│          │                      │    │          │                          │
│          ▼                      │    │          ▼                          │
│ Phase 4: Interactive Auto-fix   │    │ Phase 4: Auto-resolve               │
│  ┌──────────────────────┐       │    │  4a: Task(validate-reviewer) × N    │
│  │ 유저에게 이슈 목록 제시 │       │    │      Socratic 타당성 검증            │
│  │ 이슈별 resolution 선택 │       │    │      + resolution direction 결정     │
│  │  (a) /dev 안내        │       │    │          │                          │
│  │  (b) CLAUDE.md 수정   │       │    │          ▼                          │
│  │  (c) 코드 직접 수정    │       │    │  4b: AskUser (ambiguous만)          │
│  │  (d) Convention 완화  │       │    │      reviewer 판단 불가 시만 요청     │
│  └──────────────────────┘       │    │          │                          │
│          │                      │    │          ▼                          │
│          ▼                      │    │  4c: Task(validate-resolver) × N    │
│  Direct Edit (기존 함수 내)      │    │      simulation-based 검증           │
│  유저 승인 후 CLAUDE.md 수정     │    │      타당 → 수정 실행               │
│          │                      │    │      부당 → skip + 사유 기록         │
│          ▼                      │    │          │                          │
│  Re-verify                      │    │          ▼                          │
│                                 │    │  4d: Re-verify (CLI 재검증)          │
│ Phase 5: Report                 │    │                                     │
│  이슈 목록 출력                  │    │ Phase 5: Consolidated Report        │
│                                 │    │  resolved / skipped / asked 분류     │
└─────────────────────────────────┘    └─────────────────────────────────────┘
```

### Key Differences

| 항목 | AS-IS | TO-BE |
|------|-------|-------|
| Phase 4 주체 | 유저 (interactive) | validate-reviewer + validate-resolver (자동) |
| 유저 개입 시점 | 모든 이슈마다 선택 | ambiguous 이슈만 |
| resolution 결정 | 유저가 옵션 중 선택 | reviewer가 Socratic 근거로 결정 |
| 수정 전 검증 | 없음 | resolver가 simulation-based Socratic verification |
| 수정 실행 | SKILL이 Direct Edit | resolver agent가 수행 |

## Agent Design: validate-reviewer

**역할**: validator 결과의 시맨틱 이슈를 Socratic 방식으로 검증, resolution direction 결정

**입력**: reviewer session file (validator result file + 원본 validate session file 경로)

**AskUserQuestion**: ambiguous 판정 시에만 허용

### Socratic 검증 기준 (per issue)

| # | Criterion | 검증 내용 |
|---|-----------|----------|
| 1 | Evidence Existence | cited 파일/라인이 실제로 존재하는가 |
| 2 | Evidence Relevance | cited 코드가 해당 requirement/convention과 실제로 관련 있는가 |
| 3 | Drift Causality | "이 코드가 이 이슈를 구성한다"는 인과가 논리적인가 |
| 4 | Severity Appropriateness | ERROR/WARNING/INFO 분류가 drift type 정의에 부합하는가 |
| 5 | Resolution Feasibility | 제안할 resolution이 실행 가능하고 side-effect가 제한적인가 |

### 판정 (per issue)

| Verdict | 조건 | 후속 |
|---------|------|------|
| `valid` | 5개 기준 모두 통과 | resolution direction 포함 → resolver로 |
| `invalid` | 기준 1~4 중 하나라도 실패 | 사유 기록, skip |
| `ambiguous` | 기준 통과했지만 resolution direction 결정 불가 | AskUser 필요 |

### Resolution Direction 형식

```yaml
- issue_id: REQ-DRIFT-001
  verdict: valid
  resolution_type: edit_code | edit_claude_md | edit_developers_md | run_dev
  target_file: src/auth/handler.ts
  description: "함수 validateToken()에 expiry 체크 로직 추가"
  evidence_summary: "CLAUDE.md REQ-3 '만료 토큰 거부' vs handler.ts:42 expiry 미검증"
```

`run_dev`: 새 코드 생성이 필요한 경우 — resolver가 직접 수정하지 않고 `/dev` 실행을 안내.

## Agent Design: validate-resolver

**역할**: reviewer 판정을 simulation-based Socratic verification 후 수정 실행

**입력**: resolver session file (reviewer result file + 원본 validate session file 경로)

**AskUserQuestion**: 금지 (판단 불가 시 skip)

### Step 1: Simulation-based Socratic Verification (per valid issue)

reviewer의 판정 + resolution을 받아, 수정 전에 영향 시뮬레이션.

**원칙**: "이 수정을 적용했을 때 현재 시스템의 어떤 부분이 영향을 받는가?"를
resolution_type과 수정 대상의 맥락에 맞게 자문자답한다.

**resolution_type별 시뮬레이션 관점 가이드 (예시, 강제 아님)**:

| resolution_type | 시뮬레이션 관점 |
|----------------|---------------|
| `edit_code` | caller 영향, 테스트 호환성, 다른 requirement 위반 여부 |
| `edit_claude_md` | 하위 모듈 영향, 다른 requirement와의 충돌, /dev 재생성 필요 여부 |
| `edit_developers_md` | constraint 변경이 기존 테스트와 모순되지 않는지 |
| `run_dev` | 시뮬레이션 불필요 (deferred) |

**종료 조건**: "이 수정의 side-effect를 더 이상 발견할 수 없다"고 판단될 때

**시뮬레이션 판정**:

| 결과 | 후속 |
|------|------|
| side-effect 없음 확인 | Step 2 수정 실행 |
| side-effect 발견 | skip + 영향 범위 기록 |
| evidence 불일치 발견 | skip + 사유 기록 |

### Step 2: 수정 실행 (resolution_type별)

| resolution_type | 수행 내용 | 범위 제한 |
|----------------|----------|----------|
| `edit_code` | 기존 함수/struct 내 수정 | 새 파일/함수 생성 금지 |
| `edit_claude_md` | CLAUDE.md 섹션 수정 | requirement 추가/삭제/수정 |
| `edit_developers_md` | DEVELOPERS.md 섹션 수정 | constraint/tech context 업데이트 |
| `run_dev` | 수정하지 않음, 리포트에 `/dev 필요` 기록 | resolver 범위 밖 |

### 수정 후 검증

수정한 파일에 대해 `validate-schema` CLI 실행하여 문서 구조가 깨지지 않았는지 확인. 실패 시 수정 롤백.

### 출력 형식

```yaml
- issue_id: REQ-DRIFT-001
  action: resolved | skipped | deferred_to_dev
  detail: "handler.ts:42에 expiry 체크 추가"
  files_modified: [src/auth/handler.ts]

- issue_id: REQ-DRIFT-002
  action: skipped
  detail: "reviewer가 cited한 라인 52가 실제로는 주석. evidence 불일치"
  files_modified: []
```

## Session File Flow

```
SKILL
 │
 ├─ Phase 3: validate-session-{dir-safe}.md (기존)
 │     │
 │     ▼
 │   Task(validator) → validate-result-{dir-safe}.md (기존)
 │
 ├─ Phase 4a: reviewer-session-{dir-safe}.md (신규)
 │     내용: validator result file 경로 + validate session file 경로
 │     │
 │     ▼
 │   Task(validate-reviewer) → reviewer-result-{dir-safe}.md (신규)
 │     내용: per-issue verdict (valid/invalid/ambiguous) + resolution direction
 │
 ├─ Phase 4b: AskUser (ambiguous 이슈가 있을 경우)
 │     SKILL이 reviewer-result 파일의 ambiguous verdict를
 │     유저가 선택한 resolution direction으로 교체하여 업데이트
 │
 ├─ Phase 4c: resolver-session-{dir-safe}.md (신규)
 │     내용: reviewer result file 경로 + validate session file 경로
 │     │
 │     ▼
 │   Task(validate-resolver) → resolver-result-{dir-safe}.md (신규)
 │     내용: per-issue action (resolved/skipped/deferred_to_dev) + files_modified
 │
 └─ Phase 5: Consolidated Report
```

모든 session/result file은 `${TMP_DIR}`에 저장.

## Consolidated Report (Phase 5)

```markdown
# Validation Report: {directory}

## Summary
- Total issues: 8 (Deterministic: 2, Semantic: 6)
- Auto-resolved: 3
- Skipped (invalid/side-effect): 2
- Deferred to /dev: 1
- User-resolved (ambiguous): 1
- Remaining: 1 (deterministic, already fixed in Phase 2)

## Resolved Issues
- [WARNING] REQ-DRIFT-001: handler.ts:42 expiry 체크 추가 ✓
- [WARNING] TECH-STALE-003: DEVELOPERS.md Technical Context 업데이트 ✓
- [ERROR] CONV-DEP-001: 금지된 import 제거 ✓

## Skipped Issues
- [WARNING] REQ-DRIFT-002: (invalid) reviewer cited 라인이 주석
- [WARNING] REQ-DRIFT-005: (side-effect) caller 3곳에 영향

## Deferred to /dev
- [ERROR] REQ-NOTIMPL-001: 새 함수 생성 필요 → `/dev src/auth` 실행 권장

## User-resolved
- [ERROR] REQ-VIOLATED-001: (ambiguous) 유저 판단 → CLAUDE.md 수정
```

## Scope

### In Scope
- validate-reviewer agent 신규 생성
- validate-resolver agent 신규 생성
- validate SKILL Phase 4 변경 (interactive → auto-resolve)
- validate SKILL Phase 5 변경 (consolidated report)

### Out of Scope
- validator agent 변경 없음 (기존 출력 형식 유지)
- Phase 1~3 변경 없음
- `--report-only` 동작 변경 없음
- CLI 변경 없음
