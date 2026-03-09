# Impl Review Report

## Summary

| Metric | Value |
|--------|-------|
| Directory | src/auth |
| CLAUDE.md | src/auth/CLAUDE.md |
| IMPLEMENTS.md | src/auth/IMPLEMENTS.md |
| Requirements | provided |
| Overall Score | 82/100 (Good) |
| Issues | 4 (CRITICAL: 1, WARNING: 2, INFO: 1) |
| Fixes Applied | 2 |

## Dimension Scores

| Dimension | Score | Weight | Weighted |
|-----------|-------|--------|----------|
| D1 Requirements Coverage | 85 | 30% | 25.5 |
| D2 CLAUDE.md Quality | 77 | 35% | 26.95 |
| D3 IMPLEMENTS.md Planning | 84 | 20% | 16.8 |
| D4 Cross-Document Consistency | 85 | 15% | 12.75 |
| **Overall** | | | **82** |

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

### [D3-6] 구현 누출 없음

- **Severity**: WARNING
- **Current**: Implementation Approach에 "jsonwebtoken의 verify() 함수로 HMAC-SHA256 검증" 기술
- **Issue**: Planning Section에 라이브러리 함수명 수준의 구현 디테일이 포함됨
- **Suggestion**: "서명 알고리즘 기반 토큰 무결성 검증" 수준으로 추상화
- **Rationale**: Planning Section은 전략 수준이어야 하며, 알고리즘/API 디테일은 Implementation Section에 속함

### [D4-4] Behavior ↔ Error Handling 방향

- **Severity**: INFO
- **Current**: Behavior에 에러 케이스 1개, Implementation Approach에 에러 언급 없음
- **Issue**: Behavior의 에러 시나리오가 Implementation Approach에서 예견되지 않음
- **Suggestion**: Implementation Approach에 "에러 처리: fail-fast 전략" 항목 추가
- **Rationale**: 에러 처리 방향이 Planning에 없으면 compiler가 임의로 결정하게 됨

## Fixes Applied

- [D2-2] `validateToken(token): Claims` → `validateToken(token: string): Claims` 수정 완료
- [D1-3] Behavior에 `만료된 JWT 입력 → TokenExpiredError (만료 시각 포함)` 추가 완료
