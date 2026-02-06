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
│ - Architecture Decisions (아키텍처 설계 결정)               │
│ - Module Integration Map (내부 의존성 Export 레벨 명세)     │
│ - External Dependencies (외부 의존성)                       │
│ - Implementation Approach (구현 방향)                       │
│ - Technology Choices (기술 선택 근거)                       │
├─────────────────────────────────────────────────────────────┤
│ [Implementation Section] ← /compile 이 업데이트             │
│ - Algorithm (실제 구현된 알고리즘)                          │
│ - Key Constants (상수값과 근거)                             │
│ - Error Handling (에러 처리 전략)                           │
│ - State Management (상태 관리)                              │
│ - Implementation Guide (다른 세션 참고용 정보)              │
└─────────────────────────────────────────────────────────────┘
```

## 섹션 요약

| 섹션 | 명령어 | 필수 | "None" 허용 | 설명 |
|------|--------|------|-------------|------|
| Architecture Decisions | /spec | ✓ | ✓ | 아키텍처 설계 결정과 근거 |
| Module Integration Map | /spec | 조건부 | ✓ | 내부 모듈 의존성의 Export 레벨 통합 명세 |
| External Dependencies | /spec | ✓ | ✓ | 외부 패키지 의존성 |
| Implementation Approach | /spec | ✓ | ✗ | 구현 방향과 전략 |
| Technology Choices | /spec | ✓ | ✓ | 기술 선택과 근거 |
| Algorithm | /compile | ✗ | - | 복잡하거나 비직관적인 로직만 |
| Key Constants | /compile | ✗ | - | 도메인 의미가 있는 상수만 |
| Error Handling | /compile | ✓ | ✓ | 에러 처리 전략 |
| State Management | /compile | ✓ | ✓ | 상태 관리 방식 |
| Implementation Guide | /compile | ✗ | - | compile 시 발견된 추가 참고 정보 |

---

## Planning Section (필수)

> `/spec`이 업데이트하는 섹션. 코드 구현 전 계획 단계에서 결정되는 사항.

### 1. Architecture Decisions (필수, "None" 허용)

아키텍처 설계 결정과 그 근거를 기록합니다. 기존 코드베이스 분석 결과를 바탕으로 모듈 배치, 인터페이스 설계, 의존성 방향을 결정합니다.

```markdown
## Architecture Decisions

### Module Placement
- **Decision**: `src/payment/` 신규 디렉토리 생성
- **Alternatives Considered**:
  - `src/api/payment/`: API 레이어에 포함 → 비즈니스 로직 분리 원칙 위배
  - `src/auth/payment/`: 인증과 결합 → SRP 위반
- **Rationale**: 독립적 도메인으로 분리, 향후 마이크로서비스 전환 용이

### Interface Guidelines
- 새로 정의할 인터페이스:
  - `processPayment(order: Order): Promise<PaymentResult>`
  - `refundPayment(paymentId: string): Promise<RefundResult>`
- 내부 모듈 통합: Module Integration Map 참조

### Dependency Direction
- 의존성 분석: `.claude/dependency-graph.json` 참조
- 경계 명확성 준수: ✓
```

아키텍처 결정이 필요 없는 단순한 경우:

```markdown
## Architecture Decisions

None
```

**작성 기준**:
- 신규 모듈 생성 시: Module Placement 필수
- 기존 모듈 확장 시: Interface Guidelines에 추가 인터페이스만 명시
- 단순 기능 추가 시: "None" 허용

### 2. Module Integration Map (조건부 필수 - 내부 의존성이 있는 경우)

내부 모듈 의존성을 **Export 시그니처 레벨**로 정형화하여 명시합니다.
Programmatic하게 의존성 그래프를 추출할 수 있도록 **엄격한 스키마**를 따릅니다.

> **목적**:
> - CLAUDE.md 간 의존 관계를 한 눈에 파악
> - `/compile`이 의존 모듈 CLAUDE.md를 다시 읽지 않고 인터페이스 파악 가능
> - dependency-graph에서 export 레벨 의존성 추출 가능

#### 스키마 규칙

| 요소 | 형식 | 필수 | 설명 |
|------|------|------|------|
| Entry Header | `### \`{path}\` → {name}/CLAUDE.md` | ✓ | 의존 모듈 식별자 |
| Exports Used | `#### Exports Used` + 시그니처 목록 | ✓ | 사용할 Export 시그니처 |
| Integration Context | `#### Integration Context` + 텍스트 | ✓ | 사용 목적/방식 설명 |

#### Entry Header 형식

