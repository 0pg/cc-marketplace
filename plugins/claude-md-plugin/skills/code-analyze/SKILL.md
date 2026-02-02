---
name: code-analyze
version: 1.0.0
description: (internal) 소스 코드를 분석하여 Exports, Dependencies, Behavior 추출
allowed-tools: [Read, Glob, Grep]
---

# Code Analyze Skill

## 목적

소스 코드를 분석하여 구조화된 정보를 추출합니다:
- Exports: public 함수, 클래스, 타입
- Dependencies: 외부/내부 의존성
- Behavior: 주요 동작 패턴

순수 코드 분석만 수행하며, CLAUDE.md 생성은 하지 않습니다.
**Regex 기반 분석**으로 외부 의존성 없이 동작합니다.

## 입력

```
target_path: 분석 대상 디렉토리 경로
boundary_file: boundary-resolve 결과 파일 경로
output_name: 출력 파일명 (디렉토리명 기반)
```

## 출력

`.claude/extract-results/{output_name}-analysis.json` 파일 생성

```json
{
  "path": "src/auth",
  "exports": {
    "functions": [
      {
        "name": "validateToken",
        "signature": "validateToken(token: string): Promise<Claims>",
        "description": "JWT 토큰 검증"
      }
    ],
    "types": [
      {
        "name": "Claims",
        "definition": "interface Claims { userId: string, role: Role, exp: number }",
        "description": "토큰 클레임"
      }
    ],
    "classes": []
  },
  "dependencies": {
    "external": ["jsonwebtoken"],
    "internal": ["./types", "../utils/crypto"]
  },
  "behaviors": [
    {
      "input": "유효한 JWT 토큰",
      "output": "Claims 객체 반환",
      "category": "success"
    },
    {
      "input": "만료된 토큰",
      "output": "TokenExpiredError",
      "category": "error"
    }
  ],
  "analyzed_files": ["index.ts", "middleware.ts", "types.ts"]
}
```

## 워크플로우

### Step 1: Boundary 파일 읽기

```
Read boundary_file → JSON 파싱
files_to_analyze = boundary.direct_files
```

### Step 2: 파일 확장자로 언어 감지

| 확장자 | 언어 |
|--------|------|
| `.ts`, `.tsx` | TypeScript |
| `.js`, `.jsx` | JavaScript |
| `.py` | Python |
| `.go` | Go |
| `.rs` | Rust |
| `.java` | Java |
| `.kt` | Kotlin |

### Step 3: 언어별 Regex 패턴으로 분석

#### TypeScript/JavaScript Exports

```
# 함수 export
Grep: "^export (async )?function \w+"
Grep: "^export const \w+ = (async )?\("

# 타입/인터페이스 export
Grep: "^export (interface|type) \w+"

# 클래스 export
Grep: "^export (abstract )?class \w+"

# default export
Grep: "^export default"
```

**시그니처 추출**:
- Grep으로 export 라인 찾기
- Read로 해당 라인 + 다음 몇 줄 읽기 (브래킷 완성까지)
- `function name(params): ReturnType` 형식 추출

#### Python Exports

```
# __all__ 정의 확인
Grep: "^__all__ = \["

# public 함수 (underscore로 시작하지 않음)
Grep: "^def [a-z][a-zA-Z0-9_]*\("

# 클래스 정의
Grep: "^class [A-Z]\w*"

# dataclass
Grep: "@dataclass"
```

**Export 판정**:
1. `__all__` 있으면 → 해당 목록만 export
2. `__all__` 없으면 → underscore로 시작하지 않는 최상위 정의

#### Go Exports

```
# 함수 (대문자로 시작)
Grep: "^func [A-Z]\w*\("

# 메서드 (대문자로 시작)
Grep: "^func \([^)]+\) [A-Z]\w*\("

# 타입 (대문자로 시작)
Grep: "^type [A-Z]\w+ (struct|interface|func)"

# 변수/상수 (대문자로 시작)
Grep: "^var [A-Z]\w+"
Grep: "^const [A-Z]\w+"
```

**Export 규칙**: 대문자로 시작 = public

#### Rust Exports

```
# pub 함수
Grep: "^pub (async )?fn \w+"

# pub 구조체
Grep: "^pub struct \w+"

# pub enum
Grep: "^pub enum \w+"

# pub type alias
Grep: "^pub type \w+"

# pub trait
Grep: "^pub trait \w+"
```

