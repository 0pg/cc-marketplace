---
name: impact
version: 1.0.0
aliases: [impact-analysis, affected, downstream]
description: |
  This skill should be used when the user asks to "analyze change impact", "what modules are affected",
  "show downstream dependencies", "impact analysis", or uses "/impact".
  Traces CLAUDE.md changes through module dependency graph (Grep-based, 2-hop limit)
  to identify affected modules, constraints, and source files.
  Trigger keywords: impact analysis, change impact, affected modules, downstream
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep]
---

# /impact

Analyzes the impact of CLAUDE.md changes across the module dependency graph.

## Triggers

- `/impact`
- `impact analysis`
- `change impact`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `--path` | No | auto-detect | Target module path to analyze |
| `--all` | No | false | Analyze all changed modules |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Detect changed modules

**`--path` specified:**
Target = the specified module.

**`--path` not specified:**
```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

If no changes detected → "No spec changes detected. Use --path to analyze a specific module." → exit.

If `--all`: analyze all changed modules. Otherwise, if multiple changes detected, list them and ask which to analyze.

### 2. Analyze changes

For each target module:

```bash
$CLI_PATH diff-node-history --path {path} --root {project_root} --limit 5 \
  --output "${TMP_DIR}impact-history-${dir_safe}.json"
```

Parse `SectionChange.text` entries:
- Match `REQ-\d+:` patterns → changed Requirements
- Match `CONST-\d+:` patterns → changed Constraints
- Non-matching lines → raw change entries

### 3. Build dependency graph (Grep-based)

```bash
$CLI_PATH scan-claude-md --root {project_root}
```

For each module in the index (excluding the changed module itself):

```
Grep: search CLAUDE.md and DEVELOPERS.md in each module
  for references to the changed module's path
```

Modules that reference the changed module path = **direct dependents**.

### 4. Transitive dependents (2-hop limit)

For each direct dependent found in Step 3:

```
Grep: search all other modules' CLAUDE.md and DEVELOPERS.md
  for references to the direct dependent's path
```

Modules that reference a direct dependent = **transitive dependents**.

Stop at 2 hops to prevent exponential fan-out.

### 5. Constraint mapping (advisory)

For each dependent module (direct + transitive):

```
Grep: search dependent's DEVELOPERS.md
  for references to the changed module's path
```

Extract `CONST-\d+:` lines near matches → "may need update" advisory.

This is NOT a deterministic mapping. It is string-match based and may produce false positives.

### 6. Source file mapping

For each affected module:

```
Glob: list source files in the module directory
  (exclude CLAUDE.md, DEVELOPERS.md, test files, config files)
```

### 7. Display impact report

```
=== Impact Analysis: {path} ===

Changed:
  ~ REQ-1: {text}              [MODIFY]
  + REQ-4: {text}              [ADD]

Direct dependents:
  {dependent_path}
    CONST-2, CONST-5 may need update
    Files: {source_file_list}

Transitive dependents:
  {transitive_path}
    CONST-1 may need update
    Files: {source_file_list}

Summary: {N} modules, {M} constraints, {K} source files potentially affected
===
```

When no dependents are found:
```
=== Impact Analysis: {path} ===

Changed:
  ~ REQ-1: {text}              [MODIFY]

No downstream dependents found.

===
```

## Limitations

- **Grep-based**: Module path string matching may produce false positives
- **2-hop limit**: Deeper transitive dependencies are not tracked
- **Advisory constraints**: "may need update" is not a deterministic judgment
- **resolve-boundary not used**: It reports violations only, not dependency relationships

## DO / DON'T

**DO:**
- Show all detected dependents with advisory constraint list
- Clearly label constraint mappings as "may need update"
- Handle non-git repos gracefully (skip diff-based detection, require --path)

**DON'T:**
- Dispatch any agents — this is CLI + Grep based analysis
- Modify any files — read-only operation
- Present advisory results as definitive ("will break" vs "may need update")
- Use resolve-boundary for dependency graph construction
