# Review Report: claude-md-plugin v2.2.0

**Date**: 2026-02-06
**Scope**: Rust Core (symbol_index.rs, dependency_graph.rs, schema_validator.rs, main.rs) + Plugin/Skill Structure
**Overall**: Rust Core — Needs Improvement | Plugin/Skill Structure — Good with Minor Improvements

---

## Summary

| Severity | Rust Core | Plugin/Skill | Total |
|----------|-----------|-------------|-------|
| Critical | 3 | 2 | **5** |
| Major | 6 | 4 | **10** |
| Minor | 7 | 5 | **12** |
| **Total** | **16** | **11** | **27** |

---

## Critical (5)

### C-1. `incremental_rebuild` index remapping bug — **FIXED** (Severity: Latent)
- **File**: `symbol_index.rs` (was lines 536-541)
- **Issue**: `symbols.remove(idx)` shifted subsequent indices, but `file_symbols` mapping for unchanged files retained stale indices. `save_cache()` rebuilt `file_symbols` from scratch on every save, so **no active data corruption occurred**, but any future code reading `file_symbols` mid-rebuild would hit stale indices (latent risk).
- **Root Cause**: `file_symbols`/`file_references` were redundant — `SymbolEntry.module_path` already provides the file→symbol mapping.
- **Fix Applied**: Removed `file_symbols`/`file_references` from `CachedSymbolIndex`. Replaced index-based removal with `symbols.retain()`. Bumped `CACHE_VERSION` to 2 for automatic cache invalidation. Added `test_sequential_incremental_rebuilds` verifying 4 sequential rebuild cycles (modify, modify, remove).

### C-2. `build()` double file read
- **File**: `symbol_index.rs:139+141`
- **Issue**: `std::fs::read_to_string()` at line 139, then `claude_md_parser.parse(file)` at line 141 reads the same file again internally. Violates P0.4 "single file read" requirement.
- **Fix**: Expose `parse_content(content: &str)` method on `ClaudeMdParser`.

### C-3. `schema-validate` skill missing `--with-index` documentation
- **File**: `skills/schema-validate/SKILL.md`
- **Issue**: `schema_validator.rs` has `validate_with_index()` and `main.rs` has the `--with-index` CLI flag, but the skill doc does not document it. Agents invoking schema-validate will never use cross-reference validation.
- **Fix**: Add `--with-index` flag documentation to SKILL.md.

### C-4. `schema-validate` required sections mismatch
- **File**: `skills/schema-validate/SKILL.md`
- **Issue**: Required sections list says 5, but `schema-rules.yaml` SSOT defines 7 (missing Summary and Domain Context). Valid example also lacks Summary.
- **Fix**: Sync required sections list with `schema-rules.yaml`; fix valid example.

### C-5. `validate` skill missing cross-reference validation
- **File**: `skills/validate/SKILL.md`
- **Issue**: Needs cross-reference validation as an option in the workflow.
- **Fix**: Add `--with-index` pass-through in validate workflow.

---

## Major (10)

### M-1. P0.3 double directory walk
- **File**: `symbol_index.rs:249-280`
- **Issue**: `collect_claude_md_paths()` performs two walks: `tree_parser.parse(root)` then `scan_claude_md_files_static(root)`. Violates "single directory walk" P0.3.
- **Fix**: Use a single walk and collect paths from it.

### M-2. Regex DRY violation
- **Files**: `symbol_index.rs:124`, `schema_validator.rs:80`
- **Issue**: Cross-reference regex is duplicated between two modules.
- **Fix**: Extract to a shared constant (e.g., in a common module or `schema_rules`).

### M-3. Self-reference detection fragile
- **File**: `schema_validator.rs:526-546`
- **Issue**: `to_anchor.ends_with(...)` is overly broad and can incorrectly match non-self anchors. `symbol_index.rs:400-406` approach using module paths is simpler and more correct.
- **Fix**: Adopt module-path-based self-reference detection.

### M-4. `validate-schema --no-cache` not supported
- **File**: `main.rs`
- **Issue**: `--with-index` flag exists but there is no `--no-cache` pass-through for schema validation.
- **Fix**: Add `--no-cache` pass-through when `--with-index` is used.

### M-5. `schema-validate` valid example is invalid
- **File**: `skills/schema-validate/examples/valid/CLAUDE.md`
- **Issue**: Missing the `Summary` section, which is required per `schema-rules.yaml` SSOT. This "valid" example would actually fail validation.
- **Fix**: Add Summary section to the valid example.

