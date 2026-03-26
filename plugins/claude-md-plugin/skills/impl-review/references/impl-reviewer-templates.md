<!--
  impl-reviewer-templates.md
  Consolidated reference for the impl-reviewer agent.
  Contains: Review dimensions (D1-D3), check definitions (v7 schema),
  severity levels, scoring formula, finding format, fix proposal format,
  result template, and quality anti-patterns.

  v7: CLAUDE.md has Purpose, Requirements, Domain Context.
  Review checks focus on these 3 sections + conditional sections.

  Loaded at runtime by the impl-reviewer agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/impl-review/references/impl-reviewer-templates.md"
-->

# Impl-Reviewer Templates & Reference

## Review Dimensions

### D1: Requirements Coverage (요구사항이 "N/A"이면 스킵)

원본 요구사항에서 핵심 기능/제약/맥락을 추출하여 CLAUDE.md 섹션과 대조.

| ID | Check | Severity | Criteria |
|----|-------|----------|----------|
| D1-1 | Purpose 정렬 | CRITICAL | Purpose가 요구사항의 핵심 의도를 반영하는가 |
| D1-2 | 요구사항 커버리지 | CRITICAL | 요구사항에 언급된 규칙/제한이 Requirements에 매핑되는가 |
| D1-3 | 맥락 캡처 | WARNING | 언급된 배경/근거가 Domain Context에 있는가 |
| D1-4 | 도메인 용어 | INFO | 요구사항의 도메인 용어가 문서에 보존되었는가 |

### D2: CLAUDE.md Quality

CLAUDE.md의 내재적 품질을 평가.

| ID | Check | Severity | Criteria |
|----|-------|----------|----------|
| D2-1 | 스키마 준수 | CRITICAL | 3개 always-required (Purpose, Requirements, Domain Context) + conditional 섹션 존재 (CLI 결과 반영) |
| D2-2 | Requirements 구체성 | CRITICAL | 각 requirement가 검증 가능하고 비즈니스 관점인가 |
| D2-3 | Requirements 근거 | WARNING | 각 requirement에 근거/이유가 명시되어 있는가 (e.g., PCI-DSS) |
| D2-4 | Purpose 명확성 | WARNING | 1-2 문장, 구체적 (generic이 아닌) |
| D2-5 | Domain Context 품질 | WARNING | 2-3문장, 코드만으로 알 수 없는 "왜"에 집중 |
| D2-6 | Domain Context 히스토리 금지 | WARNING | 변경 이력/날짜/버전 히스토리가 포함되어 있지 않은가 |
| D2-7 | "None" 섹션 감사 | WARNING | "None"으로 표시된 섹션이 실제로 해당 없는지 확인 |
| D2-8 | Requirements 자기완결성 | INFO | 상위 모듈의 관련 요구사항이 포함 반복되어 있는가 |

### D3: Internal Consistency

| ID | Check | Severity | Criteria |
|----|-------|----------|----------|
| D3-1 | Purpose ↔ Requirements 정렬 | CRITICAL | Requirements가 Purpose에서 논리적으로 도출 가능한가 |
| D3-2 | Domain Context ↔ Requirements 정렬 | WARNING | Domain Context의 결정 근거가 Requirements에 반영되는가 |
| D3-3 | Domain Context / Decision Log 중복 | INFO | 같은 정보가 CLAUDE.md Domain Context와 DEVELOPERS.md Decision Log 양쪽에 있으면 플래그 |
| D3-4 | Instructions ↔ Purpose 정렬 | INFO | project root의 Instructions가 Purpose와 일관되는가 |

## Scoring Formula

### Severity Deductions (per finding)

| Severity | Points | Description |
|----------|--------|-------------|
| CRITICAL | -15 | Must fix before /compile |
| WARNING | -8 | Should fix for quality |
| INFO | -3 | Nice to have improvement |

Each dimension starts at 100. Minimum score per dimension: 0.

### Dimension Weights

| Dimension | With Requirements | Without Requirements |
|-----------|-------------------|----------------------|
| D1 Requirements Coverage | 30% | — (skipped) |
| D2 CLAUDE.md Quality | 45% | 60% |
| D3 Internal Consistency | 25% | 40% |

### Grade Interpretation

