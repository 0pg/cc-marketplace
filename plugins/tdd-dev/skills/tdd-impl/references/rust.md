# Rust Code Convention

## 필수 규칙

- **Edition**: Rust 2024 (Edition 2024 features 적극 활용)
- **Error Handling**:
  - Library: `thiserror`
  - Application: `anyhow`
- **Async Runtime**: `tokio`
- **unsafe 금지**: `unsafe` 블록 사용 금지

## 네이밍

| 대상 | 스타일 | 예시 |
|------|--------|------|
| Files | `snake_case` | `order_handler.rs` |
| Types | `PascalCase` | `OrderHandler` |
| Functions | `snake_case` | `process_order` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_RETRY_COUNT` |

## 모듈 구조

`mod.rs` 대신 `foo.rs` + `foo/` 디렉터리 패턴 사용:

```
src/
├── order.rs        # mod order 정의
└── order/
    ├── handler.rs
    └── types.rs
```

## 에러 처리

- `Result` + `?` 연산자 사용
- 패닉 함수 금지 (`unwrap()`, `expect()` 등)
- 커스텀 에러 타입은 `thiserror` 매크로 활용

```rust
#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("Order not found: {0}")]
    NotFound(String),
}
```

## 품질

- `cargo clippy` 경고 0개 유지
- `pub` 항목은 문서화 필수
- `#[must_use]` 적극 활용

## 인자 타입

Borrowed types 선호로 유연성 확보:

| 지양 | 선호 | 이유 |
|------|------|------|
| `&String` | `&str` | String slice로 충분 |
| `&Vec<T>` | `&[T]` | slice로 충분 |
| `&Box<T>` | `&T` | 역참조로 충분 |

## 코드 작성 패턴

### Self 사용
```rust
// Good: 타입 변경 시 한 곳만 수정
impl OrderHandler {
    pub fn new() -> Self { Self { ... } }
}
```

### 필드 할당 순서
구조체 필드는 **선언 순서대로** 할당하여 누락 방지.

### Vec 생성
```rust
// Good
let v = Vec::new();
let v = Vec::with_capacity(10);

// Avoid
let v: Vec<T> = vec![];
```

### Iterator 수집
`collect()` 선호, `from_iter()` 지양.

## 변수 스코프

### Scoped Mutability
일시적 mutability는 블록으로 제한:

```rust
// Good
let config = {
    let mut c = Config::default();
    c.timeout = Duration::from_secs(30);
    c
};

// Avoid: mut이 이후에도 노출
let mut config = Config::default();
config.timeout = Duration::from_secs(30);
```

### 금지 패턴
- 값 없는 `let` 선언 금지
- 느슨한 스코프의 `let mut` 지양

## 함수 설계

### 반환 타입
```rust
// () 반환: 세미콜론으로 종료
fn process(&mut self) {
    self.items.clear();
}

// Result<()>: ? 연산자 + 명시적 Ok(())
fn save(&self) -> Result<()> {
    self.validate()?;
    self.persist()?;
    Ok(())
}
```

### Generic 복잡성 숨기기
- Public API: `impl Trait` 활용
- Lifetime elision 적극 활용
- 불필요한 turbofish 회피

## Builder 패턴

```rust
impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

impl ConfigBuilder {
    // self by value (not &mut self)
    pub fn timeout(self, t: Duration) -> Self {
        Self { timeout: Some(t), ..self }
    }

    // fallible build
    pub fn build(self) -> Result<Config> { ... }
}
```

## 유용한 패턴

### Default trait
```rust
#[derive(Default)]
struct Options {
    retries: u32,  // 0
    verbose: bool, // false
}
```

### mem::replace / mem::take
Clone 회피로 성능 개선:
```rust
use std::mem;
let old = mem::take(&mut self.buffer);
```

### Option as Iterator
Option은 0-1개 요소 컨테이너:
```rust
for item in optional_value {
    process(item);
}
```

## Private Dependencies

```toml
rust-modules = { git = "https://github.com/0pg/rust-modules", branch = "main" }
```
