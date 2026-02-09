<!--
  이 파일은 예시와 설명을 위한 문서입니다.
  규칙의 Single Source of Truth: references/shared/schema-rules.yaml
-->

# CLAUDE.md Schema Template

이 템플릿은 CLAUDE.md 파일의 표준 구조를 정의합니다.

**CLAUDE.md = WHAT (정형화된 PRD)**
- CLAUDE.md는 "무엇을(WHAT)" 정의하는 문서입니다.
- IMPLEMENTS.md는 "어떻게(HOW)" 구현하는지 정의합니다.
- 두 문서는 1:1로 매핑됩니다.

## 듀얼 문서 시스템

```
┌─────────────────────────────────────────────────────────────┐
│                    C 언어 비유                              │
│                                                             │
│   .h (헤더)    +  .c (소스)     ─── compile ──→  .o/.exe   │
│   WHAT            HOW                            Binary     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                         │
│                                                             │
│   CLAUDE.md     +  IMPLEMENTS.md  ─── /compile ──→  소스코드│
│   WHAT (스펙)      HOW (구현명세)                   (실행물)│
└─────────────────────────────────────────────────────────────┘
```

| 문서 | 역할 | 비유 | 주요 내용 |
|------|------|------|----------|
| **CLAUDE.md** | WHAT | .h (헤더) | 도메인맥락, PRD, 인터페이스 |
| **IMPLEMENTS.md** | HOW | .c (구현) | 알고리즘, 상수, 구현 상세 |

## 필수 섹션 요약 (7개)

| 섹션 | 필수 | "None" 허용 | 설명 |
|------|------|-------------|------|
| Purpose | ✓ | ✗ | 디렉토리의 책임 |
| Summary | ✓ | ✗ | 역할/책임/기능 1-2문장 요약 |
| Exports | ✓ | ✓ | public interface |
| Behavior | ✓ | ✓ | 동작 시나리오 |
| Contract | ✓ | ✓ | 사전/사후조건 |
| Protocol | ✓ | ✓ | 상태 전이/호출 순서 |
| Domain Context | ✓ | ✓ | compile 재현성 보장 맥락 |

> 규칙 상세: `references/shared/schema-rules.yaml` 참조

---

## 상세 설명

### 1. Purpose (필수)
디렉토리의 책임을 1-2문장으로 명시합니다.

```markdown
## Purpose
이 모듈은 사용자 인증을 담당합니다.
```

### 2. Summary (필수)
모듈의 역할/책임/주요 기능을 1-2문장으로 요약합니다.
Purpose보다 간결하게, 다른 개발자가 한눈에 파악할 수 있도록 작성합니다.
**dependency-graph CLI에서 노드 조회 시 표시**되므로 탐색 시 유용합니다.

```markdown
## Summary
인증 모듈. JWT 토큰 생성/검증/갱신 및 세션 관리 담당.
```

### 4. Structure (조건부 필수)
하위 디렉토리나 파일이 있는 경우 필수입니다.

```markdown
## Structure
- jwt/: JWT 토큰 처리 (상세는 jwt/CLAUDE.md 참조)
- session.ts: 세션 관리 로직
- types.ts: 인증 관련 타입 정의
```

### 5. Exports (필수)
모듈의 public interface를 **시그니처 레벨 + 도메인 맥락**으로 명시합니다.

**Exports = Interface Catalog**: 다른 모듈이 코드를 탐색하지 않고도 이 모듈의 인터페이스를 파악할 수 있어야 합니다.

#### Discovery Use Case
```
다른 모듈에서 이 모듈을 참조할 때:
1. 이 CLAUDE.md의 Exports 섹션 읽기 ← 인터페이스 카탈로그
2. 필요한 함수/타입 시그니처 확인
3. (선택) Behavior 섹션으로 동작 이해
4. (최후) 실제 소스코드 ← Exports로 불충분할 때만
```

#### 형식 규칙
- 함수/메서드: `Name(params) ReturnType` 형태
- 파라미터 타입과 반환 타입 필수
- **도메인 맥락**: 각 함수/타입에 역할과 용도 설명 포함
- 언어별 관용 표현 허용

#### Functions 상세 형식 (권장)