| Score | Grade | Interpretation |
|-------|-------|----------------|
| 90-100 | Excellent | `/compile` 준비 완료 |
| 75-89 | Good | 경미한 개선 권장 |
| 60-74 | Needs Work | `/compile` 전 이슈 해결 필요 |
| 0-59 | Poor | 상당한 재작업 필요 |

## Finding Format

Each finding must follow this structure:

```
### [{dimension_id}] {check_name}

- **Severity**: CRITICAL | WARNING | INFO
- **Current**: {현재 문서의 해당 부분 인용 또는 "없음"}
- **Issue**: {구체적인 문제 설명}
- **Suggestion**: {수정 제안}
- **Rationale**: {왜 이것이 문제인지 근거}
```

## Fix Proposal Format

AskUserQuestion으로 수정 제안 시 사용하는 형식.

카테고리별로 묶어서 제안 (최대 4 questions/round):

```
질문: "{dimension} 관련 {N}개 이슈를 발견했습니다. 수정을 적용할까요?"
옵션:
  - "전체 수정 적용": 해당 카테고리의 모든 수정을 Edit으로 적용
  - "선택적 수정": 개별 finding에 대해 후속 질문
  - "건너뛰기": 수정 없이 결과만 기록
```

"선택적 수정" 후속 질문:
```
질문: "[{finding_id}] {check_name}: {issue_summary}. 수정할까요?"
옵션:
  - "수정 적용"
  - "건너뛰기"
```

## Result File Template

```markdown
# Impl Review Report

## Summary

| Metric | Value |
|--------|-------|
| Directory | {directory} |
| CLAUDE.md | {claude_md_path} |
| Requirements | {provided / N/A} |
| Overall Score | {score}/100 ({grade}) |
| Issues | {total} (CRITICAL: {n}, WARNING: {n}, INFO: {n}) |
| Fixes Applied | {n} |

## Dimension Scores

| Dimension | Score | Weight | Weighted |
|-----------|-------|--------|----------|
| D1 Requirements Coverage | {score} | {weight}% | {weighted} |
| D2 CLAUDE.md Quality | {score} | {weight}% | {weighted} |
| D3 Internal Consistency | {score} | {weight}% | {weighted} |
| **Overall** | | | **{overall}** |

## Findings

{각 finding을 Finding Format으로 나열}

## Fixes Applied

{수정 적용된 항목 목록, 없으면 "None"}
```

## Quality Anti-patterns

Agent가 판단할 때 참고하는 앵커. 좋은 vs 나쁜 예시.

### Bad: Generic Purpose
```
## Purpose
데이터를 처리하는 모듈입니다.
```

### Good: Specific Purpose
```
## Purpose
사용자 업로드 CSV 파일을 파싱하여 정규화된 트랜잭션 레코드로 변환. 중복 행 제거 및 필수 컬럼(date, amount, description) 검증.
```

### Bad: Vague Requirements
```
## Requirements
- 데이터를 잘 처리해야 한다
- 에러 처리가 필요하다
```

### Good: Specific Requirements
```
## Requirements
- 입력 CSV는 UTF-8만 허용, 최대 10MB
- 필수 컬럼 누락 시 에러 반환 (누락 컬럼명 포함)
- 중복 행은 첫 번째만 유지, 중복 수 로그 기록
- 날짜 형식은 ISO 8601만 허용
```

### Bad: Changelog in Domain Context
```
## Domain Context

### Decision Rationale
- TOKEN_EXPIRY: 7일 (PCI-DSS 요구사항)
- v2.1.0에서 만료 기간을 14일 → 7일로 변경
- v2.0.1에서 캐시 무효화 버그 수정
- v1.5.0에서 RSA → HMAC-SHA256으로 전환
```

### Good: Domain Context without history
```
## Domain Context
JWT 토큰은 PCI-DSS 준수를 위해 7일 만료 정책을 적용합니다.
HMAC-SHA256은 내부 서비스 간 통신이라 RSA 불필요합니다.
```

### Bad: Non-self-contained Requirements
```
## Requirements
- refresh token 만료 시간은 access token의 2배
```
(상위 모듈의 "access token 만료 7일" 요구사항을 참조하지만 자기완결 아님)

### Good: Self-contained Requirements
```
## Requirements
- access token 만료 최대 7일 (PCI-DSS)
- refresh token 만료 최대 14일 (access token의 2배)
```
