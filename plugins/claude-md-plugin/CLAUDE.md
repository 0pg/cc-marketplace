# claude-md-plugin

## Purpose

**CLAUDE.md is the Primary SSOT -- the PM's requirements document.**

A document-code synchronization plugin that defines business requirements in CLAUDE.md,
refines them to system-level specifications in DEVELOPERS.md,
and generates source code as a derived artifact.

## Core Philosophy

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v11                       │
│                                                              │
│   CLAUDE.md (Primary SSOT — PM Requirements)                 │
│         │                                                    │
│         ├──── /spec ──→     Requirements → CLAUDE.md def     │
│         ├──── /dev ──→      Code generation from CLAUDE.md   │
│         ├──── /validate ──→ Document-code consistency check  │
│         └──── /decompile ──→ Source code → CLAUDE.md extract │
│                                                              │
│   DEVELOPERS.md (Derived Spec — Developer Specification)     │
│         │                                                    │
│         └──── Constraints = Test generation source           │
│                                                              │
│   Source Code (Derived Artifact)                             │
└──────────────────────────────────────────────────────────────┘
```

| Concept | Role | Description |
|---------|------|-------------|
| **Primary SSOT** | CLAUDE.md | PM's requirements document (Purpose, Requirements, Domain Context) |
| **Derived Spec** | DEVELOPERS.md | Developer specification (Constraints, Technical Context, Decision Log) |
| **Derived Artifact** | Source Code | Code derived from CLAUDE.md |

**On mismatch**: Regenerate the code (CLAUDE.md is the SSOT).

## 2-Document System

```
module/
├── CLAUDE.md              ← Human-authored / Auto-loaded / 200-600 tok
│   PM's requirements document. Rules and context needed immediately when modifying code.
│   Claude Code loads hierarchically and automatically.
│
└── DEVELOPERS.md          ← Human-authored / On-demand / Optional
    Derived Spec. Refines Requirements to system level.
    Source for /dev to generate tests.
```

### CLAUDE.md Schema (v4.0)

| Section | Presence Rule | None Allowed | Description |
|---------|--------------|--------------|-------------|
| `## Purpose` | Always required | X | Reason for the module's existence (business value) |
| `## Requirements` | Always required | O | Business requirements (user perspective, verifiable statements) |
| `## Domain Context` | Always required | O | Business constraint background (regulations, legacy, organizational reasons) |
| `## Conventions` | Required at project/module root | X | **Present (override)** — Write only what differs from parent |
| `## Instructions` | **project root only** | X | AI behavior directives (globally applied from project root) |

### DEVELOPERS.md Schema (v5.0)

| Section | Required | None Allowed | Content |
|---------|----------|--------------|---------|
| `## Constraints` | O | O | Precise input/output contracts — convertible to tests |
| `## Data Schemas` | X | O | Public type definitions referenced by other modules |
| `## Technical Context` | O | O | Module-level implementation context (libraries, patterns, mechanisms) |
| `## Decision Log` | X | O | ADR style: context/decision/rationale |
| `## Flows` | X (project root only) | O | System-level use case execution flows |
| `## Agent Observations` | X | O | Agent-managed experiential knowledge (agent-writable only, not auto-added by converge) |

#### Agent Observations Entry Format

Each entry is an H3 with a type tag:

```markdown
### [type] title
- anchor: REQ-N or CONST-N (optional)
- since: YYYY-MM-DD (required)
- refs: N (required)
- source: /workflow agent-name (required)
- Free-form content describing the observation.
```

| Type | Description | Survival Rule | Promotion Target |
|------|-------------|---------------|-----------------|
| `structural` | Architecture patterns, known risks | Anchor deletion | Technical Context |
| `decision` | Technical choices with rationale | Anchor deletion | Decision Log |
| `tactical` | Short-lived workarounds | refs=0 + age>30d → auto-remove | (none) |
| `preference` | User-expressed coding preferences | User revocation | Constraints/Conventions |

### Conventions Section

`## Conventions` is placed in the project/module root CLAUDE.md.

6 required subsections:
- `### Project Structure` — Directory structure rules, layering patterns
- `### Module Boundaries` — Module responsibility rules, dependency direction
- `### Naming Conventions` — Module/directory/package naming
- `### Language & Runtime` — Primary language, version, runtime
- `### Coding Rules` — Basic coding rules not verifiable by linters
- `### Naming Rules` — Variable/function/class/constant naming

