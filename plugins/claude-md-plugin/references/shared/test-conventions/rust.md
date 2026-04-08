# Rust Test Conventions

## Test Directory Structure

```
module/
├── src/
│   ├── lib.rs          ← #[cfg(test)] mod tests { } (unit tests)
│   └── foo.rs          ← #[cfg(test)] mod tests { } (unit tests)
└── tests/
    ├── integration.rs   ← integration tests (separate binary)
    └── features/        ← .feature files (if using cucumber)
```

## Unit Tests

- Location: Same file as production code, inside `#[cfg(test)]` module
- Naming: `#[test] fn test_<behavior>() { }`
- Framework: Built-in `#[test]` or `rstest`

## Integration Tests

- Location: `tests/` directory at crate root
- Each `.rs` file in `tests/` compiles as a separate crate
- Shared helpers: `tests/common/mod.rs`
- Framework: Built-in `#[test]`, `cucumber` for BDD

## Acceptance Tests (BDD)

- Location: `tests/features/*.feature`
- Runner: `tests/cucumber.rs` (or similar harness)
- Framework: `cucumber-rs`

## File Naming

| Type | Pattern | Example |
|------|---------|---------|
| Unit test | Inline `#[cfg(test)]` | `src/parser.rs` → tests inside same file |
| Integration test | `tests/<name>.rs` | `tests/schema_validation.rs` |
| Feature file | `tests/features/<name>.feature` | `tests/features/parse_tree.feature` |

## Import Paths

- Unit tests: Use `use super::*;` or `use crate::<module>;`
- Integration tests: Use `use <crate_name>::<module>;`

## Assertion Strength

**STRONG** (pass):
- `assert_eq!(expected, actual)`, `assert_ne!(a, b)`
- `#[should_panic(expected = "specific message")]`

**ACCEPTABLE** (pass — when Constraint specifies shape/pattern, not exact value):
- `matches!(val, Pattern)`, `assert!(result.contains("substr"))`
- `assert!(matches!(val, Enum::Variant { .. }))`

**WEAK** (reject — must cite specific Constraint and required behavior):
- `assert!(result.is_some())`, `assert!(result.is_ok())`
- `assert!(!result.is_empty())`, `assert!(flag)`
