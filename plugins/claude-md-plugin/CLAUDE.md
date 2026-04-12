# claude-md-plugin

## Purpose

**CLAUDE.md is the Primary SSOT for module specification.**

A document-code synchronization plugin that defines business requirements in CLAUDE.md,
refines them with design decisions into system-level specifications in DEVELOPERS.md,
and generates source code as a derived artifact.

## Roles

| Role | Definition | Workflows |
|------|------------|-----------|
| **PM/PO** | Any entity (human or AI agent) that manages and modifies spec documents (CLAUDE.md, DEVELOPERS.md) | `/spec`, `/decompile`, `/validate`, `/project-setup`, `/migrate`, `/consult` |
| **DEVELOPER** | Any entity (human or AI agent) that writes source code based on specs | `/dev`, `/bugfix` (code fix) |

- PM/PO and DEVELOPER are **functional roles**, not job titles. A single AI agent can act as PM/PO in one workflow and DEVELOPER in another.
- Both CLAUDE.md and DEVELOPERS.md are **authored and managed by the PM/PO role**. DEVELOPER reads them as input.
- The only exception is `## Agent Observations` in DEVELOPERS.md, where DEVELOPER appends experiential notes (scoped by INV-8, cleaned by PM/PO via `/validate`).

## Core Philosophy

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v11                       │
│                                                              │
│   CLAUDE.md (Business Spec — auto-loaded, 200-600 tok)       │
│         │                                                    │
│         ├──── /spec ──→     Requirements → CLAUDE.md def     │
│         ├──── /dev ──→      Code generation from CLAUDE.md   │
│         ├──── /validate ──→ Document-code consistency check  │
│         └──── /decompile ──→ Source code → CLAUDE.md extract │
│                                                              │
│   DEVELOPERS.md (System Spec — on-demand)                    │
│         │                                                    │
│         └──── Constraints = Test generation source           │
│                                                              │
│   Source Code (Derived Artifact)                             │
└──────────────────────────────────────────────────────────────┘
```

| Concept | Role | Description |
|---------|------|-------------|
| **Business Spec** | CLAUDE.md | What the module does and why (Purpose, Requirements, Domain Context) — auto-loaded |
| **System Spec** | DEVELOPERS.md | How the module is built precisely (Constraints, Technical Context, Decision Log) — on-demand |
| **Derived Artifact** | Source Code | Code generated from CLAUDE.md + DEVELOPERS.md |

**On mismatch between spec and code**: Regenerate the code (specs are the SSOT).
**On mismatch between CLAUDE.md and DEVELOPERS.md**: Update DEVELOPERS.md to reflect changed Requirements (preserve unaffected Technical Context and Decision Log).

## 2-Document System

```
module/
├── CLAUDE.md              ← PM/PO-authored / Auto-loaded / 200-600 tok
│   Business Spec. What the module does, why it exists, and what rules to follow.
│   Claude Code loads hierarchically and automatically.
│
└── DEVELOPERS.md          ← PM/PO-authored / On-demand / Optional
    System Spec. Refines Requirements with design decisions into precise contracts.
    Source for /dev to generate tests.
```

**Split rationale**: Auto-loaded (always in context) vs On-demand (loaded when developing). Both managed by PM/PO role.

### CLAUDE.md Schema (v4.0)

| Section | Presence Rule | None Allowed | Description |
|---------|--------------|--------------|-------------|
| `## Purpose` | Always required | X | Reason for the module's existence (business value) |
| `## Requirements` | Always required | O | Business requirements (user perspective, verifiable statements) |
| `## Domain Context` | Always required | O | Business constraint background (regulations, legacy, organizational reasons) |
| `## Conventions` | Required at project/module root | X | **Present (override)** — Write only what differs from parent |
| `## Instructions` | **project root only** | X | AI behavior directives (globally applied from project root) |

### DEVELOPERS.md Schema (v5.1)

