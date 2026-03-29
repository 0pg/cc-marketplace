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
```

## Drift Types

### Requirements Drift

| Type | Severity | 설명 |
|------|----------|------|
| REQUIREMENTS_NOT_IMPLEMENTED | ERROR | 코드에서 구현 흔적 없음 |
| REQUIREMENTS_PARTIALLY_IMPLEMENTED | WARNING | 일부만 구현됨 |
| REQUIREMENTS_VIOLATED | ERROR | 코드가 Requirements와 모순 |

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
- Evidence: No matching code found for keywords: [...]
- Searched: {files searched}

### [WARNING] CONVENTION_STRUCTURE_VIOLATION
- Rule: "{convention rule}"
- Evidence: {file}:{line} — {violation description}

### [INFO] TECH_CONTEXT_STALE
- Context: "{stated technology}"
- Evidence: {file} uses {actual technology} instead
```

## Evidence Requirements

Every finding MUST include:
1. **Source**: Which document section defines the expectation
2. **Evidence**: Concrete code reference (file:line or "not found")
3. **Severity**: ERROR / WARNING / INFO

Findings without evidence are invalid.
