# Python Patterns

## Exports 추출

### __all__ 정의 확인
```
Grep: "^__all__ = \["
```

### public 함수 (underscore로 시작하지 않음)
```
Grep: "^def [a-z][a-zA-Z0-9_]*\("
```

### 클래스 정의
```
Grep: "^class [A-Z]\w*"
```

### dataclass
```
Grep: "@dataclass"
```

### Export 판정
1. `__all__` 있으면 → 해당 목록만 export
2. `__all__` 없으면 → underscore로 시작하지 않는 최상위 정의

## Dependencies 추출

```
# External
Grep: "^import (\w+)"
Grep: "^from (\w+) import"

# Internal (relative)
Grep: "^from (\.[^ ]+) import"
```

## Behavior 추론

### Exception 패턴

```
Grep: "raise \w+Error"
Grep: "except (\w+Error):"
```

### 독스트링 추출

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
