# IMPLEMENTS.md Schema Template

이 템플릿은 IMPLEMENTS.md 파일의 표준 구조를 정의합니다.

**IMPLEMENTS.md = HOW (구현 명세)**
- CLAUDE.md가 WHAT(무엇을)을 정의한다면, IMPLEMENTS.md는 HOW(어떻게)를 정의합니다.
- 소스코드를 보기 전에 다른 세션이 알아야 할 정보만 기술합니다.
- 코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 문서화합니다.

## 문서 쌍 규칙

```
∀ CLAUDE.md ∃ IMPLEMENTS.md (1:1 mapping)
path(IMPLEMENTS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'IMPLEMENTS.md')
```

## 섹션 구조

```
┌─────────────────────────────────────────────────────────────┐
│ IMPLEMENTS.md 섹션 구조                                     │
├─────────────────────────────────────────────────────────────┤
│ [Planning Section] ← /spec 이 업데이트                      │
│ - Dependencies Direction (필요 의존성, 위치)                │
│ - Implementation Approach (구현 방향)                       │
│ - Technology Choices (기술 선택 근거)                       │
├─────────────────────────────────────────────────────────────┤
│ [Implementation Section] ← /compile 이 업데이트             │
│ - Algorithm (실제 구현된 알고리즘)                          │
│ - Key Constants (상수값과 근거)                             │
│ - Error Handling (에러 처리 전략)                           │
│ - State Management (상태 관리)                              │
│ - Implementation Guide (다른 세션 참고용 정보)                     │
└─────────────────────────────────────────────────────────────┘
```

## 섹션 요약

| 섹션 | 명령어 | 필수 | "None" 허용 | 설명 |
|------|--------|------|-------------|------|
| Dependencies Direction | /spec | ✓ | ✗ | 필요 의존성과 위치 |
| Implementation Approach | /spec | ✓ | ✗ | 구현 방향과 전략 |
| Technology Choices | /spec | ✓ | ✓ | 기술 선택과 근거 |
| Algorithm | /compile | ✗ | - | 복잡하거나 비직관적인 로직만 |
| Key Constants | /compile | ✗ | - | 도메인 의미가 있는 상수만 |
| Error Handling | /compile | ✓ | ✓ | 에러 처리 전략 |
| State Management | /compile | ✓ | ✓ | 상태 관리 방식 |
| Implementation Guide | /compile | ✗ | - | 다른 세션 참고 정보 |

---

## Planning Section (필수)

> `/spec`이 업데이트하는 섹션. 코드 구현 전 계획 단계에서 결정되는 사항.

### 1. Dependencies Direction (필수)

의존성의 위치와 사용 목적을 명시합니다.

```markdown
## Dependencies Direction

### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 기존 프로젝트 호환)
- `lodash@4.17.21`: 유틸리티 함수 (선택 이유: 번들 사이즈 vs 편의성)

### Internal
- `../utils/crypto`: 암호화 유틸리티 (경로, 사용할 함수)
- `../config`: 환경 설정 (JWT_SECRET 로드)
```

**목적**: 코드 탐색 없이 의존성 구조를 파악

### 2. Implementation Approach (필수)

구현의 전략적 방향과 고려했으나 선택하지 않은 대안을 명시합니다.

```markdown
## Implementation Approach

### 전략
- HMAC-SHA256 기반 토큰 검증
- 캐시 레이어로 성능 최적화
- 실패 시 즉시 반환 (fail-fast)

### 고려했으나 선택하지 않은 대안
- RSA: 키 관리 복잡성 → 내부 서비스라 HMAC 충분
- jose 라이브러리: 기존 jsonwebtoken과 호환성 이슈
- Redis 캐시: 단일 인스턴스 환경이라 메모리 캐시 충분
```

**목적**: "왜 이 방식인가?"에 대한 근거 제공

### 3. Technology Choices (필수, "None" 허용)

기술 선택과 그 근거를 테이블 형태로 명시합니다.

```markdown
## Technology Choices

| 선택 | 대안 | 선택 이유 |
|------|------|----------|
| jsonwebtoken | jose | 기존 코드 호환 |
| Map 캐시 | Redis | 단일 인스턴스 환경 |
| HMAC-SHA256 | RSA | 키 관리 단순화 |
```

선택할 기술이 없는 경우:

```markdown
## Technology Choices

None
```

---

## Implementation Section (조건부)

> `/compile`이 업데이트하는 섹션. 실제 구현 후 발견된 사항을 기록.

### 4. Algorithm (조건부 - 복잡하거나 비직관적인 경우만)

