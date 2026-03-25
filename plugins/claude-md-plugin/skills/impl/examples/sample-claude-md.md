# auth

## Purpose

사용자 인증을 담당하는 모듈. JWT 토큰 검증과 세션 관리를 제공합니다.

## Constraints

- 토큰 만료 최대 7일 (PCI-DSS 컴플라이언스 요구사항)
- 동시 세션 최대 5개
- 유효한 토큰 → Claims 객체 반환 (userId, exp, permissions 포함)
- 잘못된 형식의 토큰 → InvalidTokenError
- 만료된 토큰 (refresh 없음) → TokenExpiredError
- 만료된 토큰 + refresh 옵션 → 새 토큰 쌍 반환
- token must be non-empty string
- 레거시 UUID v1 형식 지원 필요

## Domain Context

- PCI-DSS 컴플라이언스에 따라 토큰 만료 기간 제한
- 비밀번호는 bcrypt로 해시하여 저장 (utils/crypto 모듈 활용)
- 레거시 시스템과의 호환성을 위해 UUID v1 형식 지속 지원
