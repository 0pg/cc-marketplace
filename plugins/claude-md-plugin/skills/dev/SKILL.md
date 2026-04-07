---
name: dev
version: 1.0.0
aliases: [gen, generate, build, compile]
description: |
  This skill should be used when the user asks to "develop from CLAUDE.md", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/dev". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Performs 4-agent TDD pipeline: test-writer → test-reviewer loop → green-coder → refactorer.
  Trigger keywords: code generation, develop, code from CLAUDE.md
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

# /dev

Generates source code based on CLAUDE.md.

## Triggers

- `/dev`
- `code generation`
- `code from CLAUDE.md`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `--path` | No | `.` | Target path |
| `--all` | No | false | Target all CLAUDE.md files (instead of incremental) |
| `--conflict` | No | `skip` | File conflict handling: `skip` \| `overwrite` |
| `--dry-run` | No | false | Display targets only without generating actual files |
| `--validate` | No | false | Automatically run /validate after compilation |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Determine dev targets

**`--all` mode:**
```
Glob("{path}/**/CLAUDE.md")
```

**Incremental mode (default):**
```bash
$CLI_PATH diff-compile-targets --root {path}
```

Result branching:
- Not a git repository → fallback to all targets
- No changes → "All up-to-date. Use --all for full dev." → exit
- Changes found → display target list + reasons

Exit if no targets.

### 2. Auto-detect language

Analyze file extensions in each target directory to infer the language:
1. Source file extensions in the directory → determine language
2. If no source files, reference parent directory
3. If all fail, ask via `AskUserQuestion`

### 3. Check dev-context (optional)

If a `dev-context.md` exists in the same directory as each CLAUDE.md, use it as reference.
Normal operation continues even if absent.

### 4. Determine dependency order (leaf-first)

Sort by directory depth (deepest first).
Independent modules at the same depth can be executed in parallel (up to 3).

### 5. `--dry-run` handling

Output only the target list and exit:
```
Dev targets:
  • src/auth/jwt (depth=3, typescript)
  • src/auth (depth=2, typescript)
  • src/utils (depth=2, typescript)
```

### 6. Create session files

For each target, read CLAUDE.md + DEVELOPERS.md + Convention hierarchy and create session files:

0. (`--all` excluded) Search for spec commits — execute per target directory:
   a. Find last dev commit:
      ```bash
      LAST_DEV=$(git log -1 --format="%H" --grep="^dev({path}):" 2>/dev/null || echo "")
      ```
   b. Find spec commits after that:
      ```bash
      if [ -n "$LAST_DEV" ]; then
        SPEC_COMMITS=$(git log --format="%H" --grep="^spec({path}):" ${LAST_DEV}..HEAD 2>/dev/null)
      else
        SPEC_COMMITS=$(git log --format="%H" --grep="^spec({path}):" 2>/dev/null)
      fi
      ```
   c. If found — extract diff + message for each spec commit:
      ```bash
      # Extract diff (root commit guard)
      PARENT=$(git rev-parse --verify {hash}~1 2>/dev/null || echo "")
      if [ -n "$PARENT" ]; then
        git diff {hash}~1..{hash} -- {path}/CLAUDE.md {path}/DEVELOPERS.md
      else
        git diff --root {hash} -- {path}/CLAUDE.md {path}/DEVELOPERS.md
      fi

      # Extract commit message
      git log -1 --format="%B" {hash}
      ```
   d. If not found: do not include Spec Changes section in the session file
1. Read target CLAUDE.md → extract Requirements, Domain Context
2. Read target DEVELOPERS.md → extract Constraints, Technical Context
3. Resolve Convention hierarchy (module > project > general)
4. Read dev-context.md (optional) → extract Dependencies, approach
5. Write session file → `${TMP_DIR}dev-session-{dir-safe}.md`
6. (If spec commits found in sub-step 0) Add Spec Changes section:
   - Extract transition context from commit message body → `### Transition Context`
   - Parse commit message Changes section → `### Added`, `### Modified`, `### Removed`
   - If BREAKING flag present → add `breaking: true` metadata

Session file format: see "Dev Session File Format" in dev-templates.md.

**6e. Derive Implementation Tasks (only when Spec Changes present)**

When the session file contains `## Spec Changes`:
1. Added → `[ADD]` task: tests+implementation needed for new Constraint/Requirement
2. Modified → `[MODIFY]` task: modify tests+implementation to match changed Constraint/Requirement
3. Removed → `[DELETE]` task: remove code+tests related to deleted Constraint/Requirement

Add `## Implementation Tasks` section to the session file:
```markdown
## Implementation Tasks (only when Spec Changes present)
- [ADD] CONST-N: {description}
- [MODIFY] CONST-N: {change details}
- [DELETE] CONST-N: {deletion target}
```

**6f. Execute [DELETE] tasks (only when present)**

SKILL handles DELETE directly before the TDD pipeline:

1. Search for imports/references of deletion targets via Grep
2. Collect list of referencing files
3. Delete target files/functions (Bash rm or Edit)
4. Remove imports/calls from referencing files (Edit)
5. Delete related test files
6. Run regression tests → report warning on failure

### 7. Test Writing Loop (per target, sequential per module)

`round = 1`, `max_safety = 5`

