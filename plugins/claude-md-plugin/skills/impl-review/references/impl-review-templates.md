# Impl Review Templates

## Review Session File Template

```markdown
# Impl Review Session
type: impl-review | target: src/auth | project_root: /path/to/project
dir_safe: src-auth

## CLAUDE.md Content
## Purpose
Handles user authentication...

## Requirements
- REQ-1: Auto-refresh on access with expired token
- REQ-2: Maximum 5 concurrent login devices

## Domain Context
Token expiration period limited per PCI-DSS compliance

## DEVELOPERS.md Content
## Constraints
- CONST-1: Given expired access token, when API request, then auto-refresh
- CONST-2: Given 6th device login, when session count > 5, then terminate oldest

## Technical Context
JWT with RS256, refresh token rotation

## Deterministic Results
### Schema Validation
pass

### Convention Validation
N/A

### Language Validation
pass
```

## Review Criteria

| Criterion | What to Check | Severity |
|-----------|---------------|----------|
| Purpose clarity | 1-2 sentences, business value stated | WARNING |
| Requirements measurability | No vague terms ("appropriately", "quickly"), REQ-N format | ERROR |
| REQ → CONST coverage | Every REQ-N has at least one corresponding CONST-N | WARNING |
| Constraints precision | Input/output/error types all specified | ERROR |
| Domain Context sufficiency | Understandable by non-domain expert | INFO |

## Verdict Rules

- `pass`: No ERROR-level findings
- `needs_improvement`: Any ERROR-level finding present

WARNING and INFO findings do not affect verdict but are reported.
