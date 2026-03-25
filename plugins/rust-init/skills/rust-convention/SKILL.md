---
name: rust-convention
description: |
  Rust code convention reference. Load this skill when:
  - generating or modifying Rust source files (.rs)
  - deciding between trait dispatch and enum dispatch for polymorphism
  - implementing error handling with Result types
  - structuring modules or naming types, functions, and files
user_invocable: false
---

# Rust Code Convention

---

## 1. Error Handling

### Library code → `thiserror`
Crates and public modules define concrete error types.

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

### Application code → `anyhow`
Binaries and entry points propagate errors with `anyhow::Result`.

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = load_config()
        .context("failed to load config")?;
    run(config)?;
    Ok(())
}
```

### Boundary rule
- Public module boundaries within the same crate follow the library rule
- `anyhow` is only used in `main`, CLI handlers, and test code

---

## 2. Panic-Free

`unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `unreachable!()` are denied.
Enforced at compile time via `[lints.clippy]` deny rules.

### Replacement patterns

| Denied | Replacement |
|--------|-------------|
| `.unwrap()` | `?`, `.unwrap_or_default()`, `.ok_or_else(\|\| ...)` |
| `.expect("msg")` | `.context("msg")?` (anyhow) |
| `panic!("msg")` | `return Err(...)` |
| `todo!()` | Compile error or return an `unimplemented` error variant |
| `unreachable!()` | `unreachable` error variant, or eliminate impossible states via the type system |

### Example

```rust
// BAD
let value = map.get("key").unwrap();

// GOOD
let value = map.get("key")
    .ok_or_else(|| AppError::MissingKey("key".into()))?;
```

---

## 3. Polymorphism — Trait vs Enum Dispatch

### Open (externally extensible) → Trait dispatch
Use when external crates need to add implementations.

```rust
pub trait Storage: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&self, key: &str, value: &[u8]) -> Result<(), StorageError>;
}

// External crates can freely implement:
// impl Storage for RedisStorage { ... }
// impl Storage for S3Storage { ... }
```

### Closed (internally fixed) → Enum dispatch
Use when variants are fixed and dispatched via pattern matching.

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

### Decision criteria
- **"Should external crates be able to add variants of this type?"** → Yes: trait, No: enum
- Use `#[non_exhaustive]` on enums to allow future variant additions

---

## 4. Module Style

Use `{module}.rs` pattern. Do not use `mod.rs`.

```
src/
├── lib.rs
├── parser.rs          # mod parser
├── parser/
│   ├── lexer.rs       # mod parser::lexer (declared in parser.rs: mod lexer;)
│   └── token.rs       # mod parser::token
└── config.rs          # mod config
```

---

## 5. Naming Rules

| Target | Style | Example |
|--------|-------|---------|
| Files | snake_case | `order_handler.rs` |
| Types (struct, enum, trait) | PascalCase | `OrderHandler` |
| Functions, methods | snake_case | `process_order` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` |
| Type parameters | Single uppercase or PascalCase | `T`, `Item` |
| Crate names | kebab-case (Cargo.toml) / snake_case (code) | `my-crate` / `my_crate` |