```
loop:
  7a. Create test-writer session file:
      ${TMP_DIR}test-writer-session-{dir-safe}.md
      (format: see "Test Writer Session File Format" mode=write in dev-templates.md)

  7b. Dispatch Task(test-writer):
      Session file: ${TMP_DIR}test-writer-session-{dir-safe}.md
      Save results to ${TMP_DIR} and return only the path

      Extract test_dir, mapping_file from result block.

  7c. Create test-reviewer session file:
      ${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md:

      ```markdown
      # Test Review Session
      type: test-review | round: {round} | language: {lang}
      dir_safe: {dir-safe}
      mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
      test_dir: ${TMP_DIR}tests/{dir-safe}/
      spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
      ```

  7d. Dispatch Task(test-reviewer):
      Session file: ${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md
      Save results to ${TMP_DIR} and return only the path

      Extract verdict from result block.

  7e. if verdict == "approved":
        break → Step 7.5

  7f. if round >= max_safety:
        ⚠ Test review loop terminated after {max_safety} iterations.
          Proceeding with best-effort tests.
        break → Step 7.5

  7g. Create Revise session file:
      ${TMP_DIR}test-writer-session-{dir-safe}.md (overwrite):
      Change mode to revise, increment round, add feedback_file
      (format: see "Test Writer Session File Format" mode=revise in dev-templates.md)

  7h. Dispatch Task(test-writer, mode=revise):
      Session file: ${TMP_DIR}test-writer-session-{dir-safe}.md
      Save results to ${TMP_DIR} and return only the path

  7i. round++ → return to 7c
```

### 7.5. Copy TMP → target + Verify RED

```
7.5a. Copy TMP/tests/{dir-safe}/ → target directory
      Copy based on test_files paths in mapping.json

7.5b. Verify RED (SKILL executes directly via Bash):
      Run tests per language:
      | Language | Command |
      | TypeScript | npx jest --passWithNoTests 2>&1 |
      | Rust | cargo test --no-run 2>&1 (compile only) |
      | Python | python -m pytest --collect-only 2>&1 |
      | Go | go test -run "^$" ./... 2>&1 (compile only) |

7.5c. All fail confirmed → proceed to Step 8
7.5d. Some pass → record as existing implementation coverage, proceed to Step 8
7.5e. Compilation itself fails (import errors, etc.) → delegate to green-coder (import fix allowed)
```

### 8. Task(green-coder)

```
Create green-coder session file:
${TMP_DIR}green-session-{dir-safe}.md
(format: see "Green Coder Session File Format" in dev-templates.md)
Include conflict mode from dev arguments in session file header (default: skip)

Dispatch Task(green-coder):
  Session file: ${TMP_DIR}green-session-{dir-safe}.md
  Target directory: {path}
  Detected language: {language}
  Save results to ${TMP_DIR} and return only the path

Check green-result status:
- success: proceed to Step 9
- partial: collect warnings, proceed to Step 9
- failed: report error, move to next module
```

### 9. Task(refactorer)

```
Create refactorer session file:
${TMP_DIR}refactor-session-{dir-safe}.md
(format: see "Refactorer Session File Format" in dev-templates.md)
Implementation Files: implemented_files from green-result

Dispatch Task(refactorer):
  Session file: ${TMP_DIR}refactor-session-{dir-safe}.md
  Target directory: {path}
  Detected language: {language}
  Save results to ${TMP_DIR} and return only the path

refactor-result status:
- success: continue
- rolled_back: record warning (green-coder results preserved)
- skipped: continue
```

### 10. Build verification

After all modules complete, run type check based on detected language:

| Language | Command |
|----------|---------|
| Rust | `cargo check --workspace 2>&1` |
| TypeScript/JavaScript | `tsc --noEmit 2>&1` (only when tsconfig.json exists) |
| Python | `python -m py_compile $(find src -name "*.py") 2>&1` |
| Other | Skip (warning only) |

Success: proceed.

Failure:
1. Extract affected files from error message
2. Report:
   ```
   [BUILD FAILED] {error summary}
   Affected files: {file list}
   Recommended action: Review DEVELOPERS.md Constraints for the affected module and re-run /dev
   ```
3. Return dev status = `failed`, skip subsequent Steps

> **Limitation**: If new files are not declared in `mod.rs`/`lib.rs`, cargo check will not inspect those files.
> The green-coder agent must always add mod declarations when creating new files.

### 11. Display changes

```bash
git diff --stat
```

### 12. Create dev commit

If compilation completed successfully (status != failed), **create individual commits per target directory** (no consolidated commits):

```bash
# Repeat for each dev target
git add {created/modified files in the target directory}
git commit -m "dev({path}): {summary}

{1-2 sentence summary of compiled content}

Changes:
- compiled: {list of generated files}
- tests: {list of generated test files}"
```

This commit becomes the reference point for `git log --grep="^dev({path}):"` searches,
so individual commits per path are mandatory.

### 13. Post-dev verification (optional)

If `--validate` flag is present:
```
Skill("claude-md-plugin:validate", args: "{path}")
```

### 14. Result

```
---dev-result---
status: success | partial | failed
total: {n}
generated: {n}
skipped: {n}
tests: {passed} passed, {failed} failed
validate: {status} (when --validate is used)
---end-dev-result---
```

## DO / DON'T

**DO:**
- Follow leaf-first order
- Complete Convention hierarchy resolution when creating session files
- Follow test-writer → test-reviewer → green-coder → refactorer order
- Handle DELETE tasks directly via SKILL before the TDD pipeline

**DON'T:**
- Modify CLAUDE.md (read-only)
- Pass CLAUDE.md path directly to Agent (pass via session file)
- Enter green-coder without test-reviewer approval
- Use compiler agent (deprecated)

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| CLAUDE.md not found | guidance message, exit |
| test-writer failure | warn, continue with the rest |
| test-reviewer max_safety reached | best-effort proceed, warn |
| green-coder failure (single module) | warn, continue with the rest |
| refactorer regression failure | rollback, warn |
| Build verification failure (Step 10) | report error, status=failed |
| Language detection failure | AskUserQuestion |