**DRY Principle**: Since Claude Code loads CLAUDE.md hierarchically, project_root Conventions are
automatically referenced by child modules. In module_root, write only what differs from project_root.

### Tree Structure Dependencies
- **Parent → Child**: Reference allowed
- **Child → Parent**: Reference not allowed
- **Sibling ↔ Sibling**: Reference not allowed

## Architecture

### Session File Pattern

The core interface of v11: SKILLs extract information from documents to create session files, and Agents consume session files.

```
SKILL (Entry Point)
  │
  ├── CLI invocation (deterministic validation/analysis)
  ├── Read CLAUDE.md + DEVELOPERS.md
  ├── Write session file (${TMP_DIR}{type}-session-{dir-safe}.md)
  │
  └── Task(Agent)
        │
        ├── Load Skill("superpowers:{component}")
        ├── Read session file (pre-extracted spec)
        ├── Execute business logic
        └── Save result file + return result block
```

### Active Workflows (Core 4)

#### /spec (Requirements → CLAUDE.md)

```
User: /spec "requirements"
        │
        ▼
┌─────────────────────────────────────────────┐
│ spec SKILL                                  │
│                                             │
│ 1. Bash(scan-claude-md) → Build index       │
│ 2. Self Socratic Loop:                      │
│    Task(requirement-explorer) →             │
│    Task(requirement-reviewer) →             │
│    approved | last-resort AskUserQuestion   │
│ 3. Create decompose session file            │
│ 4. Task(decompose agent) → Decompose plan   │
│ 5. Scope branching:                         │
│    single → 1 Task(impl agent)              │
│    multi  → Approve → Task(impl agent) × N  │
│             root-first, max 3 parallel       │
│ 6. Show git diff                            │
└─────────────────────────────────────────────┘
        │
        ├─ scope=single ──────────────────────┐
        │                                     ▼
        │                    ┌─────────────────────────────────────┐
        │                    │ decompose AGENT                     │
        │                    │                                     │
        │                    │ 1. Scope Classification             │
        │                    │    single → Early termination       │
        │                    │    multi  → Execute Phase 2-4       │
        │                    │ 2. Module Identification             │
        │                    │ 3. Requirement Distribution         │
        │                    │ 4. Tree Validation (INV-1)          │
        │                    │ 5. Save decompose-result.json       │
        │                    └─────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl AGENT (single mode)                    │
│ ⚡ Skill("superpowers:brainstorming")       │
│                                             │
│ 1. Extract requirements + completeness eval │
│ 2. Dependency exploration (inline, index)   │
│ 3. AskUserQuestion → Clarify (max 2 times) │
│ 4. Generate CLAUDE.md + DEVELOPERS.md       │
│ 5. validate-schema verification             │
│ 6. Plan Preview → User approval             │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ impl AGENT (parallel mode, scope=multi)     │
│ (brainstorming skipped)                     │
│                                             │
│ 1. Check target_path from session file      │
│ 2. Generate CLAUDE.md + DEVELOPERS.md       │
│ 3. validate-schema verification             │
│ AskUserQuestion prohibited — best-effort    │
└─────────────────────────────────────────────┘
```

#### /dev (CLAUDE.md → Source Code)

```
User: /dev [--all] [--conflict skip|overwrite] [--dry-run] [--validate]
        │
        ▼
┌─────────────────────────────────────────────┐
│ dev SKILL                                   │
│                                             │
│ 1. Determine targets (--all or incremental) │
│ 2. Language detection + Spec Changes analysis│
│ 3. [DELETE] tasks executed directly by SKILL │
│ 4. Test Writing Loop (per target):          │
│    Task(test-writer) → Task(test-reviewer)  │
│    → feedback loop (max 5)                  │
│ 5. TMP → target copy + Verify RED           │
│ 6. Task(green-coder) per target             │
│ 7. Task(refactorer) per target              │
│ 8. Build verify + git diff + dev commit     │
└─────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────┐  ┌──────────────────────┐
│ test-writer AGENT     │  │ test-reviewer AGENT   │
│                       │  │                       │
│ Constraints → tests   │◄►│ 5-criteria validation │
│ Requirements → accept │  │ verdict: approved     │
│ Generate mapping.json │  │         | rejected    │
└───────────────────────┘  └──────────────────────┘
        │ approved
        ▼
┌───────────────────────┐  ┌──────────────────────┐
│ green-coder AGENT     │  │ refactorer AGENT      │
│                       │  │                       │
│ Based on approved     │─►│ Apply Conventions     │
│ tests, minimal impl   │  │ Rollback on regress   │
│ (max 3)               │  │                       │
└───────────────────────┘  └──────────────────────┘
```

