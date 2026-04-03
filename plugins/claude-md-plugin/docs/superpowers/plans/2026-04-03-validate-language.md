# validate-language Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two-tier document language validation — deterministic CLI character counting (Tier 1) with LLM semantic review fallback (Tier 2) — integrated into /validate workflow.

**Architecture:** New `language_validator.rs` Rust module with Unicode script detection and markdown stripping. CLI subcommand `validate-language` outputs JSON. /validate SKILL calls it as Phase 2e. Validator agent handles below-threshold cases via `## Language Check` session file section.

**Tech Stack:** Rust (no new crate dependencies — stdlib Unicode ranges + existing regex), Cucumber for acceptance tests, markdown skill/agent definitions.

---

### Task 1: Acceptance Test — Gherkin Feature File

**Files:**
- Create: `core/tests/features/language_validator.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
Feature: Language Validation
  As a developer maintaining CLAUDE.md files
  I want to validate that documents are written in the declared language
  So that document language consistency is enforced

  Background:
    Given a clean test directory

  Scenario: English document passes English validation
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides JWT token-based authentication to verify user identity.

      ## Requirements

      - Valid JWT tokens pass through with decoded user information
      - Expired tokens return a 401 Unauthorized error

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 90

  Scenario: Korean document passes Korean validation
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      사용자 인증을 위한 JWT 토큰 기반 인증을 제공합니다.

      ## Requirements

      - 유효한 JWT 토큰이 포함된 요청은 디코딩된 사용자 정보와 함께 통과
      - 만료된 토큰은 401 Unauthorized 에러를 반환

      ## Domain Context

      None
      """
    When I validate language with expected "Korean" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 70

  Scenario: Korean content in English-expected document fails
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides authentication.

      ## Requirements

      - 유효한 JWT 토큰이 포함된 요청은 통과
      - 만료된 토큰은 401 에러를 반환
      - 토큰 서명이 유효하지 않으면 거부

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "below_threshold"
    And non target lines should not be empty

  Scenario: Code blocks are excluded from character counting
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Handles user authentication for API requests.

      ## Requirements

      - Users authenticate via JWT tokens
      - Invalid tokens are rejected

      ## Domain Context

      ```typescript
      // 인증 미들웨어 설정
      const middleware = createAuthMiddleware();
      ```
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "pass"

  Scenario: Heading lines are stripped from counting
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      사용자 인증 처리 모듈입니다.

      ## Requirements

      - JWT 토큰 검증 기능 제공
      - 만료 토큰 거부 처리

      ## Domain Context

      None
      """
    When I validate language with expected "Korean" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 85

  Scenario: Insufficient content is skipped
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Auth

      ## Requirements

      None

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "skipped"

  Scenario: Unsupported language returns error
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Test module.

      ## Requirements

      None

      ## Domain Context

      None
      """
    When I validate language with expected "French" and threshold 70
    Then language validation should fail with "UnsupportedLanguage"

  Scenario: Threshold boundary — exactly 70% passes
    Given a markdown file "CLAUDE.md" with content at exactly 70 percent Latin
    When I validate language with expected "English" and threshold 70
    Then language result should be "pass"

  Scenario: Non-target line detection uses 50% per-line rule
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides authentication services for the platform.

      ## Requirements

      - Complies with 개인정보보호법 regulation for data handling
      - 유효한 JWT 토큰이 포함된 요청은 디코딩된 사용자 정보와 함께 통과합니다

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then non target lines should contain line 10
    And non target lines should not contain line 8

  Scenario: Script distribution is reported correctly
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides JWT authentication for API requests.

      ## Requirements

      - Valid tokens pass through
      - 만료된 토큰은 거부

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then script distribution should contain "Latin"
    And script distribution should contain "Hangul"

  Scenario: Japanese document with Hiragana and Kanji passes
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      ユーザー認証を処理するモジュールです。

      ## Requirements

      - 有効なトークンは通過する
      - 期限切れトークンは拒否される

      ## Domain Context

      None
      """
    When I validate language with expected "Japanese" and threshold 70
    Then language result should be "pass"

  Scenario: None markers are stripped from counting
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      사용자 인증을 위한 모듈입니다. 이 모듈은 JWT 토큰을 검증합니다.

      ## Requirements

      None

      ## Domain Context

      None
      """
    When I validate language with expected "Korean" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 90
```

- [ ] **Step 2: Commit**

```bash
git add core/tests/features/language_validator.feature
git commit -m "test: add language_validator.feature — 12 acceptance scenarios"
```

---

### Task 2: Rust Module — LanguageValidator Core

**Files:**
- Create: `core/src/language_validator.rs`

- [ ] **Step 1: Write unit tests at the bottom of the module**

