# code-convention.md Schema

이 템플릿은 프로젝트 루트의 `code-convention.md` 파일의 표준 구조를 정의합니다.

**code-convention.md = 프로젝트 코딩 스타일/컨벤션 가이드**
- `/project-setup` 실행 시 기존 코드 분석으로 자동 생성
- `/convention` 실행 시 업데이트
- `/compile` 시 참조 (REFACTOR 단계에서 적용)
- `/validate` 시 검증 (Convention 컬럼 포함)

## 섹션 요약

| 섹션 | 필수 | 설명 |
|------|------|------|
| Naming | ✓ | 변수, 함수, 타입, 상수, 파일 네이밍 규칙 |
| Formatting | ✓ | 들여쓰기, 줄 길이, 따옴표, 세미콜론 |
| Code Structure | 선택 | import 순서, export 스타일 |
| Error Handling | 선택 | 에러 처리 패턴 |
| Testing | 선택 | 테스트 프레임워크, 파일 패턴 |
| Project-Specific | 선택 | 프로젝트 고유 규칙 |

---

## 상세 설명

### 1. Naming (필수)

네이밍 규칙을 카테고리별로 정의합니다.

```markdown
## Naming

### Variables
- pattern: camelCase
- examples: userId, tokenExpiry, isValid

### Functions
- pattern: camelCase
- prefix: verb (get, set, is, has, create, update, delete, validate)
- examples: getUserById, validateToken, isExpired

### Types/Interfaces
- pattern: PascalCase
- suffix: (선택) Interface 접미사 사용 여부
- examples: User, AuthConfig, ITokenPayload

### Classes
- pattern: PascalCase
- examples: AuthService, TokenValidator

### Constants
- pattern: UPPER_SNAKE_CASE
- examples: MAX_RETRY_COUNT, API_TIMEOUT_MS

### Files
- pattern: kebab-case
- test_suffix: .test | .spec
- examples: auth-service.ts, token-validator.test.ts
```

**언어별 기본 패턴:**

| 언어 | Variables | Functions | Types | Constants | Files |
|------|-----------|-----------|-------|-----------|-------|
| TypeScript | camelCase | camelCase | PascalCase | UPPER_SNAKE | kebab-case |
| Python | snake_case | snake_case | PascalCase | UPPER_SNAKE | snake_case |
| Rust | snake_case | snake_case | PascalCase | UPPER_SNAKE | snake_case |
| Go | camelCase | PascalCase (exported) | PascalCase | PascalCase | snake_case |
| Java | camelCase | camelCase | PascalCase | UPPER_SNAKE | PascalCase |
| Kotlin | camelCase | camelCase | PascalCase | UPPER_SNAKE | PascalCase |

### 2. Formatting (필수)

코드 포맷팅 규칙을 정의합니다.

```markdown
## Formatting

- indentation: 2 spaces
- line_length: 100
- quotes: single
- semicolons: false
- trailing_comma: es5
- brace_style: 1tbs
```

**허용 값:**

| 항목 | 허용 값 |
|------|--------|
| indentation | `2 spaces`, `4 spaces`, `tab` |
| line_length | 80, 100, 120, etc. |
| quotes | `single`, `double` |
| semicolons | `true`, `false` |
| trailing_comma | `all`, `es5`, `none` |
| brace_style | `1tbs`, `allman`, `stroustrup` |

### 3. Code Structure (선택)

코드 구조 규칙을 정의합니다.

```markdown
## Code Structure

### Imports
- order: [builtin, external, internal, relative]
- grouping: true
- newline_between_groups: true

### Exports
- style: named
- barrel_files: false
- re_exports: allowed

### Function Length
- max_lines: 50
- max_parameters: 4
```

### 4. Error Handling (선택)

에러 처리 패턴을 정의합니다.

```markdown
## Error Handling

- style: throw
- custom_errors: true
- error_naming: {Domain}Error
- error_hierarchy: base → domain → specific
- examples:
  - AuthError → TokenExpiredError
  - ValidationError → InvalidInputError
```

### 5. Testing (선택)

테스트 관련 규칙을 정의합니다.

```markdown
## Testing

- framework: vitest
- file_pattern: *.test.ts
- location: co-located (같은 디렉토리)
- describe_pattern: "describe('ModuleName', ...)"
- assertion_style: expect
```

### 6. Project-Specific (선택)

프로젝트 고유의 추가 규칙을 자유 형식으로 기술합니다.

```markdown
## Project-Specific

- React 컴포넌트는 함수형만 사용 (클래스형 금지)
- API 호출은 반드시 service layer를 통해서만
- 환경 변수는 config/ 모듈에서만 접근
- 로깅은 Logger 클래스를 통해서만 수행
```

---

## 전체 예시

### TypeScript 프로젝트

```markdown
# Code Convention

## Naming

### Variables
- pattern: camelCase
- examples: userId, tokenExpiry, isValid

### Functions
- pattern: camelCase
- prefix: verb
- examples: getUserById, validateToken, isExpired

### Types/Interfaces
- pattern: PascalCase
- examples: User, AuthConfig, TokenPayload

### Constants
- pattern: UPPER_SNAKE_CASE
- examples: MAX_RETRY_COUNT, API_TIMEOUT_MS

### Files
- pattern: kebab-case
- test_suffix: .test
- examples: auth-service.ts, token-validator.test.ts

## Formatting

- indentation: 2 spaces
- line_length: 100
- quotes: single
- semicolons: false
- trailing_comma: es5

## Code Structure

### Imports
- order: [builtin, external, internal, relative]
- grouping: true

### Exports
- style: named
- barrel_files: true

## Error Handling

- style: throw
- custom_errors: true
- error_naming: {Domain}Error

## Testing

- framework: vitest
- file_pattern: *.test.ts
- location: co-located
```

### Python 프로젝트

```markdown
# Code Convention

## Naming

### Variables
- pattern: snake_case
- examples: user_id, token_expiry, is_valid

### Functions
- pattern: snake_case
- prefix: verb
- examples: get_user_by_id, validate_token, is_expired

### Types/Classes
- pattern: PascalCase
- examples: User, AuthConfig, TokenPayload

### Constants
- pattern: UPPER_SNAKE_CASE
- examples: MAX_RETRY_COUNT, API_TIMEOUT_MS

### Files
- pattern: snake_case
- test_prefix: test_
- examples: auth_service.py, test_token_validator.py

## Formatting

- indentation: 4 spaces
- line_length: 88
- quotes: double
- trailing_comma: all

## Error Handling

- style: raise
- custom_errors: true
- error_naming: {Domain}Error
- base_class: Exception

## Testing

- framework: pytest
- file_pattern: test_*.py
- location: tests/ directory
```