#### /validate (Document-Code Consistency Check)

```
User: /validate [path] [--strict] [--report-only]
        │
        ▼
┌─────────────────────────────────────────────┐
│ validate SKILL                              │
│                                             │
│ 1. Glob → Collect CLAUDE.md files           │
│ 2. Deterministic CLI validation             │
│    (schema, convention, boundary, INV-3)    │
│ 3. Create session file (doc content + CLI)  │
│ 4. Task(validator) parallel batch (max 3)   │
│ 5. Auto-fix (Interactive)                  │
│ 6. Consolidated report                      │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ validator AGENT                             │
│ ⚡ Skill("superpowers:verification")        │
│                                             │
│ 1. Requirements Drift detection             │
│ 2. Convention CODE_VIOLATION detection      │
│ 3. DEVELOPERS.md Content Drift (strict)    │
└─────────────────────────────────────────────┘
```

#### /decompile (Source Code → CLAUDE.md)

```
User: /decompile [path]
        │
        ▼
┌─────────────────────────────────────────────┐
│ decompile SKILL                             │
│                                             │
│ 1. Bash(parse-tree) → Directory structure   │
│ 2. leaf-first sorting                       │
│ 3. Create session file + Task(decompiler)   │
│    per target                               │
│ 4. git diff --stat                         │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ decompiler AGENT                            │
│ (no superpowers composition — extraction)   │
│                                             │
│ 1. resolve-boundary + analyze-code          │
│ 2. format-analysis → summary               │
│ 3. Generate CLAUDE.md + DEVELOPERS.md       │
│ 4. validate-schema verification             │
└─────────────────────────────────────────────┘
```

### Design Principles

| Component | Role | Orchestration |
|-----------|------|---------------|
| **Entry Point Skill** | User entry point | CLI invocation + session file creation + Agent dispatch |
| **Agent** | Business logic | superpowers composition + session file consumption + result return |
| **Session File** | SKILL↔Agent interface | Pre-extracted spec, debuggable intermediate artifact |

## Agents

| Agent | Superpowers Composition | Role | Observations |
|-------|------------------------|------|--------------|
| `decompose` | (none) | Large-scale spec → module decomposition plan (scope judgment + path + req distribution) | — |
| `impl` | brainstorming (single mode only) | Requirements analysis + CLAUDE.md/DEVELOPERS.md generation | read-write |
| `impl-reviewer` | (none) | Socratic review of spec plan.md (verdict: approved/rejected) | — |
| `test-writer` | (none) | RED — Spec → tests + Constraint↔Test mapping | read-write |
| `test-reviewer` | (none) | Test traceability verification against spec | read-only |
| `green-coder` | (none) | GREEN — Minimal implementation to pass approved tests | read-write |
| `refactorer` | (none) | REFACTOR — Apply Conventions + regression testing | read-write |
| `validator` | verification-before-completion | Semantic drift detection (Requirements, Convention, DEVELOPERS.md) | read-write |
| `decompiler` | (none) | Source code → CLAUDE.md/DEVELOPERS.md extraction | read-write |
| `bugfixer` | systematic-debugging | 3-layer root cause analysis + Layer 3 code fix (or doc escalation) | read-write |
| `requirement-explorer` | (none) | Domain-context exploration → requirement concretization | read-only |
| `requirement-reviewer` | (none) | 5-criteria evaluation of concretized requirements | — |

## Commands

| Command | Role |
|---------|------|
| `/project-setup` | Create/update Instructions + Conventions in CLAUDE.md (absorbed convention-update) |
| `/migrate` | Version upgrade migration (v6→v7, v9→v10, etc.) |
| `/autodev` | Thin orchestrator: Skill(spec) + Skill(dev). Autonomous end-to-end execution. |
| `/spec-step` | Resume an interrupted spec workflow from persisted state.json |

## Skills

### Core Skills (v11)

| Skill | Role |
|-------|------|
| `/spec` | Requirements → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints). Self Socratic Loop for requirement concretization before decompose. |
| `/dev` | CLAUDE.md + DEVELOPERS.md → Source code (Inline TDD from Constraints) |
| `/validate` | Document-code consistency check (Deterministic CLI + semantic drift + auto-fix) |
| `/decompile` | Source code → CLAUDE.md + DEVELOPERS.md extraction |
| `/bugfix` | Source code bug → 3-layer tracing (CLAUDE.md/DEVELOPERS.md/code) → fix at highest affected layer |

