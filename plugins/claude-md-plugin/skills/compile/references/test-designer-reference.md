# Test Designer Reference

이 문서는 test-designer agent가 런타임에 로드하는 참조 문서입니다.
Export Interface Test 방법론, 언어별 시그니처 검증 패턴, Spec→Test 변환 규칙을 정의합니다.

## 1. INV-EXPORT 불변식

**Exports = 불변식**: CLAUDE.md Exports의 시그니처는 불변이다.

- test-designer가 생성한 Export Interface Tests의 assertion은 **불변 구조**
- compiler agent(GREEN phase)는 이 테스트를 수정할 수 없음
- Export Interface Test가 실패하면 → 구현을 시그니처에 맞춰 수정 (테스트 변경 금지)
- CLAUDE.md에 없는 항목은 public으로 노출하지 않음

## 2. Export Interface Test 방법론

### 원칙

**시그니처를 "있는 그대로" 테스트로 변환**:

1. CLAUDE.md에 `validateToken(token: string): Promise<Claims>`이면:
   - `Promise<Claims>` 반환 타입을 그대로 검증
   - `token: string` 파라미터를 그대로 검증
   - 시그니처를 해석하거나 단순화하지 않음

2. 타입/인터페이스에 `Claims { userId: string, exp: number, permissions: Permission[] }`이면:
   - 각 필드의 존재와 타입을 검증
   - 필드를 추가하거나 제거하지 않음

3. Enum에 `Status: Active | Inactive | Pending`이면:
   - 각 variant의 존재를 검증
   - variant를 추가하거나 제거하지 않음

### 테스트 구조

```
describe('Export Interface Tests', () => {
  // 각 export에 대해 하나의 테스트
  // 구조 검증만 — 동작 검증은 Behavior Tests에서
});

describe('Behavior Tests', () => {
  // 각 behavior 시나리오에 대해 하나의 테스트
});
```

## 3. 언어별 시그니처 검증 패턴

### TypeScript

타입 시스템을 활용한 컴파일 타임 검증:

```typescript
import { validateToken } from './auth';
import type { Claims } from './auth';

describe('Export Interface Tests', () => {
  it('validateToken has correct signature', () => {
    // 타입 어노테이션으로 시그니처 강제
    const fn: (token: string) => Promise<Claims> = validateToken;
    expect(typeof fn).toBe('function');
  });

  it('Claims type has required fields', () => {
    // 타입 레벨 검증 (컴파일 타임)
    const claims: Claims = {} as Claims;
    // 런타임에는 존재 확인만
    const keys: (keyof Claims)[] = ['userId', 'exp', 'permissions'];
    expect(keys).toBeDefined();
  });
});
```

### Python

`inspect.signature()` 기반 런타임 검증:

```python
import inspect
from auth import validate_token

class TestExportInterfaces:
    def test_validate_token_signature(self):
        sig = inspect.signature(validate_token)
        params = list(sig.parameters.keys())
        assert params == ['token']
        assert sig.parameters['token'].annotation == str

    def test_validate_token_return_type(self):
        hints = get_type_hints(validate_token)
        assert hints['return'] == Claims
```

### Go

변수 할당으로 컴파일 타임 시그니처 검증:

```go
func TestExportInterfaces(t *testing.T) {
    // 컴파일 타임에 시그니처 불일치 시 빌드 실패
    var _ func(string) (Claims, error) = ValidateToken
}
```

### Rust

함수 포인터 할당으로 컴파일 타임 검증:

```rust
#[test]
fn test_export_interfaces() {
    // 시그니처 불일치 시 컴파일 에러
    let _: fn(&str) -> Result<Claims, AuthError> = validate_token;
}
```

### Java

리플렉션 기반 검증:

```java
@Test
void validateTokenHasCorrectSignature() throws Exception {
    Method method = AuthService.class.getMethod("validateToken", String.class);
    assertEquals(Claims.class, method.getReturnType());
}
```

### Kotlin

리플렉션 기반 검증:

```kotlin
@Test
fun `validateToken has correct signature`() {
    val method = AuthService::class.java.getMethod("validateToken", String::class.java)
    assertEquals(Claims::class.java, method.returnType)
}
```

## 4. Spec→Test 변환 규칙

### Function Export

| CLAUDE.md | 테스트 |
|-----------|-------|
| `funcName(p1: T1, p2: T2): R` | import 가능 + 파라미터 수/타입 + 반환 타입 |

### Type/Interface Export

| CLAUDE.md | 테스트 |
|-----------|-------|
| `TypeName { field1: T1, field2: T2 }` | import 가능 + 필드 존재 + 필드 타입 |

### Class Export

