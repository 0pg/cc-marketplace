# orchestrator-guide Plugin

Claude Code Plugin for multi-agent coordination with 5-element delegation protocol.

## Design Principles

This plugin defines **protocols** (what must happen) while leaving **implementation details** (how) to the model:

| Area | Plugin (What) | Model (How) |
|------|---------------|-------------|
| Task Size | "Must be evaluated" | Specific criteria |
| Split Strategy | "Can be split" | When/how to split |
| Agent Selection | "Role-based" | Which agent |
| Verification | "Must be done" | Which commands |

This separation allows the plugin to be **project-agnostic** while providing consistent coordination patterns.

## Plugin Structure

```
orchestrator-guide/
├── .claude-plugin/
│   └── plugin.json           # Plugin manifest (official spec)
├── config/
│   └── project-config.template.md  # Project-specific config template
├── skills/                   # Skills (auto-detected by Claude Code)
│   ├── orchestrator/         # Main orchestration workflow
│   ├── planner/              # Planning and spec generation
│   └── delegator/            # Delegation prompt generation
├── hooks/                    # Hook definitions
│   ├── hooks.json            # Hook configuration
│   ├── session-start.sh      # SessionStart hook script
│   ├── session-start.md      # SessionStart documentation
│   └── task-continuation.md
├── agents/                   # Agent pattern documentation
│   ├── parallel-coordinator.md   # Parallel execution patterns
│   └── verification-chain.md     # Verification workflow
└── templates/                # Templates for delegation
    ├── delegation-prompt.md  # Role-based delegation templates
    └── arb-template.md       # Agent Result Block format
```

## Core Interfaces

### 1. 5-Element Delegation Protocol

All agent delegations must include:
- **GOAL**: What to achieve (task.md reference)
- **CONTEXT**: Related files, patterns, background
- **CONSTRAINTS**: What not to do
- **SUCCESS**: Verification criteria (project-config reference)
- **HANDOFF**: Next role or follow-up

### 2. ARB (Agent Result Block)

Standardized agent result reporting format:
```yaml
---agent-result---
status: success | partial | blocked | failed
agent: {agent_name}
task_ref: {task_id}
files:
  created: []
  modified: []
verification:
  tests: pass | fail
  lint: pass | fail
issues: []
followup: []
---end-agent-result---
```

### 3. Role-Based Agent Selection

Agents are selected by **role**, not by name:

| Role | Purpose |
|------|---------|
| **Implementation** | Code writing |
| **Exploration** | Code analysis |
| **Review** | Code review |

Actual agent names are mapped from project-config or CLAUDE.md.

### 4. Verification Chain

Lint → Tests → Code Review

Verification commands are referenced from project-config.

### 5. Task Size Protocol

Before delegation:
1. Evaluate task size
2. Split if necessary (vertical, horizontal, checkpoint)
3. Handle context compaction recovery

## Project Configuration

To customize for your project, copy and fill `config/project-config.template.md`:

```yaml
roles:
  implementation:
    backend: backend-impl
    frontend: frontend-impl
  review:
    code: rust-code-reviewer

verification:
  lint:
    command: cargo clippy -p {module}
    flags: "-- -D warnings"
  test:
    command: cargo test -p {module}
```

Or define directly in CLAUDE.md.

## Usage

Skills are automatically triggered:
- `/orchestrator` - Main orchestration workflow
- `/planner` - Planning and spec generation
- `/delegator` - Delegation prompt generation

Hooks execute automatically:
- `SessionStart` - Loads context at conversation start, clear, resume, compaction

## Compatibility

### Module Path Detection

The hook script automatically detects common module patterns:
- `crates/` (Rust workspaces)
- `packages/` (Node.js monorepos)
- `modules/` (Generic)
- `libs/` (Library structures)

### Task File Location

Supported locations:
- `spec/task.md` (project root)
- `{module_pattern}/{module}/spec/task.md` (module-specific)
