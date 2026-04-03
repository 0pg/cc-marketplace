# Python Test Conventions

## Test Directory Structure

```
module/
├── src/
│   └── foo.py
└── tests/
    ├── __init__.py
    ├── test_foo.py              ← unit tests
    ├── test_foo_integration.py  ← integration tests
    └── conftest.py              ← shared fixtures
```

## Unit Tests

- Location: `tests/` directory at project/module root
- Naming: `test_<module>.py`, functions prefixed with `test_`
- Framework: pytest

## Integration Tests

- Location: `tests/`
- Naming: `test_<module>_integration.py`
- Framework: pytest

## Acceptance Tests (BDD)

- Location: `tests/features/` or `tests/`
- Naming: `<name>.feature` (Gherkin) + `test_<name>_acceptance.py` (step defs)
- Framework: pytest-bdd, behave

## File Naming

| Type | Pattern | Example |
|------|---------|---------|
| Unit test | `tests/test_<name>.py` | `tests/test_parser.py` |
| Integration test | `tests/test_<name>_integration.py` | `tests/test_db_integration.py` |
| Acceptance test | `tests/test_<name>_acceptance.py` | `tests/test_auth_acceptance.py` |
| Feature file | `tests/features/<name>.feature` | `tests/features/login.feature` |

## Import Paths

- Absolute: `from src.foo import bar`
- With conftest fixtures: Auto-discovered by pytest
