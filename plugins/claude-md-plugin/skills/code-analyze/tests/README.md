# Code Analyze Tests

Gherkin-based acceptance tests for the code-analyze skill.

## Setup

```bash
cd plugins/claude-md-plugin/skills/code-analyze/tests
pip install -r requirements.txt
```

## Running Tests

```bash
# Run all tests
pytest -v

# Run with Gherkin report
pytest --gherkin-terminal-reporter

# Run specific language tests
pytest -v -k "typescript"
pytest -v -k "python"
pytest -v -k "go"
pytest -v -k "rust"
pytest -v -k "java"
pytest -v -k "kotlin"

# Run behavior inference tests
pytest -v -k "behavior"

# Run edge case tests
pytest -v -k "empty or failure or mixed"
```

## Test Structure

- `code_analyze.feature` - Gherkin scenarios (37 scenarios)
- `conftest.py` - pytest-bdd step definitions
- `test_code_analyze.py` - pytest-bdd test runner
- `analyzer/` - Language-specific regex analyzers
  - `base.py` - Base analyzer interface
  - `typescript.py`, `python.py`, `go.py`, `rust.py`, `java.py`, `kotlin.py`

## Expected Results

```
37 passed
```
