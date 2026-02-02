# Java Patterns

## Exports 추출

### public 클래스
```
Grep: "^public (abstract |final )?class \w+"
```

### public 인터페이스
```
Grep: "^public interface \w+"
```

### public enum
```
Grep: "^public enum \w+"
```

### public 메서드
```
Grep: "^\s+public (static )?(final )?[\w<>\[\], ]+\s+\w+\("
```

### public 필드
```
Grep: "^\s+public (static )?(final )?[\w<>\[\], ]+\s+\w+\s*(=|;)"
```

**Export 규칙**: `public` 키워드 = public

### 시그니처 추출
- Grep으로 public 메서드 라인 찾기
- Read로 해당 라인 + 다음 몇 줄 읽기 (`{` 전까지)
- throws 절 포함하여 시그니처 완성

## Dependencies 추출

```
# Import 문 분석
Grep: "^import ([a-z]+(\.[a-z]+)+)\.\w+;?"

# 분류 기준:
# - java.*, javax.*, kotlin.* → 표준 라이브러리 (무시)
# - 외부 패키지 (com.*, org.*, io.* 등) → external
# - 같은 프로젝트 패키지 → internal
```

### 패키지 판정
1. `java.*`, `javax.*`, `kotlin.*`, `kotlinx.*` → 표준 라이브러리, 무시
2. `com.example.myapp.*` (프로젝트 패키지) → internal
3. 그 외 → external

## Behavior 추론

### Exception 패턴

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
