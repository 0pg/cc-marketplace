# claude-md-plugin

## Purpose

**CLAUDE.md is the Primary SSOT for module specification.**

A document-code synchronization plugin that defines business requirements in CLAUDE.md,
refines them with design decisions into system-level specifications in DEVELOPERS.md,
and generates source code as a derived artifact.

## Roles

| Role | Definition | Workflows |
|------|------------|-----------|
| **PM/PO** | Any entity (human or AI agent) that manages and modifies spec documents (CLAUDE.md, DEVELOPERS.md) | `/spec`, `/decompile`, `/validate`, `/inspect`, `/project-setup`, `/migrate` |
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
User: /spec "requirements" [--resync] [--path P] [--no-ask]
        │
        ▼
┌─────────────────────────────────────────────┐
│ spec SKILL (single-pass)                    │
│                                             │
│ 1. Bash(scan-claude-md) → Build index       │
│ 2. Pre-consult (skipped when --resync)      │
│    Task(po-consultant, cross-node) if needed│
│ 3. Single-pass spec execution:              │
│    3a. Write spec session file              │
│        (resync flag + prior docs if update) │
│    3b. Task(impl) → CLAUDE.md+DEVELOPERS.md │
│        + rationale sidecar                  │
│    3c. Task(impl-reviewer) → verdict        │
│        rejected → inject ## Reviewer        │
│        Feedback, re-dispatch (max 1 retry)  │
│    3d. 2nd reject → halt, surface questions │
│    3e. Auto-commit on approval              │
│ 4. Show git diff                            │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl AGENT (single-pass)                    │
│ ⚡ Skill("superpowers:brainstorming")       │
│                                             │
│ 1. Extract → draft → self-critique          │
│ 2. Snapshot judgment (Remove/Keep/Merge)    │
│    across Requirements, Constraints,        │
│    Data Schemas, Flows, Roadmap             │
│ 3. Diff-aware preservation (action=update)  │
│    unaffected sections copied verbatim      │
│    Agent Observations verbatim (INV-8)      │
│ 4. Resync shortcut: trivial Phase 5,        │
│    regenerate Constraints only              │
│ 5. Generate CLAUDE.md + DEVELOPERS.md       │
│    + rationale sidecar                      │
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
| `impl` | PM/PO | brainstorming | Single-pass: Requirements analysis + CLAUDE.md/DEVELOPERS.md generation + rationale sidecar. Diff-aware on update. | read-write |
| `impl-reviewer` | PM/PO | (none) | Reviews generated CLAUDE.md/DEVELOPERS.md + rationale (verdict: approved/rejected, max 1 retry) | — |
| `requirement-explorer` | PM/PO | (none) | Domain-context exploration → requirement concretization (retained for future reuse; not called by /spec) | read-only |
| `requirement-reviewer` | PM/PO | (none) | 5-criteria evaluation of concretized requirements (retained for future reuse; not called by /spec) | — |
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

## Skills

### Core Skills (v18)

| Skill | Role |
|-------|------|
| `/spec` | Requirements → CLAUDE.md + DEVELOPERS.md. Single-pass (extract → draft → self-critique → snapshot judgment → generate) with optional reviewer gate (max 1 retry). `--resync` shortcut regenerates Constraints only. |
| `/dev` | CLAUDE.md + DEVELOPERS.md → Source code (Per-Constraint R-G-R via tdd-coder + post-TDD review). Displays spec change summary before TDD pipeline. |
| `/validate` | Document-code consistency check (Deterministic CLI + semantic drift + auto-fix) |
| `/decompile` | Source code → CLAUDE.md + DEVELOPERS.md extraction |
| `/bugfix` | Source code bug → 3-layer tracing (CLAUDE.md/DEVELOPERS.md/code) → fix at highest affected layer (judgment internalized in bugfixer agent) |
| `/impact` | PM/PO: Change impact analysis across module dependency graph (Grep-based, 2-hop) |
| `/inspect` | PM/PO: Unified read-only inspection. `--focus health \| quality \| feasibility \| all` (default `health`; `all` = health + quality is opt-in). Absorbs former `/status`, `/impl-review`, `/consult`. |

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
  /spec               → CLAUDE.md + DEVELOPERS.md (document definition; diff-aware on update)
  /spec --resync      → DEVELOPERS.md Constraints update (partial, preserves other sections verbatim)
  /decompile          → CLAUDE.md + DEVELOPERS.md (document extraction) + Agent Observations (append-only)
  /validate           → Violation reporting + interactive resolution + Agent Observations cleanup
                        + improvement 항목 Roadmap promote 검토
  /impact             → Read-only impact analysis (no file modifications)
  /inspect            → Read-only: health dashboard + quality review + feasibility judgment
                        (--focus health | quality | feasibility | all)

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