```rust
// core/src/language_validator.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LanguageValidatorError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Invalid encoding (not UTF-8): {0}")]
    InvalidEncoding(String),
    #[error("Unsupported language: '{0}'. Supported: English, Korean, Japanese, Chinese")]
    UnsupportedLanguage(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Script {
    Latin,
    Hangul,
    Cjk,
    Hiragana,
    Katakana,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageValidationResult {
    pub file: String,
    pub expected_language: String,
    pub expected_script: String,
    pub threshold: u32,
    pub result: String, // "pass" | "below_threshold" | "skipped"
    pub target_percentage: f64,
    pub script_distribution: HashMap<String, f64>,
    pub total_classified_chars: usize,
    pub non_target_lines: Vec<usize>,
}

pub struct LanguageValidator;

impl LanguageValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate that a file's content matches the expected language.
    pub fn validate(
        &self,
        file: &Path,
        expected: &str,
        threshold: u32,
    ) -> Result<LanguageValidationResult, LanguageValidatorError> {
        let content = std::fs::read_to_string(file)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    LanguageValidatorError::FileNotFound(file.display().to_string())
                } else {
                    LanguageValidatorError::InvalidEncoding(file.display().to_string())
                }
            })?;

        self.validate_content(&content, file.display().to_string(), expected, threshold)
    }

    /// Validate content string directly (for testing without files).
    pub fn validate_content(
        &self,
        content: &str,
        file_name: String,
        expected: &str,
        threshold: u32,
    ) -> Result<LanguageValidationResult, LanguageValidatorError> {
        let target_scripts = Self::language_to_scripts(expected)?;
        let script_label = Self::language_to_script_label(expected);

        let stripped_lines = Self::strip_markdown(content);

        // Count scripts per line and overall
        let mut script_counts: HashMap<&str, usize> = HashMap::new();
        let mut total_classified: usize = 0;
        let mut non_target_lines: Vec<usize> = Vec::new();

        for (original_line_num, line_content) in &stripped_lines {
            let mut line_target = 0usize;
            let mut line_non_target = 0usize;
            let mut line_counts: HashMap<&str, usize> = HashMap::new();

            for ch in line_content.chars() {
                if let Some(script) = Self::classify_char(ch) {
                    let label = Self::script_label(script);
                    *line_counts.entry(label).or_insert(0) += 1;

                    if target_scripts.contains(&script) {
                        line_target += 1;
                    } else {
                        line_non_target += 1;
                    }
                }
                // Neutral characters (digits, punctuation, whitespace) are skipped
            }

            let line_total = line_target + line_non_target;
            if line_total > 0 {
                // Per-line >50% rule for non_target_lines
                let non_target_pct = (line_non_target as f64 / line_total as f64) * 100.0;
                if non_target_pct > 50.0 {
                    non_target_lines.push(*original_line_num);
                }
            }

            for (label, count) in line_counts {
                *script_counts.entry(label).or_insert(0) += count;
            }
            total_classified += line_target + line_non_target;
        }

        // Skip if insufficient content
        if total_classified < 20 {
            return Ok(LanguageValidationResult {
                file: file_name,
                expected_language: expected.to_string(),
                expected_script: script_label.to_string(),
                threshold,
                result: "skipped".to_string(),
                target_percentage: 0.0,
                script_distribution: HashMap::new(),
                total_classified_chars: total_classified,
                non_target_lines: vec![],
            });
        }

        // Calculate distribution
        let mut distribution: HashMap<String, f64> = HashMap::new();
        let mut target_count: usize = 0;

        for (label, count) in &script_counts {
            let pct = (*count as f64 / total_classified as f64) * 100.0;
            let pct_rounded = (pct * 10.0).round() / 10.0;
            distribution.insert(label.to_string(), pct_rounded);

            // Check if this label belongs to target scripts
            if target_scripts.iter().any(|s| Self::script_label(*s) == *label) {
                target_count += *count;
            }
        }

        // Add "Other" for unaccounted
        let accounted: usize = script_counts.values().sum();
        if accounted < total_classified {
            let other_pct = ((total_classified - accounted) as f64 / total_classified as f64) * 100.0;
            distribution.insert("Other".to_string(), (other_pct * 10.0).round() / 10.0);
        }

        let target_pct = (target_count as f64 / total_classified as f64) * 100.0;
        let target_pct_rounded = (target_pct * 10.0).round() / 10.0;

        let result_str = if target_pct_rounded >= threshold as f64 {
            "pass"
        } else {
            "below_threshold"
        };

        Ok(LanguageValidationResult {
            file: file_name,
            expected_language: expected.to_string(),
            expected_script: script_label.to_string(),
            threshold,
            result: result_str.to_string(),
            target_percentage: target_pct_rounded,
            script_distribution: distribution,
            total_classified_chars: total_classified,
            non_target_lines,
        })
    }

    fn language_to_scripts(language: &str) -> Result<Vec<Script>, LanguageValidatorError> {
        match language.to_lowercase().as_str() {
            "english" => Ok(vec![Script::Latin]),
            "korean" => Ok(vec![Script::Hangul]),
            "japanese" => Ok(vec![Script::Hiragana, Script::Katakana, Script::Cjk]),
            "chinese" => Ok(vec![Script::Cjk]),
            _ => Err(LanguageValidatorError::UnsupportedLanguage(language.to_string())),
        }
    }

    fn language_to_script_label(language: &str) -> &'static str {
        match language.to_lowercase().as_str() {
            "english" => "Latin",
            "korean" => "Hangul",
            "japanese" => "Hiragana+Katakana+CJK",
            "chinese" => "CJK",
            _ => "Unknown",
        }
    }

    fn classify_char(ch: char) -> Option<Script> {
        let cp = ch as u32;
        match cp {
            // Latin: Basic Latin letters + Latin Extended
            0x0041..=0x024F => Some(Script::Latin),
            // Hangul Jamo
            0x1100..=0x11FF => Some(Script::Hangul),
            // Hangul Compatibility Jamo
            0x3130..=0x318F => Some(Script::Hangul),
            // Hiragana
            0x3040..=0x309F => Some(Script::Hiragana),
            // Katakana
            0x30A0..=0x30FF => Some(Script::Katakana),
            // CJK Unified Ideographs
            0x4E00..=0x9FFF => Some(Script::Cjk),
            // Hangul Syllables
            0xAC00..=0xD7AF => Some(Script::Hangul),
            // Everything else: neutral (digits, punctuation, whitespace, symbols)
            _ => None,
        }
    }

    fn script_label(script: Script) -> &'static str {
        match script {
            Script::Latin => "Latin",
            Script::Hangul => "Hangul",
            Script::Cjk => "CJK",
            Script::Hiragana => "Hiragana",
            Script::Katakana => "Katakana",
        }
    }

    /// Strip markdown non-prose content, return (original_line_number_1indexed, stripped_content) pairs.
    fn strip_markdown(content: &str) -> Vec<(usize, String)> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result: Vec<(usize, String)> = Vec::new();
        let mut in_code_block = false;

        let url_re = regex::Regex::new(r"https?://\S+").unwrap();
        let abs_path_re = regex::Regex::new(r"/[\w/.\-]+").unwrap();
        let rel_path_re = regex::Regex::new(r"\./[\w/.\-]+").unwrap();
        let inline_code_re = regex::Regex::new(r"`[^`]+`").unwrap();
        let list_marker_re = regex::Regex::new(r"^(\s*)([-*+]|\d+\.)\s+").unwrap();
        let table_separator_re = regex::Regex::new(r"^\s*\|?[\s\-:|]+\|?\s*$").unwrap();

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1; // 1-indexed
            let trimmed = line.trim();

            // Toggle code block state
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                continue;
            }

            // Strip heading lines entirely
            if trimmed.starts_with('#') {
                continue;
            }

            // Strip None marker lines
            if trimmed == "None" || trimmed == "N/A" || trimmed == "none" || trimmed == "n/a" {
                continue;
            }

            // Strip table separator rows
            if table_separator_re.is_match(trimmed) {
                continue;
            }

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            let mut processed = line.to_string();

            // Remove inline code
            processed = inline_code_re.replace_all(&processed, "").to_string();
            // Remove URLs
            processed = url_re.replace_all(&processed, "").to_string();
            // Remove absolute paths
            processed = abs_path_re.replace_all(&processed, "").to_string();
            // Remove relative paths
            processed = rel_path_re.replace_all(&processed, "").to_string();
            // Strip list markers (preserve content after marker)
            processed = list_marker_re.replace(&processed, "").to_string();
            // Remove blockquote markers
            if processed.trim_start().starts_with('>') {
                processed = processed.trim_start().strip_prefix('>').unwrap_or(&processed).to_string();
            }
            // Remove table pipe characters
            processed = processed.replace('|', "");

            let final_trimmed = processed.trim();
            if !final_trimmed.is_empty() {
                result.push((line_num, final_trimmed.to_string()));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_latin() {
        assert_eq!(LanguageValidator::classify_char('A'), Some(Script::Latin));
        assert_eq!(LanguageValidator::classify_char('z'), Some(Script::Latin));
        assert_eq!(LanguageValidator::classify_char('é'), Some(Script::Latin));
    }

    #[test]
    fn test_classify_hangul() {
        assert_eq!(LanguageValidator::classify_char('가'), Some(Script::Hangul));
        assert_eq!(LanguageValidator::classify_char('힣'), Some(Script::Hangul));
    }

    #[test]
    fn test_classify_cjk() {
        assert_eq!(LanguageValidator::classify_char('漢'), Some(Script::Cjk));
        assert_eq!(LanguageValidator::classify_char('字'), Some(Script::Cjk));
    }

    #[test]
    fn test_classify_hiragana() {
        assert_eq!(LanguageValidator::classify_char('あ'), Some(Script::Hiragana));
    }

    #[test]
    fn test_classify_katakana() {
        assert_eq!(LanguageValidator::classify_char('ア'), Some(Script::Katakana));
    }

    #[test]
    fn test_classify_neutral() {
        assert_eq!(LanguageValidator::classify_char('1'), None);
        assert_eq!(LanguageValidator::classify_char(' '), None);
        assert_eq!(LanguageValidator::classify_char('.'), None);
        assert_eq!(LanguageValidator::classify_char('-'), None);
    }

    #[test]
    fn test_language_to_scripts_english() {
        let scripts = LanguageValidator::language_to_scripts("English").unwrap();
        assert_eq!(scripts, vec![Script::Latin]);
    }

    #[test]
    fn test_language_to_scripts_korean() {
        let scripts = LanguageValidator::language_to_scripts("Korean").unwrap();
        assert_eq!(scripts, vec![Script::Hangul]);
    }

    #[test]
    fn test_language_to_scripts_japanese() {
        let scripts = LanguageValidator::language_to_scripts("Japanese").unwrap();
        assert_eq!(scripts, vec![Script::Hiragana, Script::Katakana, Script::Cjk]);
    }

    #[test]
    fn test_language_to_scripts_unsupported() {
        let result = LanguageValidator::language_to_scripts("French");
        assert!(result.is_err());
    }

    #[test]
    fn test_language_case_insensitive() {
        assert!(LanguageValidator::language_to_scripts("english").is_ok());
        assert!(LanguageValidator::language_to_scripts("KOREAN").is_ok());
    }

    #[test]
    fn test_strip_markdown_code_blocks() {
        let content = "Hello world\n```\n한국어 코드\n```\nMore text";
        let result = LanguageValidator::strip_markdown(content);
        let texts: Vec<&str> = result.iter().map(|(_, s)| s.as_str()).collect();
        assert!(texts.contains(&"Hello world"));
        assert!(texts.contains(&"More text"));
        assert!(!texts.iter().any(|t| t.contains("한국어")));
    }

    #[test]
    fn test_strip_markdown_headings() {
        let content = "## Purpose\nSome content\n### Sub\nMore content";
        let result = LanguageValidator::strip_markdown(content);
        let texts: Vec<&str> = result.iter().map(|(_, s)| s.as_str()).collect();
        assert!(!texts.iter().any(|t| t.contains("Purpose")));
        assert!(!texts.iter().any(|t| t.contains("Sub")));
        assert!(texts.contains(&"Some content"));
        assert!(texts.contains(&"More content"));
    }

    #[test]
    fn test_strip_markdown_none_markers() {
        let content = "Content here\nNone\nMore content";
        let result = LanguageValidator::strip_markdown(content);
        let texts: Vec<&str> = result.iter().map(|(_, s)| s.as_str()).collect();
        assert!(!texts.iter().any(|t| *t == "None"));
        assert_eq!(texts.len(), 2);
    }

    #[test]
    fn test_strip_markdown_inline_code() {
        let content = "Use `authenticate()` function";
        let result = LanguageValidator::strip_markdown(content);
        assert_eq!(result.len(), 1);
        assert!(!result[0].1.contains("authenticate"));
    }

    #[test]
    fn test_strip_markdown_urls() {
        let content = "See https://example.com/path for details";
        let result = LanguageValidator::strip_markdown(content);
        assert_eq!(result.len(), 1);
        assert!(!result[0].1.contains("https"));
    }

    #[test]
    fn test_strip_markdown_preserves_line_numbers() {
        let content = "## Heading\n\nContent on line 3\n\nContent on line 5";
        let result = LanguageValidator::strip_markdown(content);
        assert_eq!(result[0].0, 3); // line 3 (1-indexed)
        assert_eq!(result[1].0, 5); // line 5
    }

    #[test]
    fn test_validate_content_english_pass() {
        let v = LanguageValidator::new();
        let content = "## Purpose\n\nProvides JWT token-based authentication to verify user identity for API requests. This module handles all authentication logic.\n\n## Requirements\n\n- Valid JWT tokens pass through with decoded user information\n- Expired tokens return a 401 Unauthorized error\n- Invalid signatures are rejected\n\n## Domain Context\n\nNone";
        let result = v.validate_content(content, "test.md".to_string(), "English", 70).unwrap();
        assert_eq!(result.result, "pass");
        assert!(result.target_percentage > 90.0);
    }

    #[test]
    fn test_validate_content_skipped_insufficient() {
        let v = LanguageValidator::new();
        let content = "## Purpose\n\nAuth\n\n## Requirements\n\nNone\n\n## Domain Context\n\nNone";
        let result = v.validate_content(content, "test.md".to_string(), "English", 70).unwrap();
        assert_eq!(result.result, "skipped");
    }

    #[test]
    fn test_validate_content_below_threshold() {
        let v = LanguageValidator::new();
        let content = "## Purpose\n\nProvides auth.\n\n## Requirements\n\n- 유효한 JWT 토큰이 포함된 요청은 통과합니다\n- 만료된 토큰은 401 에러를 반환합니다\n- 토큰 서명이 유효하지 않으면 거부됩니다\n\n## Domain Context\n\nNone";
        let result = v.validate_content(content, "test.md".to_string(), "English", 70).unwrap();
        assert_eq!(result.result, "below_threshold");
        assert!(!result.non_target_lines.is_empty());
    }

    #[test]
    fn test_non_target_line_50_percent_rule() {
        let v = LanguageValidator::new();
        // Line with mostly English + one Korean term: should NOT be non-target
        // Line with mostly Korean: should be non-target
        let content = "## Purpose\n\nProvides authentication services for the platform and users.\n\n## Requirements\n\n- Valid tokens authenticate properly in the system\n- Complies with regulation for data handling\n- 유효한 JWT 토큰이 포함된 요청은 디코딩된 사용자 정보와 함께 통과합니다\n\n## Domain Context\n\nNone";
        let result = v.validate_content(content, "test.md".to_string(), "English", 70).unwrap();
        // Line 9 is the Korean line — should be in non_target_lines
        assert!(result.non_target_lines.contains(&9));
        // Line 7 and 8 are English — should NOT be in non_target_lines
        assert!(!result.non_target_lines.contains(&7));
        assert!(!result.non_target_lines.contains(&8));
    }

    #[test]
    fn test_threshold_boundary_inclusive() {
        let v = LanguageValidator::new();
        // Build content that is ~70% Latin
        let content = "## Purpose\n\nThis is English text for testing purpose yes.\n\n## Requirements\n\n- 한국어 텍스트 추가\n- More English words here now today\n\n## Domain Context\n\nNone";
        let result = v.validate_content(content, "test.md".to_string(), "English", 70).unwrap();
        // The exact percentage depends on char count, but test that >= threshold means "pass"
        if result.target_percentage >= 70.0 {
            assert_eq!(result.result, "pass");
        } else {
            assert_eq!(result.result, "below_threshold");
        }
    }
}
```

- [ ] **Step 2: Run unit tests to verify they fail (RED)**

```bash
cd core && cargo test --lib language_validator -- --nocapture 2>&1 | head -30
```

Expected: Compilation succeeds, all tests run. Verify the implementation works as written (since we included it inline).

- [ ] **Step 3: Verify all unit tests pass (GREEN)**

```bash
cd core && cargo test --lib language_validator -- --nocapture
```

Expected: All 20 unit tests pass.

- [ ] **Step 4: Commit**

```bash
git add core/src/language_validator.rs
git commit -m "feat: add language_validator.rs — Unicode script-based document language validation"
```

---

### Task 3: Register CLI Subcommand

**Files:**
- Modify: `core/src/lib.rs`
- Modify: `core/src/main.rs`

- [ ] **Step 1: Export module from lib.rs**

Add to `core/src/lib.rs` after line 71 (`pub mod spec_diff;`):

```rust
pub mod language_validator;
```

Add to the `pub use` block after line 78:

```rust
pub use language_validator::LanguageValidator;
```

- [ ] **Step 2: Add Commands variant to main.rs**

Add to the `Commands` enum in `core/src/main.rs` (before the closing `}`):

```rust
    /// Validate document language consistency
    ValidateLanguage {
        /// File to validate (CLAUDE.md or DEVELOPERS.md)
        #[arg(short, long)]
        file: PathBuf,

        /// Expected language (English, Korean, Japanese, Chinese)
        #[arg(short, long)]
        expected: String,

        /// Minimum target percentage (default: 70)
        #[arg(short, long, default_value_t = 70)]
        threshold: u32,

        /// Output JSON file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
```

- [ ] **Step 3: Add match arm in main()**

Add before the closing `};` of the main match block:

```rust
        Commands::ValidateLanguage { file, expected, threshold, output } => {
            let validator = claude_md_core::LanguageValidator::new();
            match validator.validate(file, expected, *threshold) {
                Ok(result) => output_result(&result, output.as_ref(), "validate-language"),
                Err(e) => Err(e.to_string().into()),
            }
        }
