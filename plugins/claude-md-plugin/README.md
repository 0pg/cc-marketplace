# claude-md-plugin (v18)

> CLAUDE.md = Primary SSOT (PM Requirements), Source Code = Derived Artifact

## Overview

**CLAUDE.md is the Primary Source of Truth.** It defines PM-level business requirements. DEVELOPERS.md provides developer-level constraints and technical context. Source code is generated from these specifications.

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v18                       │
│                                                              │
│   CLAUDE.md (Primary SSOT — PM Requirements)                 │
│         │                                                    │
│         ├──── /spec ──→     Requirements → CLAUDE.md        │
│         ├──── /dev ──→      CLAUDE.md → Code Generation     │
│         ├──── /validate ──→ Doc-Code Consistency Check      │
│         ├──── /decompile ──→ Source Code → CLAUDE.md        │
│         ├──── /bugfix ──→   3-layer root cause + fix        │
│         ├──── /impact ──→   Change impact (2-hop graph)     │
│         └──── /inspect ──→  health / quality / feasibility  │
│                                                              │
│   DEVELOPERS.md (System Spec — on-demand)                    │
│         └──── Constraints = test generation source           │
│                                                              │
│   Source Code (Derived Artifact)                             │
└──────────────────────────────────────────────────────────────┘
```

| Concept | Role | Description |
|---------|------|-------------|
| **Business Spec** | CLAUDE.md | What + why (Purpose, Requirements, Domain Context) — auto-loaded |
| **System Spec** | DEVELOPERS.md | How precisely (Constraints, Technical Context) — on-demand |
| **Derived Artifact** | Source Code | Generated from CLAUDE.md + DEVELOPERS.md |

**When specs and code disagree**: Regenerate the code (specs are SSOT).

## Prerequisites

### Rust Toolchain (Required)

The plugin includes a Rust CLI binary in `core/`:

```bash
cd plugins/claude-md-plugin/core && cargo build --release
```

### Superpowers Plugin (Required)

This plugin composes [superpowers](../superpowers) components for process discipline:

| Agent | Superpowers Component |
|-------|----------------------|
| impl | `superpowers:brainstorming` |
| validator | `superpowers:verification-before-completion` |
| tdd-coder | `superpowers:test-driven-development` |

Install the superpowers plugin before using `/spec` or `/dev`.

## Quick Start

| Situation | Command | Result |
|-----------|---------|--------|
| Define requirements | `/spec "requirements"` | CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) |
| Generate code from spec | `/dev` | Source code + tests (from DEVELOPERS.md Constraints) |
| Check doc-code consistency | `/validate` | Drift report + interactive auto-fix |
| Document existing code | `/decompile` | CLAUDE.md + DEVELOPERS.md extraction |
| Trace and fix a bug | `/bugfix "description"` | 3-layer root cause trace + fix at highest affected layer |
| Preview change blast radius | `/impact --path src/auth` | Downstream consumers (2-hop Grep) |
| Inspect project / spec / feasibility | `/inspect --focus {health\|quality\|feasibility}` | Read-only dashboard (default `health`) |

## 2-Document System

```
module/
├── CLAUDE.md              ← PM/PO-authored / Auto-loaded / 200-600 tok
│   Business Spec. What the module does, why it exists, and what rules to follow.
│   Claude Code loads hierarchically.
│
└── DEVELOPERS.md          ← PM/PO-authored / On-demand
    System Spec. Constraints (test source) + Technical Context.
```

### CLAUDE.md Schema (v4.0)

| Section | Rule | None Allowed | Description |
|---------|------|-------------|-------------|
| `## Purpose` | Always required | No | Module responsibility in 1-2 sentences |
| `## Requirements` | Always required | Yes | Business requirements (PM-level, verifiable) |
| `## Domain Context` | Always required | Yes | Business constraint background (regulations, legacy) |
| `## Conventions` | project/module root | No | Unified coding rules (6 subsections) |
| `## Instructions` | project root only | No | AI behavior directives |

### DEVELOPERS.md Schema (v5.1)

| Section | Required | None Allowed | Description |
|---------|----------|-------------|-------------|
| `## Constraints` | Yes | Yes | Precise I/O contracts — test-convertible |
| `## Data Schemas` | No | Yes | Public type definitions referenced by other modules |
| `## Technical Context` | Yes | Yes | Technology choices + rationale |
| `## Decision Log` | No | Yes | ADR-style: Context/Decision/Rationale |
| `## Flows` | No (project root only) | Yes | System-level use case execution flows |
| `## Roadmap` | No | Yes | PM/PO forward planning — Short/Long/Deferred |
| `## Agent Observations` | No | Yes | Agent-managed experiential knowledge (INV-8 write scope) |

