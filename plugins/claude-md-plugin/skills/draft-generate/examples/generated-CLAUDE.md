# src/auth

## Purpose

JWT 기반 인증 처리 모듈. 토큰 생성, 검증, 미들웨어를 제공합니다.

## Structure

```
src/auth/
├── index.ts       # 진입점, 토큰 생성/검증 함수 export
├── middleware.ts  # Express 인증 미들웨어
├── types.ts       # 인증 관련 타입 정의
├── constants.ts   # 인증 상수 (토큰 만료 시간 등)
└── jwt/           # JWT 세부 구현 (jwt/CLAUDE.md 참조)
```

## Exports

| Name | Signature | Description |
|------|-----------|-------------|
| `validateToken` | `(token: string): Promise<Claims>` | JWT 토큰 검증 |
| `generateToken` | `(payload: TokenPayload): string` | JWT 토큰 생성 |
| `authMiddleware` | `(req, res, next): void` | Express 인증 미들웨어 |

### Types

| Name | Definition |
|------|------------|
| `Claims` | `{ userId: string, role: Role, exp: number }` |
| `TokenPayload` | `{ userId: string, role: Role }` |

## Behavior

| Input | Output |
|-------|--------|
| 유효한 JWT 토큰 | Claims 객체 반환 |
| 만료된 토큰 | TokenExpiredError |
| 잘못된 형식의 토큰 | JsonWebTokenError |
| Authorization 헤더 없음 | 401 Unauthorized |

## Dependencies

- **External**: `jsonwebtoken`
- **Internal**: `./types`, `../utils/crypto`