### Phase 2 (Planned after Core stabilization)

| Skill | Role |
|-------|------|
| `/impl-review` | CLAUDE.md quality review |
| `/impact` | Document change → affected module analysis |
| `/diff-spec` | Semantic diff between document versions |
| `/status` | Project health dashboard |
| `/refactor` | Module split/merge |

### CLI Subcommands (Rust Core)

| CLI | Role |
|-----|------|
| `scan-claude-md` | Build index of existing CLAUDE.md files |
| `diff-compile-targets` | Detect changed CLAUDE.md/DEVELOPERS.md files |
| `resolve-boundary` | Determine boundary |
| `analyze-code` | Code analysis (6 languages) |
| `parse-claude-md` | Parse CLAUDE.md |
| `validate-schema` | Schema validation |
| `format-exports` | Generate Exports markdown |
| `format-analysis` | Generate analysis summary markdown |
| `validate-convention` | Validate Conventions section |
| `fix-schema` | Auto-add missing sections |
| `contract-hash` | SHA-256 hash (change detection) |
| `parse-tree` | Parse directory structure |
| `validate-language` | Document language validation |
| `diff-spec-range` | Detect changed Requirements and source files since last spec commit (deprecated, use diff-node-history) |
| `diff-node-history` | Section-level diffs from recent N commits touching a node's CLAUDE.md/DEVELOPERS.md |

## Invariants

### INV-1: Tree Structure Dependencies
```
node.dependencies ⊆ node.children
```

### INV-2: Self-contained Boundary
```
validate(node) = validate(node.claude_md, node.direct_files)
```

### INV-3: CLAUDE.md ↔ DEVELOPERS.md Pairing
```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
In --strict mode, absence of DEVELOPERS.md is reported as a warning
```

### INV-4: Update Responsibility
```
/spec      → CLAUDE.md + DEVELOPERS.md (document definition)
/dev       → Source Code + DEVELOPERS.md:Agent Observations (append-only)
/bugfix    → Source Code + DEVELOPERS.md:Agent Observations (append-only)
/decompile → CLAUDE.md + DEVELOPERS.md (document extraction) + Agent Observations (append-only)
/validate  → Violation reporting + interactive resolution + Agent Observations cleanup
```

### INV-5: Conventions Section Placement Rules
```
project_root/CLAUDE.md MUST contain ## Conventions (6 required subsections)
module_root/CLAUDE.md MAY contain ## Conventions (override; inherits from project_root if absent)
```

### INV-6: Language Validation Opt-in
```
validate-language runs IFF Document language ∈ project root ## Instructions
No Document language → no validation (zero false positives for unconfigured projects)
```

### INV-7: Two-Tier Separation
```
Tier 1 (CLI): deterministic character counting, no LLM tokens
Tier 2 (LLM): only triggered when CLI result = below_threshold
```

### INV-8: Agent Observations Write Scope
```
∀ agent write to DEVELOPERS.md:
  write_target ⊆ "## Agent Observations"
  ∧ write_mode ∈ {append, update-refs, delete-stale}
```

### INV-9: Observation Anchor Dependency
```
∀ entry ∈ Agent Observations:
  entry.anchor ≠ none → entry.anchor ∈ current(Requirements ∪ Constraints)
  ∨ entry.stale = true
```

### INV-10: Observation Promotion Path
```
promote(entry) requires:
  entry.type ∈ {structural, decision, preference}
  ∧ user_approval = true
post-condition:
  entry ∉ Agent Observations ∧ promoted_content ∈ target_section
```

## Development Principles

1. **ATDD**: Write Gherkin features first, then implement
2. **Language-agnostic**: Automatic detection based on file extensions
3. **File-based results**: Agent results are saved to files, only paths are returned
4. **Simple retry**: Schema validation once, test retry 3 times
5. **Version management**: Must bump the `version` field in `.claude-plugin/plugin.json` on changes

## Superpowers Coexistence

claude-md composes superpowers domain components to create the "document-driven development" business.

### Responsibility Division

| Layer | Owner | Tools |
|-------|-------|-------|
| Spec definition, validation, tracking | claude-md | /spec, /validate, /decompile |
| Batch code regeneration | claude-md | /dev (batch) |
| Incremental code writing | superpowers | TDD (based on CLAUDE.md/DEVELOPERS.md) |
| Process discipline | superpowers | brainstorming, plans, debugging, verification |

