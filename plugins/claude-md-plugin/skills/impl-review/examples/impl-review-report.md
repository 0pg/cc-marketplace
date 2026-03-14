# Impl Review Report

## Summary

| Metric | Value |
|--------|-------|
| Directory | src/auth |
| CLAUDE.md | src/auth/CLAUDE.md |
| Requirements | provided |
| Overall Score | 82/100 (Good) |
| Issues | 4 (CRITICAL: 1, WARNING: 2, INFO: 1) |
| Fixes Applied | 2 |

## Dimension Scores

| Dimension | Score | Weight | Weighted |
|-----------|-------|--------|----------|
| D1 Requirements Coverage | 85 | 30% | 25.5 |
| D2 CLAUDE.md Quality | 77 | 40% | 30.8 |
| D3 Internal Consistency | 84 | 30% | 25.2 |
| **Overall** | | | **81.5** |

## Findings

### [D1-3] 시나리오 커버리지

- **Severity**: WARNING
- **Current**: Behavior 섹션에 성공 케이스만 존재: "유효한 JWT → Claims 반환"
- **Issue**: 요구사항에 "만료된 토큰 처리"가 명시되어 있으나 error behavior 누락
- **Suggestion**: `만료된 JWT 입력 → TokenExpiredError (만료 시각 포함)` 추가
- **Rationale**: 요구사항의 핵심 시나리오가 Behavior에 반영되지 않으면 compiler가 에러 처리를 누락할 수 있음

### [D2-2] Export 구체성

- **Severity**: CRITICAL
- **Current**: `validateToken(token): Claims`
- **Issue**: 파라미터 타입 누락 (token의 타입이 string인지 불명확)
- **Suggestion**: `validateToken(token: string): Claims` 로 수정
- **Rationale**: 타입 누락은 compiler가 잘못된 시그니처를 생성하는 원인

### [D3-1] Exports ↔ Behavior 정렬

- **Severity**: WARNING
- **Current**: Exports에 `refreshToken` 함수가 있으나 Behavior에 대응 시나리오 없음
- **Issue**: Export된 함수에 대한 동작 시나리오가 누락되면 compiler가 임의로 구현
- **Suggestion**: Behavior에 `유효한 리프레시 토큰 → 새 액세스 토큰 반환` 시나리오 추가
- **Rationale**: 각 Export에 최소 1개 Behavior 시나리오가 있어야 compiler가 정확히 구현

### [D3-3] Domain Context ↔ Contract 정렬

- **Severity**: INFO
- **Current**: Domain Context에 "PCI-DSS 토큰 만료 7일" 제약이 있으나 Contract에 미반영
- **Issue**: Domain Context의 제약이 Contract에 반영되지 않으면 compiler가 제약을 무시할 수 있음
- **Suggestion**: Contract에 `validateToken: token.expiry <= 7days (PCI-DSS)` precondition 추가
- **Rationale**: Domain Context 제약은 Contract로 구체화되어야 compiler가 올바르게 구현

## Fixes Applied

- [D2-2] `validateToken(token): Claims` → `validateToken(token: string): Claims` 수정 완료
- [D1-3] Behavior에 `만료된 JWT 입력 → TokenExpiredError (만료 시각 포함)` 추가 완료
