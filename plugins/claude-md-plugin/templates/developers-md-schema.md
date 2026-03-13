# DEVELOPERS.md Schema

## Purpose

DEVELOPERS.md는 CLAUDE.md와 1:1로 매핑되는 "왜(WHY)" 문서입니다.
개발자 온보딩과 유지보수에 필요한 맥락 정보를 제공합니다.

## 문서 쌍 규칙

```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
```

## SOT 구조

```
CLAUDE.md (WHAT) + DEVELOPERS.md (WHY) → Source Code
```

| 문서 | 역할 | 수명 |
|------|------|------|
| CLAUDE.md | WHAT — 인터페이스, 스펙 | 영구 |
| DEVELOPERS.md | WHY — 파일관계, 결정근거, 운영 | 영구 |

## 필수 섹션 (4개)

### ## File Map

파일별 역할 및 의존관계, 코드 패스를 기술합니다.

```markdown
## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
| index.ts | 진입점, 라우팅 | auth.ts, db.ts |
| auth.ts | JWT 검증 로직 | config.ts |
```

### ## Data Structures

내부 자료구조 관계, 데이터 흐름을 기술합니다.

```markdown
## Data Structures

### TokenPayload → Claims 변환
TokenPayload(raw JWT) → Claims(validated) → UserContext(enriched)
```

### ## Decision Log

비직관적 매직넘버 근거, 분기로직 히스토리를 기술합니다.

```markdown
## Decision Log

| 결정 | 근거 | 날짜 |
|------|------|------|
| 토큰 만료 7일 | PCI-DSS 요구사항 | 2024-01 |
| bcrypt rounds=12 | 보안팀 권고 | 2024-03 |
```

### ## Operations

배포/모니터링, 트러블슈팅, 알려진 gotchas를 기술합니다.

```markdown
## Operations

### 알려진 Gotchas
- Redis 연결 풀 크기가 10 미만이면 토큰 검증 지연 발생
- 환경변수 JWT_SECRET 미설정 시 기본키 사용 (개발 전용)
```

## 생명주기

CLAUDE.md와 동일한 생성/수정/삭제 주기를 따릅니다.

| 명령어 | DEVELOPERS.md |
|--------|---------------|
| /decompile | (향후) 생성 |
| /bugfix | L2 진단 참조 (있으면 활용, 없으면 skip) |
| /validate | (향후) drift 검증 |