### INV-13: Node Dependency Notification
```
∀ spec change to node N that modifies ## Data Schemas (exported interface):
  N.pm_po MUST surface this change as one of:
    [decision] Agent Observation entry
    ∨ ## Decision Log entry (DEVELOPERS.md)
  dependent nodes MAY trigger /impact → /spec --resync cycle
  ∧ not_feasible rejection from N.pm_po toward a calling party:
      → escalate via AskUserQuestion before proceeding
```

### INV-14: Node Access Through PM/PO
```
∀ agent A performing cross-node context access to node N:
  A.node ≠ N ∧ A.role ≠ N.pm_po ∧ A.role ≠ N.developer
  → A MUST obtain context via Task(po-consultant, N) verdict
  ∧ A MUST NOT Read(N/CLAUDE.md) or Read(N/DEVELOPERS.md) as judgment input

Permitted exceptions:
  - project_root/CLAUDE.md: accessible by all agents (auto-loaded, public contract)
  - DEVELOPER reading own node's spec: permitted (INV-4)
  - PM/PO reading own node's spec: permitted (INV-4)
  - /validate, /decompile: system-level read-only scans exempt
  - /impact: dependency graph traversal exempt (read-only, no judgment)
  - /inspect: health dashboard + quality review scans exempt (read-only, no judgment)
  - /bugfix: multi-module bug tracing exempt (read-only cross-node, doc escalation path)
  - /spec --resync: PM/PO DEVELOPERS.md partial update (own node) — covered by PM/PO own-node exception
```

### INV-15: po-consultant Verdict Execution

Scope: this invariant governs only po-consultant verdicts consumed by `/spec` and
`/autodev --auto-sync`. Other reviewer agents already follow the v16 `progress`
convergence pattern and are not affected.

```
∀ ownership decision D delegated to po-consultant in /spec or /autodev --auto-sync:
  the consultant's self-described outcome (verdict, execution, reason, redirect_to)
    MUST be executed verbatim by the orchestrating SKILL
  ∧ the SKILL MUST NOT re-interpret, override, or synthesize a substitute decision
  ∧ when no consultant's verdict can resolve D → halt and surface state to the caller
```

Corollaries (all defaults with explicitly judged exceptions, per Harness Design Principles):

- Default: a single `auto_executable` candidate among peers → proceed.
- Default: zero `auto_executable` candidates → halt with each consultant's reason preserved.
  Exception (interactive only): AskUserQuestion with the same reasons.
- Default: multiple `auto_executable` candidates → halt; surface the ownership conflict.
  (No authority resolves cross-node ownership; caller must decide.)
- Default: `redirect_to` honored → re-consult at the new target.
  Exception: detected cycle (target revisited) → halt with the cycle chain.
- Default: consumer chain (`--auto-sync`) stops on the first non-`auto_executable` consumer.

This invariant explicitly **does not** introduce hardcoded depth caps, score-based
tiebreaks, or per-enum decision procedures. The SKILL is an executor of the
consultant's verdict, not a decision-maker.

## Development Principles

1. **ATDD**: Write Gherkin features first, then implement
2. **Language-agnostic**: Automatic detection based on file extensions
3. **File-based results**: Agent results are saved to files, only paths are returned
4. **Simple retry**: Schema validation once, test retry 3 times
5. **Version management**: Must bump the `version` field in `.claude-plugin/plugin.json` on changes
6. **Node Ownership**: Each node's PM/PO and DEVELOPER hold full authority over that node's spec. External agents access node context exclusively via po-consultant verdict (INV-14), not direct file reads.
7. **Harness ≠ Cage** (v16, grounded in Anthropic Managed Agents 2026): The plugin is a **Brain-layer workflow guide**, not a procedural cage. Brain (SKILLs + Agents) guides; Hands (Rust CLI) executes; Session (session files + state.json) persists. Every Brain-layer rule must answer *"Can the model do this itself now?"* — if yes, delete it. Stronger future models must benefit from, not be throttled by, our SKILLs and agents. See "Harness Design Principles" below.

## Harness Design Principles

**Foundational framework** — Anthropic's Managed Agents architecture (2026) distinguishes three layers that evolve independently:

| Layer | Anthropic definition | Our plugin's implementation |
|-------|---------------------|----------------------------|
| **Brain** | Claude + the harness — orchestration logic, control flow, decision-making | SKILLs (`/spec`, `/dev`, `/validate`, ...) and Agents (impl, impl-reviewer, tdd-coder, ...) — Markdown-defined guides |
| **Hands** | Concrete capabilities the model invokes directly — sandboxes, tools, code execution | Rust CLI in `core/` — deterministic subcommands (parse-tree, validate-schema, diff-node-history, ...) |
| **Session** | Append-only durable log separate from context window | `${TMP_DIR}*-session-*.md`, `.claude/workflows/{dir-safe}/state.json`, git commit history |