| CLAUDE.md | 테스트 |
|-----------|-------|
| `ClassName(p1: T1)` | import 가능 + new 가능 + constructor 파라미터 |

### Enum Export

| CLAUDE.md | 테스트 |
|-----------|-------|
| `EnumName: V1 \| V2 \| V3` | import 가능 + 모든 variant 존재 |

### Variable/Constant Export

| CLAUDE.md | 테스트 |
|-----------|-------|
| `CONST_NAME = value` | import 가능 + 값 또는 타입 일치 |
| `CONST_NAME: Type` | import 가능 + 타입 일치 |

## 5. Behavior→Test 변환

### Success 시나리오

```
CLAUDE.md: 유효한 토큰 → Claims 객체 반환
```

```typescript
it('should return Claims for valid token', async () => {
  const result = await validateToken(validToken);
  expect(result).toBeDefined();
  expect(result.userId).toBeDefined();
});
```

### Error 시나리오

```
CLAUDE.md: 만료된 토큰 → TokenExpiredError
```

```typescript
it('should throw TokenExpiredError for expired token', async () => {
  await expect(validateToken(expiredToken))
    .rejects.toThrow(TokenExpiredError);
});
```

### Contract Precondition

```
CLAUDE.md Contract: token must be non-empty string
```

```typescript
it('should reject empty token (precondition)', async () => {
  await expect(validateToken('')).rejects.toThrow();
});
```

### Contract Postcondition

```
CLAUDE.md Contract: returns Claims with valid userId
```

```typescript
it('should return Claims with valid userId (postcondition)', async () => {
  const result = await validateToken(validToken);
  expect(result.userId).toBeTruthy();
});
```

## 6. Mock 전략

### Dependency Mock 생성

dependency CLAUDE.md의 Exports를 읽어 mock을 생성:

```
dependency CLAUDE.md Exports:
  hashPassword(password: string): Promise<string>
  verifySignature(data: string, sig: string): boolean
```

**TypeScript (vitest):**
```typescript
vi.mock('../utils/crypto', () => ({
  hashPassword: vi.fn().mockResolvedValue('hashed'),
  verifySignature: vi.fn().mockReturnValue(true),
}));
```

**Python (unittest.mock):**
```python
@patch('auth.crypto.hash_password', return_value='hashed')
@patch('auth.crypto.verify_signature', return_value=True)
def test_validate(self, mock_verify, mock_hash):
    ...
```

### Mock 원칙

- Mock 인터페이스는 dependency CLAUDE.md Exports 기반 (소스코드 참조 금지)
- Mock 반환값은 테스트 시나리오에 맞게 설정
- Mock이 불필요한 경우 (순수 함수) mock 생략

## 7. Incremental 모드

### Delta 계산 (compile skill이 제공)

```
대상 exports:
  - { name: "revokeToken", action: "added", signature: "revokeToken(tokenId: string): Promise<void>" }
  - { name: "validateToken", action: "modified", signature: "validateToken(token: string, options?: ValidateOptions): Promise<Claims>" }
  - { name: "legacyAuth", action: "removed" }
```

### 처리 규칙

| Action | Export Interface Test | Behavior Test |
|--------|---------------------|---------------|
| added | 새 테스트 추가 | 관련 behavior 테스트 추가 |
| modified | 해당 테스트 수정 | 관련 behavior 테스트 수정 |
| removed | 해당 테스트 제거 | 관련 behavior 테스트 제거 |

### 기존 테스트 보존

- delta에 포함되지 않은 export의 테스트는 절대 수정하지 않음
- 테스트 파일의 구조 (import, describe 블록)는 유지하면서 delta만 Edit

## 8. 피드백 루프 (에러 컨텍스트)

compiler가 3회 재시도 후 실패하면, compile skill이 에러 컨텍스트와 함께 test-designer를 재호출합니다.

### 수정 가능 범위

| 수정 가능 | 수정 금지 |
|-----------|----------|
| Mock 설정 (누락된 mock 추가) | Export Interface Test assertion |
| Import 경로 | Behavior Test assertion |
| 테스트 인프라 (setup/teardown) | 시그니처 검증 로직 |
| 테스트 환경 설정 | 기대값 변경 |

### 에러 분류 및 대응

| 에러 유형 | 원인 | 수정 |
|-----------|------|------|
| `Cannot find module` | Mock 미설정 또는 import 경로 오류 | Mock 추가 또는 경로 수정 |
| `is not a function` | Mock 반환값 타입 오류 | Mock 구현 수정 |
| `TypeError` | 테스트 setup 문제 | Setup 코드 수정 |
| `assertion failed` | 구현 문제 (compiler가 해결해야 함) | 수정하지 않음 — compiler에게 위임 |
