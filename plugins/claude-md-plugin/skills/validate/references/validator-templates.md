# Validator Templates

## Validate Session File Format

```markdown
# Validate Task: {path}
type: validate | target: {path} | strict: {true|false}

## CLAUDE.md Content
Purpose: {parsed purpose}
Requirements:
- {requirement 1}
- {requirement 2}
Domain Context: {parsed domain context}

## Conventions (resolved)
{hierarchy-resolved Conventions — focusing on architecture rules}

## DEVELOPERS.md Content (strict only)
Constraints:
- {constraint 1}
- {constraint 2}
Technical Context:
{technical context content}

## Deterministic Results
{summary of issues found in Phase 2 CLI verification}

## Changed Requirements (diff-spec-range result)
all_requirements: {true|false}
source_changed: {true|false}  ← value filtered to target_dir scope
Added/Changed: {changed_requirements list — action + text}
Changed source files (within target_dir): {target_source_files list}

## Test Coverage Map
[
  {
    "source_file": "{path}",
    "public_fns": ["{fn_name}", ...],
    "test_files_found": {0 or N},
    "test_cases": [
      { "name": "{test fn name}", "calls": ["{function_name}"], "line": "{file:line}" }
    ]
  }
]
```

## Drift Types

### Requirements Drift

| Type | Severity | Determination Criteria |
|------|----------|----------------------|
| REQUIREMENTS_NOT_IMPLEMENTED | ERROR | No test cases covering the Requirement in Test Coverage Map; or `source_changed=false` AND Requirements added |
| REQUIREMENTS_PARTIALLY_IMPLEMENTED | WARNING | Some tests exist but items among changed_requirements are not covered |
| REQUIREMENTS_VIOLATED | ERROR | Tests verify behavior that explicitly contradicts Requirements |

**Determination priority:**
1. `test_files_found=0` → report TEST_MISSING (WARNING) first
2. Tests exist but Changed Requirements not covered → REQUIREMENTS_NOT_IMPLEMENTED (ERROR)
3. `source_changed=false` AND Requirements added → REQUIREMENTS_NOT_IMPLEMENTED (ERROR)

### Test Coverage Drift

| Type | Severity | Determination Criteria |
|------|----------|----------------------|
| TEST_MISSING | WARNING | `test_files_found=0` — no test files for source file |
| TEST_NOT_CALLING_IMPL | WARNING | Test case's `calls` list is empty |

### Convention CODE_VIOLATION

| Type | Severity | Description |
|------|----------|-------------|
| CONVENTION_DEPENDENCY_VIOLATION | ERROR | Dependency direction violation |
| CONVENTION_STRUCTURE_VIOLATION | WARNING | Directory structure rule violation |

### DEVELOPERS.md Content Drift (strict only)

| Type | Severity | Description |
|------|----------|-------------|
| CONSTRAINT_NOT_ENFORCED | WARNING | Constraint not reflected in code |
| TECH_CONTEXT_STALE | INFO | Stated technology doesn't match actual usage |
| DATA_SCHEMA_STALE | WARNING | Types defined in Data Schemas don't match code |
| FLOWS_MISPLACED | WARNING | Flows section exists in DEVELOPERS.md that is not at project root |
| LANGUAGE_MISMATCH | Document content in unexpected language | WARNING | CLI below_threshold + agent confirms untranslated |
| LANGUAGE_ACCEPTABLE | Non-target script is legitimate | (dismissed) | CLI below_threshold + agent dismisses |

## Validation Report Format

```markdown
# Validation Report: {directory}

## Summary
- Total issues: N
- Errors: N
- Warnings: N
- Info: N

## Issues

### [ERROR] REQUIREMENTS_NOT_IMPLEMENTED
- Requirement: "{requirement text}"
- Coverage Map: test_files_found=0 for {source_file}  ← or →
- Test: "{test_case_name}" at {file:line} — does not cover this requirement

### [WARNING] TEST_MISSING
- Requirement: "{requirement text}"
- Coverage Map: test_files_found=0 for {source_file}

### [WARNING] TEST_NOT_CALLING_IMPL
- Requirement: "{requirement text}"
- Test: "{test_case_name}" at {file:line}
- Calls: [] (no implementation function calls)

### [WARNING] CONVENTION_STRUCTURE_VIOLATION
- Rule: "{convention rule}"
- Evidence: {file}:{line} — {violation description}

### [INFO] TECH_CONTEXT_STALE
- Context: "{stated technology}"
- Evidence: {file} uses {actual technology} instead
```

## Evidence Requirements

All determinations must cite **Test Coverage Map** items from the session file.

**When tests exist:**
```
Source: "{requirement text}" in CLAUDE.md ## Requirements
Test: "{test_case_name}" at {file:line}
Calls: [{function_name}]
```

**When no tests exist (TEST_MISSING):**
```
Source: "{requirement text}" in CLAUDE.md ## Requirements
Coverage Map: test_files_found=0 for {source_file}
```

**When tests exist but calls are empty (TEST_NOT_CALLING_IMPL):**
```
Source: "{requirement text}" in CLAUDE.md ## Requirements
Test: "{test_case_name}" at {file:line}
Calls: [] (empty — cannot confirm implementation function calls)
```

> **Requirements Drift only**: Do not determine Requirements implementation status for files not in the Test Coverage Map.
> Files not in the Map = "outside verification scope". Independent code exploration for Requirements Drift determination is prohibited.
> Convention Drift (`CONVENTION_*`, `CONSTRAINT_*`) determination allows Grep/Read.

Every finding MUST include:
1. **Source**: Which document section defines the expectation
2. **Evidence**: Test Coverage Map item citation (one of the formats above)
3. **Severity**: ERROR / WARNING / INFO

Findings without Test Coverage Map citation are invalid.
