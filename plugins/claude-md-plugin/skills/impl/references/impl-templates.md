# Impl Templates

## CLAUDE.md Schema (v4)

| 섹션 | 존재 규칙 | None 허용 | 설명 |
|------|----------|----------|------|
| `## Purpose` | 항상 필수 | X | 모듈의 존재 이유 (비즈니스 가치) |
| `## Requirements` | 항상 필수 | O | 비즈니스 요구사항 (사용자 관점, 검증 가능한 문장) |
| `## Domain Context` | 항상 필수 | O | 비즈니스 제약 배경 |
| `## Conventions` | project/module root 필수 | X | 6개 필수 서브섹션 |
| `## Instructions` | project root only | X | AI 행동 지시 |

## DEVELOPERS.md Schema

| 섹션 | 필수 | None 허용 | 내용 |
|------|------|----------|------|
| `## Constraints` | O | O | 정밀한 입출력 계약 — 테스트 변환 가능 |
| `## Technical Context` | O | O | 기술 선택과 근거 |
| `## Decision Log` | X | O | ADR 스타일 |
| `## Operations` | X | O | Gotchas, 배포 |

## Scope Assessment 기준

| Dimension | 있음 | 추론 가능 | 없음 |
|-----------|------|----------|------|
| D1 (Purpose) | 명시적 목적 서술 | 키워드에서 유추 가능 | 목적 불명 |
| D2 (Interface) | 리터럴 시그니처 | 동사/명사에서 추론 | 인터페이스 미언급 |
| D3 (Constraints) | 수치/규칙 명시 | 도메인에서 유추 | 제약 미언급 |

Completeness = (D1 + D2 + D3):
- **high**: 3개 모두 "있음"
- **medium**: 1-2개 "있음" 또는 "추론 가능"
- **low**: 대부분 "없음"

## Tiered Clarification

| Tier | 대상 | 질문 예시 |
|------|------|----------|
| Tier 1 | 핵심 책임, 위치, 범위 | "이 모듈의 핵심 책임은?" |
| Tier 2 | 인터페이스 시그니처, 에러 | "어떤 함수를 export?" |
| Tier 3 | 도메인 컨텍스트, 비즈니스 규칙 | "왜 이 제약이 필요한가?" |

## 예시: 생성된 CLAUDE.md

```markdown
# auth

## Purpose

JWT 토큰 기반 인증을 제공하여 API 요청의 사용자 신원을 검증한다.

## Requirements

- 유효한 JWT 토큰이 포함된 요청은 디코딩된 사용자 정보와 함께 통과시킨다
- 만료된 토큰은 401 Unauthorized 에러를 반환한다
- 서명이 유효하지 않은 토큰은 거부한다

## Domain Context

- RS256 알고리즘 사용 (조직 보안 정책)
- 토큰 만료 시간은 운영팀 요구로 최대 24시간
```

## 예시: 생성된 DEVELOPERS.md

```markdown
# auth

## Constraints

- `validateToken(token: string)` → `{ userId: string, role: string }` 또는 `AuthError` throw
- 토큰 만료 판정: `exp` 클레임 < 현재 시각 (UTC)
- 서명 검증: RS256, 공개키는 환경변수 `JWT_PUBLIC_KEY`에서 로드

## Technical Context

- jsonwebtoken@9.0.0 라이브러리 사용 (jose 대비 동기 API로 미들웨어 호환성 우수)
- Express 미들웨어 패턴 적용

## Decision Log

None

## Operations

None
```
