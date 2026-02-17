<!--
  decompile-templates.md
  Consolidated reference for the decompiler agent.
  Contains: Phase 3-4 workflow, CLAUDE.md/IMPLEMENTS.md templates,
  Exports/Contract/Protocol extraction guides, IMPLEMENTS.md writing criteria,
  and CLI output JSON structures.

  Loaded at runtime by the decompiler agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/decompile/references/decompile-templates.md"
-->

# Decompiler Templates & Reference

## Phase 3: 불명확한 부분 질문 (필요시)

분석 결과에서 불명확한 부분이 있으면 사용자에게 질문합니다.

**질문 안 함** (코드에서 추론 가능):
- 함수명에서 목적이 명확한 경우
- 상수 값을 계산할 수 있는 경우
- 표준 패턴을 따르는 경우

**질문 함** (코드만으로 불명확):
- 비표준 매직 넘버의 비즈니스 의미
- 도메인 전문 용어
- 컨벤션을 벗어난 구현의 이유
- **Domain Context 관련**: 결정 근거, 외부 제약, 호환성 요구
- **Implementation 관련**: 기술 선택 근거, 대안 미선택 이유

불명확한 부분이 있으면 AskUserQuestion으로 사용자에게 질문합니다. 예를 들어, 매직 넘버의 비즈니스 배경을 확인하기 위해 "GRACE_PERIOD_DAYS = 7의 비즈니스 배경이 있나요?" 같은 질문을 합니다. 옵션으로 "법적 요구사항", "비즈니스 정책", "기술적 제약" 등을 제시합니다.

### Domain Context 질문 (CLAUDE.md용 - 코드에서 추출 불가)

Domain Context는 코드에서 추론할 수 없는 "왜?"에 해당합니다.
상수 값, 설계 결정, 특이한 구현이 있을 때 반드시 질문합니다:

Domain Context 추출을 위해 다음 카테고리의 질문을 합니다:

1. **상수 값의 결정 근거 (Decision Rationale)**: 매직 넘버가 발견되면 값을 선택한 이유를 질문합니다. 옵션: 컴플라이언스(PCI-DSS, GDPR 등), SLA/계약, 내부 정책, 기술적 산출
2. **외부 제약 조건 (Constraints)**: 지켜야 할 외부 제약이 있는지 질문합니다. 옵션: 있음(규제, 라이선스 등), 없음
3. **호환성 요구 (Compatibility)**: 레거시 패턴이 발견되면 호환성 요구가 있는지 질문합니다. 옵션: 있음(특정 버전/형식 지원 필요), 없음

질문이 있으면 AskUserQuestion으로 한 번에 전달합니다.

### Implementation 관련 질문 (IMPLEMENTS.md용)

기술 선택과 구현 방향에 대해 다음 카테고리의 질문을 합니다:

1. **기술 선택 근거**: 외부 의존성이 있으면 해당 라이브러리를 선택한 이유를 질문합니다. 옵션: 성능(벤치마크), 호환성(기존 코드), 팀 경험(숙련도), 커뮤니티(문서화/지원)
2. **대안 미선택 이유**: 고려했으나 선택하지 않은 대안이 있는지 질문합니다. 옵션: 있음(대안 설명 가능), 없음

질문이 있으면 AskUserQuestion으로 한 번에 전달합니다.

## Phase 4: CLAUDE.md 초안 생성 (WHAT)

분석 결과를 기반으로 CLAUDE.md를 직접 생성합니다:

1. **자식 CLAUDE.md Purpose 추출**: 각 자식 CLAUDE.md 파일이 존재하면 읽어서 Purpose 섹션을 추출합니다.
2. **CLAUDE.md 템플릿 생성**: 다음 템플릿에 맞게 CLAUDE.md를 작성합니다:

```markdown
# {directory_name}

## Purpose

{분석에서 추출한 목적 또는 사용자 응답}

## Exports

### Functions
{분석에서 추출한 함수 목록}

### Types
{분석에서 추출한 타입 목록}

## Behavior

### 정상 케이스
{성공 시나리오 목록}

### 에러 케이스
{에러 시나리오 목록}

## Contract

{계약 조건 또는 "None"}

## Protocol

{프로토콜 또는 "None"}

## Domain Context

{사용자 응답 기반 도메인 컨텍스트 또는 "None"}

## Dependencies

- external:
{외부 의존성 목록}

- internal:
{resolved CLAUDE.md 경로 기반 내부 의존성: 심볼만 나열}
```

**Internal Dependencies 포맷팅 규칙:**
- code-analyze JSON의 `dependencies.internal` 배열을 읽어서 출력합니다.
- resolution이 "Exact" 또는 "Ancestor"이면:
  ```
  - `{dep.claude_md_path}`: {사용하는 심볼들}
  ```
- resolution이 "Unresolved"이면:
  ```
  - `{dep.raw_import}` <!-- UNRESOLVED: 수동 확인 필요 -->
  ```
- 예시:
  ```
  - `core/domain/transaction/CLAUDE.md`: WithdrawalResultSynchronizer
  - `vendors/vendor-common/CLAUDE.md`: FinancialServiceProvider
  ```