| Section | Required | None Allowed | Content |
|---------|----------|--------------|---------|
| `## Constraints` | O | O | Precise input/output contracts — convertible to tests |
| `## Data Schemas` | X | O | Public type definitions referenced by other modules |
| `## Technical Context` | O | O | Module-level implementation context (libraries, patterns, mechanisms) |
| `## Decision Log` | X | O | ADR style: context/decision/rationale |
| `## Flows` | X (project root only) | O | System-level use case execution flows |
| `## Roadmap` | X | O | PM/PO forward planning — Short-term/Long-term/Deferred |
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
| `improvement` | Technical debt, performance issues, refactoring needs (DEVELOPER-written) | anchor 있음: anchor 삭제 ∨ Roadmap 흡수; anchor 없음: refs=0 + age>60d → auto-remove | Roadmap short-term |

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
│ 3. Spec execution:                          │
│    3a. Task(impl, mode=plan) → plan.md      │
│    3b. Socratic Loop:                       │
│        Task(impl-reviewer) → verdict        │
│        reject → Task(impl, mode=revise)     │
│    3c. Task(impl, mode=execute)             │
│        → CLAUDE.md + DEVELOPERS.md          │
│    3d. Auto-commit                          │
│ 4. Show git diff                            │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl AGENT                                  │
│ ⚡ Skill("superpowers:brainstorming")       │
│                                             │
│ mode=plan:                                  │
│   1. Extract requirements + completeness    │
│   2. Dependency exploration (inline, index) │
│   3. AskUserQuestion → Clarify (max 2)     │
│   4. Write plan.md (Requirements+Constraints)│
│                                             │
│ mode=revise:                                │
│   Address reviewer Critical Questions       │
│   Update plan.md                            │
│                                             │
│ mode=execute:                               │
│   Generate CLAUDE.md + DEVELOPERS.md        │
│   from approved plan.md                     │
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
│ 4. Task(tdd-coder) per target               │
│ 5. SKILL test verify (agent result check)   │
│ 6. Task(test-reviewer) post-TDD verify      │
│    → reject: Task(tdd-coder revise) (max 3) │
│ 7. Task(refactorer) per target (conditional) │
│ 8. SKILL test verify (refactorer check)     │
│ 9. Cross-module gate + Build + Commit       │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ tdd-coder AGENT                             │
│ ⚡ Skill("superpowers:tdd")                 │
│                                             │
│ Per-Constraint Red-Green-Refactor cycle:    │
│   RED: Write test → verify FAIL             │
│   GREEN: Minimal impl → verify PASS         │
│   REFACTOR: Clean up → still green          │
│ Generate mapping.json                       │
└─────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────┐  ┌──────────────────────┐
│ test-reviewer AGENT  │  │ refactorer AGENT      │
│ (post-TDD verify)    │  │                       │
│                      │  │ Apply Conventions     │
│ 5-criteria review    │  │ Rollback on regress   │
│ tests + impl code    │  │                       │
│ verdict: approved    │  │                       │
│         | rejected   │  │                       │
└──────────────────────┘  └──────────────────────┘
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

| Agent | Functional Role | Superpowers Composition | Description | Observations |
|-------|----------------|------------------------|-------------|--------------|
| `impl` | PM/PO | brainstorming | Requirements analysis + CLAUDE.md/DEVELOPERS.md generation | read-write |
| `impl-reviewer` | PM/PO | (none) | Socratic review of spec plan.md (verdict: approved/rejected) | — |
| `requirement-explorer` | PM/PO | (none) | Domain-context exploration → requirement concretization | read-only |
| `requirement-reviewer` | PM/PO | (none) | 5-criteria evaluation of concretized requirements | — |
| `validator` | PM/PO | verification-before-completion | Semantic drift detection (Requirements, Convention, DEVELOPERS.md) | read-write |
| `decompiler` | PM/PO | (none) | Source code → CLAUDE.md/DEVELOPERS.md extraction | read-write |
| `tdd-coder` | DEVELOPER | test-driven-development | Per-Constraint R-G-R cycle: test + impl + mapping generation | read-write |
| `test-reviewer` | DEVELOPER | (none) | Post-TDD verification: tests + impl traceability, boundary, assertion, honesty | read-only |
| `refactorer` | DEVELOPER | (none) | REFACTOR — Apply Conventions + regression testing | read-write |
| `bugfixer` | DEVELOPER | systematic-debugging | 3-layer root cause analysis + Layer 3 code fix (or doc escalation) | read-write |
| `spec-quality-reviewer` | PM/PO | (none) | 5-criteria spec quality review (verdict: pass/needs_improvement) | read-only |
| `po-consultant` | PM/PO | (none) — 판단 작업 | 세 지식층 동원 feasibility 판단. read-only | read-only |

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
| `/spec` | Requirements → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints). Self Socratic Loop for requirement concretization, then plan → review → execute. |
| `/dev` | CLAUDE.md + DEVELOPERS.md → Source code (Per-Constraint R-G-R via tdd-coder + post-TDD review). Displays spec change summary before TDD pipeline. |
| `/validate` | Document-code consistency check (Deterministic CLI + semantic drift + auto-fix) |
| `/decompile` | Source code → CLAUDE.md + DEVELOPERS.md extraction |
| `/bugfix` | Source code bug → 3-layer tracing (CLAUDE.md/DEVELOPERS.md/code) → fix at highest affected layer |
| `/sync` | PM/PO: DEVELOPERS.md partial update for changed Requirements (skips full /spec workflow) |
| `/impact` | PM/PO: Change impact analysis across module dependency graph (Grep-based, 2-hop) |
| `/impl-review` | PM/PO: CLAUDE.md + DEVELOPERS.md quality review (deterministic CLI + semantic 5-criteria) |
| `/status` | PM/PO: Project health dashboard (schema, pairing, drift, conventions) |
| `/consult` | 외부 추상적 요구사항 → PM/PO 판단 (feasible/partially_feasible/not_feasible + constraints + history + roadmap_fit + suggested_path). Read-only. |

