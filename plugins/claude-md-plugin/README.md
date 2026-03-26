# claude-md-plugin (v7)

> CLAUDE.md = Primary SSOT (PM Requirements), Source Code = Derived Artifact

## Overview

**CLAUDE.md is the Primary Source of Truth.** It defines PM-level business requirements. DEVELOPERS.md provides developer-level constraints and technical context. Source code is generated from these specifications.

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v7                        │
│                                                              │
│   CLAUDE.md (Primary SSOT — PM Requirements)                 │
│         │                                                    │
│         ├──── /impl ──→     요구사항 → CLAUDE.md 정의        │
│         ├──── /compile ──→  CLAUDE.md 기반 코드 생성         │
│         ├──── /validate ──→ 문서-코드 일치 검증              │
│         └──── /bugfix ──→   3계층 추적 → 수정               │
│                                                              │
│   DEVELOPERS.md (Derived Spec — Developer Constraints)       │
│         └──── Constraints = test generation source           │
│                                                              │
│   Source Code (Derived Artifact)                             │
│         └──── /decompile ──→ CLAUDE.md + DEVELOPERS.md 추출  │
└──────────────────────────────────────────────────────────────┘
```

| Concept | Role | Description |
|---------|------|-------------|
| **Primary SSOT** | CLAUDE.md | PM requirements (Purpose, Requirements, Domain Context) |
| **Derived Spec** | DEVELOPERS.md | Developer constraints (Constraints, Technical Context) |
| **Derived Artifact** | Source Code | Generated from CLAUDE.md + DEVELOPERS.md |

**When documents and code disagree**: Regenerate the code (CLAUDE.md is SSOT).

## Prerequisites

### Rust Toolchain (Required)

The plugin includes a Rust CLI binary in `core/`:

```bash
cd plugins/claude-md-plugin/core && cargo build --release
```

## Quick Start

| Situation | Command | Result |
|-----------|---------|--------|
| Natural language request | `/dev "request"` | Routes to appropriate skill |
| Define requirements | `/impl "requirements"` | CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) |
| Document existing code | `/decompile` | CLAUDE.md + DEVELOPERS.md |
| Generate code from spec | `/compile` | Source code + tests (from DEVELOPERS.md Constraints) |
| Check doc-code consistency | `/validate` | Drift report |
| Fix runtime bug | `/bugfix --error "error"` | 3-layer trace → code regeneration |
| Review spec quality | `/impl-review` | Quality report |

## 2-Document System

```
module/
├── CLAUDE.md              ← Human-authored / Auto-loaded / 200-600 tok
│   PM requirements document. Critical rules and context.
│   Claude Code loads hierarchically.
│
└── DEVELOPERS.md          ← Human-authored / On-demand
    Derived spec. Constraints (test source) + Technical Context.
```

### CLAUDE.md Schema (v4.0)

| Section | Rule | None Allowed | Description |
|---------|------|-------------|-------------|
| `## Purpose` | Always required | No | Module responsibility in 1-2 sentences |
| `## Requirements` | Always required | Yes | Business requirements (PM-level, verifiable) |
| `## Domain Context` | Always required | Yes | Business constraint background (regulations, legacy) |
| `## Conventions` | project/module root | No | Unified coding rules (6 subsections) |
| `## Instructions` | project root only | No | AI behavior directives |

### DEVELOPERS.md Schema

| Section | Required | None Allowed | Description |
|---------|----------|-------------|-------------|
| `## Constraints` | Yes | Yes | Precise I/O contracts — test-convertible |
| `## Technical Context` | Yes | Yes | Technology choices + rationale |
| `## Decision Log` | No | Yes | ADR-style: Context/Decision/Rationale |
| `## Operations` | No | Yes | Deployment, monitoring, troubleshooting |

### Conventions (6 Required Subsections)

`## Conventions` is placed in project/module root CLAUDE.md:

- `### Project Structure` — Directory structure rules
- `### Module Boundaries` — Module responsibility, dependency direction
- `### Naming Conventions` — Module/directory/package naming
- `### Language & Runtime` — Primary language, versions, runtime
- `### Coding Rules` — Coding rules not enforceable by linters
- `### Naming Rules` — Variable/function/class/constant naming

## Commands

### `/dev` — Natural Language Routing

Routes natural language requests to the appropriate skill:

| Category | Keywords | Target |
|----------|----------|--------|
| FEATURE | add, create, new, 추가, 생성 | `/impl` |
| BUGFIX | fix, bug, error, 버그, 에러 | `/bugfix` |
| COMPILE | compile, generate, build | `/compile` |
| DECOMPILE | decompile, extract, document existing | `/decompile` |
| VALIDATE | validate, check, verify, drift | `/validate` |
| RESOLVE | resolve, fix-drift, handle violation | `/resolve` |
| IMPACT | impact, affected, breaking, depends | `/impact` |
| DIFF | diff, compare, what changed | `/diff-spec` |
| STATUS | status, health, dashboard | `/status` |
| REFACTOR | refactor, split, merge, restructure | `/refactor` |

### `/impl` — Requirements → CLAUDE.md + DEVELOPERS.md

