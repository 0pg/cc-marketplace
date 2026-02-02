# TypeScript/JavaScript Patterns

## Exports 추출

### 함수 export
```
Grep: "^export (async )?function \w+"
Grep: "^export const \w+ = (async )?\("
```

### 타입/인터페이스 export
```
Grep: "^export (interface|type) \w+"
```

### 클래스 export
```
Grep: "^export (abstract )?class \w+"
```

### default export
```
Grep: "^export default"
```

### 시그니처 추출
- Grep으로 export 라인 찾기
- Read로 해당 라인 + 다음 몇 줄 읽기 (브래킷 완성까지)
- `function name(params): ReturnType` 형식 추출

**멀티라인 처리**:
```typescript
// 제네릭이 긴 경우
export function createHandler<
  TInput extends BaseInput,
  TOutput extends BaseOutput
>(config: Config<TInput>): Handler<TOutput>

// Read로 { 전까지 읽어서 시그니처 완성
```

## Dependencies 추출

```
# External (node_modules)
Grep: "^import .* from ['\"]([^./][^'\"]*)['\"]"

# Internal (relative path)
Grep: "^import .* from ['\"](\.[^'\"]+)['\"]"
```

## Behavior 추론

### Try-Catch 패턴

```
Grep: "try \{"
Grep: "catch \((e|err|error)"
Grep: "throw new (\w+Error)"
```

**추론**:
- catch 블록의 에러 타입 → error behavior
- try 블록의 return → success behavior

### 시그니처 추출 예시

```typescript
// 입력
export async function validateToken(token: string): Promise<Claims> {

// Grep 결과
export async function validateToken

// Read로 전체 시그니처 추출
validateToken(token: string): Promise<Claims>
```
