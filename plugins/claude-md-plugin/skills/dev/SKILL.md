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
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
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

0. (`--all` excluded) Collect node history via CLI — execute per target directory:
      ```bash
      LAST_DEV=$(git log -1 --format="%H" --grep="^dev({path}):" 2>/dev/null || echo "")
      $CLI_PATH diff-node-history \
        --path {path} --root {project_root} --limit 20 \
        --grep "^spec({path}):" \
        ${LAST_DEV:+--since-commit "$LAST_DEV"} \
        --output "${TMP_DIR}node-history-${dir_safe}.json"
      ```
   If `has_history` is false in the JSON output: do not include Spec Changes section.
1. Read target CLAUDE.md → extract Requirements, Domain Context
2. Read target DEVELOPERS.md → extract Constraints, Technical Context
3. Resolve Convention hierarchy (module > project > general)
4. Read dev-context.md (optional) → extract Dependencies, approach
5. Write session file → `${TMP_DIR}dev-session-{dir-safe}.md`
6. (If `has_history` is true in sub-step 0) Add Spec Changes section from node-history JSON:
   - Extract transition context from `CommitEntry.body` → `### Transition Context`
   - Parse `file_diffs[].sections[].changes[]` to derive `### Added`, `### Modified`, `### Removed`
   - If `CommitEntry.breaking` is true → add `breaking: true` metadata

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
        1. Read last 2 rounds' test-reviewer result files:
           ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round-1}.md
           ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md
        2. Extract Critical Question IDs (CONST-N / REQ-N) from each

        3. Classify by comparing ID sets:

        CONVERGING — last round's issue IDs ⊂ previous round's issue IDs
          (issues are being resolved, some remain):
          → ⚠ "[REVIEW INCOMPLETE] {path}: proceeding with {N} known gaps: {IDs}"
          → Append to green-coder session: unreviewed_gaps: [{last round's Critical Questions}]
          → break → Step 7.5

        STUCK — last round's issue IDs == previous round's issue IDs
          OR (last ⊄ prev AND |last| ≤ |prev|)
          (identical or oscillating issues, test-writer cannot converge):
          → HALT module
          → "[TEST LOOP STUCK] {path}: unresolvable after {max_safety} rounds
             Stuck on: {issue IDs and summaries}
             Action: Review DEVELOPERS.md Constraints for testability"
          → module status = "skipped"
          → skip to next module

        DIVERGING — |last round's issue IDs| > |previous round's issue IDs|
          (issues growing, not converging):
          → HALT module
          → "[TEST LOOP DIVERGING] {path}: issues growing after {max_safety} rounds
             Action: Review Constraints for ambiguity or test-reviewer criteria"
          → module status = "skipped"
          → skip to next module

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
      Actually run tests to confirm they fail without implementation:
      | Language   | Command                                            |
      | TypeScript | npx jest --no-cache {test_files} 2>&1              |
      | Rust       | cargo test 2>&1                                    |
      | Python     | python -m pytest {test_files} -v 2>&1              |
      | Go         | go test ./... -v 2>&1                              |

7.5c. Interpret results (exit code + output analysis):

      - exit != 0 AND assertion/test failures in output:
        → RED confirmed, proceed to Step 8

      - exit != 0 AND only compilation/import errors (no assertion failures):
        → delegate to green-coder for import fix (existing behavior)

      - exit != 0 AND runtime/infrastructure errors
        (e.g., DB connection refused, network timeout, missing system dependency):
        → WARN: "[RED UNVERIFIABLE] {path}: external dependency unavailable"
        → proceed with caution (record as unverified)

      - exit == 0 AND ALL tests pass:
        → Check mapping.json: if ALL mapped tests are Existence-type (STRUCT-XXX)
          → exempt, RED confirmed (existence tests legitimately pass before impl)
        → Otherwise: [RED VIOLATION] — tests pass without implementation
          red_violation_count++
          if red_violation_count > 2:
            → HALT module
            → "[RED FAILED] {path}: tests pass without implementation
               after {red_violation_count} rewrites — likely tautological"
            → module status = "skipped"
          else:
            → return to Step 7 test-writer loop:
              round++ (reuse existing max_safety counter)
              Create feedback as Critical Question format:
              "RED VIOLATION: All tests pass without implementation.
               Assertions must verify specific output values per Constraint I/O contract,
               not merely existence/type/truthiness."

      - exit == 0 AND SOME pass:
        → WARN: "[RED PARTIAL] {path}: {N}/{total} tests already pass — existing coverage"
        → record as existing implementation coverage, proceed to Step 8
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
- partial:
    1. Extract tests_passed, tests_failed from green-result
    2. Calculate pass_rate = tests_passed / (tests_passed + tests_failed)
    3. Log: "⚠ [GREEN PARTIAL] {path}: {tests_passed}/{total} tests passing ({pass_rate}%)"
    4. Tag module as "gate_required"
       (Step 10.5 Final Test Gate will hard-gate before commit)
    5. Proceed to Step 9 (refactorer may fix structural/naming issues such as
       file location mismatches, import path errors, or naming convention violations;
       logic gaps — missing algorithm implementation, TODO stubs — will be caught by
       the Final Test Gate at Step 10.5)
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

### 10.5. Final Test Gate (mandatory)

For each module that passed build verification, run the actual test suite as a hard commit gate.

```
For each module (target path + mapping.json):

1. Run test suite:
   | Language   | Command                                            |
   | TypeScript | npx jest {test_files from mapping.json} 2>&1       |
   | Rust       | cargo test 2>&1                                    |
   | Python     | python -m pytest {test_files from mapping.json} -v 2>&1 |
   | Go         | go test ./... -v 2>&1                              |

2. Evaluate:
   - ALL pass → module gate_status = "passed"
   - SOME fail:
     a. Cross-reference failing tests with mapping.json
        → identify unmet Constraint IDs and Requirement IDs
     b. Report:
        [TEST GATE FAILED] {path}: {N} tests failing
        Unmet Constraints: {CONST-IDs}
        Unmet Requirements: {REQ-IDs}
     c. Determine rollback file list from agent results:
        - refactor-result.status = "success":
          rollback_files = refactored_files (from refactor-result)
          (refactorer may have moved/renamed files; use its output, not green-coder's)
        - refactor-result.status = "rolled_back" | "skipped" | absent:
          rollback_files = implemented_files (from green-result)
        Rollback:
        - Staged files:    git reset HEAD -- {rollback_files}
        - Tracked files:   git checkout -- {rollback_files}
        - New untracked:   git clean -fd -- {files in rollback_files that are untracked}
     d. Module gate_status = "gate_failed", do NOT commit this module
   - Execution crash/timeout:
     a. Report: [TEST GATE ERROR] {path}: {error}
     b. Module gate_status = "gate_failed", do NOT commit

3. Cross-module verification:
   After all per-module gates, if multiple modules passed individually:
   - Skip if language uses workspace-wide test runner (Rust `cargo test`, Go `go test ./...`)
     — per-module gate already executed the full workspace; re-running is redundant
   - For file-targeted runners (TypeScript, Python):
     Collect test files: union of test_files from each passing module's mapping.json
     Run full test suite:
     | Language   | Command                                                    |
     | TypeScript | npx jest {all collected test_files} 2>&1                  |
     | Python     | python -m pytest {all collected test_files} -v 2>&1       |
   - If cross-module failures detected:
     → identify interfering modules from failure output
     → mark affected modules as gate_failed
     → rollback affected modules (same as step 2c)
```

### 11. Display changes

```bash
git diff --stat
```

### 12. Create dev commit

If compilation completed successfully (status != failed), **create individual commits per target directory** that passed the Final Test Gate (gate_status = "passed"). Modules with gate_status = "gate_failed" are excluded from commit:

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
gate_passed: {n}
gate_failed: {n}
gate_details:
  - path: {path}, gate_status: passed, tests: {passed}/{total}
  - path: {path}, gate_status: gate_failed, tests: {passed}/{total}, unmet: [{IDs}]
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
| test-reviewer max_safety CONVERGING | best-effort proceed with documented gaps |
| test-reviewer max_safety STUCK/DIVERGING | HALT module, skip to next |
| green-coder failure (single module) | warn, continue with the rest |
| refactorer regression failure | rollback, warn |
| Build verification failure (Step 10) | report error, status=failed |
| Final Test Gate failure (Step 10.5) | rollback module, exclude from commit |
| Cross-module test interference (Step 10.5) | rollback affected modules |
| Language detection failure | AskUserQuestion |
