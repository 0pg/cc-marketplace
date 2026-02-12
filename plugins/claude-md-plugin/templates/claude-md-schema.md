<!--
  이 파일은 예시와 설명을 위한 문서입니다.
  규칙의 Single Source of Truth: skills/schema-validate/references/schema-rules.yaml
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

## 필수 섹션 요약 (6개)

| 섹션 | 필수 | "None" 허용 | 설명 |
|------|------|-------------|------|
| Purpose | ✓ | ✗ | 디렉토리의 책임 |
| Exports | ✓ | ✓ | public interface |
| Behavior | ✓ | ✓ | 동작 시나리오 |
| Contract | ✓ | ✓ | 사전/사후조건 |
| Protocol | ✓ | ✓ | 상태 전이/호출 순서 |
| Domain Context | ✓ | ✓ | compile 재현성 보장 맥락 |

> 규칙 상세: `skills/schema-validate/references/schema-rules.yaml` 참조

---

## 상세 설명

### 1. Purpose (필수)
디렉토리의 책임을 1-2문장으로 명시합니다.

```markdown
## Purpose
이 모듈은 사용자 인증을 담당합니다.
```

### 2. Structure (조건부 필수)
하위 디렉토리나 파일이 있는 경우 필수입니다.

```markdown
## Structure
- jwt/: JWT 토큰 처리 (상세는 jwt/CLAUDE.md 참조)
- session.ts: 세션 관리 로직
- types.ts: 인증 관련 타입 정의
```

### 3. Exports (필수)
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

### 4. Dependencies (조건부)
외부 의존성이 있는 경우 명시합니다.

```markdown
## Dependencies

- external:
  - `jsonwebtoken@9.0.0`: sign, verify
  - `@aws-sdk/client-s3`: S3Client

- internal:
  - `utils/crypto/CLAUDE.md`: hashPassword, verifySignature
  - `core/domain/transaction/CLAUDE.md`: WithdrawalResultSynchronizer
```

**규칙:**
- internal 경로는 project-root-relative CLAUDE.md 파일 경로
- colon 뒤에 import하는 심볼 나열
- tree-parse 결과의 디렉토리 목록과 1:1 대응

### 5. Behavior (필수)
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

### 6. Constraints (선택)
지켜야 할 규칙이나 제약사항입니다.

```markdown
## Constraints
- 토큰 만료 시간은 최대 24시간
- refresh token은 secure storage에만 저장
- 동시 세션은 최대 5개
```

### 7. Contract (필수, "None" 허용)
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

### 8. Protocol (필수, "None" 허용)
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

### 9. Domain Context (필수, "None" 허용)
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

## 검증 규칙

> 규칙의 Single Source of Truth: `skills/schema-validate/references/schema-rules.yaml`

### 필수 섹션 검증 (6개)
- Purpose: 반드시 존재, "None" 불가
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

### 10. Project Convention (조건부 - project_root 또는 module_root)

프로젝트 수준 아키텍처/구조 규칙입니다. project_root CLAUDE.md에 필수이며, module_root에서는 optional override로 사용됩니다.

```markdown
## Project Convention

### Project Structure
src/ 하위에 기능별 디렉토리 구성.
각 기능 디렉토리에 index.ts를 진입점으로 사용.

### Module Boundaries
각 모듈은 자체 CLAUDE.md를 가지며, 모듈 간 의존성은 Exports만 참조.
순환 의존 금지.

### Naming Conventions
디렉토리: kebab-case
파일: camelCase
패키지: @scope/package-name
```

**필수 서브섹션:**

| 서브섹션 | 필수 | 설명 |
|----------|------|------|
| Project Structure | Yes | 디렉토리 구조 규칙, 레이어링 패턴 |
| Module Boundaries | Yes | 모듈 책임 규칙, 의존성 방향 |
| Naming Conventions | Yes | 모듈/디렉토리/패키지 네이밍 |

### 11. Code Convention (조건부 - module_root)

소스코드 수준 코딩 규칙입니다. module_root CLAUDE.md에 필수입니다. 싱글 모듈 프로젝트에서는 project_root CLAUDE.md에 함께 배치합니다.

```markdown
## Code Convention

### Language & Runtime
TypeScript 5.0, Node.js 20 LTS

### Code Style
- 들여쓰기: 2 spaces
- 따옴표: single quotes
- 세미콜론: 필수
- 줄 길이: 120자

### Naming Rules
- 변수/함수: camelCase
- 클래스/타입: PascalCase
- 상수: UPPER_SNAKE_CASE
- private: _prefix
```

**필수 서브섹션:**

| 서브섹션 | 필수 | 설명 |
|----------|------|------|
| Language & Runtime | Yes | 주요 언어, 버전, 런타임 |
| Code Style | Yes | 포맷팅, 들여쓰기, 줄 길이 |
| Naming Rules | Yes | 변수/함수/클래스/상수 네이밍 |

**Convention 섹션 검증:**

```bash
# CLI로 deterministic 검증
claude-md-core validate-convention --project-root /path/to/project
```

## 관련 문서

- **IMPLEMENTS.md**: HOW(구현 명세)를 정의하는 쌍 문서
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

**INV-5: Convention 섹션 배치 규칙**
```
project_root/CLAUDE.md MUST contain ## Project Convention
module_root/CLAUDE.md MUST contain ## Code Convention
module_root/CLAUDE.md MAY contain ## Project Convention (override)
```
