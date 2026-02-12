<!--
  decompiler-workflow.md
  Extracted detailed workflow reference for the decompiler agent.
  Contains: Phase 3-4 pseudocode, CLAUDE.md/IMPLEMENTS.md templates,
  Skill invocation chain, and detailed format examples.

  This file is loaded at runtime by the decompiler agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/decompile/references/decompiler-workflow.md"
-->

# Decompiler Detailed Workflow Reference

## Phase 3: 불명확한 부분 질문 (필요시)

분석 결과에서 불명확한 부분이 있으면 사용자에게 질문합니다.

**질문 안 함** (코드에서 추론 가능):
- 함수명에서 목적이 명확한 경우
- 상수 값을 계산할 수 있는 경우
- 표준 패턴을 따르는 경우

**질문 함** (코드만으로 불명확):
- 비표준 매직 넘버의 비즈니스 의미
- 도메인 전문 용어
- 컨벤션을 벗어난 구현의 이유
- **Domain Context 관련**: 결정 근거, 외부 제약, 호환성 요구
- **Implementation 관련**: 기술 선택 근거, 대안 미선택 이유

불명확한 부분이 있으면 AskUserQuestion으로 사용자에게 질문합니다. 예를 들어, 매직 넘버의 비즈니스 배경을 확인하기 위해 "GRACE_PERIOD_DAYS = 7의 비즈니스 배경이 있나요?" 같은 질문을 합니다. 옵션으로 "법적 요구사항", "비즈니스 정책", "기술적 제약" 등을 제시합니다.

### Domain Context 질문 (CLAUDE.md용 - 코드에서 추출 불가)

Domain Context는 코드에서 추론할 수 없는 "왜?"에 해당합니다.
상수 값, 설계 결정, 특이한 구현이 있을 때 반드시 질문합니다:

Domain Context 추출을 위해 다음 카테고리의 질문을 합니다:

1. **상수 값의 결정 근거 (Decision Rationale)**: 매직 넘버가 발견되면 값을 선택한 이유를 질문합니다. 옵션: 컴플라이언스(PCI-DSS, GDPR 등), SLA/계약, 내부 정책, 기술적 산출
2. **외부 제약 조건 (Constraints)**: 지켜야 할 외부 제약이 있는지 질문합니다. 옵션: 있음(규제, 라이선스 등), 없음
3. **호환성 요구 (Compatibility)**: 레거시 패턴이 발견되면 호환성 요구가 있는지 질문합니다. 옵션: 있음(특정 버전/형식 지원 필요), 없음

질문이 있으면 AskUserQuestion으로 한 번에 전달합니다.

### Implementation 관련 질문 (IMPLEMENTS.md용)

기술 선택과 구현 방향에 대해 다음 카테고리의 질문을 합니다:

1. **기술 선택 근거**: 외부 의존성이 있으면 해당 라이브러리를 선택한 이유를 질문합니다. 옵션: 성능(벤치마크), 호환성(기존 코드), 팀 경험(숙련도), 커뮤니티(문서화/지원)
2. **대안 미선택 이유**: 고려했으나 선택하지 않은 대안이 있는지 질문합니다. 옵션: 있음(대안 설명 가능), 없음

질문이 있으면 AskUserQuestion으로 한 번에 전달합니다.

## Phase 4: CLAUDE.md 초안 생성 (WHAT)

분석 결과를 기반으로 CLAUDE.md를 직접 생성합니다:

1. **자식 CLAUDE.md Purpose 추출**: 각 자식 CLAUDE.md 파일이 존재하면 읽어서 Purpose 섹션을 추출합니다.
2. **CLAUDE.md 템플릿 생성**: 다음 템플릿에 맞게 CLAUDE.md를 작성합니다:

```markdown
# {directory_name}

## Purpose

{분석에서 추출한 목적 또는 사용자 응답}

## Exports

### Functions
{분석에서 추출한 함수 목록}

### Types
{분석에서 추출한 타입 목록}

## Behavior

### 정상 케이스
{성공 시나리오 목록}

### 에러 케이스
{에러 시나리오 목록}

## Contract

{계약 조건 또는 "None"}

## Protocol

{프로토콜 또는 "None"}

## Domain Context

{사용자 응답 기반 도메인 컨텍스트 또는 "None"}

## Dependencies

- external:
{외부 의존성 목록}

- internal:
{resolved CLAUDE.md 경로 기반 내부 의존성: 심볼만 나열}
```

