---
name: project-setup
description: |
  Adds or updates the Conventions section (unified 6 subsections) in project/module CLAUDE.md.
  For existing projects, extracts conventions from source code; for new projects, collects them interactively.
  Use the --update option to modify an existing Conventions section (absorbs the former /convention-update).
argument-hint: "[project_root_path] [--update [content]]"
allowed-tools: [Bash, Read, Glob, Grep, Write, AskUserQuestion]
---

# /project-setup

Adds/updates the Convention section in the project CLAUDE.md so it can be referenced during the `/dev` REFACTOR phase.

## Triggers

- `/project-setup`
- `project setup`
- `create conventions`
- `update conventions` (--update mode)
- `modify conventions` (--update mode)

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `project_root_path` | No | Auto-detect | Project root path |
| `--update` | No | false | Existing Conventions update mode |
| `content` | No | - | Change instructions to apply with --update (interactive if omitted) |

## Workflow

### 1. Determine Project Root

If an argument is provided, use that path. Otherwise, check the CWD for project root markers (`.git`, `package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod`, etc.).

If no marker is found, request path input via AskUserQuestion.

**No parent directory traversal** — do not search outside the CWD.

### 2. Module Root Detection

Auto-detect module roots based on build marker files (`package.json`, `Cargo.toml`, `go.mod`, etc.).
If no module root is found, treat the project root as a single module.

### 3. Check Existing Convention Section + Mode Branching

Check for `## Conventions` in the project_root CLAUDE.md.

**If `--update` mode or Conventions exist:**
→ Branch to Step 3-U (update mode)

**If Conventions do not exist:**
→ Proceed to Step 4 (new creation)

### 3-U. Update Mode (absorbs former /convention-update)

#### 3-U-A. When argument content is provided

Auto-determine target subsection via content analysis:
| Keywords | Target |
|----------|--------|
| directory, folder, structure | Project Structure |
| module, dependency, layer | Module Boundaries |
| package name, directory name | Naming Conventions |
| language, version, runtime | Language & Runtime |
| coding, pattern, rule | Coding Rules |
| variable name, function name, naming | Naming Rules |

Apply content to the target subsection → user confirmation → save.

#### 3-U-B. When argument content is not provided (interactive)

Display the current 6 Conventions subsections and AskUserQuestion:
"Select the subsection to modify: [1-6]"

Display the current content of the selected subsection and collect modifications.

After completion, proceed to Step 9 (verification).

### 4. Determine Project Type

Distinguish between existing and new projects based on source file presence.

### 5. Extract or Collect Conventions

#### 5-A. Existing Project: Extract via Code Analysis

| Analysis Target | Method |
|-----------------|--------|
| Language/Runtime | File extension statistics, build config files |
| Directory patterns | Top-level directory structure analysis |
| Coding rules | Async patterns, error handling, type usage, etc. |
| Naming rules | Variable/function/class/constant pattern analysis |
| Test patterns | Framework, file patterns, mock strategy |

> **Lint exclusion principle**: If formatter/linter config files exist, items handled by those tools are excluded from Conventions.

Show analysis results to the user and confirm via AskUserQuestion.

#### 5-B. New Project: Interactive Collection

Q1. Language selection → Q2. Structure style → Q3. Coding style

### 5-C. Ask Document Language

Ask the user via AskUserQuestion which language CLAUDE.md and DEVELOPERS.md should be written in:

```
Which language should CLAUDE.md and DEVELOPERS.md be written in?
Examples: English, Korean, Japanese, Chinese, etc.
(Default: English)
```

If the user does not specify, default to `English`.

### 6. Generate `## Instructions` Section

If `## Instructions` does not exist in the project root CLAUDE.md, generate it:

```markdown
## Instructions

- Document language: {selected language}
- CLAUDE.md is the SSOT. Source code is a derived artifact generated from CLAUDE.md.
- When code disagrees with CLAUDE.md, regenerate code via /dev (not modify docs).
- To change requirements, update CLAUDE.md first, then code follows.
- Derive tests from DEVELOPERS.md Constraints.
- Generate source code via /dev. Do not create source files directly with the Write tool.
- Must run /validate --strict before declaring completion.
```

### 7. Generate `## Conventions` Section

Include the 6 required subsections:

```markdown
## Conventions

### Project Structure
### Module Boundaries
### Naming Conventions
### Language & Runtime
### Coding Rules
### Naming Rules
```

Optional subsections: API Design, Error Strategy, Testing Strategy, Test Convention, etc.

### 8. Per-Module Convention Handling (DRY Principle)

Skip if single module.
Multi-module: inherit if same as project_root, write override if different.

### 9. Verification

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
$CLI_PATH validate-convention --project-root {project_root}
```

If CLI build fails, ask via AskUserQuestion whether to install or skip.

### 10. Result Report

Display list of generated/updated files and inheritance information.

## Error Handling

| Situation | Response |
|-----------|----------|
| Project root detection failed | Request path input |
| No file write permissions | Error message |
| Source analysis failed | Fall back to interactive collection |
| CLI build failed | Ask whether to install or skip |
