# auth

## Purpose

사용자 인증을 담당하는 모듈. JWT 토큰 검증과 세션 관리를 제공합니다.

## Structure

- jwt/: JWT 토큰 처리 (상세는 jwt/CLAUDE.md 참조)
- session.ts: 세션 관리 로직
- types.ts: 인증 관련 타입 정의

## Exports

### Functions

#### validateToken
`validateToken(token: string) -> Claims`

JWT 토큰을 검증하고 Claims를 추출합니다.
- **입력**: Bearer 토큰 (Authorization 헤더에서 추출된 문자열)
- **출력**: 사용자 식별 정보와 권한 목록을 포함하는 Claims
- **역할**: API 요청의 인증 게이트키퍼

### Types

#### Claims
`Claims { userId: string, exp: number, permissions: Permission[] }`

인증된 사용자의 신원과 권한을 나타내는 타입

## Behavior

### 정상 케이스
- 유효한 토큰 → Claims 객체 반환
- 만료된 토큰 + refresh 옵션 → 새 토큰 쌍 반환

### 에러 케이스
- 잘못된 형식의 토큰 → InvalidTokenError
- 만료된 토큰 (refresh 없음) → TokenExpiredError

## Contract

### validateToken
- **Preconditions**: token must be non-empty string
- **Postconditions**: returns Claims with valid userId

## Protocol

None

## Domain Context

### Decision Rationale
- TOKEN_EXPIRY: 7일 (PCI-DSS 컴플라이언스 요구사항)

### Constraints
- 동시 세션 최대 5개

### Compatibility
- 레거시 UUID v1 형식 지원 필요

## Dependencies

- external:
  - `jsonwebtoken@9.0.0`: sign, verify

- internal:
  - `utils/crypto/CLAUDE.md`: hashPassword
