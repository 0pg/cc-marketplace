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

## 섹션 (2 필수 + 2 선택, 모두 None 허용)

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

## 스킬별 활용

| 스킬 | DEVELOPERS.md 활용 | 상세 |
|------|-------------------|------|
| `/impl` | Constraints + Technical Context 생성 | CLAUDE.md Requirements를 구체화 |
| `/decompile` | 전체 생성 | 소스코드에서 4섹션 추출 |
| `/compile` | 테스트 생성 원천 | Constraints에서 테스트 케이스 생성 |
| `/validate` | drift 검증 | Constraints ↔ Source Code 일치 검증 |
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