```

- [ ] **Step 4: Add error handler entry**

Add to the command_name match in the error handler:

```rust
            Commands::ValidateLanguage { .. } => "validate-language",
```

- [ ] **Step 5: Add import**

Add to the imports at the top of main.rs:

```rust
use claude_md_core::language_validator; // for error type access
```

- [ ] **Step 6: Build and verify**

```bash
cd core && cargo build 2>&1 | tail -5
```

Expected: Build succeeds.

```bash
cd core && cargo run -- validate-language --help
```

Expected: Shows help with `--file`, `--expected`, `--threshold`, `--output` options.

- [ ] **Step 7: Commit**

```bash
git add core/src/lib.rs core/src/main.rs
git commit -m "feat: register validate-language CLI subcommand"
```

---

### Task 4: Cucumber Step Definitions

**Files:**
- Modify: `core/tests/cucumber.rs`

- [ ] **Step 1: Add LanguageValidationResult to TestWorld**

Add to the `TestWorld` struct:

```rust
    // Language validator fields
    language_result: Option<LanguageValidationResult>,
    language_error: Option<String>,
```

Add import at top:

```rust
use claude_md_core::language_validator::{LanguageValidator, LanguageValidationResult};
```

- [ ] **Step 2: Write step definitions**

Add step definitions at the end of cucumber.rs (before the `main` function):

```rust
// ============== Language Validator Steps ==============

