# src/auth

## Purpose

JWT 기반 인증 처리 모듈.

## Exports

| Name | Signature | Description |
|------|-----------|-------------|
| `validateToken` | `(token: string): Promise<Claims>` | JWT 토큰 검증 |

## Behavior

| Input | Output |
|-------|--------|
| 유효한 JWT 토큰 | Claims 객체 반환 |

## Dependencies

- **Internal**: `../utils/crypto` <!-- Parent reference - will fail validation -->
