---
name: dev
version: 2.0.0
aliases: [gen, generate, build, compile]
description: |
  This skill should be used when the user asks to "develop from CLAUDE.md", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/dev". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Dispatches tdd-coder for per-Constraint Red-Green-Refactor cycles, then test-reviewer for post-TDD verification.
  Trigger keywords: code generation, develop, code from CLAUDE.md
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

# /dev

Generates source code based on CLAUDE.md via TDD Red-Green-Refactor cycles.

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
| `--targets` | No | - | Explicit target list (comma-separated, `"."` for root). Skips diff-compile-targets. |

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Determine dev targets

**`--targets` mode (explicit):**
```
Parse comma-separated paths from --targets argument.
For each path, create target entry with reason: "explicit".
"." is treated as root (dir: ".", claude_md_path: "CLAUDE.md").
```

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
  - src/auth/jwt (depth=3, typescript)
  - src/auth (depth=2, typescript)
  - src/utils (depth=2, typescript)
```

### 6. Create session files

For each target, create a tdd-session file:

0. (`--all` excluded) Collect node history via CLI:
      ```bash
      LAST_DEV=$(git log -1 --format="%H" --grep="^dev({path}):" 2>/dev/null || echo "")
      $CLI_PATH diff-node-history \
        --path {path} --root {project_root} --limit 20 \
        --grep "^spec({path}):" \
        ${LAST_DEV:+--since-commit "$LAST_DEV"} \
        --output "${TMP_DIR}node-history-${dir_safe}.json"
      ```
   For root target (path = "."):
   - `--grep "^spec(.):"` matches root spec commits
   - `--path .` scans root-level files
   If `has_history` is false: do not include Spec Changes section.
1. Read target CLAUDE.md → extract Requirements, Domain Context
2. Read target DEVELOPERS.md → extract Constraints, Technical Context, Data Schemas
3. Resolve Convention hierarchy (module > project > general)
4. Read dev-context.md (optional) → extract Dependencies, approach
5. Write session file → `${TMP_DIR}tdd-session-{dir-safe}.md`
6. (If `has_history` is true) Add Spec Changes section from node-history JSON:
   - Extract transition context from `CommitEntry.body` → `### Transition Context`
   - Parse `file_diffs[].sections[].changes[]` to derive `### Added`, `### Modified`, `### Removed`
   - If `CommitEntry.breaking` is true → add `breaking: true` metadata

Session file format: see "TDD Session File Format" in dev-templates.md.

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

### 7. Task(tdd-coder) — Red-Green-Refactor cycles

Dispatch tdd-coder per target. Parallel batches for independent modules at same depth (max 3).

```
Dispatch Task(tdd-coder):
  Session file: ${TMP_DIR}tdd-session-{dir-safe}.md
  Save results to ${TMP_DIR} and return only the path

Extract from result block:
  status, implemented_files, test_files, mapping_file, tests_passed, tests_failed
```

Status handling:
- `success` → proceed to Step 8
- `partial` → log WARNING with unmapped items, proceed to Step 8
- `failed` → log ERROR, skip module

### 8. SKILL test verification (tdd-coder result check)

**Do not trust agent self-reports. Verify by running tests directly.**

```
For each completed module:

1. Read mapping_file → extract test_files list

2. Run test suite:
   | Language   | Command                                            |
   | TypeScript | npx jest --no-cache {test_files} 2>&1              |
   | Rust       | cargo test 2>&1                                    |
   | Python     | python -m pytest {test_files} -v 2>&1              |
   | Go         | go test ./... -v 2>&1                              |

3. Evaluate:
   - ALL pass → proceed to Step 9
   - SOME fail:
     a. Log: "[TDD VERIFY FAILED] {path}: {N} tests failing"
     b. Determine rollback files from tdd-result:
        rollback_files = implemented_files ∪ test_files
     c. Rollback:
        - Tracked files: git checkout -- {rollback_files}
        - Untracked: git clean -fd -- {untracked in rollback_files}
     d. Module status = "verify_failed", skip to next module
```

### 9. Task(test-reviewer) — Post-TDD verification

```
round = 1, max_rounds = 3

loop:
  9a. Create test-reviewer session file:
      ${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md:

      ```markdown
      # Test Review Session
      type: test-review | round: {round} | language: {lang} | target: {path}
      dir_safe: {dir-safe}
      mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
      spec_session_file: ${TMP_DIR}tdd-session-{dir-safe}.md
      implemented_files: [{from tdd-result or previous revise result}]
      test_files: [{from tdd-result or previous revise result}]
      ```

  9b. Dispatch Task(test-reviewer):
      Session file: ${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md
      Save results to ${TMP_DIR} and return only the path

      Extract verdict from result block.

  9c. if verdict == "approved":
        break → Step 10

  9d. if round >= max_rounds:
        Log: "⚠ [REVIEW INCOMPLETE] {path}: proceeding with {N} known gaps"
        break → Step 10

  9e. Create tdd-coder revise session:
      Overwrite ${TMP_DIR}tdd-session-{dir-safe}.md with added fields:
        mode: revise
        round: {round}
        feedback_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md

  9f. Dispatch Task(tdd-coder, mode=revise):
      Session file: ${TMP_DIR}tdd-session-{dir-safe}.md
      Save results to ${TMP_DIR} and return only the path

  9g. SKILL test verification (same as Step 8):
      Run tests → if fail → rollback revise changes, keep previous state
      → break with warning

  9h. round++ → return to 9a
```

