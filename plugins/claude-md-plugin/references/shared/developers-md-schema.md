# DEVELOPERS.md Schema

## Purpose

DEVELOPERS.md는 CLAUDE.md와 1:1로 매핑되는 Derived Spec 문서입니다.
CLAUDE.md Requirements를 시스템 레벨로 구체화하며, /compile이 테스트를 생성하는 원천입니다.

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
CLAUDE.md (Primary SSOT) → DEVELOPERS.md (Derived Spec) → Source Code (Derived Artifact)
```

| 문서 | 역할 | 대상 |
|------|------|------|
| CLAUDE.md | Requirements (PM의 요구사항) | PM, AI 에이전트 |
| DEVELOPERS.md | Constraints + Technical Context (개발자 명세) | 개발자, /compile |

## 섹션 (2 필수 + 3 선택, 모두 None 허용)

### ## Constraints (필수, None 허용)

CLAUDE.md Requirements를 시스템 레벨로 구체화한 정밀한 입출력 계약.
**테스트로 변환 가능해야 합니다.**

```markdown
## Constraints
- 유효한 JWT access token → Claims{userId, exp, permissions}
- 만료된 access token + 유효한 refresh token → TokenPair{accessToken, refreshToken}
- 만료된 access token + 만료된 refresh token → AuthenticationError{REFRESH_EXPIRED}
- refresh token은 one-time use — 사용 즉시 무효화
- 잘못된 형식의 토큰 → InvalidTokenError
- 활성 세션 ≥ 5 + 새 세션 → SessionLimitError{currentCount}
- access token TTL ≤ 168h (7일)
- refresh token TTL ≤ 720h (30일)
```

**Constraints 작성 원칙:**
- 입력 → 출력/에러 형식 (동작)
- 정확한 타입명, 에러 코드
- 수치는 구체적 (최대/최소/경계값)
- 모호함 불허 (모호하면 CLAUDE.md Requirements에 남겨둠)

**패턴:**
```
[입력/조건] → [결과/출력]           (동작)
[위반 조건] → [에러 타입]{세부정보}  (에러)
[속성] [비교연산자] [값]             (제한)
```

### ## Data Schemas (선택, None 허용)

모듈의 공개 타입 정의. **Constraints가 참조하는 타입들을 중앙화**한다.
Constraints는 행동(`f(x) → Claims`)에 집중하고, Data Schemas는 타입 구조 정의에 집중한다.
`/decompile`이 `analyze-code`의 ExportedType(interface/type/struct/enum)에서 자동 추출.

```markdown
## Data Schemas

### Claims
| 필드 | 타입 | 설명 |
|------|------|------|
| userId | string | 사용자 식별자 |
| exp | number | 만료 시각 (Unix timestamp, UTC) |
| permissions | string[] | 권한 목록 |

### AuthError
| 필드 | 타입 | 값 |
|------|------|-----|
| code | AUTH_ERROR_CODE | EXPIRED \| INVALID_SIGNATURE \| SESSION_LIMIT |
| message | string | 사람 읽기용 메시지 |
```

### ## Technical Context (필수, None 허용)

기술 선택과 그 근거. 라이브러리, 알고리즘, 아키텍처 패턴 등.

```markdown
## Technical Context
- JWT 서명: RS256 (PCI-DSS 요구에 따라 비대칭 키)
- 비밀번호: bcrypt, cost factor 12 (보안팀 승인 2024-01)
- 레거시 호환: UUID v1 형식 유지 (utils/legacy-id 모듈)
```

### ## Decision Log (선택, None 허용)

ADR(Architecture Decision Record) 스타일. 각 결정을 소제목으로, 고정 스키마(맥락/결정/근거) 준수.
날짜 필드 없음 — 현재 유효한 결정만 기록. 철회된 결정은 삭제 (git에 이력 남음).

> **Bilingual support**: 필드명은 English 권장, Korean alias 허용.
> - `Context` | `맥락`
> - `Decision` | `결정`
> - `Rationale` | `근거`

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

### ## Operations (선택, None 허용)

4개 서브섹션 (bilingual 허용): Configuration|설정, Gotchas, Deployment|배포, Monitoring|모니터링.
`/decompile`이 `analyze-code`의 env_vars에서 `### Configuration`을 자동 추출.

```markdown
## Operations

### Configuration
| 환경변수 | 타입 | 필수/기본값 | 설명 |
|----------|------|-----------|------|
| JWT_PUBLIC_KEY | string | required | RS256 공개키 (PEM 형식) |
| TOKEN_TTL_HOURS | number | default: 168 | access token 유효 시간 |
| MAX_SESSIONS | number | default: 5 | 최대 동시 세션 수 |

### Gotchas
- 토큰 만료 시간은 UTC 기준

### 배포
- 배포 시 캐시 워밍업 5분 필요

### 모니터링
- `auth.validation.duration` 메트릭 확인
- 에러율 > 5% 시 알람
```

### ## Flows (선택, is_project_root only, None 허용)

**project root DEVELOPERS.md에만 허용.** 시스템 수준 use case 실행 흐름.
Cross-module 호출 순서와 데이터 타입을 기술한다. non-project-root에 작성하면 경고.

```markdown
## Flows

### 사용자 로그인
1. `api/auth` ← POST /login { email, password }
2. `domain/auth` — validateCredentials(email, password) → Session | AuthError
3. `domain/session` — createSession(userId) → SessionToken
4. `api/auth` → Response 200 { token: SessionToken } | Response 401

### JWT 검증 (매 요청)
1. `middleware/auth` — extractToken(headers.Authorization) → JWT | null
2. `domain/auth` — validateToken(JWT) → Claims | AuthError
3. `middleware/auth` — req.user = Claims 주입 | Response 401
```

**형식 규칙:**
- 각 단계: `` `모듈/경로` — 함수명(입력) → 출력 ``
- 모듈 경로는 project root 상대 경로
- 타입은 Data Schemas 또는 Constraints에서 정의된 타입 참조

## 스킬별 활용

| 스킬 | DEVELOPERS.md 활용 | 상세 |
|------|-------------------|------|
| `/impl` | Constraints + Data Schemas + Technical Context 생성 | CLAUDE.md Requirements를 구체화 |
| `/decompile` | 전체 생성 | 소스코드에서 6섹션 추출 (Data Schemas, Configuration 자동 추출 포함) |
| `/compile` | 테스트 생성 원천 | Constraints에서 테스트 케이스 생성 (Data Schemas는 타입 참조용) |
| `/validate` | drift 검증 | Constraints + Data Schemas drift 검출 (--strict) |
| `/bugfix` | L2 진단 | 3-layer 분석의 L2 계층 (Constraints) |

## 생명주기

CLAUDE.md와 동일한 생성/수정/삭제 주기를 따릅니다.

| 명령어 | DEVELOPERS.md |
|--------|---------------|
| /impl | 생성 (Constraints + Technical Context 필수, 나머지 선택) |
| /decompile | 전체 생성 (4섹션) |
| /compile | 테스트 생성 원천 (Constraints) |
| /bugfix | L2 진단 참조 |
| /validate | drift 검증 (Constraints ↔ Source Code) |