### Phase 2 (Planned)

| Skill | Role |
|-------|------|
| `/diff-spec` | Semantic diff between document versions |
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
PM/PO role:
  /spec      → CLAUDE.md + DEVELOPERS.md (document definition)
  /sync      → DEVELOPERS.md Constraints update (partial, preserves other sections)
  /decompile → CLAUDE.md + DEVELOPERS.md (document extraction) + Agent Observations (append-only)
  /validate  → Violation reporting + interactive resolution + Agent Observations cleanup
               + improvement 항목 Roadmap promote 검토
  /impact    → Read-only impact analysis (no file modifications)
  /impl-review → Read-only quality review (no file modifications)
  /status    → Read-only health dashboard (no file modifications)
  /consult   → Read-only judgment (no file modifications)

DEVELOPER role:
  /dev       → Source Code + DEVELOPERS.md:Agent Observations (append-only)
  /bugfix    → Source Code + DEVELOPERS.md:Agent Observations (append-only)

Cross-role boundary: DEVELOPER writes only to Agent Observations (INV-8)
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
  entry.type ∈ {structural, decision, preference, improvement}
  ∧ user_approval = true
post-condition:
  entry ∉ Agent Observations ∧ promoted_content ∈ target_section

Promotion Target:
  structural  → Technical Context
  decision    → Decision Log
  preference  → Constraints/Conventions
  improvement → Roadmap short-term
```

### INV-11: Roadmap Authorship
```
∀ modification to DEVELOPERS.md ## Roadmap:
  modifier.role = PM/PO
  ∧ Roadmap.item ∉ {already_in_constraints, already_in_requirements}
  ∧ Deferred.item → Deferred.item.reason ≠ empty
post-condition (/spec으로 요구사항 확정 시):
  확정된 Roadmap 항목을 PM/PO가 즉시 Roadmap에서 제거
```

### INV-12: Improvement Observation Lifecycle
```
∀ entry ∈ Agent Observations where entry.type = improvement:
  entry.source.role = DEVELOPER
  ∧ entry.anchor ≠ none:
      (entry.anchor_deleted ∨ entry.promoted_to_roadmap) → entry.stale = true
  ∧ entry.anchor = none:
      (entry.refs = 0 ∧ age(entry) > 60d) → entry.stale = true
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
| Spec definition, validation, tracking | claude-md | /spec, /sync, /validate, /decompile, /impact, /impl-review, /status |
| TDD code generation | claude-md | /dev (per-Constraint R-G-R via tdd-coder) |
| TDD process discipline | superpowers | test-driven-development (composed by tdd-coder) |
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
| tdd-coder | test-driven-development | Per-Constraint R-G-R cycle with TDD iron law |
| test-reviewer | (none) | Post-TDD traceability + honesty verification, return verdict |
| refactorer | (none) | Apply Conventions + regression protection |
| validator | verification-before-completion | Evidence-based verification discipline |
| decompiler | (none) | Extraction work, no process discipline needed |
| spec-quality-reviewer | (none) | 5-criteria spec quality review, return verdict |
| po-consultant | (none) | Read-only feasibility judgment — 3-layer reasoning (spec/history/roadmap), return verdict |

## Instructions

- Document language: English
- CLAUDE.md + DEVELOPERS.md are the SSOT. Source code is a derived artifact.
- When code disagrees with specs, regenerate code via /dev (not modify docs).
- To change requirements, update CLAUDE.md first (PM/PO), then DEVELOPERS.md follows, then code follows.
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
- Skill, agent, and command files: `kebab-case.md` (e.g., `tdd-coder.md`, `validate.md`)
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
