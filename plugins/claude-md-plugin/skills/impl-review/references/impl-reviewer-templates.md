<!--
  impl-reviewer-templates.md
  Consolidated reference for the impl-reviewer agent.
  Contains: Review dimensions (D1-D4), check definitions, severity levels,
  scoring formula, finding format, fix proposal format, result template,
  and quality anti-patterns.

  Loaded at runtime by the impl-reviewer agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/impl-review/references/impl-reviewer-templates.md"
-->

# Impl-Reviewer Templates & Reference

## Review Dimensions

### D1: Requirements Coverage (요구사항이 "N/A"이면 스킵)

원본 요구사항에서 핵심 기능/시나리오/제약을 추출하여 CLAUDE.md 섹션과 대조.

| ID | Check | Severity | Criteria |
|----|-------|----------|----------|
| D1-1 | Purpose 정렬 | CRITICAL | Purpose가 요구사항의 핵심 의도를 반영하는가 |
| D1-2 | 기능 커버리지 | CRITICAL | 요구사항에 언급된 기능이 Exports에 매핑되는가 |
| D1-3 | 시나리오 커버리지 | WARNING | 요구사항에 내포된 에러/엣지 케이스가 Behavior에 있는가 |
| D1-4 | 제약 캡처 | WARNING | 언급된 제약이 Contract 또는 Domain Context에 있는가 |
| D1-5 | 도메인 용어 | INFO | 요구사항의 도메인 용어가 문서에 보존되었는가 |

### D2: CLAUDE.md Quality

CLAUDE.md의 내재적 품질을 평가.

| ID | Check | Severity | Criteria |
|----|-------|----------|----------|
| D2-1 | 스키마 준수 | CRITICAL | 6개 필수 섹션 존재 (CLI 결과 반영) |
| D2-2 | Export 구체성 | CRITICAL | 각 export에 파라미터 타입 + 반환 타입 명시 |
| D2-3 | Export 설명 | WARNING | 각 export에 역할/목적 설명 포함 |
| D2-4 | Behavior 완성도 | WARNING | success + error 케이스 모두 존재 |
| D2-5 | Behavior 형식 | WARNING | "input → output" 패턴 준수 |
| D2-6 | Purpose 명확성 | WARNING | 1-2 문장, 구체적 (generic이 아닌) |
| D2-7 | Contract 구체성 | INFO | 함수별 precondition/postcondition 명시 |
| D2-8 | Domain Context | INFO | 비자명한 결정에 대한 근거 문서화 |
| D2-9 | "None" 섹션 감사 | INFO | "None"으로 표시된 섹션이 실제로 해당 없는지 확인 |

### D3: Internal Consistency

| ID | Check | Severity | Criteria |
|----|-------|----------|----------|
| D3-1 | Exports ↔ Dependencies 정렬 | CRITICAL | 의존성에서 import하는 심볼이 실제 Exports에 존재 |
| D3-2 | Purpose ↔ Exports 정렬 | WARNING | Exports가 Purpose에서 논리적으로 도출되는가 |
| D3-3 | Behavior ↔ Contract 정합성 | WARNING | 에러 Behavior에 대응하는 Contract/throws 존재 |
| D3-4 | Domain Context 활용 | INFO | Domain Context 제약이 Contract 또는 Behavior에 반영 |

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

### Bad: Exports without types
```
### Functions
- processData: 데이터를 처리합니다
- validate: 검증합니다
```

### Good: Exports with full signatures
```
### Functions
- processData(input: RawCsvRow[]): NormalizedTransaction[] — CSV 행 배열을 정규화된 트랜잭션으로 변환
- validate(row: RawCsvRow): ValidationResult — 단일 행의 필수 컬럼 존재/형식 검증
```

### Bad: Incomplete Behavior (success only)
```
## Behavior
- CSV 파일 입력 → 트랜잭션 목록 반환
```

### Good: Complete Behavior (success + error)
```
## Behavior
- 유효한 CSV 입력 → 정규화된 트랜잭션 목록 반환
- 빈 CSV 입력 → 빈 배열 반환
- 필수 컬럼 누락 → ValidationError (누락 컬럼명 포함)
- 중복 행 존재 → 첫 번째 행만 유지, 중복 수 로그
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

### Decision Rationale
- TOKEN_EXPIRY: 7일 (PCI-DSS 컴플라이언스 요구사항)
- HMAC-SHA256: 내부 서비스 간 통신이라 RSA 불필요

### Compatibility
- UUID v1 형식 지원 필요 (2023 마이그레이션)
```
