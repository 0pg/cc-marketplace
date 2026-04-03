# Design: validate-language — Two-Tier Document Language Validation

**Date**: 2026-04-03
**Status**: Approved
**Scope**: CLI subcommand + /validate SKILL integration + validator agent extension

## Problem

When `Document language` is configured in project root `## Instructions`, there is no mechanism to verify that CLAUDE.md and DEVELOPERS.md are actually written in the declared language. Documents may contain untranslated content from previous edits or agent generation errors.

## Solution

Two-tier validation:
1. **Tier 1 (CLI, deterministic)**: Count characters by Unicode script, calculate target language percentage
2. **Tier 2 (LLM, semantic)**: If below threshold (70%), agent reviews non-target lines for false positives

## CLI Subcommand: `validate-language`

### Interface

```bash
$CLI_PATH validate-language \
  --file <path>            # CLAUDE.md or DEVELOPERS.md
  --expected <language>    # "English", "Korean", "Japanese", "Chinese"
  --threshold 70           # minimum % (default: 70)
  --output <path>          # JSON result
```

### Algorithm

1. Read file content as UTF-8
2. Strip non-prose content:
   - Fenced code blocks (``` and ~~~ pairs)
   - Inline code (backtick-delimited)
   - URLs (`https?://\S+`)
   - Absolute paths (`/[\w/.-]+`)
   - Relative paths (`\./[\w/.-]+`)
   - Markdown syntax characters (`#`, `|`, `-`, `>`, `*`, `[`, `]`, `(`, `)`)
3. Classify each remaining character by Unicode script range
4. Calculate: `target_script_chars / total_classified_chars * 100`
5. If `total_classified_chars < 20` → skip validation (insufficient content)
6. Compare percentage against threshold

### Language → Script Mapping

| `--expected` | Target Script | Unicode Ranges |
|---|---|---|
| English | Latin | U+0041-U+024F |
| Korean | Hangul | U+AC00-U+D7AF, U+1100-U+11FF, U+3130-U+318F |
| Japanese | Hiragana+Katakana+CJK | U+3040-U+30FF, U+4E00-U+9FFF |
| Chinese | CJK | U+4E00-U+9FFF |

Unsupported `--expected` values → error with supported language list.

**Known limitations**:
- Chinese and Japanese share the CJK range. A purely-Kanji Japanese document may pass a Chinese check and vice versa. Character-range detection cannot distinguish them without NLP. This is an acceptable tradeoff for a deterministic first-tier check.
- Cannot distinguish between Latin-script languages (English, French, German, Spanish, etc.). Tier 1 treats all Latin-script content equally. A French document would pass an English check. This is inherent to character-range detection — NLP would be required for intra-script language identification.

### Output JSON

```json
{
  "file": "src/auth/CLAUDE.md",
  "expected_language": "English",
  "expected_script": "Latin",
  "threshold": 70,
  "result": "pass | below_threshold | skipped",
  "target_percentage": 92.3,
  "script_distribution": {
    "Latin": 92.3,
    "Hangul": 5.1,
    "Other": 2.6
  },
  "total_classified_chars": 1847,
  "non_target_lines": [12, 34, 56]
}
```

- `result: "pass"` — target_percentage >= threshold
- `result: "below_threshold"` — target_percentage < threshold → triggers LLM review
- `result: "skipped"` — total_classified_chars < 20

`non_target_lines` contains original file line numbers (1-indexed, not stripped content line numbers) where non-target script characters were detected. A line is classified as "non-target" when **>50% of its classified characters** belong to a non-target script. Used by the LLM tier to read only relevant sections.

### Stripping Rules Detail

| Content Type | Detection | Action |
|---|---|---|
| Fenced code blocks | ``` or ~~~ open/close pairs | Remove entire block including delimiters |
| Inline code | Single backtick pairs | Remove content between backticks |
| URLs | `https?://\S+` | Remove match |
| Absolute paths | `/[\w/.-]+` | Remove match |
| Relative paths | `\./[\w/.-]+` | Remove match |
| Markdown headings | Lines starting with `#` | Strip entire heading line (headings are structural, not prose; schema-defined names like `## Purpose` would inflate Latin count in non-English docs) |
| None markers | Lines that are exactly `None` | Strip entire line (schema-defined marker, always Latin) |
| Table delimiters | `\|`, separator rows (`---`) | Remove `\|` and pure-separator rows |
| List markers | `- `, `* `, `1. ` | Strip marker only (preserve list text) |
| Blockquotes | `> ` | Strip `>` only (preserve quoted text) |

### Character Classification

Characters are classified into one of these buckets:

| Bucket | Character Types | Counted |
|---|---|---|
| Script-specific | Latin, Hangul, CJK, Hiragana, Katakana | Yes — contributes to percentage |
| Neutral | Digits (0-9), punctuation, whitespace, symbols | No — excluded from calculation |

### Error Handling

| Situation | Response |
|---|---|
| File not found | Error: `FileNotFound` |
| File not valid UTF-8 | Error: `InvalidEncoding` |
| Unsupported `--expected` | Error: `UnsupportedLanguage` with supported list |
| total_classified_chars < 20 | result: `skipped` (no error) |

### Rust Implementation Notes

- No new crate dependencies. Rust `char` provides Unicode scalar values for range checks.
- New module: `core/src/language_validator.rs`
- Struct: `LanguageValidator` with `validate(file, expected, threshold) -> LanguageValidationResult`
- Export from `lib.rs`, add `ValidateLanguage` variant to `Commands` enum in `main.rs`
- Follow existing pattern: `thiserror::Error` for error types, `serde::Serialize` for output

## /validate SKILL Integration

### Phase 2e: Language Validation (new)

Insert after Phase 2d (DEVELOPERS.md existence check), before Phase 2.5 (Changed Spec Detection).

```
### 2e. Language Validation (conditional)

Read `Document language` from project root `## Instructions`.
Skip entirely if not configured.

For each CLAUDE.md target:
  $CLI_PATH validate-language \
    --file "$claude_md" \
    --expected "$document_language" \
    --threshold 70 \
    --output "${TMP_DIR}language-${dir_safe}.json"

  If DEVELOPERS.md exists:
    $CLI_PATH validate-language \
      --file "$developers_md" \
      --expected "$document_language" \
      --threshold 70 \
      --output "${TMP_DIR}language-dev-${dir_safe}.json"

Collect results:
  - result=pass → no issue
  - result=skipped → no issue
  - result=below_threshold → add to language_issues count,
    include non_target_lines in validator agent session file
```

### Session File Extension

When `result=below_threshold`, add to the validator agent session file:

```markdown
## Language Check
- file: src/auth/CLAUDE.md | expected: English | actual: 64.2% | non_target_lines: [4, 7, 12]
- file: src/auth/DEVELOPERS.md | expected: English | actual: 58.1% | non_target_lines: [3, 8]
```

Multiple files may appear when both CLAUDE.md and DEVELOPERS.md are below threshold.
If all language checks pass (or are skipped), this section is omitted. The agent skips language review when the section is absent.

### Unified Report Extension

Add `language_issues` to the Phase 4 summary:

```
total_violations = schema_errors + convention_issues + boundary_issues + semantic_drift + language_issues
```

Issue types:
- `LANGUAGE_BELOW_THRESHOLD` — deterministic (from CLI)
- `LANGUAGE_MISMATCH` — semantic (from agent, confirmed untranslated content)
- `LANGUAGE_ACCEPTABLE` — semantic (from agent, dismissed as legitimate)

### Auto Mode Exclusion

In `--report-only` mode (used by `/spec --auto`), `language_issues` are **excluded** from `total_violations` for the purpose of triggering spec retry. Language translation is a separate concern that auto-mode's spec update loop cannot address.

```
auto_mode_violations = schema_errors + convention_issues + boundary_issues + semantic_drift
# language_issues excluded — reported but does not trigger retry
```

## Validator Agent Extension

### 4. Document Language Drift (conditional)

Add as a new section in the validator agent, after existing checks:

```markdown
### 4. Document Language Drift (conditional)

Only executed when `## Language Check` section is present in the session file.

**Input**: `non_target_lines`, `expected` language, `file` path from session file.

**Process**:
1. Read only the `non_target_lines` from the original file
2. For each non-target line, classify content:
   - **Legitimate**: proper nouns, domain-specific terms, quoted foreign text, standard abbreviations, technical terms → dismiss
   - **Untranslated**: actual prose in a different language → flag

**Output**:
- Legitimate content only → issue type: `LANGUAGE_ACCEPTABLE` (dismissed, not counted)
- Any untranslated content → issue type: `LANGUAGE_MISMATCH` (WARNING severity)

**Evidence format**:
  [WARNING] LANGUAGE_MISMATCH
  - {file}:{line}: "{non-target text excerpt}" — expected {language}, found {detected}
```

## Simulations

### Simulation 1: Well-configured English project
- Input: All content in English, `Document language: English`
- CLI: Latin 95.2% ≥ 70% → PASS
- Result: No LLM invoked. Zero extra tokens.

### Simulation 2: Mixed content (untranslated Korean in English doc)
- Input: `Document language: English`, one Korean requirement line
- CLI: Latin 68.4% < 70% → BELOW_THRESHOLD, non_target_lines: [4]
- LLM: reads line 4 → "untranslated Korean requirement"
- Result: WARNING LANGUAGE_MISMATCH at line 4

### Simulation 3: Code blocks with Korean comments
- Input: `Document language: English`, Korean inside fenced code block
- CLI: Code block stripped. Latin 97.1% → PASS
- Result: Correct — code block content excluded

### Simulation 4: No Document language configured
- Input: No `Document language` in Instructions
- Result: Phase 2e skipped entirely. No validation, no false positives.

### Simulation 5: Japanese doc with CJK content
- Input: `Document language: Japanese`, Hiragana + Kanji content
- CLI: CJK+Hiragana+Katakana 89.2% → PASS
- Known limitation: Pure-CJK Chinese content would also pass Japanese check.

### Simulation 6: Domain terms in foreign script
- Input: `Document language: English`, occasional Japanese/Russian domain terms
- CLI: Latin 82.3% ≥ 70% → PASS
- Result: Small amounts of foreign domain terms don't trigger review.

### Simulation 7: Threshold boundary (exactly 70%)
- Input: Latin 70.0%
- CLI: 70.0 ≥ 70 → PASS (threshold is inclusive)

### Simulation 8: All sections "None"
- Input: `Document language: Korean`, only Purpose has Korean, rest is "None"
- CLI: heading lines (`## Purpose`, etc.) stripped entirely. "None" lines stripped entirely.
- Remaining: only Purpose body text. If >20 Hangul chars → Hangul ~100% → PASS
- Edge: If ALL sections "None" → all headings + None stripped → total_classified_chars ≈ 0 → SKIPPED

### Simulation 9: Markdown table-heavy doc
- Input: `Document language: Korean`, tables with Korean text and "JWT" acronym
- CLI: Heading lines stripped. `|` and `-` stripped. "None" stripped.
- Remaining: "항목", "설명", "인증", "JWT", "기반"
- "JWT" = 3 Latin, rest = 6 Hangul. Hangul 66.7% < 70% → BELOW_THRESHOLD
- LLM review: "JWT is a technical abbreviation" → LANGUAGE_ACCEPTABLE
- For real-sized docs, much more Korean content → well above threshold.

## Invariants

### INV-6: Language Validation Opt-in
```
validate-language runs IFF Document language ∈ project root ## Instructions
No Document language → no validation (zero false positives for unconfigured projects)
```

### INV-7: Two-Tier Separation
```
Tier 1 (CLI): deterministic character counting, no LLM tokens
Tier 2 (LLM): only triggered when CLI result = below_threshold
```

## Non-Goals

- **Auto-fix**: Language translation is too complex for auto-fix. Report only.
- **Per-section language**: All sections must be in the same language. Mixed-language documents are not supported.
- **NLP-based detection**: No natural language processing. Character ranges only for Tier 1.
- **Non-document files**: Only validates CLAUDE.md and DEVELOPERS.md, not source code comments.
