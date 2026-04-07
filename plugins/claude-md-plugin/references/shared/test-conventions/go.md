# Go Test Conventions

## Test Directory Structure

```
module/
├── foo.go
├── foo_test.go          ← unit tests (same package)
└── foo_integration_test.go  ← integration tests
```

Or with `testdata/`:

```
module/
├── foo.go
├── foo_test.go
└── testdata/
    └── fixture.json     ← test fixtures
```

## Unit Tests

- Location: Same directory as production code
- Naming: `<file>_test.go`, same package
- Framework: Built-in `testing` package

## Integration Tests

- Location: Same directory, or `_test` package suffix for black-box tests
- Naming: `<file>_integration_test.go`
- Build tag: `//go:build integration` to separate from unit tests
- Framework: Built-in `testing`, `testify`

## Acceptance Tests (BDD)

- Location: Same directory or dedicated `features/` directory
- Naming: `<name>.feature` + `<name>_test.go` (step defs)
- Framework: godog

## File Naming

| Type | Pattern | Example |
|------|---------|---------|
| Unit test | `<name>_test.go` | `parser_test.go` |
| Integration test | `<name>_integration_test.go` | `db_integration_test.go` |
| Feature file | `features/<name>.feature` | `features/login.feature` |

## Import Paths

- Same package: Direct access to unexported symbols
- Black-box (`_test` package): `import "module/package"` — only exported symbols

## Assertion Strength

**STRONG** (pass):
- `require.Equal(t, expected, actual)`, `assert.Equal(t, expected, actual)`
- `require.ErrorIs(t, err, target)`, `assert.ErrorAs(t, err, &target)`
- `if got != want { t.Errorf("got %v, want %v", got, want) }`

**ACCEPTABLE** (pass — when Constraint specifies shape/pattern, not exact value):
- `assert.Contains(t, str, substr)`, `require.IsType(t, &Type{}, val)`
- `assert.Regexp(t, pattern, str)`

**WEAK** (reject — must cite specific Constraint and required behavior):
- `require.NotNil(t, val)`, `assert.True(t, ok)`
- `require.NoError(t, err)` when Constraint specifies a return value
