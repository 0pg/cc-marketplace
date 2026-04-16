# DEVELOPERS.md Schema

## Purpose

DEVELOPERS.md is a System Spec document that maps 1:1 with CLAUDE.md.
It refines CLAUDE.md Requirements with design decisions into precise contracts and serves as the source for /dev test generation.
Both CLAUDE.md and DEVELOPERS.md are authored and managed by the PM/PO role.

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

**On mismatch between CLAUDE.md and DEVELOPERS.md**: Update DEVELOPERS.md to reflect changed Requirements (preserve unaffected Technical Context and Decision Log).

## SOT Structure

```
CLAUDE.md (Business Spec) + Design Decisions → DEVELOPERS.md (System Spec) → Source Code (Derived Artifact)
```

| Document | Role | Managed By | Consumed By |
|----------|------|------------|-------------|
| CLAUDE.md | Business Spec (what + why) | PM/PO | PM/PO, DEVELOPER |
| DEVELOPERS.md | System Spec (how precisely) | PM/PO | DEVELOPER (/dev) |

## Sections (2 required + 5 optional, all allow None)

> **v5.0 change**: Operations and Public API sections removed. Environment variables moved to Constraints; operational procedures moved to Decision Log. structural observation promotion target changed from Operations to Technical Context.

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

### ## Roadmap (Optional, None Allowed, PM/PO-Managed)

PM/PO가 node의 전략적 방향을 관리하는 섹션. **아직 Constraints/Requirements로 확정되지 않은 것만 기록한다.**
이미 Constraints나 Requirements에 존재하는 항목을 중복 등록하지 않는다.

```markdown
## Roadmap

### Short-term
- 가까운 시일 내 구현 예정인 개선 항목 (아직 스펙 아님)

### Long-term
- 아직 Requirements로 확정되지 않은 전략적 의도

### Deferred
- 보류된 방향과 보류 이유 (Decision Log 참조 또는 인라인 이유 명시)
```

**작성 원칙:**
- Short-term/Long-term: 아직 스펙(Constraints/Requirements)이 아닌 것이어야 함
- Deferred: 보류 이유 없는 항목은 의미 없음 — 반드시 이유 명시
- 항목이 /spec으로 Requirements로 확정되면 PM/PO가 해당 Roadmap 항목을 즉시 제거

### ## Agent Observations (Optional, None Allowed, Agent-Managed)

Experiential knowledge recorded by agents during work. **Only agents write to this section.**
Not auto-added by converge — created when an agent first records an observation.

Each entry is an H3 with a type tag and required metadata:

```markdown
## Agent Observations

### [structural] auth-utils circular import
- anchor: REQ-2
- since: 2026-03-15
- refs: 3
- source: /dev tdd-coder
- auth → utils → auth cycle. Resolved with type-only import.

### [decision] SQLite test fixture
- anchor: CONST-3
- since: 2026-03-18
- refs: 1
- source: /dev tdd-coder
- Using SQLite in-memory instead of real DB. User approved.
```

**Entry Types:**

| Type | Description | Auto-remove Condition | Promotion Target |
|------|-------------|----------------------|-----------------|
| `structural` | Architecture patterns, known risks | Anchor deleted | Technical Context |
| `decision` | Technical choices with rationale | Anchor deleted | Decision Log |
| `tactical` | Short-lived workarounds, temp notes | refs=0 + age>30d | (removed, no promotion) |
| `preference` | User-expressed coding preferences | User revocation | Constraints/Conventions |
| `improvement` | Technical debt, perf issues, refactoring needs (DEVELOPER-written) | anchor 있음: anchor 삭제 ∨ Roadmap 흡수 → remove; anchor 없음: refs=0 + age>60d → auto-remove | Roadmap short-term |

**Required Fields:** `since`, `refs`, `source`
**Optional Fields:** `anchor` (REQ-N or CONST-N)

**Lifecycle:**
- Created by agents during /dev, /bugfix, /spec, /decompile
- Cleaned up by /validate (stale anchor removal, consolidation, promotion report)
- Promoted to formal sections (Decision Log, Technical Context, etc.) with user approval

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
| `/decompile` | Full generation | Extracts sections from source code (includes Data Schemas auto-extraction) |
| `/dev` | Test generation source + Agent Observations write | Generates test cases from Constraints; records observations |
| `/validate` | Drift verification + Agent Observations cleanup | Constraints drift detection; stale observation removal + promotion report |
| `/bugfix` | L2 diagnosis + Agent Observations write | 3-layer analysis; records structural observations |
| `/inspect --focus feasibility` | Read-only (Constraints + Roadmap + Agent Observations) | PM/PO feasibility judgment |

## Lifecycle

Follows the same create/modify/delete cycle as CLAUDE.md.

| Command | DEVELOPERS.md |
|---------|---------------|
| /spec | Created (Constraints + Technical Context required, rest optional) |
| /decompile | Full generation (4 sections) |
| /dev | Test generation source (Constraints) |
| /bugfix | L2 diagnosis reference |
| /validate | Drift verification (Constraints ↔ Source Code) |
