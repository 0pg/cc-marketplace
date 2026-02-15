<!-- impl-workflow.md
     Extracted from agents/impl.md for context efficiency.
     Contains detailed workflow pseudocode for the impl agent:
     - Phase 0: Scope Assessment (completeness + multi-module detection)
     - Phase 1: Requirements Analysis
     - Phase 1.5: dep-explorer delegation
     - Phase 2: Tiered Clarification (Tier 1→2→3, max 2 rounds)
     - Phase 3: Target path determination
     - Phase 4: Merge strategy
     - Phase 5: CLAUDE.md generation templates
     - Phase 5.5: IMPLEMENTS.md Planning Section generation
     - Phase 6: Schema validation
     - Phase 6.5: Plan Preview & User Approval
     - Phase 7: Final save & result
     - Context efficiency notes
-->

## Workflow

### Phase 0: Scope Assessment

요구사항을 분석하기 전에 완성도와 스코프를 먼저 평가합니다.

#### 완성도 분류

| 레벨 | 기준 | 예시 | 다음 단계 |
|------|------|------|----------|
| **high** | 구체적 시그니처/타입/동작 포함 | `validateToken(token: string) → Claims` | Phase 2 건너뛰기 (단, 경로 미지정 시 LOCATION만 질문) |
| **medium** | 명확한 목적 + 모호한 인터페이스 | "JWT 토큰 검증 모듈" | Phase 2 Tier 2부터 |
| **low** | 추상적 기능 설명만 | "사용자 관리 기능" | Phase 2 Tier 1부터 |

판단 기준:
- Exports 추론 가능? → medium 이상
- 구체적 시그니처 있음? → high
- 둘 다 아님? → low

#### 멀티 모듈 감지

요구사항에 독립된 도메인이 2개 이상 언급되면 multi-module로 판단합니다.

감지 신호:
- 나열형: "A, B, C를 지원"
- AND 연결: "A와 B 기능"
- 독립된 책임: 각 항목이 별도 Exports를 가질 수 있음

multi-module 감지 시 AskUserQuestion:

질문: "여러 도메인이 감지되었습니다. 어떻게 진행할까요?"
옵션:
1. 모듈별 분해 (권장) — 첫 모듈만 생성, 나머지는 /impl 가이드 제공
2. 도메인 그룹 생성 — Structure로 하위 모듈을 참조하는 상위 CLAUDE.md 생성
3. 단일 모듈 유지 — 모든 기능을 하나의 CLAUDE.md에

"모듈별 분해" 선택 시:
- 사용자에게 먼저 처리할 모듈 선택 요청
- 해당 모듈만 Phase 1~7 진행
- 최종 결과에 나머지 모듈용 /impl 명령어 가이드 포함

"도메인 그룹 생성" 선택 시:
- 상위 디렉토리에 CLAUDE.md + IMPLEMENTS.md 생성
  - Purpose: 도메인 그룹 설명 (예: "결제 도메인 — 카드 결제, 정산, 환불을 관할")
  - Structure: 하위 모듈 디렉토리 참조 (예: `payment/`, `settlement/`, `refund/`)
  - Exports: None (개별 모듈이 각자 export)
  - Behavior/Contract/Protocol/Domain Context: 모듈 간 관계가 있으면 기술, 없으면 None
- 하위 모듈 CLAUDE.md는 생성하지 않음 → 나머지 모듈용 /impl 가이드 제공
- Phase 1~7을 그룹 노드 기준으로 진행 (Phase 2 Tiered Clarification에서 하위 모듈 구성 확인)

### Phase 1: Requirements Analysis

Extract the following information from requirements:

| 추출 항목 | 추출 방법 |
|-----------|----------|
| Purpose | 핵심 기능/책임 식별 |
| Exports | 언급된 함수, 타입, 클래스 |
| Behaviors | input → output 패턴 |
| Contracts | 전제조건, 후조건, 에러 조건 |
| Protocol | 상태 전이, 라이프사이클 (있는 경우) |
| Domain Context | 결정 근거, 제약 조건, 호환성 요구 |
| Location | 명시된 경로 또는 추론 |