### 3-Layer Composition Structure

| Layer | Role | Implementation |
|-------|------|----------------|
| Layer 0 | Composition setup | /project-setup → Auto-generate `## Instructions` (Claude Code auto-loads) |
| Layer 1 | Spec extraction | SKILL → Create session file → Agent dispatch |
| Layer 2 | Pure execution | Agent executes with session file + Skill(superpowers:xxx) composition |

### Agent-Level Composition

| Agent | Superpowers Component | Composition Method |
|-------|----------------------|-------------------|
| impl | brainstorming | Load brainstorming before requirements exploration/design |
| impl-reviewer | (none) | Socratic review of spec plan.md, return verdict |
| test-writer | (none) | Generate tests + mapping directly from spec |
| test-reviewer | (none) | Traceability verification, return verdict |
| green-coder | (none) | Implementation based on approved tests |
| refactorer | (none) | Apply Conventions + regression protection |
| validator | verification-before-completion | Evidence-based verification discipline |
| decompiler | (none) | Extraction work, no process discipline needed |

## Instructions

- Document language: English
- CLAUDE.md is the SSOT. Source code is a derived artifact generated from CLAUDE.md.
- When code disagrees with CLAUDE.md, regenerate code via /dev (not modify docs).
- To change requirements, update CLAUDE.md first, then code follows.
- Derive tests from DEVELOPERS.md Constraints.
- Generate source code via /dev. Do not create source files directly with the Write tool.
- Must run /validate --strict before declaring completion.

## Conventions

### Project Structure

```
claude-md-plugin/
├── core/              — Rust CLI engine (deterministic operations, no LLM)
│   ├── src/           — Production source files (one module per .rs file)
│   ├── tests/
│   │   ├── features/  — Cucumber .feature files (BDD acceptance tests)
│   │   └── cucumber.rs — Step definitions
│   └── Cargo.toml
├── skills/            — Skill definition Markdown files (plugin entry points)
├── agents/            — Agent definition Markdown files
├── commands/          — Command definition Markdown files
├── hooks/             — Hook definition files
├── scripts/           — Shell utility scripts
├── docs/              — Documentation
└── references/        — Reference materials
```

### Module Boundaries

- `core/` provides deterministic CLI subcommands; no LLM calls or network I/O
- Skills, agents, and commands are Markdown plugin definitions consumed by Claude Code; they have no Rust dependencies
- Each source module in `core/src/` is self-contained and validates only its own files (INV-2)
- No cross-module imports between sibling modules; dependencies flow through `lib.rs` re-exports

### Naming Conventions

- Rust source files: `snake_case.rs`
- Skill, agent, and command files: `kebab-case.md` (e.g., `test-writer.md`, `validate.md`)
- Cucumber feature files: `snake_case.feature`
- CLI subcommand names: `kebab-case` (e.g., `validate-schema`, `parse-tree`)
- Directories under `core/src/`: `snake_case/` for sub-modules

### Language & Runtime

- Language: Rust, edition 2021
- Toolchain: stable (no nightly features)
- Key dependencies: `clap 4.4` (CLI parsing), `serde + serde_json` (serialization), `walkdir 2.4` (filesystem traversal), `regex 1.10`, `thiserror 1.0` (error types), `sha2 0.10` (hashing)
- Test dependencies: `cucumber 0.21`, `tokio` (rt-multi-thread, for async test runner only), `tempfile 3.9`

### Coding Rules

- Use `thiserror::Error` for all custom error types; do not use ad-hoc `String` or `Box<dyn Error>` for library errors
- Use `serde::{Serialize, Deserialize}` for all data types that cross CLI boundaries (stdout JSON)
- CLI subcommands write results as JSON to stdout; errors go to stderr
- Use `OnceLock` for lazily initialized statics; avoid `Mutex<Option<T>>` for read-heavy globals
- No async in production code; async is limited to the Tokio test runtime
- Public constants belong in `lib.rs` as `pub const`; do not scatter constants across modules
- Use `Result<T, E>` for all fallible operations; no `unwrap()` or `expect()` in library code

### Naming Rules

- Functions and local variables: `snake_case`
- Structs, enums, and traits: `PascalCase`
- Constants and static variables: `SCREAMING_SNAKE_CASE`
- Enum variants: `PascalCase`
- Cucumber step functions: `snake_case` with descriptive full-sentence names matching the step text

## Domain Context
None

## Requirements
None