**Internal Dependencies 포맷팅 규칙:**
- code-analyze JSON의 `dependencies.internal` 배열을 읽어서 출력합니다.
- resolution이 "Exact" 또는 "Ancestor"이면:
  ```
  - `{dep.claude_md_path}`: {사용하는 심볼들}
  ```
- resolution이 "Unresolved"이면:
  ```
  - `{dep.raw_import}` <!-- UNRESOLVED: 수동 확인 필요 -->
  ```
- 예시:
  ```
  - `core/domain/transaction/CLAUDE.md`: WithdrawalResultSynchronizer
  - `vendors/vendor-common/CLAUDE.md`: FinancialServiceProvider
  ```

**CLAUDE.md vs IMPLEMENTS.md 차이:**
- CLAUDE.md: 심볼만 나열 (위 포맷)
- IMPLEMENTS.md: 심볼 + 선택 이유/방향 포함 (예: `- \`path/CLAUDE.md\`: SymbolName — 이 모듈의 인증 기능 활용`)

3. **대상 디렉토리에 직접 Write** (scratchpad 미사용)

## Phase 4.5: IMPLEMENTS.md 초안 생성 (HOW - 전체 섹션)

분석 결과와 사용자 응답을 기반으로 IMPLEMENTS.md를 직접 생성합니다:

다음 템플릿에 맞게 IMPLEMENTS.md를 작성합니다:

```markdown
# {directory_name}/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /impl 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Dependencies Direction

### External
{외부 의존성과 선택 이유 (사용자 응답 기반)}

### Internal
{resolved CLAUDE.md 경로 기반 내부 의존성: 심볼 + 선택 이유 포함}

## Implementation Approach

### 전략
{코드에서 추론된 전략 또는 분석 결과}

### 고려했으나 선택하지 않은 대안
{사용자 응답 기반 대안 또는 "None"}

## Technology Choices

{기술 선택 근거 (사용자 응답 기반) 또는 "None"}

<!-- ═══════════════════════════════════════════════════════ -->
<!-- IMPLEMENTATION SECTION - /compile 이 업데이트            -->
<!-- ═══════════════════════════════════════════════════════ -->

## Algorithm

{분석에서 추출한 알고리즘 또는 "(No complex algorithms found)"}

## Key Constants

{분석에서 추출한 상수와 도메인 맥락 또는 "(No domain-significant constants)"}

## Error Handling

{에러 처리 패턴 또는 "None"}

## State Management

{상태 관리 패턴 또는 "None"}

## Implementation Guide

- {current_date}: Initial extraction from existing code
```

대상 디렉토리에 직접 Write합니다 (scratchpad 미사용).

## Skill 호출 체인

```
┌─────────────────────────────────────────────────────────────┐
│                     decompiler Agent                          │
│                                                              │
│  ┌─ Phase 0: Bash(jq) ──────────────────────────────────┐   │
│  │ tree.json에서 디렉토리 정보 조회                      │   │
│  │ (source_file_count, subdir_count)                     │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Skill("boundary-resolve") ─────────────────────────┐   │
│  │ 바운더리 분석 → 결과 파일로 저장                      │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Skill("code-analyze") ─────────────────────────────┐   │
│  │ 코드 분석 (WHAT + HOW 모두)                          │   │
│  │ - exports, deps, behaviors, contracts, protocol      │   │
│  │ - algorithms, constants, error handling, state       │   │
│  │ → 결과 파일로 저장                                    │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─ AskUserQuestion (선택적) ──────────────────────────┐   │
│  │ Domain Context 질문 (CLAUDE.md용)                    │   │
│  │ Implementation 질문 (IMPLEMENTS.md용)                │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─ CLAUDE.md 생성 (WHAT) ─────────────────────────────┐   │
│  │ Purpose, Exports, Behavior, Contract, Protocol, DC   │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─ IMPLEMENTS.md 생성 (HOW - 전체) ───────────────────┐   │
│  │ Planning Section + Implementation Section            │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Skill("schema-validate") ──────────────────────────┐   │
│  │ CLAUDE.md 스키마 검증 (1회, 실패 시 경고)            │   │
│  │ → 결과 파일로 저장                                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```
