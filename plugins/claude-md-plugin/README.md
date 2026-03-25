# claude-md-plugin (v6)

> Source Code = SSOT, CLAUDE.md = Pre-learning Index + Human Knowledge Store

## Overview

**Source Code is the single source of truth.** CLAUDE.md provides a compact pre-learning index and stores human knowledge (constraints, context, conventions) that cannot be derived from code alone.

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v6                        │
│                                                              │
│   Source Code (SSOT)                                         │
│         │                                                    │
│         ├──── /decompile ──→ CLAUDE.md + DEVELOPERS.md 추출  │
│         ├──── /validate ──→  문서-코드 일치 검증             │
│         │                                                    │
│   CLAUDE.md (pre-learning index + human knowledge)           │
│         │                                                    │
│         ├──── /impl ──→     요구사항 → CLAUDE.md 정의        │
│         ├──── /compile ──→  CLAUDE.md 기반 코드 생성         │
│         └──── /bugfix ──→   3계층 추적 → 수정               │
└──────────────────────────────────────────────────────────────┘
```

| Concept | Role | Description |
|---------|------|-------------|
| **Source of Truth** | Source Code | The actual implementation, the only truth |
| **Pre-learning Index** | CLAUDE.md | Compact index for faster code understanding |
| **Human Knowledge** | CLAUDE.md | Constraints, context, conventions not in code |
| **Deep Context** | DEVELOPERS.md | WHY — decision rationale, invariants, operations |

**When documents and code disagree**: Update the documents (code is SSOT).

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
| Define requirements | `/impl "requirements"` | CLAUDE.md + compile-context |
| Document existing code | `/decompile` | CLAUDE.md + DEVELOPERS.md |
| Generate code from spec | `/compile` | Source code + tests |
| Check doc-code consistency | `/validate` | Drift report |
| Fix runtime bug | `/bugfix --error "error"` | 3-layer trace → code regeneration |
| Review spec quality | `/impl-review` | Quality report |

## 3-Document System

```
module/
├── CLAUDE.md              ← Human-authored / Auto-loaded / 200-600 tok
│   Critical rules and context needed when modifying code.
│   Claude Code loads hierarchically.
│
├── DEVELOPERS.md          ← Human-authored / On-demand
│   Deep context (WHY). Loaded via Instructions + plugin commands.
│
└── .claude/
    └── index.md           ← Auto-generated / On-demand
        Interface/behavior/structure index extracted from code.
        Generated via /sync.
```

### CLAUDE.md Schema (v3.1)

| Section | Rule | None Allowed | Description |
|---------|------|-------------|-------------|
| `## Purpose` | Always required | No | Module responsibility in 1-2 sentences |
| `## Constraints` | Always required | Yes | Rules code must follow (self-contained) |
| `## Domain Context` | Always required | Yes | Key context in 2-3 sentences |
| `## Conventions` | project/module root | No | Unified coding rules (6 subsections) |
| `## Instructions` | project root only | No | AI behavior directives |

### DEVELOPERS.md Schema

| Section | Required | None Allowed | Description |
|---------|----------|-------------|-------------|
| `## Domain Context` | Yes | Yes | Extended module domain context |
| `## Invariants` | Yes | Yes | Business invariants + rationale |
| `## Decision Log` | Yes | Yes | ADR-style: Context/Decision/Rationale |
| `## Operations` | Yes | Yes | Deployment, monitoring, troubleshooting |
| `## File Map` | Yes | Yes | File roles and relationships |

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
| VALIDATE | validate, check, verify | `/validate` |

### `/impl` — Requirements → CLAUDE.md

Analyzes requirements and generates CLAUDE.md (Purpose, Constraints, Domain Context) + compile-context (ephemeral session spec).

```bash
/impl "JWT token validation authentication module"
```

### `/compile` — CLAUDE.md → Source Code

Generates source code from CLAUDE.md specifications via 2-agent TDD workflow (test-designer → compiler).

```bash
/compile                          # Changed CLAUDE.md files
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
- **Constraints Drift**: Code violates documented constraints
- **Domain Context Drift**: Context no longer applies
- **Convention Drift**: Code violates coding conventions
- **DEVELOPERS.md Drift**: File Map out of sync, missing DEVELOPERS.md
- **Boundary Violations**: Tree dependency violations

```bash
/validate
/validate src/auth
```

### `/bugfix` — Runtime Bug → 3-Layer Trace → Fix

Traces root cause through 3 layers:
- **L1** (CLAUDE.md): Constraint gaps or mismatches
- **L2** (DEVELOPERS.md): Invariant violations, algorithm flaws
- **L3** (Source Code): Logic errors, spec divergence

```bash
/bugfix --error "TypeError: validateToken is not a function" --path src/auth
/bugfix --test "should return empty array for no results"
```

### `/impl-review` — CLAUDE.md Quality Review

Reviews CLAUDE.md quality across 3 dimensions:
- **D1**: Requirements coverage
- **D2**: CLAUDE.md quality (constraints specificity, purpose clarity)
- **D3**: Internal consistency

```bash
/impl-review src/auth
```

### Other Commands

| Command | Description |
|---------|-------------|
| `/project-setup` | Initialize Conventions section in project CLAUDE.md |
| `/convention-update` | Update Conventions section |
| `/migrate` | Migrate CLAUDE.md to new schema version |

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
| `impl` | active | Requirements analysis → CLAUDE.md + compile-context |
| `dep-explorer` | active | Dependency exploration (requirement/module modes) |
| `decompiler` | active | Source code → CLAUDE.md + DEVELOPERS.md extraction |
| `compiler` | active | CLAUDE.md Constraints/Domain Context → source code (GREEN + REFACTOR) |
| `debugger` | active | 3-layer bug trace orchestrator |
| `debug-layer-analyzer` | active | Single layer (L1/L2/L3) analysis (sub-agent) |
| `impl-reviewer` | active | CLAUDE.md quality review + requirements coverage |
| `validator` | active | Constraints/Domain Context/Convention drift detection |

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
claude-md-core diff-compile-targets --root .          # Changed CLAUDE.md detection
claude-md-core format-exports --input analysis.json   # Exports markdown generation
claude-md-core format-analysis --input analysis.json  # Analysis summary generation
claude-md-core fix-schema --file CLAUDE.md            # Auto-add missing sections
claude-md-core compile-order --root .                 # Dependency-based compile order
```

## License

MIT
