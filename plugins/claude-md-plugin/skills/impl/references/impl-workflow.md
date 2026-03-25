<!-- impl-workflow.md
     Extracted from agents/impl.md for context efficiency.
     Contains detailed workflow pseudocode for the impl agent:
     - Phase 0: Scope Assessment (completeness + multi-module detection)
     - Phase 1: Requirements Analysis (v6: 4 elements)
     - Phase 1.5: dep-explorer delegation
     - Phase 2: Tiered Clarification (Tier 1→2→3, max 2 rounds)
     - Phase 3: Target path determination
     - Phase 4: Merge strategy
     - Phase 5: CLAUDE.md generation (v6 schema: Purpose, Constraints, Domain Context)
     - Phase 5.25: DEVELOPERS.md generation
     - Phase 5.5: compile-context generation (ephemeral spec)
     - Phase 6: Schema validation
     - Phase 6.5: Plan Preview & User Approval
     - Phase 7: Final save & result
     - Context efficiency notes
-->

## Workflow

### Phase 0: Scope Assessment

요구사항을 분석하기 전에 완성도와 스코프를 먼저 평가합니다.

#### 3차원 증거 기반 완성도 분류

요구사항을 3개 차원으로 평가합니다:

| 차원 | 평가 대상 | 평가 방법 |
|------|----------|----------|
| **D1: Purpose 명확도** | 모듈 책임이 특정되었는가 | 핵심 동사+목적어가 명확하면 "있음", 추상적이면 "추론 가능", 불명이면 "없음" |
| **D2: Constraints 구체성** | 제약/규칙이 구체적으로 명시되었는가 | 수치, 조건이 **리터럴로** 존재하면 "있음", 암시적이면 "추론 가능", 없으면 "없음" |
| **D3: Domain Context 포함** | 결정 근거, 도메인 규칙이 있는가 | 구체적 배경(PCI-DSS, SLA 등) 명시면 "있음", 암시적이면 "추론 가능", 없으면 "없음" |

각 차원을 `있음` / `추론 가능` / `없음`으로 평가 → completeness 결정:

| D1 | D2 | completeness | 다음 단계 |
|----|-----|-------------|----------|
| 있음 | 있음 | **high** | Phase 2 건너뛰기 (경로 미지정 시 LOCATION만 질문) |
| 있음 | 추론 가능 또는 없음 | **medium** | Phase 2 Tier 2부터 |
| 추론 가능 이하 | 무관 | **low** | Phase 2 Tier 1부터 |

**D3의 역할:** D3는 completeness 결정에는 영향을 주지 않습니다.
대신 Phase 2 Tier 3 질문 포함 여부를 결정합니다:
- D3="없음" → Phase 2에서 Tier 3 질문(DOMAIN_CONTEXT)을 반드시 포함
- D3="추론 가능" → Phase 2에서 Tier 3 질문을 선택적으로 포함
- D3="있음" → Tier 3 질문 생략 가능

#### `---scope-assessment---` 구조화 출력

```
---scope-assessment---
completeness: {high|medium|low}
scope: {single-module|multi-module}
evidence:
  D1_purpose: {있음|추론 가능|없음} — "{인용 근거}"
  D2_constraints: {있음|추론 가능|없음} — "{인용 근거}"
  D3_domain_context: {있음|추론 가능|없음} — "{인용 근거}"
next_phase: {Phase 2 진입점}
---end-scope-assessment---
```

#### 멀티 모듈 감지

요구사항에 독립된 도메인이 2개 이상 언급되면 multi-module로 판단합니다.

감지 신호:
- 나열형: "A, B, C를 지원"
- AND 연결: "A와 B 기능"
- 독립된 책임: 각 항목이 별도 Purpose를 가질 수 있음

multi-module 감지 시 AskUserQuestion:

질문: "여러 도메인이 감지되었습니다. 어떻게 진행할까요?"
옵션:
1. 모듈별 분해 (권장) — 첫 모듈만 생성, 나머지는 /impl 가이드 제공
2. 도메인 그룹 생성 — Purpose로 하위 모듈을 참조하는 상위 CLAUDE.md 생성
3. 단일 모듈 유지 — 모든 기능을 하나의 CLAUDE.md에

"모듈별 분해" 선택 시:
- 사용자에게 먼저 처리할 모듈 선택 요청
- 해당 모듈만 Phase 1~7 진행
- 최종 결과에 나머지 모듈용 /impl 명령어 가이드 포함

