# Go Patterns

## Exports 추출

### 함수 (대문자로 시작)
```
Grep: "^func [A-Z]\w*\("
```

### 메서드 (대문자로 시작)
```
Grep: "^func \([^)]+\) [A-Z]\w*\("
```

### 타입 (대문자로 시작)
```
Grep: "^type [A-Z]\w+ (struct|interface|func)"
```

### 변수/상수 (대문자로 시작)
```
Grep: "^var [A-Z]\w+"
Grep: "^const [A-Z]\w+"
```

**Export 규칙**: 대문자로 시작 = public

## Dependencies 추출

```
# External (github, etc.)
Grep: "\"(github\.com/[^\"]+)\""
Grep: "\"([a-z]+\.[a-z]+/[^\"]+)\""

# Standard library
Grep: "\"([a-z]+)\""
```

## Behavior 추론

### Error Return 패턴

```
Grep: "return nil, (Err\w+|errors\.New)"
Grep: "return .*, nil"
```

**추론**:
- `return nil, Err*` → error behavior
- `return value, nil` → success behavior

### 주석 추출

```go
// ValidateToken validates a JWT token and returns claims.
func ValidateToken(tokenString string, secret string) (*Claims, error) {
```

**추출**:
1. Grep으로 함수 정의 찾기
2. 이전 줄 Read → `// Name ...` 형식이면 description
