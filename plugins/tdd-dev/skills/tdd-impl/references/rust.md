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

## 테스트 용이성 패턴

Rust에서 테스트가 어려우면 trait 기반 의존성 주입으로 해결합니다.

### 시간 추상화 (Clock 패턴)

`Utc::now()` 직접 호출은 테스트에서 시간 제어가 불가능합니다.

```rust
// trait으로 추상화
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> { Utc::now() }
}

// 프로덕션 코드: trait을 주입받음
pub struct TokenValidator<C: Clock> {
    clock: C,
}

// 테스트: 고정 시간 사용
#[cfg(test)]
struct FixedClock(DateTime<Utc>);
#[cfg(test)]
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> { self.0 }
}
```

### Trait 기반 의존성 주입

```rust
// Bad: 구체 타입에 직접 의존
pub fn process(db: &PostgresPool) -> Result<()> { ... }

// Good: trait으로 추상화
pub fn process(repo: &impl OrderRepository) -> Result<()> { ... }
```

**적용 기준:**
- 외부 I/O (DB, HTTP, 파일시스템) → trait 추상화 필수
- 비결정적 값 (시간, 난수, UUID) → trait 추상화 필수
- 순수 계산 로직 → trait 불필요, 직접 단위 테스트

### #[cfg(test)] 모듈 패턴

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockRepo { /* ... */ }
    impl OrderRepository for MockRepo { /* ... */ }

    #[test]
    fn test_process_order() {
        let repo = MockRepo { /* setup */ };
        assert!(process(&repo).is_ok());
    }
}
```

## Export 불변식 테스트 패턴

CLAUDE.md Exports에서 STRUCT-XXX가 추출된 경우, Rust에서 구조적 불변식을 검증하는 패턴입니다.

### 함수 시그니처 검증

함수 포인터 바인딩으로 시그니처 불변식을 컴파일 타임에 검증합니다.

```rust
#[test]
fn validate_token_exists_with_correct_signature() {
    // 함수 포인터에 바인딩 → 시그니처 불일치 시 컴파일 에러
    let _f: fn(&str) -> Result<Claims, TokenError> = validate_token;
}
```

### 타입 필드 검증

구조체 필드 접근으로 타입 구조 불변식을 검증합니다.

```rust
#[test]
fn claims_has_required_fields() {
    let claims = Claims { sub: "user".into(), exp: 0, iat: 0 };
    let _: &str = claims.sub.as_str();
    let _: u64 = claims.exp;
    let _: u64 = claims.iat;
}
```

### Enum variants 검증

각 variant를 생성하여 존재를 검증합니다.

```rust
#[test]
fn token_error_variants_exist() {
    let _ = TokenError::Expired;
    let _ = TokenError::Invalid("".into());
    let _ = TokenError::MissingClaim("sub".into());
}
```

### Trait export 검증

`_probe` 함수 패턴으로 trait 메서드 시그니처를 검증합니다.
`_` prefix는 "호출되지 않는 검증 전용 함수"를 나타내며, 컴파일러 경고를 억제합니다.

```rust
#[test]
fn token_store_trait_has_required_methods() {
    // trait 메서드 시그니처 검증 (구현 불필요)
    fn _probe<T: TokenStore>(store: &T, token: &str) {
        let _: Result<(), TokenError> = store.revoke(token);
        let _: bool = store.is_revoked(token);
    }
}
```

### Contract 불변식 검증

> Contract 테스트는 STRUCT RED-GREEN 사이클에서 작성합니다.
> GREEN 단계에서 최소 스텁(빈 문자열 → 에러, 유효 입력 → 기본값)을 구현합니다.
> 행위 로직(REQ-XXX)과는 분리되며, 계약 위반/보장만 검증합니다.

사전조건/사후조건을 별도 테스트로 분리합니다.

```rust
#[test]
fn validate_token_rejects_empty_token() {
    let err = validate_token("").unwrap_err();
    assert!(matches!(err, TokenError::Invalid(_)));
}

#[test]
fn validate_token_guarantees_exp_field() {
    let claims = validate_token("valid-token").unwrap();
    assert!(claims.exp > 0, "Claims must always contain exp > 0");
}
```

### 테스트 파일 구조

구조적/계약/행위 테스트를 모듈로 분리합니다.

```rust
#[cfg(test)]
mod structural_tests {
    //! STRUCT-XXX: Export 존재 및 시그니처 검증
}

#[cfg(test)]
mod contract_tests {
    //! STRUCT-XXX: Contract 사전/사후조건 검증
}

#[cfg(test)]
mod behavior_tests {
    //! REQ-XXX: 행위적 요구사항 검증
}
```

## Private Dependencies

```toml
rust-modules = { git = "https://github.com/0pg/rust-modules", branch = "main" }
```