### Conventions (6 Required Subsections)

`## Conventions` is placed in project/module root CLAUDE.md:

- `### Project Structure` — Directory structure rules
- `### Module Boundaries` — Module responsibility, dependency direction
- `### Naming Conventions` — Module/directory/package naming
- `### Language & Runtime` — Primary language, versions, runtime
- `### Coding Rules` — Coding rules not enforceable by linters
- `### Naming Rules` — Variable/function/class/constant naming

## Core Skills (v18)

### `/spec` — Requirements → CLAUDE.md + DEVELOPERS.md

Analyzes requirements and generates CLAUDE.md (Purpose, Requirements, Domain Context) + DEVELOPERS.md (Constraints, Technical Context) in a single pass (extract → draft → self-critique → snapshot judgment → generate). An `impl-reviewer` gate may reject once, triggering one retry with `## Reviewer Feedback` injected. On update, unaffected sections — including Agent Observations (INV-8) — are copied verbatim (diff-aware preservation).

```bash
/spec "JWT token validation authentication module"
/spec --resync --path src/auth         # Regenerate only DEVELOPERS.md Constraints after manual CLAUDE.md edits
```

### `/dev` — CLAUDE.md → Source Code

Generates source code via per-Constraint Red-Green-Refactor TDD cycles (from DEVELOPERS.md Constraints).

```bash
/dev                          # Changed CLAUDE.md/DEVELOPERS.md files
/dev --all                    # All CLAUDE.md files
/dev --path src/auth          # Specific path
/dev --conflict overwrite     # Overwrite existing files
/dev --validate               # Post-compile validation
/dev --dry-run                # Show targets only
```

### `/validate` — Document-Code Consistency + Auto-fix

Two-phase validation: deterministic CLI checks, then semantic drift detection with interactive resolution.

**Phase 1 — Deterministic (CLI):**
- Schema validation + auto-fix
- Convention structure validation
- Boundary validation (tree dependency)
- DEVELOPERS.md existence (INV-3)

**Phase 2 — Semantic (validator agent):**
- Requirements drift detection
- Convention CODE_VIOLATION detection
- DEVELOPERS.md content drift (`--strict` only)

**Phase 3 — Auto-fix (Interactive):**
- Drift-type-specific resolution with user approval
- SSOT principle: code fixes for code drift, doc updates for doc drift

```bash
/validate
/validate src/auth
/validate --strict              # DEVELOPERS.md content drift included
/validate --report-only         # No auto-fix
```

### `/decompile` — Source Code → CLAUDE.md

Extracts CLAUDE.md + DEVELOPERS.md from existing source code (leaf-first order).

```bash
/decompile
/decompile src/auth
```

### `/bugfix` — Bug → 3-Layer Root Cause Tracing → Fix

Traces a reported bug through all three layers (CLAUDE.md Requirements → DEVELOPERS.md Constraints → source code) and fixes at the highest affected layer.

```bash
/bugfix "login returns 500 instead of 401 for invalid credentials"
/bugfix "description" --path src/auth
/bugfix "description" --file src/auth/login.ts
/bugfix "description" --error "TypeError: cannot read property..."
```

### `/impact` — Change Impact Analysis

Traverses the module dependency graph (Grep-based, 2-hop) to surface downstream effects of a change.

```bash
/impact --path src/auth
```

### `/inspect` — Unified Read-only Inspection

Single entry point covering project health, spec quality review, and feasibility consultation. Replaces former `/status`, `/impl-review`, `/consult`. Dispatches to `references/inspect/{health,quality,feasibility}.md` so each invocation only loads the focus it needs.

```bash
/inspect                                      # default --focus health (lightweight dashboard)
/inspect --focus quality --path src/auth      # 5-criteria semantic review
/inspect --focus feasibility --path src/auth "Can we add OAuth2?"
/inspect --focus all                          # opt-in: health + quality together
```

## Commands

| Command | Description |
|---------|-------------|
| `/project-setup` | Initialize/update Instructions + Conventions in project CLAUDE.md |
| `/migrate` | Migrate to new schema version (v6→v7, v9→v10 supported) |
| `/autodev` | Autonomously run requirements → CLAUDE.md → code → validation loop without manual steps |

## Planned Skills

| Skill | Role |
|-------|------|
| `/diff-spec` | Semantic diff between spec versions |
| `/refactor` | Module split/merge |