### Phase 1.5: 의존성 탐색 (dep-explorer 위임)

의존성 탐색은 dep-explorer agent에 위임합니다.
impl agent는 결과 JSON만 로드하여 사용합니다.

dep-explorer agent를 Task로 호출합니다. 프롬프트에 사용자 요구사항(`user_requirement`), 프로젝트 루트(`project_root`), index 파일 경로(`claude_md_index_file`)를 전달합니다. 결과는 `.claude/extract-results/dep-analysis-{module_name}.json`에 저장됩니다.

Task 완료 후 결과 JSON을 로드하여 `dep_result`에 저장합니다.

**결과 활용:**
- `dep_result["internal_deps"]` → Dependencies Direction Internal 섹션
- `dep_result["external_deps"]` → Dependencies Direction External 섹션
- `dep_result["requirement_summary"]` → 요구사항 요약 참조

### Phase 2: Tiered Clarification (계층화된 질문)

AskUserQuestion 제약(최대 4질문, 2-4옵션)을 고려하여 계층적으로 질문합니다.

#### Tier 구조

| Tier | 카테고리 | 질문 조건 | 질문 예시 |
|------|---------|----------|----------|
| **1 (범위)** | PURPOSE, LOCATION | completeness=low 이거나 경로 미지정 | "핵심 책임은?", "위치는?" |
| **2 (인터페이스)** | EXPORTS, BEHAVIOR | 인터페이스/시나리오 불명확 | "어떤 함수 export?", "에러 시나리오?" |
| **3 (제약)** | CONTRACT, DOMAIN_CONTEXT, DEPENDENCY | 유효성 규칙/제약/의존성 불명확 | "전제조건?", "만료 기간?", "외부 라이브러리?" |

#### 라운드 실행 로직

Round 1: completeness=low이면 Tier 1 질문 (최대 4개)
         completeness=medium이면 Round 1 건너뛰고 Round 2로
         completeness=high이면 Phase 2 건너뛰기
           단, 경로가 명시되지 않았고 모듈명에서 추론도 불확실하면 LOCATION 질문 1개만 실행

Round 2: Tier 2 질문(최대 2개) + Tier 3 질문(최대 2개) 합산하여 최대 4개 질문
         필요 없는 카테고리는 건너뜀

최대 2라운드 후에도 모호하면: 최선의 추측으로 진행 + Phase 6.5에서 사용자 확인

#### 질문 안 함 (기존 유지)
- 요구사항에 구체적 시그니처가 있는 경우
- 프로젝트 컨벤션에서 추론 가능한 경우
- 표준 패턴을 따르는 경우
- Phase 1.5에서 매칭되는 CLAUDE.md가 명확한 경우

### Phase 3: 대상 위치 결정

대상 위치를 다음 우선순위로 결정합니다:

1. **사용자가 명시적으로 지정한 경우**: 해당 경로를 사용합니다. 기존 CLAUDE.md가 있으면 "update", 없으면 "create" 모드입니다.
2. **요구사항에서 모듈명 추론 가능한 경우**: 프로젝트에서 일치하는 디렉토리를 Glob으로 검색합니다.
   - 후보가 1개이면 해당 경로를 사용합니다 ("update" 모드).
   - 후보가 여러 개이면 AskUserQuestion으로 사용자에게 선택을 요청합니다.
   - 후보가 없으면 새 디렉토리 경로를 제안합니다 ("create" 모드).
3. **기본값**: 현재 디렉토리(`.`)를 사용합니다. `./CLAUDE.md`가 있으면 "update", 없으면 "create" 모드입니다.

### Phase 4: 기존 CLAUDE.md 확인 및 병합

대상 경로에 CLAUDE.md가 존재하고 "update" 모드이면, `claude-md-core parse-claude-md` CLI로 기존 CLAUDE.md를 파싱한 후 새 스펙과 smart merge합니다. 존재하지 않으면 새 스펙을 그대로 사용합니다.

