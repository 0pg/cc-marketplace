# DEVELOPERS.md Schema

## Purpose

DEVELOPERS.md는 CLAUDE.md와 1:1로 매핑되는 "왜(WHY)" 문서입니다.
개발자 온보딩과 유지보수에 필요한 맥락 정보를 제공합니다.

## 핵심 원칙

**현재 상태만 기록, 히스토리는 git에 의존.**
- CLAUDE.md, DEVELOPERS.md는 항상 "현재 상태"만을 기록
- 과거 맥락(변경 이력, 날짜, 버전 히스토리)은 문서에 포함하지 않음
- 히스토리가 필요하면 `git log`, `git blame`을 사용

## 문서 쌍 규칙 (INV-3)

```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
```

DEVELOPERS.md 부재 시 경고로 보고 (`--strict` 모드).

## SOT 구조

```
CLAUDE.md (WHAT) + DEVELOPERS.md (WHY) → Source Code
```

| 문서 | 역할 | 대상 |
|------|------|------|
| CLAUDE.md | WHAT (사전학습 인덱스) | AI 에이전트, 외부 소비자 |
| DEVELOPERS.md | WHY (인간 지식 저장소) | 내부 개발자용 상세 근거 |

## 필수 섹션 (5개, 모두 None 허용)

### ## Domain Context (필수, None 허용)

CLAUDE.md Domain Context의 확장. 모듈 도메인 맥락을 상세하게 기술합니다.

```markdown
## Domain Context
JWT 토큰은 PCI-DSS 준수를 위해 7일 만료 정책을 적용합니다.
Redis 캐시를 사용하여 인증 지연을 최소화합니다.
내부 서비스 간 통신은 mTLS로 보호됩니다.
```

### ## Invariants (필수, None 허용)

모듈 내부 불변식. 코드가 항상 만족해야 할 조건입니다.

```markdown
## Invariants
- 토큰 생성 시 expiry는 반드시 7일 이내
- refresh token은 1회 사용 후 즉시 무효화
- 캐시 TTL은 토큰 만료 시간보다 항상 짧아야 함
```

### ## Decision Log (필수, None 허용)

ADR(Architecture Decision Record) 스타일. 각 결정을 소제목으로, 고정 스키마(맥락/결정/근거) 준수.
날짜 필드 없음 — 현재 유효한 결정만 기록. 철회된 결정은 삭제 (git에 이력 남음).

> **Bilingual support**: 필드명은 English 권장, Korean alias 허용.
> - `Context` | `맥락`
> - `Decision` | `결정`
> - `Rationale` | `근거`

> **Domain Context 중복 금지**: CLAUDE.md Domain Context에 이미 있는 값은 Decision Log에서 반복하지 않고 참조만 합니다.

```markdown
## Decision Log

### HMAC-SHA256 선택
- **맥락**: 내부 서비스 간 토큰 검증 방식 필요
- **결정**: HMAC-SHA256 사용
- **근거**: 내부 서비스라 RSA 키 관리 복잡성 불필요. 성능도 우수

### 메모리 캐시
- **맥락**: 반복 토큰 검증 성능 최적화 필요
- **결정**: Map 기반 인메모리 캐시
- **근거**: 단일 인스턴스 환경이라 Redis는 오버스펙
```

### ## Operations (필수, None 허용)

3개 서브섹션 (bilingual 허용): Gotchas, Deployment|배포, Monitoring|모니터링.

```markdown
## Operations

### Gotchas
- 토큰 만료 시간은 UTC 기준

### 배포
- SECRET_KEY 환경변수 필수
- 배포 시 캐시 워밍업 5분 필요

### 모니터링
- `auth.validation.duration` 메트릭 확인
- 에러율 > 5% 시 알람
```

### ## File Map (필수, None 허용)

테이블 형식. 파일별 역할과 내부 의존관계.

```markdown
## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
| index.ts | 진입점, 라우팅 | validator.ts, types.ts |
| validator.ts | 토큰 검증 로직 | types.ts |
| types.ts | 타입 정의 | - |
```

## 스킬별 활용

| 스킬 | DEVELOPERS.md 활용 | 상세 |
|------|-------------------|------|
| `/impl` | Decision Log 생성 | CLAUDE.md와 함께 DEVELOPERS.md(최소 Decision Log) 생성 |
| `/decompile` | 전체 생성 | 소스코드에서 5섹션 모두 추출 |
| `/validate` | drift 검증 확장 | File Map ↔ 실제 파일구조, INV-3 검증 |
| `/bugfix` | L2 진단 | 3-layer 분석의 L2 계층 |
| `/compile` | 참조 안 함 | — |

## 생명주기

CLAUDE.md와 동일한 생성/수정/삭제 주기를 따릅니다.

| 명령어 | DEVELOPERS.md |
|--------|---------------|
| /impl | 생성 (최소 Decision Log, 나머지 None) |
| /decompile | 전체 생성 (5섹션) |
| /bugfix | L2 진단 참조 |
| /validate | drift 검증 (INV-3 + File Map drift) |