#[given(expr = "a markdown file {string} with content:")]
fn create_markdown_file(world: &mut TestWorld, filename: String, step: &cucumber::gherkin::Step) {
    let content = step.docstring().expect("Expected docstring content");
    let dir = world.temp_dir.as_ref().expect("Need temp dir");
    let file_path = dir.path().join(&filename);
    fs::write(&file_path, content).expect("Failed to write markdown file");
}

#[given("a markdown file {string} with content at exactly 70 percent Latin")]
fn create_70_percent_latin_file(world: &mut TestWorld, filename: String) {
    // 70 Latin chars + 30 Hangul chars (after stripping headings/None)
    // Each Hangul syllable is 1 char, each Latin letter is 1 char
    let content = "## Purpose\n\nThis is a test document with enough English text to reach seventy percent of all content chars\n\n## Requirements\n\n- 한국어 요구사항 텍스트를 여기에 작성합니다 추가합니다\n\n## Domain Context\n\nNone";
    let dir = world.temp_dir.as_ref().expect("Need temp dir");
    let file_path = dir.path().join(&filename);
    fs::write(&file_path, content).expect("Failed to write markdown file");
}

#[when(expr = "I validate language with expected {string} and threshold {int}")]
fn validate_language(world: &mut TestWorld, expected: String, threshold: i32) {
    let dir = world.temp_dir.as_ref().expect("Need temp dir");
    let file_path = dir.path().join("CLAUDE.md");
    let validator = LanguageValidator::new();
    match validator.validate(&file_path, &expected, threshold as u32) {
        Ok(result) => {
            world.language_result = Some(result);
            world.language_error = None;
        }
        Err(e) => {
            world.language_result = None;
            world.language_error = Some(e.to_string());
        }
    }
}

