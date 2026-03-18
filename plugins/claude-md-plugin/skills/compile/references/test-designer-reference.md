# Test Designer Reference

이 문서는 test-designer agent가 런타임에 로드하는 참조 문서입니다.
Export Interface Test 방법론, 언어별 시그니처 검증 패턴, Spec→Test 변환 규칙을 정의합니다.

## 1. INV-EXPORT 불변식

**Exports = 계약**: CLAUDE.md Exports의 시그니처는 코드가 만족해야 할 계약이다.

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

## 4. Contract Test 강제 생성 규칙

Contract 섹션이 `None`이 아닌 경우, Contract 위반 테스트를 **반드시** 생성합니다.

### Contract 테스트 구조

```
describe('Contract Tests', () => {
  describe('Preconditions', () => {
    // 각 precondition 위반 시나리오
  });
  describe('Postconditions', () => {
    // 각 postcondition 검증 시나리오
  });
  describe('Error Contracts', () => {
    // 각 throws 조항 검증
  });
});
```

### Precondition → Test 변환

```
CLAUDE.md Contract: preconditions: ["token must be non-empty string"]
```

```typescript
describe('Preconditions', () => {
  it('should reject empty token (precondition: token must be non-empty)', async () => {
    await expect(validateToken('')).rejects.toThrow();
  });
  it('should reject null token (precondition: token must be non-empty)', async () => {
    await expect(validateToken(null as any)).rejects.toThrow();
  });
});
```

### Postcondition → Test 변환

```
CLAUDE.md Contract: postconditions: ["returns Claims with valid userId"]
```

```typescript
describe('Postconditions', () => {
  it('should return Claims with valid userId (postcondition)', async () => {
    const result = await validateToken(validToken);
    expect(result.userId).toBeTruthy();
    expect(typeof result.userId).toBe('string');
  });
});
```

### Throws → Test 변환

```
CLAUDE.md Contract: throws: ["InvalidTokenError on malformed token"]
```

```typescript
describe('Error Contracts', () => {
  it('should throw InvalidTokenError on malformed token', async () => {
    await expect(validateToken('malformed'))
      .rejects.toThrow(InvalidTokenError);
  });
});
```

### Invariant → Test 변환

```
CLAUDE.md Contract: invariants: ["cache size <= maxSize"]
```

```typescript
describe('Invariants', () => {
  it('should maintain cache size <= maxSize after operations', () => {
    const cache = new Cache(10);
    for (let i = 0; i < 20; i++) cache.add(i);
    expect(cache.size).toBeLessThanOrEqual(10);
  });
});
```

### 강제 생성 조건

| 조건 | 동작 |
|------|------|
| Contract 섹션 없음 | Contract Tests 생략 |
| Contract: None | Contract Tests 생략 |
| Contract에 preconditions 있음 | precondition 위반 테스트 필수 |
| Contract에 postconditions 있음 | postcondition 검증 테스트 필수 |
| Contract에 throws 있음 | 에러 계약 테스트 필수 |
| Contract에 invariants 있음 | 불변식 검증 테스트 필수 |

## 5. Async Contract Test 생성 규칙

Async Contract 섹션이 `None`이 아닌 경우 비동기 패턴 테스트를 생성합니다.

### Execution Order → Test 변환

```
CLAUDE.md Async Contract: execution_order: ["fetchUser → validatePermissions → executeAction (순차)"]
```

```typescript
describe('Async Contract Tests', () => {
  it('should execute in order: fetchUser → validatePermissions → executeAction', async () => {
    const callOrder: string[] = [];
    // spy on each function to record call order
    vi.spyOn(module, 'fetchUser').mockImplementation(async () => { callOrder.push('fetchUser'); });
    vi.spyOn(module, 'validatePermissions').mockImplementation(async () => { callOrder.push('validatePermissions'); });
    vi.spyOn(module, 'executeAction').mockImplementation(async () => { callOrder.push('executeAction'); });

    await module.process(request);
    expect(callOrder).toEqual(['fetchUser', 'validatePermissions', 'executeAction']);
  });
});
```

### Cancellation → Test 변환

```typescript
it('should support cancellation via AbortSignal', async () => {
  const controller = new AbortController();
  const promise = module.longRunningOperation({ signal: controller.signal });
  controller.abort();
  await expect(promise).rejects.toThrow(/abort/i);
});
```

### Timeout → Test 변환

```typescript
it('should timeout API calls after 5000ms', async () => {
  vi.useFakeTimers();
  const promise = module.callExternalApi();
  vi.advanceTimersByTime(5001);
  await expect(promise).rejects.toThrow(/timeout/i);
  vi.useRealTimers();
});
```

## 5.5. Error Taxonomy Test 생성 규칙

Error Taxonomy 섹션이 `None`이 아닌 경우 에러 계층 테스트를 생성합니다.

### Error Hierarchy → instanceof 체인 테스트

```
CLAUDE.md Error Taxonomy: error_hierarchy: "AppError > AuthError > InvalidTokenError"
```

```typescript
describe('Error Taxonomy Tests', () => {
  it('InvalidTokenError should be instanceof AuthError', () => {
    const error = new InvalidTokenError('bad token');
    expect(error).toBeInstanceOf(AuthError);
    expect(error).toBeInstanceOf(AppError);
  });

  it('AuthError should be instanceof AppError', () => {
    const error = new AuthError('auth failed');
    expect(error).toBeInstanceOf(AppError);
  });
});
```

### Recovery Strategy → 재시도 테스트

```typescript
it('should retry NetworkError 3 times with exponential backoff', async () => {
  const spy = vi.fn().mockRejectedValue(new NetworkError('connection failed'));
  await expect(module.callWithRetry(spy)).rejects.toThrow(NetworkError);
  expect(spy).toHaveBeenCalledTimes(4); // 1 initial + 3 retries
});
```

### Propagation → 에러 변환 테스트

```typescript
it('should convert AuthError to HTTP 401', () => {
  const error = new InvalidTokenError('bad token');
  const httpError = module.toHttpError(error);
  expect(httpError.status).toBe(401);
});
```

## 5.6. Concurrency Model Test 생성 규칙

Concurrency Model 섹션이 `None`이 아닌 경우 동시성 테스트를 생성합니다.

### Thread Safety → 동시 접근 테스트

```typescript
describe('Concurrency Model Tests', () => {
  it('TokenCache should be safe under concurrent access', async () => {
    const cache = new TokenCache();
    const operations = Array.from({ length: 100 }, (_, i) =>
      i % 2 === 0
        ? cache.set(`key-${i}`, `value-${i}`)
        : cache.get(`key-${i - 1}`)
    );
    // Should not throw or corrupt data
    await Promise.all(operations);
    expect(cache.size).toBeLessThanOrEqual(100);
  });
});
```

### Race Condition Prevention → 정합성 테스트

```typescript
it('should maintain data integrity under concurrent writes', async () => {
  const counter = new AtomicCounter(0);
  await Promise.all(
    Array.from({ length: 50 }, () => counter.increment())
  );
  expect(counter.value).toBe(50);
});
```

## 6. Spec→Test 변환 규칙

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

## 6. Behavior→Test 변환

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

## 7. Mock 전략

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

## 8. Incremental 모드

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

## 9. 피드백 루프 (에러 컨텍스트)

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
