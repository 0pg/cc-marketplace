---
name: validate
version: 1.0.0
aliases: [check, verify, lint, fix-drift, resolve]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "lint documentation",
  "resolve drift", "fix documentation mismatch", "sync docs with code", or uses "/validate".
  Runs deterministic CLI validation (schema, convention structure, boundary, DEVELOPERS.md existence)
  and semantic validator agent for comprehensive drift detection, with interactive auto-fix.
  Trigger keywords: validate CLAUDE.md, document verification, drift check, resolve drift
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

# /validate

Verifies consistency between CLAUDE.md and actual code, and interactively resolves discovered issues.

## Triggers

- `/validate`
- `validate CLAUDE.md`
- `drift check`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `path` | No | `.` | Target path |
| `--strict` | No | false | Treat DEVELOPERS.md absence as ERROR, enable content drift verification |
| `--report-only` | No | false | Perform verification only, skip auto-fix |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Collect CLAUDE.md files

```
Glob("{path}/**/CLAUDE.md")
```

If no files collected: "No CLAUDE.md found in the target path." → exit.

### 2. Deterministic verification (CLI only, no agent)

#### 2a. Schema verification + auto-fix

For each CLAUDE.md:

```bash
$CLI_PATH validate-schema --file "$claude_md" --dir "$dir" --output "${TMP_DIR}schema-${dir_safe}.json"
```

On failure, auto-fix:
```bash
$CLI_PATH fix-schema --file "$claude_md"
$CLI_PATH validate-schema --file "$claude_md" --dir "$dir" --output "${TMP_DIR}schema-${dir_safe}.json"
```

If still failing after auto-fix → report schema error, exclude from Phase 3.

#### 2b. Convention structure verification

```bash
$CLI_PATH validate-convention --project-root {project_root} --output "${TMP_DIR}convention-result.json"
```

Collect `MISSING_CONVENTION`, `MISSING_SUBSECTION` issues. Does not block Phase 3.

#### 2c. Boundary verification

For each directory that passed schema:
```bash
$CLI_PATH resolve-boundary --path {dir} --claude-md {dir}/CLAUDE.md --output "${TMP_DIR}boundary-${dir_safe}.json"
```

Collect `PARENT_REFERENCE`, `SIBLING_REFERENCE` issues.

#### 2d. DEVELOPERS.md existence check (INV-3)

Check DEVELOPERS.md existence in each CLAUDE.md directory:
- If absent: ERROR in `--strict` mode, WARNING otherwise

### 2.5 Build changed spec + test coverage map

For each target that passed schema:

#### 2.5a. Detect changed specs

```bash
$CLI_PATH diff-spec-range --file {dir}/CLAUDE.md --root {project_root} \
  --output "${TMP_DIR}spec-diff-${dir_safe}.json"
```

Result fields: `changed_requirements[]`, `source_changed_files[]`, `source_changed`, `all_requirements`

- `all_requirements=true`: not a git repo or first commit → all Requirements are verification targets
- `source_changed=false` AND `changed_requirements` is empty → no changes, semantic verification can be skipped

#### 2.5b. Build test coverage map

**Step 1: Filter to target_dir scope + re-determine source_changed**

Filter `source_changed_files` from `spec-diff-${dir_safe}.json` to files under `{target_dir}`:
```
target_source_files = source_changed_files.filter(f => f.startsWith({target_dir}))
```
- If `all_requirements=true`: `target_source_files` = full Glob(`{target_dir}/**/*.{rs,ts,js,py}`)
- If `target_source_files` is empty AND `changed_requirements` is empty → **skip semantic verification** (no actual changes within the module)

**Step 2: Extract public functions per source file**

For each `target_source_file`, extract public function list using language-specific patterns:

| Language | Grep Pattern | Extraction Target |
|----------|-------------|-------------------|
| Rust | `^pub fn \|^    pub fn ` in {source_file} | fn names |
| TypeScript/JS | `^export (function\|const\|async function) ` in {source_file} | symbol names |
| Python | `^def [^_]\|^    def [^_]` in {source_file} | fn names (excluding private _) |

Result: `public_fns = [fn_name, ...]` per source_file

**Step 3: Search for test files**

Language-specific search paths (inside target_dir + project_root/tests/ integration tests):

| Language | Search Paths | Test Identification Pattern |
|----------|-------------|---------------------------|
| Rust | `{target_dir}/**/*.rs` + `{project_root}/tests/**/*.rs` | `#[test]` (-A 1 → fn name) |
| TypeScript/JS | `{target_dir}/**/*.{test,spec}.{ts,js}` + `{project_root}/**/__tests__/**` | `it\(\|test\(\|describe\(` (-A 1 → name) |
| Python | `{target_dir}/**/{test_*.py,*_test.py}` + `{project_root}/tests/**/*.py` | `^def test_` (-A 1 → fn name) |

`test_files_found` = number of test files discovered

**Step 4: Check function references (populate calls[])**

