# DEVELOPERS.md Schema

## Purpose

DEVELOPERS.md is a Derived Spec document that maps 1:1 with CLAUDE.md.
It concretizes CLAUDE.md Requirements at the system level and serves as the source for /dev test generation.

## Core Principles

**Record only the current state; rely on git for history.**
- CLAUDE.md and DEVELOPERS.md always record only the "current state"
- Past context (change history, dates, version history) is not included in documents
- Use `git log` or `git blame` when history is needed

## Document Pairing Rules (INV-3)

```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
```

DEVELOPERS.md absence is reported as a warning (`--strict` mode).

## SOT Structure

```
CLAUDE.md (Primary SSOT) → DEVELOPERS.md (Derived Spec) → Source Code (Derived Artifact)
```

| Document | Role | Audience |
|----------|------|----------|
| CLAUDE.md | Requirements (PM's requirements) | PM, AI agents |
| DEVELOPERS.md | Constraints + Technical Context (developer specification) | Developers, /dev |

## Sections (2 required + 4 optional, all allow None)

### ## Constraints (Required, None Allowed)

Precise input/output contracts that concretize CLAUDE.md Requirements at the system level.
**Must be convertible to tests.**

```markdown
## Constraints
- Valid JWT access token → Claims{userId, exp, permissions}
- Expired access token + valid refresh token → TokenPair{accessToken, refreshToken}
- Expired access token + expired refresh token → AuthenticationError{REFRESH_EXPIRED}
- Refresh token is one-time use — invalidated immediately upon use
- Malformed token → InvalidTokenError
- Active sessions ≥ 5 + new session → SessionLimitError{currentCount}
- Access token TTL ≤ 168h (7 days)
- Refresh token TTL ≤ 720h (30 days)
```

**Constraints Writing Principles:**
- Input → output/error format (behavior)
- Exact type names, error codes
- Specific numerical values (max/min/boundary)
- No ambiguity allowed (if ambiguous, leave it in CLAUDE.md Requirements)

**Patterns:**
```
[Input/Condition] → [Result/Output]           (behavior)
[Violation condition] → [Error type]{details}  (error)
[Property] [comparison operator] [value]       (limit)
```

### ## Data Schemas (Optional, None Allowed)

Public type definitions for the module. **Centralizes types referenced by Constraints.**
Constraints focus on behavior (`f(x) → Claims`), while Data Schemas focus on type structure definitions.
`/decompile` auto-extracts from `analyze-code`'s ExportedType (interface/type/struct/enum).

```markdown
## Data Schemas

### Claims
| Field | Type | Description |
|-------|------|-------------|
| userId | string | User identifier |
| exp | number | Expiration time (Unix timestamp, UTC) |
| permissions | string[] | Permission list |

### AuthError
| Field | Type | Value |
|-------|------|-------|
| code | AUTH_ERROR_CODE | EXPIRED \| INVALID_SIGNATURE \| SESSION_LIMIT |
| message | string | Human-readable message |
```

### ## Technical Context (Required, None Allowed)

Technical choices and their rationale. Libraries, algorithms, architecture patterns, etc.

```markdown
## Technical Context
- JWT signing: RS256 (asymmetric keys per PCI-DSS requirements)
- Password: bcrypt, cost factor 12 (security team approved 2024-01)
- Legacy compatibility: UUID v1 format maintained (utils/legacy-id module)
```

### ## Decision Log (Optional, None Allowed)

ADR (Architecture Decision Record) style. Each decision as a subheading, following a fixed schema (Context/Decision/Rationale).
No date field — only currently valid decisions are recorded. Revoked decisions are deleted (history remains in git).

> **Bilingual support**: English field names are recommended, Korean aliases are allowed.
> - `Context` | `맥락`
> - `Decision` | `결정`
> - `Rationale` | `근거`

```markdown
## Decision Log

### HMAC-SHA256 Selection
- **Context**: Need a token verification method between internal services
- **Decision**: Use HMAC-SHA256
- **Rationale**: Internal services don't need RSA key management complexity. Performance is also superior

### Memory Cache
- **Context**: Need to optimize repeated token verification performance
- **Decision**: Map-based in-memory cache
- **Rationale**: Single-instance environment makes Redis overkill
```

### ## Operations (Optional, None Allowed)

4 subsections (bilingual allowed): Configuration, Gotchas, Deployment, Monitoring.
`/decompile` auto-extracts `### Configuration` from `analyze-code`'s env_vars.

```markdown
## Operations

### Configuration
| Env Variable | Type | Required/Default | Description |
|-------------|------|-----------------|-------------|
| JWT_PUBLIC_KEY | string | required | RS256 public key (PEM format) |
| TOKEN_TTL_HOURS | number | default: 168 | Access token validity period |
| MAX_SESSIONS | number | default: 5 | Maximum concurrent sessions |

### Gotchas
- Token expiration time is UTC-based

### Deployment
- 5-minute cache warmup required on deployment

### Monitoring
- Check `auth.validation.duration` metric
- Alarm when error rate > 5%
```

### ## Public API (Optional, None Allowed)

List of functions/types this module exports to the outside (parent or sibling modules).
Automatically added when `/spec` agent discovers current module function references in parent module's DEVELOPERS.md.
Explicitly documents cross-module interface contracts for use in `diff-spec-range` based validation.

```markdown
## Public API

| Symbol | Signature | Called by |
|--------|-----------|-----------|
| spawn_agent | `fn spawn_agent(tx: Sender<OrchestratorMsg>, issue: Issue) -> JoinHandle<()>` | orchestrator |
| AgentResult | `enum AgentResult { Success(Output), Failed(AgentError) }` | orchestrator |

None
```

**Writing Principles:**
- Only record symbols called/referenced externally (exclude internal functions)
- Signature uses actual language syntax (type aliases allowed)
- Called by uses module path (relative path or crate name)
- `None` allowed when there is no external exposure

### ## Flows (Optional, is_project_root only, None Allowed)

**Allowed only in project root DEVELOPERS.md.** System-level use case execution flows.
Describes cross-module call order and data types. Warning if written in non-project-root.

```markdown
## Flows

### User Login
1. `api/auth` ← POST /login { email, password }
2. `domain/auth` — validateCredentials(email, password) → Session | AuthError
3. `domain/session` — createSession(userId) → SessionToken
4. `api/auth` → Response 200 { token: SessionToken } | Response 401

### JWT Verification (per request)
1. `middleware/auth` — extractToken(headers.Authorization) → JWT | null
2. `domain/auth` — validateToken(JWT) → Claims | AuthError
3. `middleware/auth` — req.user = Claims injection | Response 401
```

**Format Rules:**
- Each step: `` `module/path` — functionName(input) → output ``
- Module path is relative to project root
- Types reference those defined in Data Schemas or Constraints

## Usage by Skill

| Skill | DEVELOPERS.md Usage | Details |
|-------|---------------------|---------|
| `/spec` | Generates Constraints + Data Schemas + Technical Context | Concretizes CLAUDE.md Requirements |
| `/decompile` | Full generation | Extracts all 6 sections from source code (includes Data Schemas, Configuration auto-extraction) |
| `/dev` | Test generation source | Generates test cases from Constraints (Data Schemas used for type reference) |
| `/validate` | Drift verification | Constraints + Data Schemas drift detection (--strict); cross-module contract verification via Public API |
| `/bugfix` | L2 diagnosis | L2 layer of 3-layer analysis (Constraints) |

## Lifecycle

Follows the same create/modify/delete cycle as CLAUDE.md.

| Command | DEVELOPERS.md |
|---------|---------------|
| /spec | Created (Constraints + Technical Context required, rest optional) |
| /decompile | Full generation (4 sections) |
| /dev | Test generation source (Constraints) |
| /bugfix | L2 diagnosis reference |
| /validate | Drift verification (Constraints ↔ Source Code) |
