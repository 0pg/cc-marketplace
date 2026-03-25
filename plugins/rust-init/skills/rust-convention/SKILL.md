---
name: rust-convention
description: |
  Rust 코드 컨벤션 레퍼런스. Rust 코드를 생성하거나 수정할 때 이 스킬을 참조하여
  프로젝트의 코딩 규칙을 준수합니다. 다른 스킬이 내부적으로 로드합니다.
user_invocable: false
---

# Rust Code Convention

---

## 1. Error Handling

### Library 코드 → `thiserror`
외부에 노출하는 크레이트/모듈은 구체적 에러 타입을 정의한다.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid header: {0}")]
    InvalidHeader(String),
    #[error("unexpected EOF at position {position}")]
    UnexpectedEof { position: usize },
}

pub fn parse(input: &str) -> Result<Parsed, ParseError> {
    // ...
}
```

### Application 코드 → `anyhow`
바이너리/엔트리포인트는 `anyhow::Result`로 에러를 전파한다.

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = load_config()
        .context("failed to load config")?;
    run(config)?;
    Ok(())
}
```

### 경계 규칙
- 같은 크레이트 내부에서도 **pub 모듈 경계**는 library 규칙 적용
- `anyhow`는 `main`, CLI handler, test 코드에서만 사용

---

## 2. Panic-Free

`unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `unreachable!()` 사용 금지.
`[lints.clippy]`에서 deny로 컴파일 에러 발생.

### 대체 패턴

| 금지 | 대체 |
|------|------|
| `.unwrap()` | `?`, `.unwrap_or_default()`, `.ok_or_else(\|\| ...)` |
| `.expect("msg")` | `.context("msg")?` (anyhow) |
| `panic!("msg")` | `return Err(...)` |
| `todo!()` | 컴파일 에러로 남기거나 `unimplemented` 에러 variant 반환 |
| `unreachable!()` | `unreachable` 에러 variant, 또는 타입 시스템으로 불가능한 상태 제거 |

### 예시

```rust
// BAD
let value = map.get("key").unwrap();

// GOOD
let value = map.get("key")
    .ok_or_else(|| AppError::MissingKey("key".into()))?;
```

---

## 3. Polymorphism — Trait vs Enum Dispatch

### 외부 확장 가능 (Open) → Trait dispatch
외부에서 구현체를 추가할 수 있어야 하는 경우.

```rust
pub trait Storage: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&self, key: &str, value: &[u8]) -> Result<(), StorageError>;
}

// 외부 크레이트에서 자유롭게 구현 가능
// impl Storage for RedisStorage { ... }
// impl Storage for S3Storage { ... }
```

### 내부로 닫힌 (Closed) → Enum dispatch
변형이 내부에서 고정되고 패턴 매칭으로 분기하는 경우.

```rust
pub enum Command {
    Create { name: String },
    Delete { id: u64 },
    Update { id: u64, payload: Payload },
}

impl Command {
    pub fn execute(&self, ctx: &Context) -> Result<(), AppError> {
        match self {
            Self::Create { name } => ctx.create(name),
            Self::Delete { id } => ctx.delete(*id),
            Self::Update { id, payload } => ctx.update(*id, payload),
        }
    }
}
```

### 판단 기준
- **"이 타입의 변형을 사용자(외부 크레이트)가 추가할 수 있어야 하는가?"** → Yes: trait, No: enum
- enum은 `#[non_exhaustive]`로 향후 variant 추가에 대비할 수 있음

---

## 4. Module Style

`{module}.rs` 패턴 사용. `mod.rs` 금지.

```
src/
├── lib.rs
├── parser.rs          # mod parser
├── parser/
│   ├── lexer.rs       # mod parser::lexer (parser.rs에서 mod lexer;)
│   └── token.rs       # mod parser::token
└── config.rs          # mod config
```

---

## 5. Naming Rules

| 대상 | 스타일 | 예시 |
|------|--------|------|
| Files | snake_case | `order_handler.rs` |
| Types (struct, enum, trait) | PascalCase | `OrderHandler` |
| Functions, methods | snake_case | `process_order` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` |
| Type parameters | 단일 대문자 또는 PascalCase | `T`, `Item` |
| Crate names | kebab-case (Cargo.toml) / snake_case (코드) | `my-crate` / `my_crate` |
