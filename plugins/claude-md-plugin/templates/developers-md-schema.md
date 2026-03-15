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

DEVELOPERS.md 부재 시 에러로 보고 (`--strict` 모드).

## SOT 구조

```
CLAUDE.md (WHAT) + DEVELOPERS.md (WHY) → Source Code
```

| 문서 | 역할 | 대상 |
|------|------|------|
| CLAUDE.md | WHAT (스펙) | 외부 소비자 (다른 모듈, /compile) |
| CLAUDE.md Domain Context | 간략 맥락 | 외부 소비자용 인터페이스 카탈로그의 일부 |
| DEVELOPERS.md | WHY (맥락) | 내부 개발자용 상세 근거 |
| compile-context | HOW (구현 방향) | 세션 임시, /impl→/compile 핸드오프 |

## 필수 섹션 (4개)

### ## File Map (필수, None 불가)

테이블 형식. 파일별 역할과 내부 의존관계.

```markdown
## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
| index.ts | 진입점, 라우팅 | validator.ts, types.ts |
| validator.ts | 토큰 검증 로직 | types.ts |
| types.ts | 타입 정의 | - |
```

### ## Data Structures (필수, None 허용)

내부 자료구조 관계도. Exports에 노출되지 않는 내부 타입과 변환 흐름.

```markdown
## Data Structures

### 내부 타입
- `RawToken`: 파싱 전 원시 토큰 문자열
- `DecodedPayload`: JWT 디코딩 후 중간 객체

### 변환 흐름
RawToken → DecodedPayload → Claims (export)
```

### ## Decision Log (필수, None 허용)

ADR(Architecture Decision Record) 스타일. 각 결정을 소제목으로, 고정 스키마(맥락/결정/근거) 준수.
날짜 필드 없음 — 현재 유효한 결정만 기록. 철회된 결정은 삭제 (git에 이력 남음).

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

3개 서브섹션: Gotchas, 배포, 모니터링.

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

## 스킬별 활용

| 스킬 | DEVELOPERS.md 활용 | 상세 |
|------|-------------------|------|
| `/impl` | Decision Log 생성 | CLAUDE.md와 함께 DEVELOPERS.md(최소 Decision Log) 생성 |
| `/decompile` | 전체 생성 | 소스코드에서 4섹션 모두 추출 |
| `/validate` | drift 검증 확장 | File Map ↔ 실제 파일구조, INV-3 검증 |
| `/bugfix` | L2 진단 | 3-layer 분석의 L2 계층 |
| `/compile` | 참조 안 함 | — |

## 생명주기

CLAUDE.md와 동일한 생성/수정/삭제 주기를 따릅니다.

| 명령어 | DEVELOPERS.md |
|--------|---------------|
| /impl | 생성 (최소 Decision Log, 나머지 None) |
| /decompile | 전체 생성 (4섹션) |
| /bugfix | L2 진단 참조 |
| /validate | drift 검증 (INV-3 + File Map drift) |