Analyzes requirements and generates CLAUDE.md (Purpose, Requirements, Domain Context) + DEVELOPERS.md (Constraints, Technical Context).

```bash
/impl "JWT token validation authentication module"
```

### `/compile` — CLAUDE.md → Source Code

Generates source code via Inline TDD (tests from DEVELOPERS.md Constraints, then implements).

```bash
/compile                          # Changed CLAUDE.md/DEVELOPERS.md files
/compile --all                    # All CLAUDE.md files
/compile --path src/auth          # Specific path
/compile --conflict overwrite     # Overwrite existing files
```

### `/decompile` — Source Code → CLAUDE.md

Extracts CLAUDE.md + DEVELOPERS.md from existing source code (leaf-first order).

```bash
/decompile
```

### `/validate` — Document-Code Consistency

Validates drift between CLAUDE.md and actual code:
- **Requirements Drift**: Code violates documented requirements
- **Convention Drift**: Code violates coding conventions
- **DEVELOPERS.md Drift**: Missing DEVELOPERS.md, Constraints/Technical Context stale
- **Boundary Violations**: Tree dependency violations

```bash
/validate
/validate src/auth
```

### `/bugfix` — Runtime Bug → 3-Layer Trace → Fix

Traces root cause through 3 layers:
- **L1** (CLAUDE.md): Requirements gaps or mismatches
- **L2** (DEVELOPERS.md): Constraint violations, algorithm flaws
- **L3** (Source Code): Logic errors, spec divergence

```bash
/bugfix --error "TypeError: validateToken is not a function" --path src/auth
/bugfix --test "should return empty array for no results"
```

### `/impl-review` — CLAUDE.md Quality Review

Reviews CLAUDE.md quality across 3 dimensions:
- **D1**: Requirements coverage
- **D2**: CLAUDE.md quality (requirements specificity, purpose clarity)
- **D3**: Internal consistency

```bash
/impl-review src/auth
```

### `/status` — Project Health Dashboard

Shows project-wide health overview (schema pass rate, drift count, DEVELOPERS.md coverage).

### `/resolve` — Interactive Drift Resolution

Reads `/validate` results and interactively resolves each drift issue (Fix Code, Fix Doc, Skip).

### `/impact` — Change Impact Analysis

Analyzes which modules are affected by a CLAUDE.md change (Requirements-based dependency tracing).

### `/diff-spec` — Semantic Diff

Compares document versions to show semantic changes between revisions.

### `/refactor` — Module Split/Merge

Splits or merges modules based on Requirements grouping analysis.

### Other Commands

| Command | Description |
|---------|-------------|
| `/project-setup` | Initialize Conventions section in project CLAUDE.md |
| `/convention-update` | Update Conventions section |
| `/migrate` | Migrate CLAUDE.md to new schema version (v6→v7 supported) |

## Workflow Examples

### A. New Module Development

```
/impl "requirements" → /compile → /validate
```

### B. Legacy Code Documentation

```
/decompile → /validate → (fix issues) → /validate
```

### C. Runtime Bug Fix

```
/bugfix --error "error" → (auto /compile) → /validate
```

### D. Spec Quality Review

```
/impl-review → (apply fixes) → /compile
```

## Architecture

### Agents

| Agent | Status | Role |
|-------|--------|------|
| `impl` | active | Requirements analysis → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) |
| `dep-explorer` | active | Dependency exploration (requirement/module modes) |
| `decompiler` | active | Source code → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) extraction |
| `compiler` | active | DEVELOPERS.md Constraints → Inline TDD → source code (GREEN + REFACTOR) |
| `debugger` | active | 3-layer bug trace orchestrator (L1=Requirements, L2=Constraints, L3=Code) |
| `debug-layer-analyzer` | active | Single layer (L1/L2/L3) analysis (sub-agent) |
| `impl-reviewer` | active | CLAUDE.md quality review + requirements coverage |
| `validator` | active | Requirements/Convention/DEVELOPERS.md/Boundary drift detection |

### Tree Dependency

- **Parent → Child**: References allowed
- **Child → Parent**: Forbidden
- **Sibling ↔ Sibling**: Forbidden

Each CLAUDE.md must be self-contained within its boundary.

## CLI Tools

```bash
claude-md-core parse-tree --root .                    # Directory tree analysis
claude-md-core resolve-boundary --path src/auth       # Boundary resolution
claude-md-core analyze-code --path src/auth           # Code analysis
claude-md-core parse-claude-md --file CLAUDE.md       # CLAUDE.md → JSON
claude-md-core validate-schema --file CLAUDE.md       # Schema validation
claude-md-core validate-convention --project-root .   # Convention validation
claude-md-core scan-claude-md --root .                # Project-wide CLAUDE.md index
claude-md-core diff-compile-targets --root .          # Changed CLAUDE.md/DEVELOPERS.md detection
claude-md-core format-exports --input analysis.json   # Exports markdown generation
claude-md-core format-analysis --input analysis.json  # Analysis summary generation
claude-md-core fix-schema --file CLAUDE.md            # Auto-add missing sections
claude-md-core contract-hash --file CLAUDE.md         # CLAUDE.md SHA-256 hash for change detection
```

## License

MIT