"도메인 그룹 생성" 선택 시:
- 상위 디렉토리에 CLAUDE.md 생성
  - Purpose: 도메인 그룹 설명 (예: "결제 도메인 — 카드 결제, 정산, 환불을 관할")
  - Constraints: 그룹 레벨 제약 (개별 모듈 공통)
  - Domain Context: 모듈 간 관계가 있으면 기술, 없으면 None
- 하위 모듈 CLAUDE.md는 생성하지 않음 → 나머지 모듈용 /impl 가이드 제공
- Phase 1~7을 그룹 노드 기준으로 진행

### Phase 1: Requirements Analysis

#### Step 1: 요구사항 형식 식별

요구사항의 형식을 먼저 판별하여 추출 전략을 결정합니다:

| 형식 | 판별 기준 | 추출 전략 |
|------|----------|----------|
| **User Story** | "~로서 ~를 ~하고 싶다" 패턴 | 역할→Purpose, 행위→Constraints, 가치→Domain Context |
| **Feature List** | 나열형 기능 목록, 번호 매김 | 각 항목→Constraints 후보, 항목 관계→Domain Context |
| **자연어** | 비정형 설명, 대화체 | 핵심 동사+목적어→Purpose, 조건절→Constraints |
| **기술 요구사항** | 수치, 제한, API 스펙 포함 | 리터럴 추출 (confirmed), 나머지 추론 |

**혼합형 요구사항:** 여러 형식이 섞여 있으면 `자연어`로 분류합니다.
단, 리터럴 수치/제한이 포함된 부분은 `기술 요구사항` 전략으로 추출합니다 (리터럴→confirmed).

#### Step 2: 4개 스펙 요소 순차 추출

각 요소마다 신뢰도(`confirmed` / `inferred` / `gap`)를 부여합니다:

| 추출 항목 | 추출 방법 | 신뢰도 기준 |
|-----------|----------|------------|
| **Purpose** | "핵심 동사+목적어" 패턴으로 모듈 책임 식별 | 명시적 문장→confirmed, 맥락 추론→inferred |
| **Constraints** | 수치 제한, 형식 제약, "반드시/필수" 키워드에서 추출 | 명시적 규칙→confirmed, 관례 추론→inferred |
| **Domain Context** | 결정 근거, 도메인 규칙, 호환성 요구, 규정(PCI-DSS 등) 추출 | 명시→confirmed, 표준 관행 추론→inferred |
| **Location** | 명시된 경로 추출, 없으면 모듈명에서 추론 | 경로 명시→confirmed, 이름 추론→inferred, 불명→gap |

#### Step 3: `---extraction-summary---` 구조화 출력

gap 항목이 Phase 2 질문 대상을 직접 결정합니다:

```
---extraction-summary---
format: {user-story|feature-list|natural-language|technical-spec}
purpose: {추출된 Purpose} [{confirmed|inferred|gap}]
constraints: {confirmed|inferred|gap}
domain_context: {confirmed|inferred|gap}
location: {path or "unknown"} [{confirmed|inferred|gap}]
gaps: [{Phase 2 질문 대상}]
---end-extraction-summary---
```

### Phase 1.5: 의존성 탐색 (dep-explorer 위임)

의존성 탐색은 dep-explorer agent에 위임합니다.
impl agent는 결과 JSON만 로드하여 사용합니다.

#### 호출 타이밍 분기

completeness에 따라 dep-explorer 호출 시점이 다릅니다:

| completeness | 호출 시점 | 이유 |
|-------------|----------|------|
| **high / medium** | Phase 1.5 (현재 위치) | Purpose가 확정되어 의존성 탐색 정확 |
| **low** | Phase 2 Round 1 응답 반영 후 | Purpose가 확정된 상태에서 의존성 탐색이 더 정확 |

`completeness=low`인 경우 Phase 2 Round 1에서 PURPOSE를 확정한 후 dep-explorer를 호출합니다.

#### dep-explorer 호출

dep-explorer agent를 Task로 호출합니다. 프롬프트에 사용자 요구사항(`user_requirement`), 프로젝트 루트(`project_root`), index 파일 경로(`claude_md_index_file`)를 전달합니다. 결과는 `.claude/extract-results/dep-analysis-{module_name}.json`에 저장됩니다.