#[then(expr = "language result should be {string}")]
fn check_language_result(world: &mut TestWorld, expected_result: String) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert_eq!(result.result, expected_result,
        "Expected result '{}', got '{}' (percentage: {:.1}%, chars: {})",
        expected_result, result.result, result.target_percentage, result.total_classified_chars);
}

#[then(expr = "target percentage should be greater than {int}")]
fn check_target_percentage(world: &mut TestWorld, min_pct: i32) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(result.target_percentage > min_pct as f64,
        "Expected percentage > {}, got {:.1}", min_pct, result.target_percentage);
}

#[then("non target lines should not be empty")]
fn check_non_target_lines_not_empty(world: &mut TestWorld) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(!result.non_target_lines.is_empty(),
        "Expected non-target lines to be non-empty, got empty");
}

#[then(expr = "non target lines should contain line {int}")]
fn check_non_target_contains_line(world: &mut TestWorld, line: i32) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(result.non_target_lines.contains(&(line as usize)),
        "Expected non_target_lines to contain {}, got {:?}", line, result.non_target_lines);
}

#[then(expr = "non target lines should not contain line {int}")]
fn check_non_target_not_contains_line(world: &mut TestWorld, line: i32) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(!result.non_target_lines.contains(&(line as usize)),
        "Expected non_target_lines to NOT contain {}, got {:?}", line, result.non_target_lines);
}