#### Smart Merge 전략

| 섹션 | 병합 전략 |
|------|----------|
| Purpose | 기존 유지 또는 확장 (사용자 선택) |
| Exports | 이름 기준 병합 (기존 유지 + 신규 추가) |
| Behavior | 시나리오 추가 (중복 제거) |
| Contract | 함수명 기준 병합 |
| Protocol | 상태/전이 병합 |
| Dependencies | Union |

### Phase 5: CLAUDE.md 생성 (WHAT)

템플릿 기반으로 CLAUDE.md를 생성합니다:

```markdown
# {module_name}

## Purpose

{spec.purpose}

## Exports

{format_exports(spec.exports)}

## Behavior

{format_behaviors(spec.behaviors)}

## Contract

{format_contracts(spec.contracts)}

## Protocol

{format_protocol(spec.protocol) or "None"}

## Domain Context

{format_domain_context(spec.domain_context) or "None"}

{optional_sections}
```

#### Exports 형식

| 예시 | 설명 |
|------|------|
| `validateToken(token: string): Promise<Claims>` | 함수 |
| `Claims { userId: string, role: Role }` | 타입/인터페이스 |
| `TokenError extends Error` | 클래스 |
| `Role = "admin" \| "user"` | 타입 별칭 |

#### Behaviors 형식

| 카테고리 | 예시 |
|----------|------|
| success | `valid token → Claims object` |
| error | `expired token → TokenExpiredError` |
| edge | `empty token → InvalidTokenError` |

### Phase 5.5: IMPLEMENTS.md Planning Section 생성 (HOW 계획)

요구사항 분석 결과와 Phase 1.5의 dep-explorer 결과를 기반으로 IMPLEMENTS.md의 Planning Section을 생성합니다.

dep-explorer 결과 JSON에서 의존성 정보를 포맷팅합니다:
- **External**: `existing`과 `new` 배열을 합산하여 외부 의존성 목록을 구성합니다.
- **Internal**: `internal_deps` 배열에서 CLAUDE.md 경로와 symbols를 추출하여 내부 의존성 목록을 구성합니다.

```markdown
# {module_name}/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /impl 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Dependencies Direction

### External
{format_external_from_dep_result(dep_result["external_deps"])}

### Internal
{format_internal_from_dep_result(dep_result["internal_deps"])}

## Implementation Approach

### 전략
{spec.implementation_strategy}

### 고려했으나 선택하지 않은 대안
{spec.rejected_alternatives}

## Technology Choices

{format_technology_choices(spec.tech_choices) or "None"}

<!-- ═══════════════════════════════════════════════════════ -->
<!-- IMPLEMENTATION SECTION - /compile 이 업데이트            -->
<!-- (이 섹션은 /compile 시 자동 생성됨)                       -->
<!-- ═══════════════════════════════════════════════════════ -->

## Algorithm

(To be filled by /compile)

## Key Constants

(To be filled by /compile)

## Error Handling

(To be filled by /compile)

## State Management

(To be filled by /compile)

## Implementation Guide

(To be filled by /compile)
```

#### Dependencies Direction 형식

```markdown
### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 성숙한 라이브러리, 프로젝트 호환)

### Internal
- `utils/crypto/CLAUDE.md`: hashPassword, verifyPassword (해시 유틸리티)
- `config/CLAUDE.md`: loadConfig (환경 설정 로드)
```

**Internal deps 규칙:**
- dep-explorer 결과의 `claude_md_path`를 그대로 사용 (project-root-relative)
- dep-explorer 결과의 `symbols`를 colon 뒤에 나열
- 기존 CLAUDE.md가 없는 새 모듈이면 `{expected_path}/CLAUDE.md` 형태로 작성

#### Implementation Approach 형식