## Workflow Examples

### A. New Module Development

```
/spec "requirements" → /dev → /validate
```

### B. Legacy Code Documentation

```
/decompile → /project-setup → /validate
```

### C. Incremental Changes

```
Edit CLAUDE.md → /dev → /validate
```

## Architecture

### Session File Pattern

v18's core interface: Skills extract context into session files, Agents consume them.

```
SKILL (Entry Point)
  │
  ├── CLI calls (deterministic validation/analysis)
  ├── Read CLAUDE.md + DEVELOPERS.md
  ├── Write session file (${TMP_DIR}{type}-session-{dir-safe}.md)
  │
  └── Task(Agent)
        ├── Load Skill("superpowers:{component}")
        ├── Read session file (pre-extracted specs)
        ├── Execute business logic
        └── Save result file + return result block
```

### Agents

| Agent | Superpowers Composition | Role |
|-------|------------------------|------|
| `impl` | brainstorming | Single-pass Requirements analysis + CLAUDE.md/DEVELOPERS.md generation (diff-aware on update) |
| `tdd-coder` | test-driven-development | Per-Constraint R-G-R cycle: test + impl + mapping generation |
| `test-reviewer` | (none) | Post-TDD verification: traceability, boundary, assertion, honesty |
| `refactorer` | (none) | REFACTOR — conventions application + regression tests |
| `validator` | verification-before-completion | Semantic drift detection (Requirements, Convention, DEVELOPERS.md) |
| `decompiler` | (none) | Source code → CLAUDE.md/DEVELOPERS.md extraction |
| `impl-reviewer` | (none) | Reviews generated CLAUDE.md/DEVELOPERS.md + rationale (max 1 retry) |
| `spec-quality-reviewer` | (none) | 5-criteria spec quality review for /inspect |
| `po-consultant` | (none) | Read-only feasibility judgment (spec/history/roadmap layers) |
| `bugfixer` | systematic-debugging | 3-layer root cause analysis + Layer 3 code fix (or doc escalation) |

### Design Principles

| Component | Role | Orchestration |
|-----------|------|---------------|
| **Entry Point Skill** | User entry point | CLI calls + session file creation + Agent dispatch |
| **Agent** | Business logic | superpowers composition + session file consumption + result return |
| **Session File** | SKILL↔Agent interface | Pre-extracted specs, debuggable intermediate artifact |

### Tree Dependency

- **Parent → Child**: References allowed
- **Child → Parent**: Forbidden
- **Sibling ↔ Sibling**: Forbidden

Each CLAUDE.md must be self-contained within its boundary.

### Invariants

- **INV-1**: `node.dependencies ⊆ node.children`
- **INV-2**: `validate(node) = validate(node.claude_md, node.direct_files)`
- **INV-3**: Every CLAUDE.md has a corresponding DEVELOPERS.md (1:1)
- **INV-4**: /spec → docs, /dev → code, /validate → report + resolve, /decompile → extract
- **INV-5**: project_root CLAUDE.md MUST have Conventions; module_root MAY override

## CLI Tools

```bash
claude-md-core scan-claude-md --root .                    # Project-wide CLAUDE.md index
claude-md-core diff-compile-targets --root .              # Changed CLAUDE.md/DEVELOPERS.md detection
claude-md-core diff-node-history --path src/auth --root . # Section-level diffs from recent commits
claude-md-core detect-schema-change --before a --after b  # Data Schemas change detection
claude-md-core parse-tree --root .                        # Directory tree analysis
claude-md-core resolve-boundary --path src/auth           # Boundary resolution
claude-md-core analyze-code --path src/auth               # Code analysis (6 languages)
claude-md-core parse-claude-md --file CLAUDE.md           # CLAUDE.md → JSON
claude-md-core validate-schema --file CLAUDE.md           # Schema validation
claude-md-core validate-convention --project-root .       # Convention validation
claude-md-core validate-language --file CLAUDE.md         # Document-language validation
claude-md-core fix-schema --file CLAUDE.md                # Auto-add missing sections
claude-md-core contract-hash --file CLAUDE.md             # SHA-256 hash for change detection
claude-md-core format-exports --input analysis.json       # Exports markdown generation
claude-md-core format-analysis --input analysis.json      # Analysis summary generation
claude-md-core impact-scan --target src/auth --root .     # Downstream consumers (2-hop)
claude-md-core diff-preservation --prior P --new N --sections "A,B"  # v18.1: preservation audit
```

## License

MIT
