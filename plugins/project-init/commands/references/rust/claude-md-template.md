# {{PROJECT_NAME}}

## Build and Test Commands

- **Build**: `cargo build`
- **Test**: `cargo test`
- **Lint**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt --check`
- **Run**: `cargo run`

## Code Convention

### Rust Edition 2024 | Types: {{PROJECT_TYPES}}

### Rules
- **Panic-free**: unwrap(), expect(), panic!(), todo!(), unimplemented!(), unreachable!() 사용 금지
- **unsafe 금지**: unsafe 블록 사용 금지
- **Error handling**: Result + ? 연산자. Library -> thiserror, Application -> anyhow
- **Module style**: {module}.rs 패턴 사용 (mod.rs 금지)

### Naming Rules
| 대상 | 스타일 | 예시 |
|------|--------|------|
| Files | snake_case | order_handler.rs |
| Types | PascalCase | OrderHandler |
| Functions | snake_case | process_order |
| Constants | SCREAMING_SNAKE_CASE | MAX_RETRY_COUNT |

## Workflow
- Run `cargo test` after making code changes
- Run `cargo clippy` before committing
- Run `cargo fmt` to format code