**CLAUDE.md vs IMPLEMENTS.md 차이:**
- CLAUDE.md: 심볼만 나열 (위 포맷)
- IMPLEMENTS.md: 심볼 + 선택 이유/방향 포함 (예: `- \`path/CLAUDE.md\`: SymbolName — 이 모듈의 인증 기능 활용`)

3. **대상 디렉토리에 직접 Write** (${TMP_DIR} 미사용)

---

## Exports 형식 가이드

### Export Candidates + LLM Review

**Exports 섹션은 2단계로 생성됩니다:**

1. **Phase 2.5**: `format-exports` CLI가 analyze-code JSON에서 export **후보(candidates)** 목록을 생성
2. **Phase 4**: LLM이 코드를 읽고 후보를 리뷰하여 최종 public exports 결정

`format-exports` 출력은 **후보 목록**입니다 (regex 기반이므로 false positive 포함 가능):
```markdown
### Functions
- `validateToken(token: string): Promise<Claims>`
- `generateToken(userId: string, role: string): string`

### Types
- `Claims { userId: string, role: Role }`

### Variables
- `DEFAULT_TIMEOUT` (number)
```

LLM review 후 (false positive 제거 + description 추가):
```markdown
### Functions
- `validateToken(token: string): Promise<Claims>` - JWT 토큰을 검증하고 Claims를 추출
- `generateToken(userId: string, role: string): string` - 새 JWT 토큰 생성

### Types
- `Claims { userId: string, role: Role }` - 인증된 사용자 정보
```
(예: `DEFAULT_TIMEOUT`이 내부 전용이라 판단되면 제거)

**LLM Review 규칙:**
- 후보 중 실제 public이 아닌 항목 제거 가능 (false positive 필터링)
- 각 항목에 description 추가
- 시그니처는 format-exports 출력 기준 (수정 시 근거 필요)
- 후보에 없는 export 추가는 원칙적으로 금지 (CLI 패턴 개선으로 대응)

### 상세 형식 (복잡한 모듈에서 추가 가능)

public interface가 5개 초과이거나 도메인 맥락이 풍부한 경우, `format-exports` 후보 목록을 기반으로 각 항목에 상세 설명 블록을 추가할 수 있습니다:

```markdown
#### validateToken
`validateToken(token: string) -> Claims`

JWT 토큰을 검증하고 Claims를 추출합니다.
- **입력**: Bearer 토큰 (Authorization 헤더에서 추출된 문자열)
- **출력**: 사용자 식별 정보와 권한 목록을 포함하는 Claims
- **역할**: API 요청의 인증 게이트키퍼
- **도메인 맥락**: PCI-DSS 준수를 위해 7일 만료 정책 적용
```

**주의**: 상세 형식에서도 시그니처는 `format-exports` 출력을 기준으로 사용합니다.

**선택 기준**: public interface가 5개 이하이고 도메인 맥락이 적으면 간략 형식(description만 추가), 그 외 상세 형식.

---

## Behavior 형식

시나리오 레벨 (input → output)로 명시:

```markdown
### 정상 케이스
- 유효한 토큰 → Claims 객체 반환
- 만료된 토큰 + refresh 옵션 → 새 토큰 쌍 반환

### 에러 케이스
- 잘못된 형식의 토큰 → InvalidTokenError
- 위조된 토큰 → SignatureVerificationError
```

---

## Contract 추출 패턴

Contract 정보 추출 소스:

1. **JSDoc/docstring 태그**: `@precondition`, `@postcondition`, `@invariant`, `@throws`
2. **코드 패턴 추론**:
   - `if (!x.prop) throw new Error` → precondition: `x.prop is required`
   - `if (arr.length === 0) throw` → precondition: `arr not empty`
   - `asserts x is T` → precondition: `x must be T`
   - Validation 로직 (guard clauses) → preconditions

없으면 `None` 명시.

---

## Protocol 추출 패턴

Protocol 정보 추출 소스:

1. **State enum**: `enum State { Idle, Loading, Loaded, Error }` 패턴 → State Machine 섹션
2. **Lifecycle 태그**: `@lifecycle N` 순서 태그 → Lifecycle 섹션
3. **순서 의존 호출**: `init()` → `start()` → `stop()` 패턴 → Lifecycle

없으면 `None` 명시.

---

## Phase 4.5: IMPLEMENTS.md 초안 생성 (HOW - 전체 섹션)

분석 결과와 사용자 응답을 기반으로 IMPLEMENTS.md를 직접 생성합니다:

다음 템플릿에 맞게 IMPLEMENTS.md를 작성합니다:

```markdown
# {directory_name}/IMPLEMENTS.md
<!-- 소스코드에서 읽을 수 없는 "왜?"와 "어떤 맥락?"을 기술 -->

<!-- ═══════════════════════════════════════════════════════ -->
<!-- PLANNING SECTION - /impl 이 업데이트                     -->
<!-- ═══════════════════════════════════════════════════════ -->

## Dependencies Direction

### External
{외부 의존성과 선택 이유 (사용자 응답 기반)}

### Internal
{resolved CLAUDE.md 경로 기반 내부 의존성: 심볼 + 선택 이유 포함}

## Implementation Approach

### 전략
{코드에서 추론된 전략 또는 분석 결과}

### 고려했으나 선택하지 않은 대안
{사용자 응답 기반 대안 또는 "None"}

## Technology Choices

{기술 선택 근거 (사용자 응답 기반) 또는 "None"}

<!-- ═══════════════════════════════════════════════════════ -->
<!-- IMPLEMENTATION SECTION - /compile 이 업데이트            -->
<!-- ═══════════════════════════════════════════════════════ -->

## Algorithm

{분석에서 추출한 알고리즘 또는 "(No complex algorithms found)"}

## Key Constants

{분석에서 추출한 상수와 도메인 맥락 또는 "(No domain-significant constants)"}

## Error Handling

{에러 처리 패턴 또는 "None"}

## State Management

{상태 관리 패턴 또는 "None"}

## Implementation Guide

- {current_date}: Initial extraction from existing code
```

대상 디렉토리에 직접 Write합니다 (${TMP_DIR} 미사용).

---

## IMPLEMENTS.md 조건부 섹션 작성 기준

### Algorithm — "이 로직을 이해하려면 코드를 읽어야 하는가?"
- **Yes** → Algorithm 섹션에 기술
- **No** → `(No complex algorithms found)` 표기

### Key Constants — "이 값이 변경되면 비즈니스에 영향이 있는가?"
- **Yes** → Key Constants 섹션에 기술 (테이블: Name, Value, Rationale, 영향 범위)
- **No** → `(No domain-significant constants)` 표기

### Implementation Guide — "소스코드 탐색 없이 구현 시작점을 알 수 있는가?"
- 의존 모듈의 CLAUDE.md 경로 + Export 이름 나열
- 도메인 맥락(값의 근거)은 CLAUDE.md Domain Context에 기술

---

## IMPLEMENTS.md 축소 예시 (auth/)

```markdown
# auth/IMPLEMENTS.md

## Dependencies Direction
### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 기존 프로젝트 호환)
### Internal
- `../utils/crypto`: hashPassword, verifyPassword

## Implementation Approach
### 전략
- HMAC-SHA256 기반 토큰 검증, 메모리 캐시로 성능 최적화
### 고려했으나 선택하지 않은 대안
- RSA 서명: 키 관리 복잡성 → 내부 서비스라 HMAC 충분

## Technology Choices
| 선택 | 대안 | 선택 이유 |
|------|------|----------|
| jsonwebtoken | jose | 기존 코드베이스 호환성 |

## Algorithm
### tokenCache 무효화 전략
1. 토큰 갱신 시 → 해당 userId의 캐시 삭제
2. 주기적 정리 (5분) → 만료된 캐시 항목 제거

## Key Constants
| Name | Value | Rationale | 영향 범위 |
|------|-------|-----------|----------|
| TOKEN_EXPIRY_DAYS | 7 | PCI-DSS 요구사항 | 보안 정책 |

## Error Handling
| Error | Retry | Recovery | Log Level |
|-------|-------|----------|-----------|
| TokenExpiredError | ✗ | 401 반환, 재로그인 유도 | WARN |

## State Management
- tokenCache: Map<userId, CachedClaims>, 메모리 전용, 5분 주기 정리

## Implementation Guide
- 토큰 검증 → ../jwt/CLAUDE.md#validateToken
- 암호화 → ../utils/crypto/CLAUDE.md#hashPassword
```

---

## CLI 출력 JSON 구조

### boundary-resolve 출력

```json
{
  "path": "src/auth",
  "direct_files": [{"name": "index.ts", "type": "typescript"}, ...],
  "subdirs": [{"name": "jwt", "has_claude_md": true}, ...],
  "source_file_count": 3,
  "subdir_count": 2,
  "violations": [{"violation_type": "Parent", "reference": "../utils", "line_number": 15}]
}
```

### code-analyze 출력

```json
{
  "path": "src/auth",
  "exports": {
    "functions": [{"name": "validateToken", "signature": "validateToken(token: string): Promise<Claims>", "description": "..."}],
    "types": [...],
    "classes": [...]
  },
  "dependencies": {
    "external": ["jsonwebtoken"],
    "internal_raw": ["./types", "../utils/crypto"],
    "internal": [
      {"raw_import": "./types", "resolved_dir": "src/auth/types", "claude_md_path": "src/auth/types/CLAUDE.md", "resolution": "Exact"}
    ]
  },
  "behaviors": [{"input": "유효한 JWT 토큰", "output": "Claims 객체 반환", "category": "success"}],
  "analyzed_files": ["index.ts", "middleware.ts"]
}
```
Note: `internal` 필드는 `--tree-result`가 주어졌을 때만 채워짐.

### schema-validate 출력

```json
{"file": "path/CLAUDE.md", "valid": true, "errors": [], "warnings": []}
```
`valid: true`이면 통과. `valid: false`이면 `errors` 배열에서 이슈 확인.
