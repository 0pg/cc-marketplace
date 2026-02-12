---
name: impl
version: 1.0.0
aliases: [define, requirements]
description: |
  This skill should be used when the user asks to "define requirements", "write spec",
  "create CLAUDE.md from requirements", "define behavior before coding", or uses "/impl".
  Analyzes natural language requirements and generates CLAUDE.md without implementing code.
  Follows ATDD principle: specification first, then code generation via /compile.
  Trigger keywords: 요구사항 정의, 스펙 작성, 명세 먼저
user_invocable: true
allowed-tools: [Read, Glob, Write, Task, Skill, AskUserQuestion, Bash]
---

# Impl Skill

## 목적

요구사항(자연어 또는 User Story)을 분석하여 **CLAUDE.md + IMPLEMENTS.md** 쌍을 생성/업데이트.
**코드 구현 없이** behavior 정의만 수행하여 ATDD/TDD의 "명세 먼저" 원칙을 따름.

## 듀얼 문서 시스템

```
/impl "요구사항"
    │
    ▼
Bash(claude-md-core scan-claude-md) → 기존 CLAUDE.md 인덱스
    │
    ▼
Task(impl agent) + claude_md_index_file
    │
    ├─→ CLAUDE.md 생성/업데이트 (WHAT)
    │   - Purpose, Domain Context, Exports, Behavior, Contract, Protocol
    │
    └─→ IMPLEMENTS.md [Planning Section] 업데이트 (HOW 계획)
        - Dependencies Direction (CLAUDE.md 경로로 resolve)
        - Implementation Approach
        - Technology Choices
```

## 아키텍처

```
User: /impl "요구사항"
        │
        ▼
┌──────────────────────────────────────────────────┐
│ impl SKILL (Entry Point)                         │
│                                                  │
│ 1. Bash(scan-claude-md) → 기존 CLAUDE.md 인덱스 │
│ 2. Task(impl agent) + claude_md_index_file       │
└──────────────────────────────────────────────────┘

        │
        ▼
┌─────────────────────────────────────────────┐
│ impl agent AGENT                            │
│                                             │
│ 0. ⭐ Scope Assessment (completeness +      │
│    multi-module 감지)                       │
│ 1. 요구사항 분석                            │
│ 1.5. Task(dep-explorer) → 의존성 탐색      │
│ 2. Tiered Clarification (Tier 1→2→3,       │
│    최대 2라운드)                             │
│ 3. 대상 위치 결정                           │
│ 4. 기존 CLAUDE.md 존재시 병합               │
│ 5. CLAUDE.md 생성                           │
│ 5.5. IMPLEMENTS.md Planning Section 생성   │
│ 6. Skill("schema-validate") → 검증          │
│ 6.5 ⭐ Plan Preview → AskUserQuestion       │
│    (approve/modify/cancel)                  │
│ 7. 승인 시에만 CLAUDE.md + IMPLEMENTS.md 저장│
└─────────────────────────────────────────────┘
```

## 워크플로우

### 1. 요구사항 수신

사용자로부터 요구사항을 수신:
- 자연어 설명
- User Story (As a..., I want..., So that...)
- Feature 목록
- 기능 요청

### 2. 기존 CLAUDE.md 인덱스 생성 (scan-claude-md)

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi

# CLI로 기존 CLAUDE.md 파일의 경량 인덱스 생성
mkdir -p .claude/extract-results
$CLI_PATH scan-claude-md --root {project_root} --output .claude/extract-results/claude-md-index.json
```

### 3. CLAUDE.md + IMPLEMENTS.md 생성 (impl agent)

`claude_md_index_file`은 `.claude/extract-results/claude-md-index.json`입니다.

impl agent를 Task로 호출합니다. 프롬프트에 사용자 요구사항(`user_requirement`), 프로젝트 루트(`project_root`), `claude_md_index_file` 경로를 전달합니다. 입력 형식은 impl agent의 Input 섹션을 따릅니다. description은 "Generate CLAUDE.md + IMPLEMENTS.md from requirements"입니다.

**impl agent 워크플로우:**
0. **⭐ Scope Assessment** — 완성도 분류 (high/medium/low) + 멀티 모듈 감지
1. 요구사항에서 Purpose, Exports, Behaviors, Contracts 추출
1.5. **Task(dep-explorer)** — 의존성 탐색 위임 (internal CLAUDE.md + external packages)
2. **⭐ Tiered Clarification** — Tier 1(범위) → Tier 2(인터페이스) → Tier 3(제약), 최대 2라운드
   - completeness=high이면 Phase 2 건너뛰기 (경로 미지정 시 LOCATION만 질문)
3. 대상 경로 결정 (명시적 경로, 모듈명 추론, 사용자 선택)
4. 기존 CLAUDE.md 존재시 smart merge
5. 템플릿 기반 CLAUDE.md 생성
5.5. **IMPLEMENTS.md Planning Section 생성**
   - Dependencies Direction: dep-explorer 결과에서 CLAUDE.md 경로로 resolve된 의존성
   - Implementation Approach: 구현 전략과 대안
   - Technology Choices: 기술 선택 근거
6. 스키마 검증 (1회)
6.5. **⭐ Plan Preview** — 생성 계획 요약 제시 → 사용자 승인 (approve/modify/cancel)
7. 최종 저장 (승인된 경우만)

### 4. 최종 결과 보고

```
=== /impl 완료 ===

