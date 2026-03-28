# Resolve 세션

## 입력: /validate 결과 (3개 이슈)

1. src/auth: Requirements VIOLATED — "토큰 만료 최대 7일" 제약이 코드에서 14일로 설정됨
2. src/utils: Domain Context STALE — "Redis 캐시 사용" 맥락이 코드에서 미사용
3. src/legacy: DEVELOPERS.md MISSING

## 대화형 해소

### [1/3] src/auth: Requirements VIOLATED

> "토큰 만료 최대 7일" 제약이 코드에서 14일로 설정됨
> 해소 방법: [Fix Code / Fix Doc / Skip]
> → Fix Code

/compile --path src/auth --conflict overwrite 실행... 완료.

### [2/3] src/utils: Domain Context STALE

> "Redis 캐시 사용" 맥락이 코드에서 미사용
> 해소 방법: [Update / Keep]
> → Update

CLAUDE.md Domain Context 업데이트 완료.

### [3/3] src/legacy: DEVELOPERS.md MISSING

> 해소 방법: [Generate / Skip]
> → Skip

## Resolve 결과

| 모듈 | Drift | 해소 방법 |
|------|-------|----------|
| src/auth | Requirements VIOLATED | Fix Code |
| src/utils | Domain Context STALE | Update |
| src/legacy | DEVELOPERS.md MISSING | Skip |

총 이슈: 3
  - Fix Code: 1
  - Update: 1
  - Skip: 1