Task 완료 후 결과 JSON을 로드하여 `dep_result`에 저장합니다.

**결과 활용:**
- `dep_result["internal_deps"]` → compile-context Dependencies Direction Internal 섹션
- `dep_result["external_deps"]` → compile-context Dependencies Direction External 섹션

### Phase 2: Tiered Clarification (계층화된 질문)

AskUserQuestion 제약(최대 4질문, 2-4옵션)을 고려하여 계층적으로 질문합니다.

#### Tier 구조

| Tier | 카테고리 | 질문 조건 | 질문 예시 |
|------|---------|----------|----------|
| **1 (범위)** | PURPOSE, LOCATION | completeness=low 이거나 경로 미지정 | "핵심 책임은?", "위치는?" |
| **2 (제약)** | CONSTRAINTS | Constraints 불명확 | "어떤 제약?", "에러 처리?" |
| **3 (맥락)** | DOMAIN_CONTEXT, DEPENDENCY | D3="없음"이면 필수, D3="추론 가능"이면 선택적 | "배경은?", "외부 라이브러리?" |

#### 라운드 실행 로직

Round 1: completeness=low이면 Tier 1 질문 (최대 4개)
         completeness=medium이면 Round 1 건너뛰고 Round 2로
         completeness=high이면 Phase 2 건너뛰기
           단, 경로가 명시되지 않았고 모듈명에서 추론도 불확실하면 LOCATION 질문 1개만 실행

Round 2: Tier 2 질문(최대 2개) + Tier 3 질문(최대 2개) 합산하여 최대 4개 질문
         Tier 3 포함 여부는 D3 값으로 결정 (D3="없음" → 필수, "추론 가능" → 선택적, "있음" → 생략)
         필요 없는 카테고리는 건너뜀

최대 2라운드 후에도 모호하면: 최선의 추측으로 진행 + Phase 6.5에서 사용자 확인

#### 질문 안 함 (기존 유지)
- 요구사항에 구체적 수치/제한이 있는 경우
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

대상 경로에 CLAUDE.md가 존재하고 "update" 모드이면, 기존 CLAUDE.md를 Read한 후 새 스펙과 smart merge합니다. 존재하지 않으면 새 스펙을 그대로 사용합니다.

#### Smart Merge 전략

| 섹션 | 병합 전략 |
|------|----------|
| Purpose | 기존 유지 또는 확장 (사용자 선택) |
| Constraints | 기존 유지 + 신규 추가 (중복 제거) |
| Domain Context | 기존 유지 또는 확장 (사용자 선택) |

### Phase 5: CLAUDE.md 생성 (v6 스키마)

#### Step 0: 기존 패턴 참조

scan 인덱스에 기존 모듈이 3개 이상 있으면, 기존 CLAUDE.md에서 패턴을 추론합니다:

- **Constraints 스타일**: 불릿 리스트 vs 문단
- **Domain Context 수준**: 상세 vs 간략
- **Convention 준수 여부**: Convention이 있으면 우선 참조

#### Step 1: 템플릿 기반 생성

템플릿 기반으로 CLAUDE.md를 생성합니다:

```markdown
# {module_name}

## Purpose

{spec.purpose}

## Constraints

{format_constraints(spec.constraints) or "None"}

## Domain Context

{format_domain_context(spec.domain_context) or "None"}
```

### Phase 5.25: DEVELOPERS.md 생성 (WHY 맥락)

CLAUDE.md와 함께 DEVELOPERS.md를 생성합니다. /impl에서는 최소한의 내용으로 생성합니다.

**DEVELOPERS.md 스키마 참조:**
```bash
cat "${CLAUDE_PLUGIN_ROOT}/templates/developers-md-schema.md"
```

**생성 규칙:**
- **Domain Context**: CLAUDE.md Domain Context의 확장. 없으면 None
- **Invariants**: None (구현 전이므로 내부 불변식 미확정)
- **Decision Log**: compile-context의 Technology Choices에서 시드. 각 선택을 ADR 형식(맥락/결정/근거)으로 변환
- **Operations**: None (구현 전이므로 운영 정보 미확정)
- **File Map**: None (아직 소스 파일이 없으므로)

**Decision Log 변환 예시:**

compile-context Technology Choices:
```
| jsonwebtoken | jose | 기존 코드베이스 호환성 |
```

