# Rust Patterns

## Exports 추출

### pub 함수
```
Grep: "^pub (async )?fn \w+"
```

### pub 구조체
```
Grep: "^pub struct \w+"
```

### pub enum
```
Grep: "^pub enum \w+"
```

### pub type alias
```
Grep: "^pub type \w+"
```

### pub trait
```
Grep: "^pub trait \w+"
```

**Export 규칙**: `pub` 키워드 = public

## Dependencies 추출

```
# External crates
Grep: "^use ([a-z_]+)::"
Grep: "^use ::([a-z_]+)"

# Internal modules
Grep: "^use (crate|self|super)::"
Grep: "^mod \w+"
```

## Behavior 추론

### Result 패턴

```
Grep: "-> Result<"
Grep: "Err\((\w+)::"
Grep: "Ok\("
```

**추론**:
- `Err(Type::Variant)` → error behavior
- `Ok(value)` → success behavior