```
### `{relative_path}` → {module_name}/CLAUDE.md
```

- `{relative_path}`: 현재 모듈 기준 상대 경로 (e.g., `../auth`, `../utils/crypto`)
- `{module_name}/CLAUDE.md`: 대상 모듈의 CLAUDE.md 식별자

**파싱 패턴:**
```regex
^###\s+`([^`]+)`\s*→\s*(.+/CLAUDE\.md)$
```

- Capture Group 1: 상대 경로 (`../auth`)
- Capture Group 2: CLAUDE.md 식별자 (`auth/CLAUDE.md`)

#### Exports Used 형식

```
#### Exports Used
- `{signature}` — {역할 설명}
```

- CLAUDE.md Exports 시그니처 형식 준수 (기존 스키마 동일)
- Functions: `Name(params): ReturnType`
- Types: `TypeName { field: Type, field2: Type }`
- Classes: `ClassName(params)`
- 각 항목 뒤 ` — {역할 설명}` 은 선택 사항

**파싱 패턴:**
```regex
^[-*]\s+`([^`]+)`(?:\s*—\s*(.+))?$
```

- Capture Group 1: Export 시그니처 (`validateToken(token: string): Promise<Claims>`)
- Capture Group 2: 역할 설명 (선택)

#### Integration Context 형식

```
#### Integration Context
{자유 형식 텍스트, 1-3문장}
```

사용 목적과 통합 방식을 서술합니다. 코드 탐색 없이 "왜 이 export가 필요한가"를 이해할 수 있도록 합니다.

#### 예시

```markdown
## Module Integration Map

### `../auth` → auth/CLAUDE.md

#### Exports Used
- `validateToken(token: string): Promise<Claims>` — API 요청 인증 게이트키퍼
- `Claims { userId: string, exp: number, permissions: Permission[] }` — 인증 정보 타입

#### Integration Context
모든 보호된 API 엔드포인트에서 미들웨어로 호출.
Claims.userId로 요청자 식별 후 권한 검증에 사용.

### `../config` → config/CLAUDE.md

#### Exports Used
- `loadConfig(): Config` — 환경 설정 로드

#### Integration Context
초기화 시 1회 호출. Config.JWT_SECRET으로 토큰 서명.
```

내부 의존성이 없는 경우:

```markdown
## Module Integration Map

None
```

**작성 기준**:
- 내부 모듈 의존이 있으면 필수
- Export 시그니처는 **대상 CLAUDE.md Exports 섹션에서 복사** (스냅샷)
- `/validate` 실행 시 스냅샷과 실제 CLAUDE.md Exports 일치 여부 검증 가능

#### 교차 검증 규칙

| 검증 | 설명 | 검증 시점 |
|------|------|----------|
| Export 존재 | 참조한 Export가 대상 CLAUDE.md Exports에 존재 | /validate |
| 시그니처 일치 | 스냅샷 시그니처와 대상 CLAUDE.md 시그니처가 동일 | /validate |
| 경계 준수 | 상대 경로가 유효한 모듈을 가리킴 | /spec (dependency-graph) |

### 3. External Dependencies (필수, "None" 허용)

외부 패키지 의존성과 선택 근거를 명시합니다.

```markdown
## External Dependencies

- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 기존 프로젝트 호환, 성숙한 라이브러리)
- `lodash@4.17.21`: 유틸리티 함수 (선택 이유: 번들 사이즈 vs 편의성)
```

외부 의존성이 없는 경우:

```markdown
## External Dependencies

None
```

**목적**: 외부 패키지의 버전과 선택 근거를 기록

### 3. Implementation Approach (필수)

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

### 4. Technology Choices (필수, "None" 허용)

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

### 5. Algorithm (조건부 - 복잡하거나 비직관적인 경우만)

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

### 6. Key Constants (조건부 - 도메인 의미가 있는 값만)

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

### 7. Error Handling (필수, "None" 허용)

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

### 8. State Management (필수, "None" 허용)

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

### 9. Implementation Guide (선택)

`/compile` 과정에서 발견된 추가 참고 정보를 기록합니다.
Module Integration Map에 이미 기록된 Export 참조 정보와 **중복하지 않습니다**.

```markdown
## Implementation Guide

- 비동기 초기화 순서: config 로드 → DB 연결 → 캐시 워밍 (순서 변경 불가)
- 테스트 시 JWT_SECRET mock 필수 (../config 모듈의 테스트 헬퍼 활용)
```

