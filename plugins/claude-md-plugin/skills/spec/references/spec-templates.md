# Spec Templates

## CLAUDE.md Schema (v4)

| Section | Existence Rule | None Allowed | Description |
|---------|---------------|--------------|-------------|
| `## Purpose` | Always required | X | Reason for the module's existence (business value) |
| `## Requirements` | Always required | O | Business requirements (user perspective, verifiable statements) |
| `## Domain Context` | Always required | O | Business constraint background |
| `## Conventions` | Required at project/module root | X | 6 required subsections |
| `## Instructions` | Project root only | X | AI behavior directives |

## DEVELOPERS.md Schema (v4.2)

| Section | Required | None Allowed | Content |
|---------|----------|--------------|---------|
| `## Constraints` | O | O | Precise input/output contracts — convertible to tests |
| `## Data Schemas` | X | O | Module public type definitions — types referenced by Constraints |
| `## Technical Context` | O | O | Technology choices and rationale |
| `## Decision Log` | X | O | ADR style |
| `## Flows` | X (project root only) | O | System-level use case execution flows |
| `## Agent Observations` | X | O | Agent-managed experiential knowledge (not auto-added by converge) |

## Session File Fields

| Field | Source | Description |
|-------|--------|-------------|
| `document_language` | Project root `## Instructions` → `Document language` | Language for generated CLAUDE.md/DEVELOPERS.md content. Empty if not configured (agent will ask user in single mode, default to English in parallel mode). |

## Scope Assessment Criteria

| Dimension | Present | Inferable | Absent |
|-----------|---------|-----------|--------|
| D1 (Purpose) | Explicit purpose statement | Inferable from keywords | Purpose unclear |
| D2 (Interface) | Literal signatures | Inferable from verbs/nouns | Interface not mentioned |
| D3 (Constraints) | Numeric values/rules stated | Inferable from domain | Constraints not mentioned |

Completeness = (D1 + D2 + D3):
- **high**: all 3 "Present"
- **medium**: 1-2 "Present" or "Inferable"
- **low**: mostly "Absent"

## Tiered Clarification

| Tier | Target | Example Questions |
|------|--------|-------------------|
| Tier 1 | Core responsibility, location, scope | "What is this module's core responsibility?" |
| Tier 2 | Interface signatures, errors | "Which functions are exported?" |
| Tier 3 | Domain context, business rules | "Why is this constraint needed?" |

## Example: Generated CLAUDE.md

```markdown
# auth

## Purpose

Provides JWT token-based authentication to verify user identity for API requests.

## Requirements

- Requests containing valid JWT tokens pass through with decoded user information
- Expired tokens return a 401 Unauthorized error
- Tokens with invalid signatures are rejected

## Domain Context

- RS256 algorithm used (organization security policy)
- Token expiration time is maximum 24 hours per operations team requirement
```

## Example: Generated DEVELOPERS.md

```markdown
# auth

## Constraints

- `validateToken(token: string)` → `{ userId: string, role: string }` or throws `AuthError`
- Token expiration check: `exp` claim < current time (UTC)
- Signature verification: RS256, public key loaded from environment variable `JWT_PUBLIC_KEY`

## Technical Context

- Uses jsonwebtoken@9.0.0 library (synchronous API provides better middleware compatibility compared to jose)
- Applies Express middleware pattern

## Decision Log

None
```
