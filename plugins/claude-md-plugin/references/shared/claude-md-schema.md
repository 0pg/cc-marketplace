<!--
  This file is for examples and explanations.
  Single Source of Truth for rules: core/schema-rules.yaml
-->

# CLAUDE.md Schema Template (v4.0.0)

This template defines the standard structure for CLAUDE.md files.

**CLAUDE.md = Primary SSOT — PM's Requirements Document**
- CLAUDE.md is a business requirements document that PMs can read and write
- DEVELOPERS.md is a Derived Spec where developers concretize Requirements at the system level
- Source code is a Derived Artifact generated from documentation

## 2-Document System

```
┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                         │
│                                                             │
│   CLAUDE.md (Primary SSOT, auto-loaded)                    │
│     → Purpose, Requirements, Domain Context                │
│                                                             │
│   DEVELOPERS.md (Derived Spec, on-demand)                  │
│     → Constraints, Technical Context, Decision Log,        │
│       Operations                                           │
└─────────────────────────────────────────────────────────────┘
```

| Document | Role | Load Method |
|----------|------|-------------|
| **CLAUDE.md** | Primary SSOT (PM requirements) | auto-loaded |
| **DEVELOPERS.md** | Derived Spec (developer specification) | on-demand |

## Required Sections Summary (3 always-required + 2 conditional)

| Section | Required | Condition | "None" Allowed | Description |
|---------|----------|-----------|----------------|-------------|
| Purpose | always | — | ✗ | The reason the module exists (business value) |
| Requirements | always | — | ✓ | Business requirements (user perspective, verifiable statements) |
| Domain Context | always | — | ✓ | Business constraint background (regulations, legacy, organizational reasons) |
| Instructions | conditional | is_project_root | ✗ | AI behavior directives (project root only) |
| Conventions | conditional | is_project_or_module_root | — | Unified project/code-level rules |

> Rule details: See `core/schema-rules.yaml`

---

## Detailed Description

### 1. Purpose (Required, None Not Allowed)
State the reason the module exists in 1-2 sentences, focused on business value.

```markdown
## Purpose
Handles user authentication. Ensures security compliance and smooth user experience.
```

### 2. Requirements (Required, None Allowed)
Describe business requirements from the user's perspective. Must be readable and writable by PMs.

When there are no requirements, explicitly state `None`.

```markdown
## Requirements
None
```

When requirements exist:

```markdown
## Requirements
- Auto-refresh on access with expired token, no user re-login required
- Maximum 5 concurrent login devices, oldest session terminated when exceeded
- Token lifetime limits per PCI-DSS regulations
```

**Requirements Writing Principles:**
- Describe behavior from the user's perspective
- Minimize technical jargon
- Focus on business value
- Ambiguity is acceptable (concretization happens in DEVELOPERS.md Constraints)

### 3. Domain Context (Required, None Allowed)
Summarize business constraint background in 2-3 sentences.

When there is no domain context, explicitly state `None`.

```markdown
## Domain Context
None
```

When domain context exists:

```markdown
## Domain Context
- Token expiration period limited per PCI-DSS compliance
- Continued support for UUID v1 format for legacy system compatibility
```

### 4. Instructions (Conditional - project root only)
Specify AI behavior directives. Only written in the project root CLAUDE.md.

The `Document language` field specifies the language for generated CLAUDE.md and DEVELOPERS.md content.
Set via `/project-setup`. If absent, agents will ask the user.

```markdown
## Instructions
- Document language: English
Always use TypeScript strict mode.
Follow the team's code review process.
```

### 5. Conventions (Conditional - project_root or module_root)

Unified project/code-level rules. Required in the project_root CLAUDE.md; used as optional override in module_root.

```markdown
## Conventions

### Project Structure
Feature-based directory organization under src/.

### Module Boundaries
Each module has its own CLAUDE.md; circular dependencies are forbidden.

### Naming Conventions
Directories: kebab-case, Files: camelCase

### Language & Runtime
TypeScript 5.0, Node.js 20 LTS

### Coding Rules
- Async: use async/await, raw Promise forbidden
- Types: strict mode, any forbidden
- Tests: `__tests__/` directory, `*.test.ts` naming

### Naming Rules
- Variables/functions: camelCase
- Classes/types: PascalCase
```

**Required Subsections (6):**

| Subsection | Required | Description |
|------------|----------|-------------|
| Project Structure | Yes | Directory structure rules |
| Module Boundaries | Yes | Module responsibility rules, dependency direction |
| Naming Conventions | Yes | Module/directory/package naming |
| Language & Runtime | Yes | Primary language, version, runtime |
| Coding Rules | Yes | Basic coding rules (including test file placement rules) |
| Naming Rules | Yes | Code-level naming rules |

## Reference Rules

### Allowed
- Parent → Child: Can reference child directories

### Forbidden
- Child → Parent: Cannot reference parent directories
- Sibling ↔ Sibling: Cannot cross-reference sibling directories

## Related Documents

- **DEVELOPERS.md**: Derived Spec — A companion document that concretizes CLAUDE.md Requirements at the system level
- Template: `references/shared/developers-md-schema.md`

### Invariants

**INV-3: CLAUDE.md ↔ DEVELOPERS.md Pairing (Active)**
```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
DEVELOPERS.md absence reported as warning in --strict mode
```

**INV-5: Convention Section Placement Rules**
```
project_root/CLAUDE.md MUST contain ## Conventions
module_root/CLAUDE.md MAY contain ## Conventions (override; inherits from project_root if absent)
Single module: project_root == module_root → placed in the same CLAUDE.md
```
