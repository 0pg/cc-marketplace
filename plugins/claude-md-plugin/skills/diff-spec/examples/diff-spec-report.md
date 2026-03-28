# 시맨틱 Diff: src/auth

**비교:** HEAD → 현재 (working copy)

## 요약

| 섹션 | 추가 | 제거 | 변경 | 상태 |
|------|------|------|------|------|
| Purpose | - | - | 1 | CHANGED |
| Requirements | 2 | 1 | 1 | BREAKING |
| Domain Context | 1 | 0 | 0 | MODIFIED |
| Conventions | 0 | 0 | 1 | MODIFIED |

## Purpose 변경

- 이전: "JWT 기반 인증 모듈"
+ 현재: "JWT 및 OAuth2 기반 인증 모듈"

## Requirements 변경

### ADDED
- `+ 동시 세션 최대 5개`
- `+ OAuth2 PKCE 필수`

### REMOVED
- `- 레거시 MD5 해시 지원` [BREAKING]

### MODIFIED
- `토큰 만료 최대 7일` → `토큰 만료 최대 14일`

### UNCHANGED
- `UTF-8 인코딩만 허용`

## Domain Context 변경

### ADDED
- `+ OAuth2 IdP: Google, GitHub 지원`

## Conventions 변경

### MODIFIED
- `Coding Rules`: 린트 규칙 추가