각 함수는 시그니처와 함께 도메인 맥락을 포함해야 합니다:

```markdown
### Functions

#### validateToken
`validateToken(token: string) -> Claims`

JWT 토큰을 검증하고 Claims를 추출합니다.
- **입력**: Bearer 토큰 (Authorization 헤더에서 추출된 문자열)
- **출력**: 사용자 식별 정보와 권한 목록을 포함하는 Claims
- **역할**: API 요청의 인증 게이트키퍼. 모든 보호된 엔드포인트에서 호출됨
- **도메인 맥락**: PCI-DSS 준수를 위해 7일 만료 정책 적용
```

#### Types 상세 형식 (권장)

```markdown
### Types

#### Claims
`Claims { userId: string, exp: number, permissions: Permission[] }`

인증된 사용자의 신원과 권한을 나타내는 타입
- **userId**: UUID v4 형식, 사용자 테이블의 PK
- **exp**: Unix timestamp (초), 토큰 만료 시점
- **permissions**: 해당 사용자가 수행 가능한 작업 목록
```

#### 간략 형식 (단순한 경우)

```markdown
### Functions
- `functionName(param: Type, param2: Type) -> ReturnType`
- `anotherFunction(param: Type) -> ReturnType`

### Types
- `TypeName { field: Type, field2: Type }`

### Classes
- `ClassName(param: Type)`
```

**참고**: 시그니처 형식은 프로젝트에서 사용하는 언어의 관용적 표현을 따릅니다.
프로젝트 root CLAUDE.md에 명시된 코딩 컨벤션을 참조하세요.

### 6. Dependencies (조건부)
외부/내부 의존성이 있는 경우 명시합니다.
내부 의존성의 **Export 레벨 상세**는 IMPLEMENTS.md `Module Integration Map`에 기술합니다.

```markdown
## Dependencies
- external: jsonwebtoken@9.0.0
- internal: ../utils/crypto (상세 → IMPLEMENTS.md Module Integration Map)
```

### 7. Behavior (필수)
동작을 **시나리오 레벨** (input → output)로 명시합니다.

```markdown
## Behavior

### 정상 케이스
- 유효한 토큰 → Claims 객체 반환
- 만료된 토큰 + refresh 옵션 → 새 토큰 쌍 반환

### 에러 케이스
- 잘못된 형식의 토큰 → InvalidTokenError
- 만료된 토큰 (refresh 없음) → TokenExpiredError
- 위조된 토큰 → SignatureVerificationError
```

### 8. Constraints (선택)
지켜야 할 규칙이나 제약사항입니다.

```markdown
## Constraints
- 토큰 만료 시간은 최대 24시간
- refresh token은 secure storage에만 저장
- 동시 세션은 최대 5개
```

### 9. Contract (필수, "None" 허용)
함수별 사전조건(preconditions), 사후조건(postconditions), 불변식(invariants) 정보입니다.

특별한 계약 조건이 없는 경우 `None`을 명시합니다.

```markdown
## Contract
None
```

계약 조건이 있는 경우:

```markdown
## Contract

### validateToken
- **Preconditions**: token must be non-empty string
- **Postconditions**: returns Claims with valid userId
- **Throws**: InvalidTokenError if token is malformed

### processOrder
- **Preconditions**:
  - order.id is required
  - order.items not empty
- **Postconditions**: returns Receipt with orderId matching input
```

#### 추출 소스
Contract 정보는 다음에서 자동 추출됩니다:

1. **JSDoc 태그** (명시적)
   - `@precondition <condition>` - 사전조건
   - `@postcondition <condition>` - 사후조건
   - `@invariant <condition>` - 불변식
   - `@throws <ErrorType>` - 예외

2. **코드 패턴 추론** (암시적)
   - Validation 로직: `if (!x.prop) throw new Error` → `x.prop is required`
   - Length 검증: `if (arr.length === 0) throw` → `arr not empty`
   - Type guards: `asserts x is T` → `x must be T`

### 10. Protocol (필수, "None" 허용)
상태 전이나 호출 순서를 명시합니다.

특별한 프로토콜이 없는 경우 `None`을 명시합니다.

```markdown
## Protocol
None
```

프로토콜이 있는 경우:

