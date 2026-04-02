# claude-md-plugin (v10)

> CLAUDE.md = Primary SSOT (PM Requirements), Source Code = Derived Artifact

## Overview

**CLAUDE.md is the Primary Source of Truth.** It defines PM-level business requirements. DEVELOPERS.md provides developer-level constraints and technical context. Source code is generated from these specifications.

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v10                       │
│                                                              │
│   CLAUDE.md (Primary SSOT — PM Requirements)                 │
│         │                                                    │
│         ├──── /spec ──→     Requirements → CLAUDE.md        │
│         ├──── /dev ──→      CLAUDE.md → Code Generation     │
│         ├──── /validate ──→ Doc-Code Consistency Check      │
│         └──── /decompile ──→ Source Code → CLAUDE.md        │
│                                                              │
│   DEVELOPERS.md (Derived Spec — Developer Constraints)       │
│         └──── Constraints = test generation source           │
│                                                              │
│   Source Code (Derived Artifact)                             │
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
| Define requirements | `/spec "requirements"` | CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) |
| Generate code from spec | `/dev` | Source code + tests (from DEVELOPERS.md Constraints) |
| Check doc-code consistency | `/validate` | Drift report + interactive auto-fix |
| Document existing code | `/decompile` | CLAUDE.md + DEVELOPERS.md extraction |

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

## Core Skills (v10)

### `/spec` — Requirements → CLAUDE.md + DEVELOPERS.md

Analyzes requirements and generates CLAUDE.md (Purpose, Requirements, Domain Context) + DEVELOPERS.md (Constraints, Technical Context).

```bash
/spec "JWT token validation authentication module"
```

### `/dev` — CLAUDE.md → Source Code

Generates source code via Inline TDD (tests from DEVELOPERS.md Constraints, then implements).

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

## Commands

| Command | Description |
|---------|-------------|
| `/project-setup` | Initialize/update Instructions + Conventions in project CLAUDE.md |
| `/migrate` | Migrate to new schema version (v6→v7, v9→v10 supported) |

## Phase 2 Skills (planned)

| Skill | Role |
|-------|------|
| `/bugfix` | Runtime bug → 3-layer trace → fix |
| `/spec-review` | CLAUDE.md quality review |
| `/impact` | Change impact analysis |
| `/diff-spec` | Semantic diff between spec versions |
| `/status` | Project health dashboard |
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

v10's core interface: Skills extract context into session files, Agents consume them.

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
| `decompose` | (none) | Large spec → module decomposition plan (scope determination + path + req distribution) |
| `impl` | brainstorming (single mode only) | Requirements analysis + CLAUDE.md/DEVELOPERS.md generation |
| `test-writer` | (none) | RED — spec → tests + Constraint↔Test mapping |
| `test-reviewer` | (none) | Spec-test traceability verification |
| `green-coder` | (none) | GREEN — minimal implementation to pass approved tests |
| `refactorer` | (none) | REFACTOR — conventions application + regression tests |
| `validator` | verification-before-completion | Semantic drift detection (Requirements, Convention, DEVELOPERS.md) |
| `decompiler` | (none) | Source code → CLAUDE.md/DEVELOPERS.md extraction |

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
claude-md-core parse-tree --root .                        # Directory tree analysis
claude-md-core resolve-boundary --path src/auth           # Boundary resolution
claude-md-core analyze-code --path src/auth               # Code analysis (6 languages)
claude-md-core parse-claude-md --file CLAUDE.md           # CLAUDE.md → JSON
claude-md-core validate-schema --file CLAUDE.md           # Schema validation
claude-md-core validate-convention --project-root .       # Convention validation
claude-md-core fix-schema --file CLAUDE.md                # Auto-add missing sections
claude-md-core contract-hash --file CLAUDE.md             # SHA-256 hash for change detection
claude-md-core format-exports --input analysis.json       # Exports markdown generation
claude-md-core format-analysis --input analysis.json      # Analysis summary generation
```

## License

MIT