Within each test case fn, Grep for `public_fns` items from Step 2:
```
for each test_fn in test_cases:
  calls = [fn for fn in public_fns if Grep(fn, in test_fn body)]
```

Result structure (JSON per module):
```json
[
  {
    "source_file": "src/agent/mod.rs",
    "public_fns": ["spawn_agent", "AgentResult"],
    "test_files_found": 1,
    "test_cases": [
      { "name": "test_spawn_agent_success", "calls": ["spawn_agent"], "line": "tests/agent_test.rs:15" }
    ]
  },
  {
    "source_file": "src/tracker/linear.rs",
    "public_fns": ["update_issue"],
    "test_files_found": 0,
    "test_cases": []
  }
]
```

If no test files: record `test_files_found: 0` (TEST_MISSING signal).

### 3. Create session files + Semantic verification (validator agent)

For each target that passed schema:

1. Parse CLAUDE.md:
```bash
$CLI_PATH parse-claude-md --file {dir}/CLAUDE.md --output "${TMP_DIR}parsed-${dir_safe}.json"
```

2. Resolve Convention hierarchy (project > module)

3. Read DEVELOPERS.md (in strict mode)

4. Write session file → `${TMP_DIR}validate-session-{dir-safe}.md`:

```markdown
# Validate Task: {path}
type: validate | target: {path} | strict: {true|false}

## CLAUDE.md Content
Purpose: {parsed purpose}
Requirements:
{parsed requirements list}
Domain Context: {parsed domain context}

## Conventions (resolved)
{hierarchy-resolved Conventions}

## DEVELOPERS.md Content (strict only)
Constraints:
{parsed constraints}
Technical Context:
{parsed technical context}

## Deterministic Results
{summary of CLI issues found in Phase 2}

## Changed Requirements (diff-spec-range result)
all_requirements: {true|false}
source_changed: {true|false}  ← value filtered to target_dir scope
Added/Changed: {changed_requirements list — action + text}
Changed source files (within target_dir): {target_source_files list}

## Test Coverage Map
{JSON array built via Grep in 2.5b}
Module scope limited: includes only {target directory}
```

5. Dispatch `Task(validator)` (parallel batch, up to 3):
```
Session file: ${TMP_DIR}validate-session-{dir-safe}.md
Verification target: {directory}
strict: {true|false}
```

### 4. Auto-fix (Interactive)

If not `--report-only`:

1. Output full issue list (deterministic + semantic)
2. For ERROR/WARNING issues, present resolution direction by drift type + AskUserQuestion:

   **Resolution direction rules (CLAUDE.md = SSOT principle):**

   | Drift Type | Default Resolution | User Options |
   |---|---|---|
   | REQUIREMENTS_NOT_IMPLEMENTED | Recommend /dev | (a) Regenerate with /dev (b) Remove requirement from CLAUDE.md |
   | REQUIREMENTS_PARTIALLY_IMPLEMENTED | Recommend /dev | (a) Regenerate with /dev (b) Adjust CLAUDE.md to match current state |
   | REQUIREMENTS_VIOLATED | User judgment required | (a) Code is wrong → /dev (b) Requirement changed → update CLAUDE.md |
   | CONVENTION_*_VIOLATION | Fix code | (a) Fix code (b) Relax Convention rule |
   | CONSTRAINT_NOT_ENFORCED | Fix code | (a) Fix code (b) Update DEVELOPERS.md Constraint |
   | TECH_CONTEXT_STALE | Fix documentation | Auto-update DEVELOPERS.md |

   **Modification scope limits:**
   - validate's direct Edit: modifications to existing code (changes within existing functions/structs)
   - New code generation (new files, new functions, new modules): only provide guidance "Regenerate with /dev {path}", do not auto-invoke /dev within validate
   - CLAUDE.md/DEVELOPERS.md modifications: only after explicit user approval

3. Apply approved modifications
4. Re-verify after modifications

### 5. Consolidated report

Collect `result_file` paths from Task(validator) result blocks and include as list:

```
---validate-result---
status: clean | issues_found | fixed
total_modules: {n}
schema_errors: {n}
convention_issues: {n}
boundary_issues: {n}
semantic_drift: {n}
auto_fixed: {n}
result_files:
  - {TMP_DIR}validate-{dir-safe}.md
  - {TMP_DIR}validate-{dir-safe-2}.md
  (only modules that passed schema and had semantic verification run. omit if none)
---end-validate-result---
```

## DO / DON'T

**DO:**
- Always perform Phase 2 CLI verification first
- Exclude schema-failed modules from semantic verification
- Delegate to validator agent via session files

**DON'T:**
- Perform semantic verification on schema-failed modules
- Modify CLAUDE.md/DEVELOPERS.md without user approval (auto-fix only via fix-schema CLI or explicit user approval)
- Generate new code (new files, new functions, new modules — that's /dev's role)

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| CLAUDE.md not found | guidance message, exit |
| validator agent failure (single module) | warn, continue with the rest |