소스코드를 보기 전에 알아야 할 구현 로직만 기술합니다.
**단순한 구현은 기술하지 않습니다** (소스코드로 충분).

```markdown
## Algorithm

### tokenCache 무효화 전략
1. 토큰 갱신 시 → 해당 userId의 모든 캐시 삭제
2. 권한 변경 시 → 영향받는 userId들의 캐시 삭제
3. 전체 리셋 시 → LRU 정책 적용 (최근 100개만 유지)

### 복잡한 비즈니스 로직
특정 조건에서만 적용되는 규칙:
- 조건 A + 조건 B → 특수 처리 경로
- 이유: 레거시 시스템 호환성
```

**작성 기준**: "이 로직을 이해하려면 코드를 읽어야 하는가?"
- Yes → Algorithm 섹션에 기술
- No → 생략 (코드로 충분)

### 5. Key Constants (조건부 - 도메인 의미가 있는 값만)

다른 세션이 알아야 할 상수만 기술합니다.
코드에서 명확한 상수는 생략합니다.

```markdown
## Key Constants

| Name | Value | Rationale | 영향 범위 |
|------|-------|-----------|----------|
| TOKEN_EXPIRY_DAYS | 7 | PCI-DSS 요구사항 | 보안 정책 |
| MAX_CONCURRENT_SESSIONS | 5 | 라이선스 제약 | 사용자 경험 |
| RATE_LIMIT_PER_MINUTE | 100 | 인프라 용량 | 서비스 안정성 |
```

**작성 기준**: "이 값이 변경되면 비즈니스에 영향이 있는가?"
- Yes → Key Constants 섹션에 기술
- No → 생략 (코드에서 명확)

### 6. Error Handling (필수, "None" 허용)

에러 유형별 처리 전략을 명시합니다.

```markdown
## Error Handling

| Error | Retry | Recovery | Log Level |
|-------|-------|----------|-----------|
| TokenExpiredError | ✗ | 재로그인 유도 | WARN |
| InvalidTokenError | ✗ | 재로그인 유도 | ERROR |
| NetworkError | ✓(3회) | exponential backoff | ERROR |
| RateLimitError | ✓(1회) | 1분 대기 후 재시도 | WARN |
```

에러 처리가 없는 경우:

```markdown
## Error Handling

None
```

### 7. State Management (필수, "None" 허용)

내부 상태의 초기화, 저장, 복구 방식을 명시합니다.

```markdown
## State Management

### Initial State
- tokenCache: Map<userId, CachedClaims>
- lastCleanup: null

### Persistence
- 위치: 메모리 (인스턴스 재시작 시 초기화)
- 형식: N/A

### Cleanup
- 5분마다 만료된 캐시 정리
- 캐시 최대 크기: 1000개 (LRU)
```

상태 관리가 없는 경우:

```markdown
## State Management

None
```

### 8. Implementation Guide (선택)

다음 세션에서 소스코드 탐색 전 알면 효율적인 구현 가이드를 기록합니다.
(도메인 맥락은 CLAUDE.md Domain Context에 기술)

```markdown
## Implementation Guide

- 토큰 검증 → ../jwt/CLAUDE.md#validateToken
- 암호화 유틸 → ../utils/crypto/CLAUDE.md#hashPassword
- 에러 타입 → ../errors/CLAUDE.md#AuthError
```