**Export 규칙**: `pub` 키워드 = public

#### Java Exports

```
# public 클래스
Grep: "^public (abstract |final )?class \w+"

# public 인터페이스
Grep: "^public interface \w+"

# public enum
Grep: "^public enum \w+"

# public 메서드
Grep: "^\s+public (static )?(final )?[\w<>\[\], ]+\s+\w+\("

# public 필드
Grep: "^\s+public (static )?(final )?[\w<>\[\], ]+\s+\w+\s*(=|;)"
```

**Export 규칙**: `public` 키워드 = public

**시그니처 추출**:
- Grep으로 public 메서드 라인 찾기
- Read로 해당 라인 + 다음 몇 줄 읽기 (`{` 전까지)
- throws 절 포함하여 시그니처 완성

#### Kotlin Exports

```
# 함수 (기본 public, private/internal 제외)
Grep: "^fun \w+\("
Grep: "^suspend fun \w+\("
Grep: "^private fun \w+\(" → 제외
Grep: "^internal fun \w+\(" → 제외

# 클래스 (기본 public)
Grep: "^(data |sealed |abstract |open )?class \w+"
Grep: "^private class \w+" → 제외
Grep: "^internal class \w+" → 제외

# object
Grep: "^object \w+"
Grep: "^companion object"

# interface
Grep: "^interface \w+"

# enum class
Grep: "^enum class \w+"
```

**Export 규칙**: 기본 public, `private`/`internal` 키워드는 제외

**시그니처 추출**:
- Grep으로 함수 정의 찾기
- Read로 해당 라인 읽기 (`: ReturnType` 포함)
- `Result<T>` 반환 타입 인식

### Step 4: Dependencies 추출

#### TypeScript/JavaScript

```
# External (node_modules)
Grep: "^import .* from ['\"]([^./][^'\"]*)['\"]"

# Internal (relative path)
Grep: "^import .* from ['\"](\.[^'\"]+)['\"]"
```

#### Python

```
# External
Grep: "^import (\w+)"
Grep: "^from (\w+) import"

# Internal (relative)
Grep: "^from (\.[^ ]+) import"
```

#### Go

```
# External (github, etc.)
Grep: "\"(github\.com/[^\"]+)\""
Grep: "\"([a-z]+\.[a-z]+/[^\"]+)\""

# Standard library
Grep: "\"([a-z]+)\""
```

#### Rust

```
# External crates
Grep: "^use ([a-z_]+)::"
Grep: "^use ::([a-z_]+)"

# Internal modules
Grep: "^use (crate|self|super)::"
Grep: "^mod \w+"
```

#### Java/Kotlin

```
# Import 문 분석
Grep: "^import ([a-z]+(\.[a-z]+)+)\.\w+;?"

# 분류 기준:
# - java.*, javax.*, kotlin.* → 표준 라이브러리 (무시)
# - 외부 패키지 (com.*, org.*, io.* 등) → external
# - 같은 프로젝트 패키지 → internal
```

**패키지 판정**:
1. `java.*`, `javax.*`, `kotlin.*`, `kotlinx.*` → 표준 라이브러리, 무시
2. `com.example.myapp.*` (프로젝트 패키지) → internal
3. 그 외 → external

### Step 5: Behavior 추론

코드 패턴에서 동작을 추론합니다.

#### Try-Catch 패턴 (TS/JS)

```
Grep: "try \{"
Grep: "catch \((e|err|error)"
Grep: "throw new (\w+Error)"
```

**추론**:
- catch 블록의 에러 타입 → error behavior
- try 블록의 return → success behavior

#### Error Return 패턴 (Go)

```
Grep: "return nil, (Err\w+|errors\.New)"
Grep: "return .*, nil"
```

**추론**:
- `return nil, Err*` → error behavior
- `return value, nil` → success behavior

#### Result 패턴 (Rust)

```
Grep: "-> Result<"
Grep: "Err\((\w+)::"
Grep: "Ok\("
```

**추론**:
- `Err(Type::Variant)` → error behavior
- `Ok(value)` → success behavior

#### Exception 패턴 (Python)

```
Grep: "raise \w+Error"
Grep: "except (\w+Error):"
```

#### Exception 패턴 (Java)

```
# throws 선언
Grep: "throws (\w+Exception)"

# catch 블록
Grep: "catch \((\w+Exception) "

# throw 문
Grep: "throw new (\w+Exception)"
```