### M-6. Hardcoded CLI paths in skill documentation
- **Files**: Multiple `SKILL.md` files
- **Issue**: Several skills reference `plugins/claude-md-plugin/core/target/release/claude-md-core` instead of `${CLAUDE_PLUGIN_ROOT}/core/target/release/claude-md-core`.
- **Fix**: Use variable reference for portability.

### M-7. No component registration in `plugin.json`
- **File**: `.claude-plugin/plugin.json`
- **Issue**: All 15 skills, 8 agents, and hooks rely entirely on filesystem auto-discovery without explicit declaration.
- **Fix**: Consider explicit registration for discoverability and validation.

### M-8. `spec/SKILL.md` missing example blocks
- **File**: `skills/spec/SKILL.md`
- **Issue**: Missing `<example>` blocks. All other entry point skills have XML example blocks.
- **Fix**: Add example blocks consistent with other entry point skills.

### M-9. Inconsistent internal skill description format
- **Files**: Various `skills/*/SKILL.md`
- **Issue**: 6 older skills use Korean `(internal)` prefix one-liners; 4 newer skills use English multi-line with invocation context.
- **Fix**: Standardize to English multi-line format.

### M-10. `validate/SKILL.md` hardcoded path
- **File**: `skills/validate/SKILL.md`
- **Issue**: Hardcoded path `./plugins/claude-md-plugin/core/target/release/claude-md-core` while other skills use just `claude-md-core`.
- **Fix**: Use consistent path reference.

---

## Minor (12)

### m-1. Description mismatch between `plugin.json` and `marketplace.json`
- `marketplace.json` has longer description mentioning Schema v2, diagrams, migration.

### m-2. Individual skill versions diverge from plugin version
- Skill frontmatter versions (e.g., compile: 1.1.0, decompile: 2.0.0) diverge from plugin version 2.2.0. Non-standard and confusing.

### m-3. `chrono` duplication
- `symbol_index.rs` uses `chrono::Utc::now()` while `dependency_graph.rs` has a hand-rolled `chrono_lite_now()`. Inconsistent.

### m-4. Step definitions not implemented for 3 new feature files
- 87 skipped cucumber scenarios (`dependency_graph_symbols.feature`, `schema_cross_reference.feature`, `symbol_index_cache.feature`).

### m-5. `test_diff_mtimes` potential flakiness
- **File**: `symbol_index.rs:983`
- Asserts exact vector order but `HashMap` iteration is non-deterministic.

### m-6. Missing test cases
- Incremental rebuild on file modification, multiple sequential rebuilds, root-level CLAUDE.md, CLAUDE.md with no exports but cross-references, self-reference exclusion.

### m-7. `compile/references/workflow.md` uses Python-specific syntax
- Violates project pseudocode guidelines (language-neutral pseudocode required).

### m-8. `schema-validate/examples/invalid-parent-ref/CLAUDE.md` ambiguous
- Missing Contract, Protocol, Domain Context — fails for multiple reasons, making it an ambiguous test case.

### m-9. Non-standard `trigger` frontmatter field
- Verify it is consumed by the runtime.

### m-10. `dependency-graph/SKILL.md` missing examples directory
- Unlike other CLI-wrapper skills.

### m-11. No v2-format examples in schema-validate
- Only v1 table-based Exports, not v2 heading-based `#### symbolName`.

### m-12. `validate/SKILL.md` hardcoded path (duplicate of M-10)
- Consolidated with M-10.

---

## Positive Aspects

- Excellent directory organization and clean entry-point/internal separation
- Consistent structured result blocks (`---{skill}-result---` format)
- Thorough error handling tables in every skill
- Good progressive disclosure in complex skills
- SSOT principle with `schema-rules.yaml`
- Robust, non-blocking SessionStart hook with graceful degradation
- SemVer strictly followed
- Version correctly synchronized between `plugin.json` and `marketplace.json`
- 64 unit tests + 90 passing cucumber scenarios

---

## Recommended Fix Order

1. ~~**C-1** (index remapping bug)~~ — **FIXED** (was latent, not active)
2. **C-2** (double file read) — P0 requirement violation
3. **M-1** (double directory walk) — P0 requirement violation
4. **M-2** (regex DRY) — quick win, shared constant
5. **C-3 + C-4 + C-5** (skill docs) — unblock cross-reference validation usage
6. **M-3** (self-reference detection) — correctness
7. **m-4** (step definitions) — unblock 87 skipped scenarios
8. Remaining Major/Minor items
