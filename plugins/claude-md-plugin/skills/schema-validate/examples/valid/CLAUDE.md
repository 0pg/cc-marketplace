# src/auth

## Purpose

JWT 기반 인증 처리 모듈.

## Summary

JWT 토큰을 검증하여 Claims 객체를 반환하는 인증 모듈. 만료된 토큰에 대한 에러 처리 포함.

## Exports

| Name | Signature | Description |
|------|-----------|-------------|
| `validateToken` | `(token: string): Promise<Claims>` | JWT 토큰 검증 |

## Behavior

| Input | Output |
|-------|--------|
| 유효한 JWT 토큰 | Claims 객체 반환 |
| 만료된 토큰 | TokenExpiredError |

## Contract

None

## Protocol

None

## Domain Context

None