**추론**:
- `throws XxxException` → error behavior
- `catch (XxxException e)` → error behavior
- return 문 → success behavior

#### Result 패턴 (Kotlin)

```
# Result 반환 타입
Grep: ": Result<"

# runCatching 사용
Grep: "runCatching \{"
Grep: "\.runCatching \{"

# recoverCatching
Grep: "\.recoverCatching \{"

# catch 블록 (try-catch)
Grep: "catch \(e: (\w+)\)"

# throw 문
Grep: "throw (\w+Exception)"
```

**추론**:
- `Result<T>` 반환 + `runCatching` → functional error handling
- throw XxxException → error behavior (Result.failure)
- 정상 반환 → success behavior (Result.success)

### Step 6: JSON 결과 생성

```json
{
  "path": "{target_path}",
  "exports": {
    "functions": [...],
    "types": [...],
    "classes": [...]
  },
  "dependencies": {
    "external": [...],
    "internal": [...]
  },
  "behaviors": [...],
  "analyzed_files": [...]
}
```

### Step 7: 결과 저장

```
mkdir -p .claude/extract-results
Write → .claude/extract-results/{output_name}-analysis.json
```

## 실행 예시

```
# 입력
target_path: src/auth
boundary_file: .claude/extract-results/auth-boundary.json
output_name: auth

# 실행 순서
1. Read .claude/extract-results/auth-boundary.json
   → direct_files: ["index.ts", "middleware.ts", "types.ts"]

2. For each file in direct_files:
   a. 확장자 → TypeScript
   b. Grep "^export (async )?function" → validateToken, generateToken
   c. Read 해당 라인 → 시그니처 추출
   d. Grep "^export (interface|type)" → Claims, TokenConfig
   e. Grep "^import .* from" → jsonwebtoken, ./types
   f. Grep "try \{" + "catch" → error behaviors

3. Write .claude/extract-results/auth-analysis.json
```

## 결과 반환

```
---code-analyze-result---
output_file: .claude/extract-results/{output_name}-analysis.json
status: success
exports_count: {함수 + 타입 + 클래스 수}
dependencies_count: {외부 + 내부 의존성 수}
behaviors_count: {동작 패턴 수}
files_analyzed: {분석된 파일 수}
---end-code-analyze-result---
```

## 상세 패턴 참조

### TypeScript 시그니처 추출

```typescript
// 입력
export async function validateToken(token: string): Promise<Claims> {

// Grep 결과
export async function validateToken

// Read로 전체 시그니처 추출
validateToken(token: string): Promise<Claims>
```

**멀티라인 처리**:
```typescript
// 제네릭이 긴 경우
export function createHandler<
  TInput extends BaseInput,
  TOutput extends BaseOutput
>(config: Config<TInput>): Handler<TOutput>

// Read로 { 전까지 읽어서 시그니처 완성
```

### Python 독스트링 추출

```python
def validate_token(token: str, secret: str) -> Claims:
    """
    Validate a JWT token and return claims.

    Args:
        token: The JWT token to validate
        ...
    """
```

**추출**:
1. Grep으로 함수 정의 찾기
2. Read로 다음 줄들 읽기
3. `"""..."""` 사이 첫 줄 = description

### Go 주석 추출

```go
// ValidateToken validates a JWT token and returns claims.
func ValidateToken(tokenString string, secret string) (*Claims, error) {
```

**추출**:
1. Grep으로 함수 정의 찾기
2. 이전 줄 Read → `// Name ...` 형식이면 description

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 읽기 실패 | 경고 로그, 해당 파일 스킵, analyzed_files에서 제외 |
| 시그니처 추출 실패 | `signature: "unknown"` 표시 |
| 빈 디렉토리 | 빈 분석 결과 반환 (exports/deps/behaviors 모두 빈 배열) |
| 지원하지 않는 확장자 | 해당 파일 스킵 |

## 테스트

테스트 fixtures와 expected 결과는 다음 위치에 있습니다:

```
plugins/claude-md-plugin/fixtures/
├── typescript/     # TypeScript 테스트 코드
├── python/         # Python 테스트 코드
├── go/             # Go 테스트 코드
├── rust/           # Rust 테스트 코드
├── java/           # Java 테스트 코드
├── kotlin/         # Kotlin 테스트 코드
└── expected/       # 예상 분석 결과 JSON
```

Gherkin 테스트: `skills/code-analyze/tests/code_analyze.feature`