#[then(expr = "language validation should fail with {string}")]
fn check_language_error(world: &mut TestWorld, error_type: String) {
    let error = world.language_error.as_ref().expect("Expected language error");
    assert!(error.contains(&error_type),
        "Expected error containing '{}', got '{}'", error_type, error);
}

#[then(expr = "script distribution should contain {string}")]
fn check_script_distribution_contains(world: &mut TestWorld, script: String) {
    let result = world.language_result.as_ref().expect("Expected language result");
    assert!(result.script_distribution.contains_key(&script),
        "Expected script distribution to contain '{}', got {:?}", script, result.script_distribution);
}
```

- [ ] **Step 3: Run cucumber tests**

```bash
cd core && cargo test --test cucumber -- core/tests/features/language_validator.feature 2>&1 | tail -20
```

Expected: All 12 scenarios pass. Some may need tuning of the test fixture content to hit exact thresholds.

- [ ] **Step 4: Fix any failing scenarios**

If thresholds are off, adjust the fixture content in the `.feature` file or the `create_70_percent_latin_file` helper to produce the expected percentages.

- [ ] **Step 5: Commit**

```bash
git add core/tests/cucumber.rs
git commit -m "test: add cucumber step definitions for language_validator"
```

---

### Task 5: /validate SKILL — Phase 2e Integration

**Files:**
- Modify: `skills/validate/SKILL.md`

- [ ] **Step 1: Add document language reading to Phase 1**

Insert after the existing Phase 1 initialization, before Phase 2:

Find the line after Phase 1 initialization and before `### 2a`. Add:

```markdown
### 1.5 Read Document language

Read `## Instructions` from project root CLAUDE.md. Extract the `Document language` value.
If not found, set `document_language` to empty (Phase 2e will be skipped).
```

- [ ] **Step 2: Insert Phase 2e after 2d**

Find `### 2.5 Build changed spec + test coverage map` and insert before it:

```markdown
#### 2e. Language validation (conditional)

**Skip entirely** if `document_language` is empty (not configured in Instructions).

For each CLAUDE.md target:
```bash
$CLI_PATH validate-language \
  --file "$claude_md" \
  --expected "$document_language" \
  --threshold 70 \
  --output "${TMP_DIR}language-${dir_safe}.json"
```

If DEVELOPERS.md exists:
```bash
$CLI_PATH validate-language \
  --file "$developers_md" \
  --expected "$document_language" \
  --threshold 70 \
  --output "${TMP_DIR}language-dev-${dir_safe}.json"
```

Collect results:
- `result=pass` → no issue
- `result=skipped` → no issue
- `result=below_threshold` → increment `language_issues` count, include in session file `## Language Check`
```

- [ ] **Step 3: Update session file template in Phase 3**

Add to the session file template (after `## Test Coverage Map`):

```markdown
## Language Check
{Only present when at least one file has result=below_threshold}
- file: {path} | expected: {language} | actual: {percentage}% | non_target_lines: [{line_nums}]
{repeat for each below_threshold file}
```

- [ ] **Step 4: Update consolidated report in Phase 5**

Add `language_issues` to the result block:

```markdown
---validate-result---
status: clean | issues_found | fixed
total_modules: {n}
schema_errors: {n}
convention_issues: {n}
boundary_issues: {n}
language_issues: {n}
semantic_drift: {n}
auto_fixed: {n}
result_files:
  - ...
---end-validate-result---
```

- [ ] **Step 5: Add auto-mode exclusion note**

In the consolidated report section, add:

```markdown
> **Auto mode note:** `language_issues` are excluded from `total_violations` when computing
> whether to trigger spec retry in auto mode. Language translation cannot be addressed by
> the auto-mode spec update loop.
```

- [ ] **Step 6: Commit**

```bash
git add skills/validate/SKILL.md
git commit -m "feat: add Phase 2e language validation to /validate SKILL"
```

---

### Task 6: Validator Agent — Language Drift Section

**Files:**
- Modify: `agents/validator.md`

- [ ] **Step 1: Add Section 5 (renumber existing Section 5 to 6)**

Find `### 4. DEVELOPERS.md Content Drift (strict only)` and its content. After it, find `### 5. Result`. Rename `### 5. Result` to `### 6. Result`. Then insert before it:

```markdown
### 5. Document Language Drift (conditional)

Only executed when `## Language Check` section is present in the session file.

**Input**: Parse the `## Language Check` section — extract `file`, `expected`, `actual`, `non_target_lines` for each entry.

**Process**:
1. For each file in the Language Check list:
   - Read only the `non_target_lines` from the original file
   - For each non-target line, classify content:
     - **Legitimate**: proper nouns, domain-specific terms (law names, protocol names), quoted foreign text, standard abbreviations, technical terms → dismiss
     - **Untranslated**: actual prose (full sentences, requirement descriptions) in a different language → flag