→ DEVELOPERS.md Decision Log:
```markdown
### jsonwebtoken 선택
- **맥락**: JWT 토큰 검증 라이브러리 선택 필요
- **결정**: jsonwebtoken 사용
- **근거**: 기존 코드베이스 호환성 (jose 대비)
```

### Phase 5.5: compile-context 생성 (세션 임시 스펙, optional)

요구사항 분석 결과와 Phase 1.5의 dep-explorer 결과를 기반으로 compile-context를 생성합니다. compile-context는 세션 임시 파일로 /compile 시 참고용입니다.

```markdown
# .claude/tmp/compile-context-{dir-hash}.md
<!-- /compile 시 참고할 구현 방향 (session temp) -->

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
```

#### Dependencies Direction 형식 (compile-context)

```markdown
### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 성숙한 라이브러리, 프로젝트 호환)

### Internal
- `utils/crypto/CLAUDE.md`: hashPassword, verifyPassword (해시 유틸리티)
- `config/CLAUDE.md`: loadConfig (환경 설정 로드)
```

### Phase 6: 스키마 검증 (1회)

`claude-md-core validate-schema` CLI를 호출하여 CLAUDE.md를 검증합니다. 검증 결과 JSON을 로드하여 `valid` 필드를 확인합니다.

검증이 실패하면 `claude-md-core fix-schema` CLI를 실행하여 누락된 allow-none 섹션을 자동 추가합니다:
```bash
$CLI_PATH fix-schema --file {target_dir}/CLAUDE.md
```
auto-fix 후에도 검증이 실패하면 사용자에게 이슈를 보고하고 경고와 함께 진행합니다.

### Phase 6.5: Plan Preview & User Approval

파일 생성 전에 완성된 계획을 사용자에게 제시하고 승인을 받습니다.

#### 계획 요약 형식

```
=== 생성 계획 ===

[Scope Assessment]
완성도: {completeness} (D1:{status}, D2:{status}, D3:{status})

대상 경로: {target_path}
액션: {created | updated}

Purpose: {purpose 요약}
Constraints: {count}개 — {주요 제약 나열}
Domain Context: {있음 | None}
Dependencies: Internal {count}개, External {count}개

⚠ 추론 항목 (사용자 확인 권장):
  - {inferred 항목 나열}
  - {gap에서 최선 추측으로 채운 항목 나열}
```

`⚠ 추론 항목` 섹션은 `inferred` 또는 `gap` 항목이 있을 때만 표시합니다.
모든 항목이 `confirmed`이면 해당 섹션을 생략합니다.

#### AskUserQuestion

질문: "이 계획으로 CLAUDE.md + compile-context를 생성할까요?"
옵션:
1. 승인 — 파일 생성 진행
2. 범위 조정 — 추가/삭제할 항목 수집 후 Phase 5~6.5 재실행
3. 위치 변경 — 새 경로 수집 후 Phase 3~6.5 재실행
4. 취소 — 파일 생성 없이 종료

"범위 조정" 선택 시:
- 추가 AskUserQuestion: "어떤 부분을 변경할까요?" (Constraints 추가/삭제, Domain Context 변경, Purpose 변경)
- 변경 반영 후 Phase 5~6.5 재실행
- 최대 1회 루프백. 2번째 Plan Preview에서는 "승인"과 "취소" 옵션만 제시

"취소" 선택 시:
- ---impl-result--- 블록에 status: cancelled_by_user 반환

### Phase 7: 최종 저장 및 결과 반환

**Phase 6.5에서 승인된 경우에만 실행합니다.**

1. 대상 디렉토리를 생성합니다 (필요시).
2. **CLAUDE.md 저장**: `{target_path}/CLAUDE.md`에 Write합니다.
3. **DEVELOPERS.md 저장**: `{target_path}/DEVELOPERS.md`에 Write합니다.
4. **compile-context 저장**: `.claude/tmp/compile-context-{dir-hash}.md`에 Write합니다 (session temp, optional).

```
---impl-result---
claude_md_file: {target_path}/CLAUDE.md
developers_md_file: {target_path}/DEVELOPERS.md
compile_context_file: .claude/tmp/compile-context-{dir-hash}.md
status: success
action: {created|updated}
validation: {passed|failed_with_warnings}
constraints_count: {len(constraints)}
domain_context: {present|none}
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
- 소스코드 읽지 않음 — CLAUDE.md만 참조
- 결과는 파일로 저장