```markdown
### 전략
- HMAC-SHA256 기반 토큰 검증
- 메모리 캐시로 반복 검증 성능 최적화

### 고려했으나 선택하지 않은 대안
- RSA 서명: 키 관리 복잡성 → 내부 서비스라 HMAC 충분
- Redis 캐시: 단일 인스턴스 환경이라 메모리 캐시 충분
```

#### Technology Choices 형식

```markdown
| 선택 | 대안 | 선택 이유 |
|------|------|----------|
| jsonwebtoken | jose | 기존 코드베이스 호환성 |
| Map 캐시 | Redis | 단일 인스턴스 환경 |
```

### Phase 6: 스키마 검증 (1회)

`claude-md-core validate-schema` CLI를 호출하여 CLAUDE.md를 검증합니다. 검증 결과 JSON을 로드하여 `valid` 필드를 확인합니다. 검증이 실패하면 사용자에게 이슈를 보고하고 경고와 함께 진행합니다.

### Phase 6.5: Plan Preview & User Approval

파일 생성 전에 완성된 계획을 사용자에게 제시하고 승인을 받습니다.

#### 계획 요약 형식

텍스트로 다음 정보를 출력합니다:

```
=== 생성 계획 ===

대상 경로: {target_path}
액션: {created | updated}

Purpose: {purpose 요약}
Exports: {count}개 — {export 이름 나열}
Behaviors: {count}개 — {주요 시나리오 나열}
Dependencies: Internal {count}개, External {count}개
```

#### AskUserQuestion

질문: "이 계획으로 CLAUDE.md + IMPLEMENTS.md를 생성할까요?"
옵션:
1. 승인 — 파일 생성 진행
2. 범위 조정 — 추가/삭제할 항목 수집 후 Phase 5~6.5 재실행
3. 위치 변경 — 새 경로 수집 후 Phase 3~6.5 재실행
4. 취소 — 파일 생성 없이 종료

"범위 조정" 선택 시:
- 추가 AskUserQuestion: "어떤 부분을 변경할까요?" (Exports 추가/삭제, Behavior 추가/삭제, Purpose 변경)
- 변경 반영 후 Phase 5~6.5 재실행
- 최대 1회 루프백. 2번째 Plan Preview에서는 "승인"과 "취소" 옵션만 제시

"취소" 선택 시:
- ---impl-result--- 블록에 status: cancelled_by_user 반환

### Phase 7: 최종 저장 및 결과 반환

**Phase 6.5에서 승인된 경우에만 실행합니다.**

1. 대상 디렉토리를 생성합니다 (필요시).
2. **CLAUDE.md 저장**: `{target_path}/CLAUDE.md`에 Write합니다.
3. **IMPLEMENTS.md 저장**: `{target_path}/IMPLEMENTS.md`가 이미 존재하면 기존 내용을 읽어 Planning Section만 업데이트합니다. 존재하지 않으면 새로 생성합니다.

```
---impl-result---
claude_md_file: {target_path}/CLAUDE.md
implements_md_file: {target_path}/IMPLEMENTS.md
status: success
action: {created|updated}
validation: {passed|failed_with_warnings}
exports_count: {len(exports)}
behaviors_count: {len(behaviors)}
dependencies_count: {len(dependencies)}
tech_choices_count: {len(tech_choices)}
---end-impl-result---
```

## Context 효율성

의존성 탐색은 dep-explorer agent에 위임되어 impl agent의 context를 절약합니다.

| 항목 | impl agent | dep-explorer |
|------|-----------|-------------|
| 인덱스 로드 | - | ~6KB (scan-claude-md 인덱스) |
| CLAUDE.md Read | - | 관련 모듈만 Read (3-5개) |
| 외부 의존성 확인 | - | package.json 등 설정 파일 Read |
| 결과 소비 | dep-analysis JSON (~1KB) | - |

- dep-explorer가 탐색을 전담 → impl agent는 결과 JSON만 소비
- 소스코드 읽지 않음 — CLAUDE.md Exports만 참조
- 결과는 파일로 저장
