# auth/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /impl 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Dependencies Direction

### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 기존 프로젝트 호환, 성숙한 라이브러리)

### Internal
- `utils/crypto`: 해시 유틸리티 (hashPassword)

## Implementation Approach

### 전략
- HMAC-SHA256 기반 토큰 검증
- 메모리 캐시로 반복 검증 성능 최적화
- Fail-fast: 첫 번째 실패 시 즉시 반환

### 고려했으나 선택하지 않은 대안
- RSA 서명: 키 관리 복잡성 → 내부 서비스 간 통신이라 HMAC으로 충분
- jose 라이브러리: 기능은 풍부하나 기존 jsonwebtoken 마이그레이션 비용

## Technology Choices

| 선택 | 대안 | 선택 이유 |
|------|------|----------|
| jsonwebtoken | jose | 기존 코드베이스 호환성 |
| HMAC-SHA256 | RS256 | 키 관리 단순화 |