생성/업데이트된 파일:
  ✓ {target_path}/CLAUDE.md (WHAT - 스펙)
  ✓ {target_path}/IMPLEMENTS.md (HOW - Planning Section)

스펙 요약:
  - Purpose: {purpose}
  - Exports: {export_count}개
  - Behaviors: {behavior_count}개
  - Contracts: {contract_count}개

구현 계획 요약:
  - Dependencies: {dependency_count}개
  - Implementation Approach: {approach_summary}
  - Technology Choices: {choice_count}개

검증 결과: 스키마 검증 통과

다음 단계:
  - /compile로 코드 구현 가능 (IMPLEMENTS.md Implementation Section도 업데이트됨)
  - /validate로 문서-코드 일치 검증 가능
```

사용자가 Plan Preview에서 취소한 경우:
```
=== /impl 취소 ===

사용자 요청으로 파일 생성이 취소되었습니다.

---impl-result---
status: cancelled_by_user
---end-impl-result---
```

## DO / DON'T

**DO:**
- Assess scope before deep analysis (Phase 0)
- Use tiered questions (Tier 1→2→3, max 2 rounds)
- Show plan preview before file creation (Phase 6.5)
- Provide /impl commands for remaining modules (multi-module decomposition)
- Clarify ambiguous requirements via AskUserQuestion
- Generate both CLAUDE.md and IMPLEMENTS.md as a pair (INV-3)
- Merge with existing CLAUDE.md when updating
- Delegate dependency discovery to dep-explorer agent

**DON'T:**
- Generate source code (use /compile)
- Modify IMPLEMENTS.md Implementation Section (only Planning Section)
- Skip schema validation
- Read source code for dependency discovery (use CLAUDE.md Exports)
- Save files without user approval (Phase 6.5)

## 오류 처리

| 상황 | 대응 |
|------|------|
| 요구사항 불명확 | impl agent가 AskUserQuestion으로 명확화 |
| 대상 경로 모호 | 후보 목록 제시 후 선택 요청 |
| 기존 CLAUDE.md와 충돌 | 병합 전략 제안 |
| 기존 IMPLEMENTS.md와 충돌 | Planning Section만 업데이트 (Implementation Section 유지) |
| 멀티 모듈 감지 | AskUserQuestion으로 분해/도메인 그룹/단일 선택 |
| 스키마 검증 실패 | 경고와 함께 이슈 보고 |
| Plan Preview 거절 | 범위 조정 또는 취소 (최대 1회 루프백) |
| Plan Preview 취소 | status: cancelled_by_user 반환 |

## 참조 문서

- `references/scan-and-orchestration.md`: scan-claude-md 호출 패턴, impl agent 워크플로우 상세 (Phase 0~7), Planning Section 생성 로직
- `examples/sample-claude-md.md`: 생성된 CLAUDE.md 예시
- `examples/sample-implements-md.md`: 생성된 IMPLEMENTS.md Planning Section 예시
- `examples/sample-vague-requirement.md`: 모호한 요구사항 처리 시나리오 (low completeness)
- `examples/sample-multi-module.md`: 멀티 모듈 요구사항 분해 시나리오

## /decompile과의 차이점

| 측면 | /decompile | /impl |
|------|------------|-------|
| 입력 | 기존 소스 코드 | 사용자 요구사항 |
| 방향 | Code → CLAUDE.md | Requirements → CLAUDE.md |
| 목적 | 기존 코드 문서화 | 새 기능 명세 정의 |
| 사용 시점 | 레거시 코드 정리 | 신규 개발 시작 전 |

## 패러다임

- **전통적 개발**: 요구사항 → 코드 → (문서는 부산물)
- **ATDD with /impl**: 요구사항 → CLAUDE.md (Source of Truth) → `/compile` → 코드

`/impl`은 ATDD의 "Acceptance Criteria 먼저" 원칙을 CLAUDE.md 기반으로 구현.

## Examples

<example>
<context>
사용자가 새 모듈의 요구사항을 정의하려고 합니다.
</context>
<user_request>/impl "JWT 토큰을 검증하는 인증 모듈이 필요합니다"</user_request>
<assistant_response>
요구사항을 분석합니다...

=== /impl 완료 ===

생성/업데이트된 파일:
  ✓ src/auth/CLAUDE.md (WHAT - 스펙)
  ✓ src/auth/IMPLEMENTS.md (HOW - Planning Section)

스펙 요약:
  - Purpose: JWT 토큰 검증 인증 모듈
  - Exports: 2개
  - Behaviors: 3개

다음 단계:
  - /compile로 코드 구현 가능
</assistant_response>
</example>