**Guide (Brain) vs Detail (Hands) distinction** — the Brain **guides** the model; the Hands **extend** the model. Conflating these is the root of over-harnessing: encoding in prose what should be a CLI call, or encoding in a CLI what should be left to judgment.

**The guiding question** (Anthropic, verbatim): ***"Can the model do this itself now? If yes, delete it."***

> "Every component in a harness encodes an assumption about what the model can't do on its own, and those assumptions are worth stress testing because they can quickly go stale as models improve." — Anthropic Engineering

> "The scaffolding we built for a Claude 3-level intelligence is a cage for a Claude 4-level one."

**Subtraction discipline** — every Brain-layer instruction (SKILL step, agent criterion, reviewer rule) is a *liability* by default. Its burden of proof is to demonstrate:
1. A concrete failure mode the model exhibits *today* without it, AND
2. That the failure is not better addressed by a Hands-layer tool (CLI subcommand) or by richer session context.

If either fails, delete. Deletion is the default move on every audit; addition requires justification.

**Applied systematically in v16 and enforced for all future plugin changes.** When writing or reviewing a SKILL/agent, check every prescriptive element against these three refactors:

| Anti-pattern | Replacement | Why |
|--------------|-------------|-----|
| **Number → Criterion** | Arbitrary caps (`max_rounds=3`, `max_retry=N`, `max 3 parallel`) → explicit convergence/outcome criteria + runaway safety net | Counters terminate by timer, not quality. A stronger model can judge "stuck" better than we can pre-declare. Keep numeric bounds only as bug-guards, labeled as such. |
| **Procedure → Outcome** | Step-by-step parsing/matching algorithms (keyword lists, regex heuristics, lexical scoring) → Goal + Input/Output + delegated judgment | Hardcoded procedures encode our current reasoning and ceiling the model at our level. State the desired outcome and let the model reason. |
| **Prohibition → Default** | Blanket bans ("never X", "always skip Y") → "default X, exception when Y" with conditions the model judges | Bans block adaptive behavior in edge cases the rule-writer didn't foresee. Defaults with judged exceptions preserve safety without capping intelligence. |

**Legitimate constraints (do NOT relax)**:
- Invariants (INV-1 ~ INV-14): safety/integrity — never soften
- Schema validation, security boundaries, INV-8 write scope — deterministic rules
- TDD discipline (tdd-coder must run before code delivery), Convention hierarchy — workflow correctness

**Red flags when drafting a SKILL/agent instruction**:
- A fixed integer (`max_*=N`) where `N` isn't justified as a bug-guard
- "Parallel (up to 3)" or similar arbitrary concurrency caps
- A parsing rule that encodes "what related means" instead of delegating relatedness judgment
- Skip rules without re-entry conditions
- Blanket word bans without context-sensitive exceptions

**Termination signals (prefer model-emitted)**:
- Reviewer `verdict: approved` — success
- Reviewer `progress: no` — stuck, surface to user with current state
- Safety net (`rounds > 10`, etc.) — bug indicator, not convergence criterion

**Coverage heuristic for future audits**: if raising the model's capability by one generation would not change how a SKILL runs, the SKILL is likely over-constraining. Revisit.

**Audit posture** (aligns with Anthropic's "art of subtraction"): every review cycle, walk the Brain layer and ask the guiding question per instruction. A plugin version that *only adds* Brain-layer rules between releases is a red flag — healthy evolution deletes staler scaffolding than it adds. Track deletion ratio in release notes when meaningful.

**Layer migration signals**:
- A Brain-layer rule that lends itself to regex/AST/schema enforcement → candidate to migrate to Hands (Rust CLI subcommand). E.g., v17 P2-a: duplicate identifier detection moved from reviewer prose to `validate-schema --strict`.
- A Hands-layer tool whose output the model always reinterprets → candidate to simplify or remove; the Brain already does the work.
- A piece of prior state that the Brain has to reconstruct from context → candidate for Session promotion (session file / state.json field). E.g., v17 Phase 0 M1: prior CLAUDE.md/DEVELOPERS.md bodies moved into session rather than re-derived.

## Superpowers Coexistence

claude-md composes superpowers domain components to create the "document-driven development" business.

### Responsibility Division

| Layer | Owner | Tools |
|-------|-------|-------|
| Spec definition, validation, tracking | claude-md | /spec (with --resync), /validate, /decompile, /impact, /inspect |
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
| impl | brainstorming | Single-pass: extract → draft → self-critique → snapshot-judge → generate (diff-aware on update) |
| impl-reviewer | (none) | Reviews generated CLAUDE.md/DEVELOPERS.md + rationale sidecar, return verdict (max 1 retry) |
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
