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
{계층 해소된 Conventions — 아키텍처 규칙 위주}

## DEVELOPERS.md Content (strict only)
Constraints:
- {constraint 1}
- {constraint 2}
Technical Context:
{technical context content}

## Deterministic Results
{Phase 2 CLI 검증에서 발견된 이슈 요약}

## Changed Requirements (diff-spec-range 결과)
all_requirements: {true|false}
source_changed: {true|false}  ← target_dir 범위로 필터링된 값
추가/변경: {changed_requirements list — action + text}
변경된 소스 파일 (target_dir 내): {target_source_files list}

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

| Type | Severity | 판정 기준 |
|------|----------|---------|
| REQUIREMENTS_NOT_IMPLEMENTED | ERROR | Test Coverage Map에서 해당 Requirement를 커버하는 테스트 케이스 없음; 또는 `source_changed=false` AND Requirements 추가됨 |
| REQUIREMENTS_PARTIALLY_IMPLEMENTED | WARNING | 일부 테스트는 있으나 changed_requirements 중 커버 안 된 항목 존재 |
| REQUIREMENTS_VIOLATED | ERROR | 테스트가 Requirements와 명시적으로 모순되는 동작을 검증함 |

**판정 우선순위:**
1. `test_files_found=0` → TEST_MISSING (WARNING) 먼저 보고
2. 테스트 있으나 Changed Requirements 미커버 → REQUIREMENTS_NOT_IMPLEMENTED (ERROR)
3. `source_changed=false` AND Requirements 추가됨 → REQUIREMENTS_NOT_IMPLEMENTED (ERROR)

### Test Coverage Drift

| Type | Severity | 판정 기준 |
|------|----------|---------|
| TEST_MISSING | WARNING | `test_files_found=0` — 소스 파일에 테스트 파일 없음 |
| TEST_NOT_CALLING_IMPL | WARNING | 테스트 케이스의 `calls` 목록이 비어있음 |

### Convention CODE_VIOLATION

| Type | Severity | 설명 |
|------|----------|------|
| CONVENTION_DEPENDENCY_VIOLATION | ERROR | 의존성 방향 위반 |
| CONVENTION_STRUCTURE_VIOLATION | WARNING | 디렉토리 구조 규칙 위반 |

### DEVELOPERS.md Content Drift (strict only)

| Type | Severity | 설명 |
|------|----------|------|
| CONSTRAINT_NOT_ENFORCED | WARNING | Constraint가 코드에 미반영 |
| TECH_CONTEXT_STALE | INFO | 명시된 기술이 실제와 불일치 |
| DATA_SCHEMA_STALE | WARNING | Data Schemas에 정의된 타입이 코드와 불일치 |
| FLOWS_MISPLACED | WARNING | Flows 섹션이 project root가 아닌 DEVELOPERS.md에 존재 |

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
- Calls: [] (구현 함수 호출 없음)

### [WARNING] CONVENTION_STRUCTURE_VIOLATION
- Rule: "{convention rule}"
- Evidence: {file}:{line} — {violation description}

### [INFO] TECH_CONTEXT_STALE
- Context: "{stated technology}"
- Evidence: {file} uses {actual technology} instead
```

## Evidence Requirements

모든 판정은 세션 파일의 **Test Coverage Map** 항목을 인용해야 함.

**테스트 있는 경우:**
```
Source: "{requirement text}" in CLAUDE.md ## Requirements
Test: "{test_case_name}" at {file:line}
Calls: [{function_name}]
```

**테스트 없는 경우 (TEST_MISSING):**
```
Source: "{requirement text}" in CLAUDE.md ## Requirements
Coverage Map: test_files_found=0 for {source_file}
```

**테스트는 있으나 calls 비어있는 경우 (TEST_NOT_CALLING_IMPL):**
```
Source: "{requirement text}" in CLAUDE.md ## Requirements
Test: "{test_case_name}" at {file:line}
Calls: [] (비어있음 — 구현 함수 호출 확인 불가)
```

> **Requirements Drift 한정**: Test Coverage Map에 없는 파일에 대해 Requirements 구현 여부를 판정하지 않는다.
> Map에 없는 파일 = "검증 범위 외". Requirements Drift 판정 목적의 독자 코드 탐색 금지.
> Convention Drift (`CONVENTION_*`, `CONSTRAINT_*`) 판정은 Grep/Read 허용.

Every finding MUST include:
1. **Source**: Which document section defines the expectation
2. **Evidence**: Test Coverage Map 항목 인용 (위 형식 중 하나)
3. **Severity**: ERROR / WARNING / INFO

Findings without Test Coverage Map citation are invalid.
