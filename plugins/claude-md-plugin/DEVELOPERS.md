# claude-md-plugin — Developer Specification

## Constraints

### CLI Subcommand Contracts
- All CLI subcommands exit 0 on success, non-zero on error
- JSON output subcommands write valid JSON to stdout or to `--output` file path
- `validate-schema` returns exit 0 even when violations are found; violations are in JSON output
- `fix-schema` modifies files in-place; it does NOT validate after fixing

### Schema Version
- `schema-rules.yaml` version field follows `MAJOR.MINOR` (e.g., `4.2`)
- Breaking schema changes (section rename, required field added) increment MAJOR
- Additive changes (new optional section) increment MINOR
- Build-time code generation (`build.rs`) reads `schema-rules.yaml` and emits Rust constants; after changing the YAML, `cargo build` must be re-run

### Session File Required Fields
- All session files begin with a markdown H1 title: `# {Skill} Task: {path}`
- `type:` field is mandatory on line 2
- Agents must not assume fields beyond what the session file format specifies

### Agent Observations Write Scope (INV-8)
- Agents may only append/update/delete within `## Agent Observations` in DEVELOPERS.md
- Agents must never modify Requirements, Constraints, or any other section
- `converge_schema` skips `## Agent Observations` (agent-managed, not auto-added)

### Environment Variables
| Variable | Description |
|----------|-------------|
| `CLI_PATH` | Path to `claude-md-core` binary |
| `TMP_DIR` | Temp directory for session files (trailing slash required) |
| `CLAUDE_PLUGIN_ROOT` | Absolute path to plugin root directory |

### INV-3: CLAUDE.md ↔ DEVELOPERS.md Pairing
- Every directory with CLAUDE.md must have a corresponding DEVELOPERS.md
- In `--strict` mode, `validate-schema` reports absence of DEVELOPERS.md as a warning

## Technical Context

### Runtime
- **CLI**: Rust binary (`claude-md-core`), built with `cargo build --release`
- **Runtime dependency**: Binary must be available as `$CLI_PATH` in skill scripts
- **Rust edition**: 2021, clap 4.4 (derive feature), serde 1.0, walkdir 2.4

### Testing
- **Unit tests**: `cargo test` (176 tests as of v11.1.0)
- **Cucumber (ATDD)**: `cargo test --test cucumber` — `.feature` files in `core/tests/features/`
- **Skipped scenarios**: Steps without implementations are skipped (not failed); currently 73 skipped
- **Schema codegen**: `build.rs` generates `schema_constants.rs` from `schema-rules.yaml` at build time

### Claude Code Plugin System
- Plugin manifest: `.claude-plugin/plugin.json`
- Skills, agents, commands registered as file paths in manifest
- `CLAUDE_PLUGIN_ROOT` env var points to plugin root at runtime (used in agent file cross-references)
- `TMP_DIR` env var provides temp directory for session files and intermediate artifacts

### Document Language
- Skills and agents are written in English (technical content)
- User-facing messages in skills may be in Korean per project convention

## Decision Log

### v4.0 CLAUDE.md Schema (2-Document System)
- **Context**: Single CLAUDE.md was becoming too large (PM requirements + developer constraints mixed)
- **Decision**: Split into CLAUDE.md (PM SSOT) and DEVELOPERS.md (derived spec, developer constraints)
- **Rationale**: Claude Code auto-loads CLAUDE.md; DEVELOPERS.md loaded on-demand → token efficiency

### Session File Pattern (v10)
- **Context**: Skills were doing too much inline logic; agents had no stable input contract
- **Decision**: Skills extract info → write session file → dispatch agent; agents only consume session files
- **Rationale**: Debuggable intermediate artifact, stable SKILL↔Agent interface, parallelizable

### Agent Observations Section (v11.1.0)
- **Context**: Agents discovered patterns and workarounds during work but had no sanctioned write target
- **Decision**: Add `## Agent Observations` to DEVELOPERS.md as agent-managed section (INV-8)
- **Rationale**: Separates agent-generated knowledge from human-authored spec; prevents spec drift; enables promotion workflow (INV-10)

### converge Excludes Agent Observations (v11.1.0)
- **Context**: `fix-schema`/`converge` auto-adds missing sections, which would overwrite agent entries
- **Decision**: `DEVELOPERS_AGENT_MANAGED_SECTIONS` list excludes these from converge
- **Rationale**: Agent-managed sections must only be modified by agents (INV-8)

### schema-rules.yaml Modification Procedure (v11.2.0)
- **Context**: `schema-rules.yaml` drives build-time codegen (`build.rs`) and all schema validation; changes require a coordinated multi-step update
- **Decision**: Follow this sequence: (1) Edit `core/schema-rules.yaml` (2) Run `cargo build` to trigger `build.rs` codegen (3) Update affected tests in `core/tests/features/` (4) Run `cargo test --test cucumber`
- **Rationale**: Out-of-order changes cause compile failures (stale constants) or silent test drift

### DEVELOPERS.md Schema v5.0 Reduction (v12.0.0)
- **Context**: Self Socratic Loop review found Operations and Public API are either not consumed by agents or derivable from code. Technical Context, Constraints, Decision Log, Agent Observations are the only sections with non-derivable, agent-consumed content.
- **Decision**: Remove Operations and Public API from schema. Migrate env vars to Constraints, modification procedures to Decision Log. structural observation promotion target changed from Operations to Technical Context.
- **Rationale**: 6→4 section reduction. Operations content absorbed by Constraints (env vars) and Decision Log (procedures). Public API derivable from code via `format-exports` CLI; intended API contracts belong in Constraints.

## Agent Observations

None
