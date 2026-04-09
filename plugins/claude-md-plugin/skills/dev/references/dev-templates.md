# Dev Templates

## TDD Session File Format

### mode=write

````markdown
# TDD Session
type: tdd | mode: write | target: {path} | language: {lang} | conflict: {mode}
dir_safe: {dir-safe}
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json
test_convention: ${CLAUDE_PLUGIN_ROOT}/references/shared/test-conventions/{lang}.md

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md
project_conventions: {project_root}/CLAUDE.md#Conventions
agent_observations: {path}/DEVELOPERS.md#Agent Observations

## Requirements (from CLAUDE.md)
{full Requirements section}

## Constraints (from DEVELOPERS.md)
{full Constraints section — test generation source}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas section — for type reference, not a test generation source}

## Technical Context
{full Technical Context section}

## Conventions (resolved)
{hierarchy-resolved Conventions}

## Dependencies
{dev-context or exploration results}

## Implementation Tasks (only when Spec Changes present)
- [ADD] CONST-N: {description}
- [MODIFY] CONST-N: {change details}
- [DELETE] CONST-N: {deletion target}

## Existing Test Directory (incremental mode, only when existing tests present)
existing_test_dir: {path}/{detected_test_dir}/

## Spec Changes (optional — included only when spec commits found)
breaking: {true|false}

### Transition Context
{transition context — from where to where, why}

### Added
{added Requirements/Constraints}

### Modified
{changed Requirements/Constraints}

### Removed
{deleted Requirements/Constraints}

## Verification Contract
- All Constraints → corresponding tests exist
- All Requirements → corresponding acceptance tests exist
- All tests pass
- /validate --strict {path}
````

### mode=revise

````markdown
# TDD Session
type: tdd | mode: revise | round: {N} | target: {path} | language: {lang} | conflict: {mode}
dir_safe: {dir-safe}
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json
test_convention: ${CLAUDE_PLUGIN_ROOT}/references/shared/test-conventions/{lang}.md
feedback_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{N}.md

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Requirements (from CLAUDE.md)
{full Requirements section}

## Constraints (from DEVELOPERS.md)
{full Constraints section}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas section}

## Technical Context
{Technical Context}

## Conventions (resolved)
{hierarchy-resolved Conventions}

## Dependencies
{dev-context or exploration results}

## Existing Test Directory (incremental mode, only when existing tests present)
existing_test_dir: {path}/{detected_test_dir}/
````

## Test Reviewer Session File Format

````markdown
# Test Review Session
type: test-review | round: {N} | language: {lang} | target: {path}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
spec_session_file: ${TMP_DIR}tdd-session-{dir-safe}.md
implemented_files: [{file list from tdd-result}]
test_files: [{file list from tdd-result}]
````

## Refactorer Session File Format

````markdown
# Refactorer Session
type: refactor | target: {path} | language: {lang}

## Conventions (resolved)
{hierarchy-resolved Conventions}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Files
{file list from tdd-result or latest revise result}
````

## Mapping JSON Format

```json
{
  "target_path": "src/auth",
  "test_files": ["src/auth/__tests__/auth.test.ts", "src/auth/__tests__/auth.acceptance.test.ts"],
  "constraints": [
    {
      "id": "CONST-1",
      "text": "authenticate(token: string) → User | AuthError",
      "tests": ["auth.test.ts::should return User for valid token", "auth.test.ts::should throw AuthError for expired token"]
    }
  ],
  "requirements": [
    {
      "id": "REQ-1",
      "text": "Users can authenticate with a valid token",
      "acceptance_tests": ["auth.acceptance.test.ts::Given valid token When authenticate Then return user"]
    }
  ],
  "unmapped_constraints": [],
  "unmapped_requirements": []
}
```

## Result Formats

### tdd-result

```
---tdd-result---
result_file: ${TMP_DIR}tdd-result-{dir-safe}.json
status: success | partial | failed
implemented_files: [...]
test_files: [...]
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
tests_passed: N
tests_failed: N
unmapped_constraints: N
unmapped_requirements: N
---end-tdd-result---
```

### test-reviewer-result

```
---test-reviewer-result---
result_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md
verdict: approved | rejected
round: {N}
---end-test-reviewer-result---
```

### refactor-result

```
---refactor-result---
result_file: ${TMP_DIR}refactor-result-{dir-safe}.json
status: success | rolled_back | skipped
refactored_files: [...]
tests_passed: N
tests_failed: N
---end-refactor-result---
```

## Error Handling

| Situation | Response |
|-----------|----------|
| Session file parsing failure | Agent returns failure |
| tdd-coder unmapped > 0 | Return partial status |
| test-reviewer max_rounds reached | Best-effort proceed, warn |
| GREEN 3 failures per constraint | Mark constraint as partial, continue |
| REFACTOR regression failure | Rollback, return rolled_back status |
| File write failure | Skip that file |