**Output**:
- Legitimate content only → issue type: `LANGUAGE_ACCEPTABLE` (not counted in issues)
- Any untranslated content → issue type: `LANGUAGE_MISMATCH` (WARNING severity)

**Evidence format**:
```
### [WARNING] LANGUAGE_MISMATCH
- {file}:{line}: "{non-target text excerpt (max 80 chars)}" — expected {language}
```

| Drift Type | Description | Severity |
|-----------|-------------|----------|
| LANGUAGE_MISMATCH | Document content in unexpected language | WARNING |
| LANGUAGE_ACCEPTABLE | Non-target script is legitimate (domain terms, proper nouns) | (dismissed) |
```

- [ ] **Step 2: Update Result section issue examples**

Add to the issue examples in the Result section (now Section 6):

```markdown
### [WARNING] LANGUAGE_MISMATCH
- {file}:{line}: "{text excerpt}" — expected {language}
```

- [ ] **Step 3: Commit**

```bash
git add agents/validator.md
git commit -m "feat: add Document Language Drift section to validator agent"
```

---

### Task 7: Reference Docs + CLAUDE.md + Version Bump

**Files:**
- Modify: `references/shared/claude-md-schema.md`
- Modify: `skills/validate/references/validator-templates.md`
- Modify: `CLAUDE.md`
- Modify: `.claude-plugin/plugin.json`

- [ ] **Step 1: Update validator-templates.md**

Read `skills/validate/references/validator-templates.md` and add a new drift type to the drift type table:

```markdown
| LANGUAGE_MISMATCH | Document content in unexpected language | WARNING | CLI below_threshold + agent confirms untranslated |
| LANGUAGE_ACCEPTABLE | Non-target script is legitimate | (dismissed) | CLI below_threshold + agent dismisses |
```

- [ ] **Step 2: Update CLAUDE.md**

Add to the CLI Subcommands table:

```markdown
| `validate-language` | Document language validation |
```

Add INV-6 and INV-7 to the Invariants section:

```markdown
### INV-6: Language Validation Opt-in
\```
validate-language runs IFF Document language ∈ project root ## Instructions
No Document language → no validation (zero false positives for unconfigured projects)
\```

### INV-7: Two-Tier Separation
\```
Tier 1 (CLI): deterministic character counting, no LLM tokens
Tier 2 (LLM): only triggered when CLI result = below_threshold
\```
```

- [ ] **Step 3: Bump version**

Update `.claude-plugin/plugin.json` version from current to next MINOR.

- [ ] **Step 4: Commit**

```bash
git add references/shared/claude-md-schema.md skills/validate/references/validator-templates.md CLAUDE.md .claude-plugin/plugin.json
git commit -m "docs: update references and CLAUDE.md for validate-language feature"
```

---

### Task 8: Integration Test — End-to-End CLI

**Files:**
- No new files — CLI manual verification

- [ ] **Step 1: Create test fixtures**

```bash
mkdir -p /tmp/validate-language-test
cat > /tmp/validate-language-test/english.md << 'EOF'
## Purpose

Provides JWT token-based authentication to verify user identity for API requests.

## Requirements

- Valid JWT tokens pass through with decoded user information
- Expired tokens return a 401 Unauthorized error
- Tokens with invalid signatures are rejected

## Domain Context

None
EOF

cat > /tmp/validate-language-test/mixed.md << 'EOF'
## Purpose

Provides authentication.

## Requirements

- 유효한 JWT 토큰이 포함된 요청은 통과
- 만료된 토큰은 401 에러를 반환
- 토큰 서명이 유효하지 않으면 거부

## Domain Context

None
EOF
```

- [ ] **Step 2: Run CLI against English doc**

```bash
cd core && cargo run -- validate-language --file /tmp/validate-language-test/english.md --expected English --threshold 70
```

Expected: JSON output with `"result": "pass"`, `target_percentage` > 90.

- [ ] **Step 3: Run CLI against mixed doc**

```bash
cd core && cargo run -- validate-language --file /tmp/validate-language-test/mixed.md --expected English --threshold 70
```

Expected: JSON output with `"result": "below_threshold"`, `non_target_lines` populated.

- [ ] **Step 4: Run full test suite**

```bash
cd core && cargo test 2>&1 | tail -10
```

Expected: All unit tests + cucumber tests pass. No regressions.

- [ ] **Step 5: Clean up**

```bash
rm -rf /tmp/validate-language-test
```

- [ ] **Step 6: Final commit (if any fixes were needed)**

```bash
git add -A
git commit -m "fix: address integration test findings for validate-language"
```