### 10. Task(refactorer) — Convention application

**Only dispatch when Conventions exist in the resolved hierarchy.**

```
Create refactorer session file:
${TMP_DIR}refactor-session-{dir-safe}.md
(format: see "Refactorer Session File Format" in dev-templates.md)
Implementation Files: implemented_files from tdd-result (or latest revise result)

Dispatch Task(refactorer):
  Session file: ${TMP_DIR}refactor-session-{dir-safe}.md
  Target directory: {path}
  Detected language: {language}
  Save results to ${TMP_DIR} and return only the path

refactor-result status:
- success: proceed to Step 11
- rolled_back: record warning (tdd-coder results preserved)
- skipped: proceed to Step 11
```

### 11. SKILL test verification (refactorer result check)

**Only when refactorer status == success (actual changes were made).**

```
1. Run test suite (same commands as Step 8)

2. Evaluate:
   - ALL pass → proceed
   - SOME fail:
     a. Log: "[REFACTOR VERIFY FAILED] {path}: reverting Convention changes"
     b. Rollback refactored_files:
        git checkout -- {refactored_files from refactor-result}
     c. tdd-coder results are preserved
```

### 12. Cross-module test gate

**Only when multiple modules completed in this dev run.**

```
1. Skip if language uses workspace-wide test runner (Rust cargo test, Go go test ./...)
   — per-module verification in Steps 8/11 already covered the full workspace

2. For file-targeted runners (TypeScript, Python):
   Collect test files: union of test_files from each passing module's mapping.json
   Run full suite:
   | Language   | Command                                                    |
   | TypeScript | npx jest --no-cache {all collected test_files} 2>&1       |
   | Python     | python -m pytest {all collected test_files} -v 2>&1       |

3. If cross-module failures:
   → identify interfering modules from failure output
   → rollback affected modules
   → mark as cross_module_failed
```

### 13. Build verification

After all modules complete, run type check:

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
   Recommended action: Review DEVELOPERS.md Constraints and re-run /dev
   ```
3. Return dev status = `failed`

> **Limitation**: If new files are not declared in `mod.rs`/`lib.rs`, cargo check will not inspect those files.
> The tdd-coder agent must always add mod declarations when creating new files.

### 14. Display changes + Create dev commit

```bash
git diff --stat
```

If dev completed successfully, **create individual commits per target directory** that passed all verification gates:

```bash
# Repeat for each dev target
git add {created/modified files in the target directory}
git commit -m "dev({path}): {summary}

{1-2 sentence summary of compiled content}

Changes:
- compiled: {list of generated files}
- tests: {list of generated test files}"
```

This commit becomes the reference point for `git log --grep="^dev({path}):"` searches.

### 15. Post-dev verification (optional)

If `--validate` flag is present:
```
Skill("claude-md-plugin:validate", args: "{path}")
```

### 16. Result

```
---dev-result---
status: success | partial | failed
total: {n}
generated: {n}
skipped: {n}
verify_failed: {n}
review_status: {approved | incomplete}
module_details:
  - path: {path}, status: success, tests: {passed}/{total}
  - path: {path}, status: verify_failed, reason: {reason}
tests: {passed} passed, {failed} failed
validate: {status} (when --validate is used)
---end-dev-result---
```

## DO / DON'T

**DO:**
- Follow leaf-first order
- Complete Convention hierarchy resolution when creating session files
- Follow tdd-coder → test-reviewer → refactorer order
- Handle DELETE tasks directly via SKILL before the TDD pipeline
- Verify agent results by running tests directly (Steps 8, 11)
- Dispatch refactorer only when Conventions exist

**DON'T:**
- Modify CLAUDE.md (read-only)
- Pass CLAUDE.md path directly to Agent (pass via session file)
- Skip SKILL test verification (Steps 8, 11) — never trust agent self-reports
- Enter refactorer without test-reviewer completion

## Error Handling

| Situation | Response |
|-----------|----------|
| CLI build failure | install-cli.sh handles automatic build |
| CLAUDE.md not found | guidance message, exit |
| tdd-coder failure (single module) | warn, continue with the rest |
| tdd-coder partial (some constraints unmet) | warn, proceed to reviewer |
| SKILL test verify failed (Step 8) | rollback module, skip |
| test-reviewer max_rounds reached | best-effort proceed with documented gaps |
| refactorer regression failure | rollback refactorer changes only |
| SKILL test verify failed (Step 11) | rollback refactorer only, preserve tdd results |
| Cross-module test interference | rollback affected modules |
| Build verification failure | report error, status=failed |
| Language detection failure | AskUserQuestion |