**작성 기준**: "소스코드 탐색 없이 구현 시작점을 알 수 있는가?"
- 의존 모듈의 CLAUDE.md 경로 + Export 이름 (예: ../jwt/CLAUDE.md#validateToken)
- 도메인 맥락(값의 근거 등)은 CLAUDE.md Domain Context에 기술

---

## 전체 템플릿

```markdown
# [모듈명]/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /spec 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Dependencies Direction

### External
- `package@version`: 용도 (선택 이유)

### Internal
- `../path/to/module`: 용도 (사용할 인터페이스)

## Implementation Approach

### 전략
- 핵심 구현 방향

### 고려했으나 선택하지 않은 대안
- 대안 A: 미선택 이유

## Technology Choices

| 선택 | 대안 | 선택 이유 |
|------|------|----------|
| 기술A | 기술B | 근거 |

<!-- ═══════════════════════════════════════════════════════ -->
<!-- IMPLEMENTATION SECTION - /compile 이 업데이트            -->
<!-- ═══════════════════════════════════════════════════════ -->

## Algorithm

### 복잡한 로직명
(복잡하거나 비직관적인 경우만 기술)

## Key Constants

| Name | Value | Rationale | 영향 범위 |
|------|-------|-----------|----------|
| CONST_NAME | value | 근거 | 범위 |

## Error Handling

| Error | Retry | Recovery | Log Level |
|-------|-------|----------|-----------|
| ErrorType | ✓/✗ | 복구 전략 | LEVEL |

## State Management

### Initial State
- 초기 상태 설명

### Persistence
- 저장 방식

### Cleanup
- 정리 전략

## Implementation Guide

- 기능 → 의존모듈/CLAUDE.md#ExportName
```

---

## 검증 규칙

### 필수 섹션 검증
- Dependencies Direction: 반드시 존재
- Implementation Approach: 반드시 존재
- Technology Choices: 반드시 존재, 선택 없으면 "None" 명시
- Error Handling: 반드시 존재, 처리 없으면 "None" 명시
- State Management: 반드시 존재, 상태 없으면 "None" 명시

### 조건부 섹션
- Algorithm: 복잡한 로직이 있을 때만 작성
- Key Constants: 도메인 의미 있는 상수가 있을 때만 작성
- Implementation Guide: 참고 사항이 있을 때만 작성

### 업데이트 책임
```
/spec → Planning Section (Dependencies Direction, Implementation Approach, Technology Choices)
/compile → Implementation Section (Algorithm, Key Constants, Error Handling, State Management, Implementation Guide)
/decompile → 전체 섹션
```

---

## 예시: auth/IMPLEMENTS.md

```markdown
# auth/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /spec 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Dependencies Direction

### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 기존 프로젝트 호환, 성숙한 라이브러리)

### Internal
- `../utils/crypto`: 해시 유틸리티 (hashPassword, verifyPassword)
- `../config`: 환경 설정 (JWT_SECRET, TOKEN_EXPIRY)

## Implementation Approach

### 전략
- HMAC-SHA256 기반 토큰 검증
- 메모리 캐시로 반복 검증 성능 최적화
- Fail-fast: 첫 번째 실패 시 즉시 반환

### 고려했으나 선택하지 않은 대안
- RSA 서명: 키 관리 복잡성 → 내부 서비스 간 통신이라 HMAC으로 충분
- jose 라이브러리: 기능은 풍부하나 기존 jsonwebtoken 마이그레이션 비용
- Redis 캐시: 현재 단일 인스턴스 환경, 분산 환경 전환 시 재검토

## Technology Choices

| 선택 | 대안 | 선택 이유 |
|------|------|----------|
| jsonwebtoken | jose | 기존 코드베이스 호환성 |
| Map 캐시 | Redis | 단일 인스턴스 환경 |
| HMAC-SHA256 | RS256 | 키 관리 단순화 |

<!-- ═══════════════════════════════════════════════════════ -->
<!-- IMPLEMENTATION SECTION - /compile 이 업데이트            -->
<!-- ═══════════════════════════════════════════════════════ -->

## Algorithm

### tokenCache 무효화 전략
1. 토큰 갱신 시 → 해당 userId의 기존 캐시 삭제
2. 로그아웃 시 → 해당 userId의 모든 캐시 삭제
3. 주기적 정리 (5분) → 만료된 캐시 항목 제거

## Key Constants

| Name | Value | Rationale | 영향 범위 |
|------|-------|-----------|----------|
| TOKEN_EXPIRY_DAYS | 7 | PCI-DSS 요구사항 | 보안 정책 |
| CACHE_TTL_MINUTES | 5 | 메모리 사용량 최적화 | 성능 |
| MAX_CACHE_SIZE | 1000 | 메모리 제한 (약 10MB) | 인프라 |

## Error Handling

| Error | Retry | Recovery | Log Level |
|-------|-------|----------|-----------|
| TokenExpiredError | ✗ | 401 반환, 재로그인 유도 | WARN |
| InvalidTokenError | ✗ | 401 반환, 재로그인 유도 | ERROR |
| SignatureError | ✗ | 401 반환, 보안 알림 | ERROR |

## State Management

### Initial State
- tokenCache: new Map<string, CachedClaims>()
- lastCleanupTime: Date.now()

### Persistence
- 위치: 메모리 전용
- 인스턴스 재시작 시 캐시 초기화 (cold start)

### Cleanup
- 5분 주기 setInterval
- 만료 시점 + 1분 버퍼 초과 항목 삭제
- 캐시 크기 1000개 초과 시 LRU 정책 적용

## Implementation Guide

- 토큰 검증 → ../jwt/CLAUDE.md#validateToken
- 암호화 → ../utils/crypto/CLAUDE.md#hashPassword, #verifyPassword
- 설정 로드 → ../config/CLAUDE.md#JWT_SECRET, #TOKEN_EXPIRY
```
