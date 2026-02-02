# Kotlin Patterns

## Exports 추출

### 함수 (기본 public, private/internal 제외)
```
Grep: "^fun \w+\("
Grep: "^suspend fun \w+\("
Grep: "^private fun \w+\(" → 제외
Grep: "^internal fun \w+\(" → 제외
```

### 클래스 (기본 public)
```
Grep: "^(data |sealed |abstract |open )?class \w+"
Grep: "^private class \w+" → 제외
Grep: "^internal class \w+" → 제외
```

### object
```
Grep: "^object \w+"
Grep: "^companion object"
```

### interface
```
Grep: "^interface \w+"
```

### enum class
```
Grep: "^enum class \w+"
```

**Export 규칙**: 기본 public, `private`/`internal` 키워드는 제외

### 시그니처 추출
- Grep으로 함수 정의 찾기
- Read로 해당 라인 읽기 (`: ReturnType` 포함)
- `Result<T>` 반환 타입 인식

## Dependencies 추출

```
# Import 문 분석
Grep: "^import ([a-z]+(\.[a-z]+)+)\.\w+;?"

# 분류 기준 (Java와 동일):
# - java.*, javax.*, kotlin.*, kotlinx.* → 표준 라이브러리 (무시)
# - 외부 패키지 (com.*, org.*, io.* 등) → external
# - 같은 프로젝트 패키지 → internal
```

## Behavior 추론

### Result 패턴

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
