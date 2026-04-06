# Decompile Templates

## Decompile Session File Format

```markdown
# Decompile Task: {path}
type: decompile | target: {path}

## Tree Info
source_file_count: {n}
subdir_count: {n}
depth: {n}

## Children CLAUDE.md
{List of already-generated child CLAUDE.md paths, or "None" if empty}

## Project Conventions
{project root Conventions or "None"}

## Agent Observations
path: {path}/DEVELOPERS.md#Agent Observations
```

## CLI Workflow

### Step 1: Boundary Resolution
```bash
$CLI_PATH resolve-boundary --dir {target_dir} --output ${TMP_DIR}decompile-boundary-{dir-safe}.json
```
Result: direct_files, subdirectories, reference_violations

### Step 2: Code Analysis
```bash
$CLI_PATH analyze-code --path {target_dir} --output ${TMP_DIR}decompile-analyze-{dir-safe}.json
```
Result: exports, dependencies, behaviors, contracts, protocol

### Step 3: Analysis Formatting
```bash
$CLI_PATH format-analysis --input ${TMP_DIR}decompile-analyze-{dir-safe}.json --output ${TMP_DIR}decompile-summary-{dir-safe}.md
```
Result: LLM-ready summary markdown

### Step 4: Schema Validation
```bash
$CLI_PATH validate-schema --file {claude_md_path} --dir {target_dir}
```
On failure:
```bash
$CLI_PATH fix-schema --file {claude_md_path}
```

## Document Generation Rules

### CLAUDE.md (Primary SSOT)

- **Purpose**: Describe the reason for the code's existence from a business value perspective. "None" not allowed.
- **Requirements**: Reverse-extract requirements the code fulfills from the user's perspective. "None" if truly none exist.
- **Domain Context**: Business constraints inferred from the code. "None" or AskUserQuestion if cannot be inferred.

### DEVELOPERS.md (Derived Spec)

- **Constraints**: Precisely describe the code's input/output contracts (convertible to tests)
- **Data Schemas**: Auto-extracted from `analyze-code` ExportedType (interface/type/struct/enum) — public type definitions
- **Technical Context**: Technologies used and their rationale
- **Decision Log**: Design decisions inferred from the code (optional)
- **Operations**: Deployment/monitoring related (optional); `### Configuration` auto-extracted from `analyze-code` env_vars
- **Flows**: Written only in project root DEVELOPERS.md — system-level use case execution flows (optional)

### Smart Merge (when existing CLAUDE.md exists)

1. Purpose: preserve existing (human-authored is more accurate)
2. Requirements: preserve existing + add undocumented items discovered from code
3. Domain Context: preserve existing + supplement

### INV-1 Compliance

```
node.dependencies ⊆ node.children
```
Reference the child CLAUDE.md list to verify dependencies are within children scope.

## Result Format

```
---decompiler-result---
status: success | failed_with_warnings
target_dir: {path}
validation: passed | failed_with_warnings
developers_md: generated | skipped
---end-decompiler-result---
```