**작성 기준**: "Module Integration Map + CLAUDE.md만으로 알 수 없는 구현 시 주의사항이 있는가?"
- 의존 모듈 Export 참조 → Module Integration Map에 기록 (중복 금지)
- compile 과정에서 발견된 순서 제약, 테스트 팁, 성능 주의사항 등

---

## 전체 템플릿

```markdown
# [모듈명]/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /spec 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Architecture Decisions

### Module Placement
- **Decision**: 배치 위치
- **Alternatives Considered**: 고려한 대안들
- **Rationale**: 선택 근거

### Interface Guidelines
- 새로 정의할 인터페이스 시그니처
- 내부 모듈 통합: Module Integration Map 참조

### Dependency Direction
- 의존성 분석 결과
- 경계 명확성 준수 여부

## Module Integration Map

### `{../relative_path}` → {module_name}/CLAUDE.md

#### Exports Used
- `{ExportSignature}` — {역할 설명}

#### Integration Context
{사용 목적과 통합 방식 1-3문장}

## External Dependencies

- `package@version`: 용도 (선택 이유)

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

- compile 시 발견된 추가 참고 정보 (Module Integration Map과 중복 금지)
```

---

## 검증 규칙

### 필수 섹션 검증
- Architecture Decisions: 반드시 존재, 결정 없으면 "None" 명시
- Module Integration Map: 내부 의존성이 있으면 필수, 없으면 "None" 명시
- External Dependencies: 반드시 존재, 외부 의존성 없으면 "None" 명시
- Implementation Approach: 반드시 존재
- Technology Choices: 반드시 존재, 선택 없으면 "None" 명시
- Error Handling: 반드시 존재, 처리 없으면 "None" 명시
- State Management: 반드시 존재, 상태 없으면 "None" 명시

### Module Integration Map 형식 검증
```yaml
entry_header:
  pattern: "^###\\s+`[^`]+`\\s*→\\s*.+/CLAUDE\\.md$"
  description: "### `{path}` → {name}/CLAUDE.md"
  required: true

exports_used:
  header: "#### Exports Used"
  item_pattern: "^[-*]\\s+`[^`]+`(?:\\s*—\\s*.+)?$"
  description: "- `{signature}` — {description}"
  min_items: 1
  signature_validation: same_as_claude_md_exports

integration_context:
  header: "#### Integration Context"
  required: true
  allow_empty: false
```

### 조건부 섹션
- Algorithm: 복잡한 로직이 있을 때만 작성
- Key Constants: 도메인 의미 있는 상수가 있을 때만 작성
- Implementation Guide: compile 시 발견된 추가 참고 사항이 있을 때만 작성

### 업데이트 책임
```
/spec → Planning Section (Architecture Decisions, Module Integration Map, External Dependencies, Implementation Approach, Technology Choices)
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

## Architecture Decisions

### Module Placement
- **Decision**: `src/auth/` 독립 디렉토리
- **Alternatives Considered**:
  - `src/api/auth/`: API 레이어 통합 → 비즈니스 로직 분리 원칙 위배
  - `src/services/auth/`: 서비스 레이어 → 도메인 경계 불명확
- **Rationale**: 인증은 독립 도메인, 다른 모듈에서 참조만 받음

### Interface Guidelines
- 새로 정의할 인터페이스:
  - `validateToken(token: string): Promise<Claims>`
  - `issueToken(userId: string): Promise<string>`
- 내부 모듈 통합: Module Integration Map 참조

### Dependency Direction
- 의존성 분석: `.claude/dependency-graph.json`
- 경계 명확성 준수: ✓

## Module Integration Map

### `../utils/crypto` → utils/crypto/CLAUDE.md

#### Exports Used
- `hashPassword(password: string): Promise<string>` — 비밀번호 해시 생성
- `verifyPassword(password: string, hash: string): Promise<boolean>` — 비밀번호 검증

#### Integration Context
사용자 인증 시 비밀번호 해시 비교에 사용.
issueToken 전 verifyPassword로 자격 증명 확인.

### `../config` → config/CLAUDE.md

#### Exports Used
- `JWT_SECRET: string` — 토큰 서명/검증 키
- `TOKEN_EXPIRY: number` — 토큰 만료 시간 (초)

#### Integration Context
초기화 시 로드. JWT 서명과 만료 정책에 직접 사용.
환경별로 값이 다르므로 하드코딩 금지.

## External Dependencies

- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 기존 프로젝트 호환, 성숙한 라이브러리)

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

- 비동기 초기화 순서: config 로드 → 캐시 초기화 (순서 변경 불가)
- 테스트 시 JWT_SECRET mock 필수
```