```markdown
## Protocol

### State Machine
States: `Idle` | `Loading` | `Loaded` | `Error`

Transitions:
- `Idle` + `load()` → `Loading`
- `Loading` + `success` → `Loaded`
- `Loading` + `failure` → `Error`

### Lifecycle
1. `init()` - 초기화
2. `start()` - 시작
3. `stop()` - 정지
4. `destroy()` - 정리
```

#### 추출 소스
Protocol 정보는 다음에서 자동 추출됩니다:

1. **상태 머신** (State enum)
   - `enum State { Idle, Loading, Loaded, Error }` 패턴 인식
   - 상태 전이 로직에서 transitions 추론

2. **라이프사이클** (JSDoc 태그)
   - `@lifecycle N` 태그로 순서 명시
   - 예: `@lifecycle 1` → 첫 번째 호출
   - 예: `@lifecycle 2` → 두 번째 호출

### 11. Domain Context (필수, "None" 허용)
compile 시 동일한 코드 재현을 보장하기 위한 맥락 정보입니다.
이 정보가 없으면 compile 결과가 달라질 수 있습니다.

**두 가지 역할:**
1. **자체 compile 재현성**: 해당 CLAUDE.md를 compile할 때 동일한 코드 생성
2. **의존자 compile 영향**: 이 모듈을 의존하는 다른 CLAUDE.md compile 시 필요한 맥락

도메인 맥락이 없는 경우 `None`을 명시합니다.

```markdown
## Domain Context
None
```

도메인 맥락이 있는 경우:

```markdown
## Domain Context

### Decision Rationale
- TOKEN_EXPIRY: 7일 (PCI-DSS 컴플라이언스 요구사항)
- MAX_RETRY: 3회 (외부 API SLA 기준)
- TIMEOUT: 2000ms (IdP 평균 응답 500ms × 4)

### Constraints
- 금융감독원 가이드라인: 비밀번호 재설정 주기 90일
- 라이선스 계약: 동시 세션 최대 5개
- 내부 정책: 민감 데이터 로깅 금지

### Compatibility
- 레거시 UUID v1 형식 지원 필요 (2023 마이그레이션)
- Node.js 18+ 필수 (native fetch 사용)
```

#### 하위 섹션

| 섹션 | 용도 | 예시 |
|------|------|------|
| **Decision Rationale** | 특정 값/설계를 선택한 이유 | `TOKEN_EXPIRY: 7일 (PCI-DSS)` |
| **Constraints** | 반드시 지켜야 할 외부 제약 | `비밀번호 재설정 90일 (금융감독원)` |
| **Compatibility** | 레거시/환경 호환성 요구 | `UUID v1 지원 (2023 마이그레이션)` |

#### compile 시 반영 방식

| Domain Context | compile 결과 |
|----------------|-------------|
| `TOKEN_EXPIRY: 7일 (PCI-DSS)` | `const TOKEN_EXPIRY_DAYS = 7; // PCI-DSS` |
| `동시 세션 최대 5개` | 세션 수 검증 로직 포함 |
| `UUID v1 지원 필요` | UUID v1 파싱 함수 포함 |

## Schema v2 기능

### Schema Version Marker

v2 파일은 첫 줄에 버전 마커를 포함합니다:

```markdown
<!-- schema: 2.0 -->
# module-name
```

### v2 Behavior 구조 (UseCase 다이어그램 지원)

v2에서는 Behavior 섹션에 Actor와 UseCase 구조를 추가할 수 있습니다:

```markdown
## Behavior

### Actors
- User: 인증이 필요한 사용자
- System: 내부 토큰 검증 시스템

### UC-1: Token Validation
- Actor: User
- 유효한 토큰 → Claims 객체 반환
- 만료된 토큰 → TokenExpiredError
- Includes: UC-3

### UC-2: Token Issuance
- Actor: System
- 사용자 정보 + 역할 → 서명된 JWT 토큰
- Extends: UC-1
```

이 구조는 `claude-md-core generate-diagram --type usecase` CLI로 Mermaid UseCase 다이어그램 생성에 사용됩니다.

### v2 Cross-Reference (Symbol-level Indexing)

v2 Exports는 heading 형식으로 작성하여 cross-reference를 지원합니다:

```markdown
### Functions

#### validateToken
`validateToken(token: string): Promise<Claims>`

JWT 토큰을 검증하고 Claims를 추출합니다.
```

다른 모듈에서 참조 시: `src/auth/CLAUDE.md#validateToken`

`claude-md-core symbol-index` CLI를 사용하여 go-to-definition, find-references 기능을 제공합니다.

### Diagram Generation CLI

| 다이어그램 | 소스 | CLI 명령어 | Mermaid 타입 |
|-----------|------|-----------|-------------|
| UseCase | Behavior (Actors + UC) | `generate-diagram --type usecase --file` | `flowchart LR` |
| State | Protocol (States + Transitions) | `generate-diagram --type state --file` | `stateDiagram-v2` |
| Component | dependency-graph (Nodes + Edges) | `generate-diagram --type component --root` | `flowchart TB` |

### Migration (v1 → v2)

```bash
# 미리보기 (파일 변경 없음)
claude-md-core migrate --root . --dry-run

# 마이그레이션 실행
claude-md-core migrate --root .
```

마이그레이션 작업:
1. `<!-- schema: 2.0 -->` 마커 추가
2. Exports bullet 형식 → heading 형식 변환
3. Actor/UC 구조 추가 제안

## 검증 규칙

> 규칙의 Single Source of Truth: `references/shared/schema-rules.yaml`

### 필수 섹션 검증 (7개)
- Purpose: 반드시 존재, "None" 불가
- Summary: 반드시 존재, "None" 불가 (1-2문장 역할/책임/기능 요약)
- Exports: 반드시 존재, public interface가 없는 경우 "None" 명시
- Behavior: 반드시 존재, 동작이 없는 경우 "None" 명시
- Contract: 반드시 존재, 계약 조건이 없는 경우 "None" 명시
- Protocol: 반드시 존재, 프로토콜이 없는 경우 "None" 명시
- Domain Context: 반드시 존재, 도메인 맥락이 없는 경우 "None" 명시

### Exports 형식 검증
```regex
# 함수 패턴: Name(params) ReturnType 형태
^[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*[:→\->]?\s*.+$
```

유효 예시:
- `validateToken(token: string): Promise<Claims>` ✓
- `validate_token(token: str) -> Claims` ✓
- `ValidateToken(token string) (Claims, error)` ✓

무효 예시:
- `validateToken` (파라미터 없음) ✗
- `validate token` (공백 포함) ✗

### Behavior 형식 검증
```regex
# 시나리오 패턴: input → output 형태
.+\s*[→\->]\s*.+
```

유효 예시:
- `유효한 토큰 → Claims 객체` ✓
- `invalid input -> specific error` ✓

무효 예시:
- `토큰을 검증합니다` (시나리오가 아님) ✗

## 참조 규칙

### 허용
- 부모 → 자식: 자식 디렉토리 참조 가능

### 금지
- 자식 → 부모: 부모 디렉토리 참조 불가
- 형제 ↔ 형제: 형제 디렉토리 상호 참조 불가

```markdown
# src/CLAUDE.md에서
## Structure
- auth/: 인증 모듈 (상세는 auth/CLAUDE.md 참조) ✓

# src/auth/CLAUDE.md에서
## Dependencies
- ../api: (부모 참조 - 금지) ✗
- ../utils: (형제 참조 - 금지) ✗
```

## 관련 문서

- **IMPLEMENTS.md**: HOW(구현 명세)를 정의하는 쌍 문서
  - **Module Integration Map**: 내부 의존성의 Export 시그니처 레벨 통합 명세
  - CLAUDE.md Dependencies의 internal 항목 상세는 Module Integration Map에서 관리
- 템플릿: `templates/implements-md-schema.md`

### 불변식

**INV-3: CLAUDE.md ↔ IMPLEMENTS.md 쌍**
```
∀ CLAUDE.md ∃ IMPLEMENTS.md (1:1 mapping)
path(IMPLEMENTS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'IMPLEMENTS.md')
```

**INV-4: Section 업데이트 책임**
```
/impl → CLAUDE.md + IMPLEMENTS.md.PlanningSection
/compile → IMPLEMENTS.md.ImplementationSection
/decompile → CLAUDE.md + IMPLEMENTS.md.* (전체)
```
