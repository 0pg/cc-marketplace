use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during language validation
#[derive(Debug, Error)]
pub enum LanguageValidatorError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Invalid encoding in file: {0}")]
    InvalidEncoding(String),
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

/// Unicode script categories used for character classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Script {
    Latin,
    Hangul,
    Cjk,
    Hiragana,
    Katakana,
}

/// Result of language validation for a single file
#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageValidationResult {
    /// File that was validated
    pub file: String,
    /// Expected language (e.g. "English", "Korean")
    pub expected_language: String,
    /// Human-readable script label (e.g. "Latin", "Hangul")
    pub expected_script: String,
    /// Threshold percentage used
    pub threshold: f64,
    /// Outcome: "pass", "below_threshold", or "skipped"
    pub result: String,
    /// Actual target script percentage (0.0–100.0)
    pub target_percentage: f64,
    /// Distribution of classified chars by script
    pub script_distribution: HashMap<String, f64>,
    /// Total number of classified (non-neutral) chars
    pub total_classified_chars: usize,
    /// Lines where >50% of classified chars are non-target
    pub non_target_lines: Vec<usize>,
}

/// Core validator — Unicode script-based document language validation
pub struct LanguageValidator;

impl LanguageValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate a file on disk against the expected language / threshold
    pub fn validate(
        &self,
        file: &Path,
        expected_language: &str,
        threshold: f64,
    ) -> Result<LanguageValidationResult, LanguageValidatorError> {
        let file_str = file.to_string_lossy().to_string();
        let content = std::fs::read_to_string(file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LanguageValidatorError::FileNotFound(file_str.clone())
            } else {
                LanguageValidatorError::InvalidEncoding(file_str.clone())
            }
        })?;
        self.validate_content(&content, &file_str, expected_language, threshold)
    }

    /// Core algorithm — validate content string for the expected language
    pub fn validate_content(
        &self,
        content: &str,
        file_name: &str,
        expected_language: &str,
        threshold: f64,
    ) -> Result<LanguageValidationResult, LanguageValidatorError> {
        let target_scripts = Self::language_to_scripts(expected_language)?;
        let expected_script = Self::language_to_script_label(expected_language).to_string();

        let stripped_lines = Self::strip_markdown(content);

        let mut script_counts: HashMap<String, usize> = HashMap::new();
        let mut total_classified_chars: usize = 0;
        let mut target_chars: usize = 0;
        let mut non_target_lines: Vec<usize> = Vec::new();

        for (line_num, line_text) in &stripped_lines {
            let mut line_target = 0usize;
            let mut line_total = 0usize;

            for ch in line_text.chars() {
                if let Some(script) = Self::classify_char(ch) {
                    let label = Self::script_label(&script).to_string();
                    *script_counts.entry(label).or_insert(0) += 1;
                    total_classified_chars += 1;
                    line_total += 1;
                    if target_scripts.contains(&script) {
                        target_chars += 1;
                        line_target += 1;
                    }
                }
            }

            // If >50% of classified chars on this line are non-target, flag the line
            if line_total > 0 {
                let non_target = line_total - line_target;
                if non_target * 2 > line_total {
                    non_target_lines.push(*line_num);
                }
            }
        }

        // Convert counts to percentages
        let script_distribution: HashMap<String, f64> = if total_classified_chars > 0 {
            script_counts.iter().map(|(k, v)| {
                let pct = (*v as f64 / total_classified_chars as f64) * 100.0;
                (k.clone(), (pct * 10.0).round() / 10.0)
            }).collect()
        } else {
            HashMap::new()
        };

        // Insufficient content — skip
        if total_classified_chars < 20 {
            return Ok(LanguageValidationResult {
                file: file_name.to_string(),
                expected_language: expected_language.to_string(),
                expected_script,
                threshold,
                result: "skipped".to_string(),
                target_percentage: 0.0,
                script_distribution,
                total_classified_chars,
                non_target_lines,
            });
        }

        let target_percentage = (target_chars as f64 / total_classified_chars as f64) * 100.0;
        let target_percentage = (target_percentage * 10.0).round() / 10.0;
        let result = if target_percentage >= threshold {
            "pass".to_string()
        } else {
            "below_threshold".to_string()
        };

        Ok(LanguageValidationResult {
            file: file_name.to_string(),
            expected_language: expected_language.to_string(),
            expected_script,
            threshold,
            result,
            target_percentage,
            script_distribution,
            total_classified_chars,
            non_target_lines,
        })
    }

    /// Strip markdown syntax and return (1-indexed line number, stripped text) pairs.
    /// Lines that are entirely stripped (e.g. headings, separators) are omitted.
    pub fn strip_markdown(content: &str) -> Vec<(usize, String)> {
        let raw_lines: Vec<&str> = content.lines().collect();
        let mut result: Vec<(usize, String)> = Vec::new();

        let mut in_fenced_block = false;
        let mut fence_char: Option<char> = None;

        for (idx, raw) in raw_lines.iter().enumerate() {
            let line_num = idx + 1; // 1-indexed
            let trimmed = raw.trim();

            // Detect fenced code block open/close (``` or ~~~)
            if !in_fenced_block {
                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    let ch = trimmed.chars().next().unwrap();
                    in_fenced_block = true;
                    fence_char = Some(ch);
                    continue; // skip the fence line itself
                }
            } else {
                let fc = fence_char.unwrap_or('`');
                let fence_str: String = std::iter::repeat(fc).take(3).collect();
                if trimmed.starts_with(&fence_str) {
                    in_fenced_block = false;
                    fence_char = None;
                }
                continue; // skip lines inside fenced blocks (including close fence)
            }

            // Skip heading lines
            if trimmed.starts_with('#') {
                continue;
            }

            // Skip table separator rows (e.g. |---|---|)
            if Self::is_table_separator(trimmed) {
                continue;
            }

            // Strip blockquote markers (preserve text)
            let line = Self::strip_blockquote(trimmed);

            // Strip list markers (preserve text)
            let line = Self::strip_list_marker(&line);

            // Strip inline code
            let line = Self::strip_inline_code(&line);

            // Strip URLs
            let line = Self::strip_urls(&line);

            // Strip absolute/relative paths
            let line = Self::strip_paths(&line);

            // Strip pipe characters (table cell separators)
            let line = line.replace('|', " ");

            // Collapse whitespace
            let line = line.split_whitespace().collect::<Vec<_>>().join(" ");

            // Check if the line is a None/N/A marker — skip it
            if Self::is_none_na_marker(&line) {
                continue;
            }

            if line.is_empty() {
                continue;
            }

            result.push((line_num, line));
        }

        result
    }

    fn is_table_separator(s: &str) -> bool {
        // A table separator row consists only of |, -, :, and whitespace
        if s.is_empty() {
            return false;
        }
        s.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
            && s.contains('-')
    }

    fn is_none_na_marker(s: &str) -> bool {
        let lower = s.trim().to_lowercase();
        lower == "none" || lower == "n/a"
    }

    fn strip_blockquote(s: &str) -> String {
        let mut line = s;
        // Strip one or more leading '>' characters
        while line.starts_with('>') {
            line = line[1..].trim_start_matches(' ');
        }
        line.to_string()
    }

    fn strip_list_marker(s: &str) -> String {
        let trimmed = s.trim_start();
        // Unordered: - , * , +
        if let Some(rest) = trimmed.strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            return rest.to_string();
        }
        // Ordered: "N. "
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                return trimmed[dot_pos + 2..].to_string();
            }
        }
        s.to_string()
    }

    fn strip_inline_code(s: &str) -> String {
        // Remove content inside backtick spans
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '`' {
                // consume until closing backtick
                while let Some(inner) = chars.next() {
                    if inner == '`' {
                        break;
                    }
                }
                result.push(' ');
            } else {
                result.push(ch);
            }
        }
        result
    }

    fn strip_urls(s: &str) -> String {
        // Remove markdown links [text](url) and bare URLs
        // Pattern: replace (http...) or [text](url) with just text
        let mut result = s.to_string();

        // Markdown image/link: ![alt](url) or [text](url) — keep text, remove url
        let mut out = String::new();
        let chars: Vec<char> = result.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '[' {
                // Find matching ]
                let mut j = i + 1;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                if j < chars.len() && j + 1 < chars.len() && chars[j + 1] == '(' {
                    // Find closing )
                    let mut k = j + 2;
                    while k < chars.len() && chars[k] != ')' {
                        k += 1;
                    }
                    if k < chars.len() {
                        // Emit the link text
                        out.extend(chars[i + 1..j].iter());
                        i = k + 1;
                        continue;
                    }
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        result = out;

        // Bare URLs: http:// or https://
        let mut out2 = String::new();
        let mut remaining = result.as_str();
        while let Some(pos) = remaining.find("http://").or_else(|| remaining.find("https://")) {
            out2.push_str(&remaining[..pos]);
            // Skip to end of URL (next whitespace or end)
            let url_part = &remaining[pos..];
            let end = url_part.find(|c: char| c.is_whitespace()).unwrap_or(url_part.len());
            remaining = &url_part[end..];
        }
        out2.push_str(remaining);
        out2
    }

    fn strip_paths(s: &str) -> String {
        // Strip tokens that look like absolute (/foo/bar) or relative (./foo or ../foo) paths
        let tokens: Vec<&str> = s.split_whitespace().collect();
        let filtered: Vec<&str> = tokens.into_iter().filter(|t| {
            let t = t.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-');
            !(t.starts_with('/') || t.starts_with("./") || t.starts_with("../"))
        }).collect();
        filtered.join(" ")
    }

    /// Classify a Unicode character into a Script category, or None if neutral
    pub fn classify_char(ch: char) -> Option<Script> {
        let cp = ch as u32;
        match cp {
            // Latin: Basic Latin letters + Latin Extended
            0x0041..=0x024F => Some(Script::Latin),
            // Hangul Syllables
            0xAC00..=0xD7AF => Some(Script::Hangul),
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
            _ => None,
        }
    }

    /// Map language name to the set of Scripts considered "target" for that language
    pub fn language_to_scripts(language: &str) -> Result<Vec<Script>, LanguageValidatorError> {
        match language.to_lowercase().as_str() {
            "english" => Ok(vec![Script::Latin]),
            "korean" => Ok(vec![Script::Hangul]),
            "japanese" => Ok(vec![Script::Hiragana, Script::Katakana, Script::Cjk]),
            "chinese" => Ok(vec![Script::Cjk]),
            other => Err(LanguageValidatorError::UnsupportedLanguage(other.to_string())),
        }
    }

    /// Human-readable script label for a language (used in result output)
    pub fn language_to_script_label(language: &str) -> &str {
        match language.to_lowercase().as_str() {
            "english" => "Latin",
            "korean" => "Hangul",
            "japanese" => "Hiragana+Katakana+CJK",
            "chinese" => "CJK",
            _ => "Unknown",
        }
    }

    /// Display name for a Script variant
    pub fn script_label(script: &Script) -> &str {
        match script {
            Script::Latin => "Latin",
            Script::Hangul => "Hangul",
            Script::Cjk => "CJK",
            Script::Hiragana => "Hiragana",
            Script::Katakana => "Katakana",
        }
    }
}

impl Default for LanguageValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_char ──────────────────────────────────────────────────────────

    #[test]
    fn test_classify_latin() {
        assert_eq!(LanguageValidator::classify_char('A'), Some(Script::Latin));
        assert_eq!(LanguageValidator::classify_char('z'), Some(Script::Latin));
        assert_eq!(LanguageValidator::classify_char('é'), Some(Script::Latin)); // U+00E9
    }

    #[test]
    fn test_classify_hangul() {
        // Hangul syllable 가 U+AC00
        assert_eq!(LanguageValidator::classify_char('가'), Some(Script::Hangul));
        // Hangul Jamo ᄀ U+1100
        assert_eq!(LanguageValidator::classify_char('\u{1100}'), Some(Script::Hangul));
        // Hangul Compatibility Jamo ㄱ U+3131
        assert_eq!(LanguageValidator::classify_char('ㄱ'), Some(Script::Hangul));
    }

    #[test]
    fn test_classify_cjk() {
        // CJK 一 U+4E00
        assert_eq!(LanguageValidator::classify_char('一'), Some(Script::Cjk));
        assert_eq!(LanguageValidator::classify_char('中'), Some(Script::Cjk));
    }

    #[test]
    fn test_classify_hiragana() {
        // Hiragana あ U+3042
        assert_eq!(LanguageValidator::classify_char('あ'), Some(Script::Hiragana));
    }

    #[test]
    fn test_classify_katakana() {
        // Katakana ア U+30A2
        assert_eq!(LanguageValidator::classify_char('ア'), Some(Script::Katakana));
    }

    #[test]
    fn test_classify_neutral() {
        assert_eq!(LanguageValidator::classify_char(' '), None);
        assert_eq!(LanguageValidator::classify_char('1'), None);
        assert_eq!(LanguageValidator::classify_char('.'), None);
        assert_eq!(LanguageValidator::classify_char('—'), None); // em dash U+2014
    }

    // ── language_to_scripts ───────────────────────────────────────────────────

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
        assert!(scripts.contains(&Script::Hiragana));
        assert!(scripts.contains(&Script::Katakana));
        assert!(scripts.contains(&Script::Cjk));
    }

    #[test]
    fn test_language_to_scripts_unsupported() {
        let result = LanguageValidator::language_to_scripts("Klingon");
        assert!(result.is_err());
        match result {
            Err(LanguageValidatorError::UnsupportedLanguage(lang)) => {
                assert_eq!(lang, "klingon");
            }
            _ => panic!("Expected UnsupportedLanguage error"),
        }
    }

    #[test]
    fn test_language_case_insensitive() {
        // lowercase / uppercase / mixed should all work
        assert!(LanguageValidator::language_to_scripts("english").is_ok());
        assert!(LanguageValidator::language_to_scripts("ENGLISH").is_ok());
        assert!(LanguageValidator::language_to_scripts("Korean").is_ok());
        assert!(LanguageValidator::language_to_scripts("KOREAN").is_ok());
    }

    // ── strip_markdown ────────────────────────────────────────────────────────

    #[test]
    fn test_strip_markdown_code_blocks() {
        let content = "Some text\n```rust\nlet x = 1;\n```\nMore text";
        let lines = LanguageValidator::strip_markdown(content);
        // "Some text" and "More text" should be present; code block lines should be absent
        let texts: Vec<&str> = lines.iter().map(|(_, t)| t.as_str()).collect();
        assert!(texts.iter().any(|t| t.contains("Some text")));
        assert!(texts.iter().any(|t| t.contains("More text")));
        assert!(!texts.iter().any(|t| t.contains("let x")));
    }

    #[test]
    fn test_strip_markdown_headings() {
        let content = "# Heading 1\n## Heading 2\nActual content here";
        let lines = LanguageValidator::strip_markdown(content);
        let texts: Vec<&str> = lines.iter().map(|(_, t)| t.as_str()).collect();
        // Headings should not appear
        assert!(!texts.iter().any(|t| t.contains("Heading")));
        // But actual content should appear
        assert!(texts.iter().any(|t| t.contains("Actual content")));
    }

    #[test]
    fn test_strip_markdown_none_markers() {
        let content = "Good content\nNone\nN/A\nMore good content";
        let lines = LanguageValidator::strip_markdown(content);
        let texts: Vec<&str> = lines.iter().map(|(_, t)| t.as_str()).collect();
        assert!(!texts.iter().any(|t| *t == "None" || *t == "N/A"));
        assert!(texts.iter().any(|t| t.contains("Good content")));
        assert!(texts.iter().any(|t| t.contains("More good content")));
    }

    #[test]
    fn test_strip_markdown_inline_code() {
        let content = "Use `cargo build` to compile the project";
        let lines = LanguageValidator::strip_markdown(content);
        let text = &lines[0].1;
        // inline code content should be stripped
        assert!(!text.contains("cargo build"));
        // surrounding prose should remain
        assert!(text.contains("Use"));
        assert!(text.contains("compile the project"));
    }

    #[test]
    fn test_strip_markdown_urls() {
        let content = "See the docs at https://example.com/docs for more info";
        let lines = LanguageValidator::strip_markdown(content);
        let text = &lines[0].1;
        assert!(!text.contains("https://"));
        assert!(text.contains("See the docs at"));
        assert!(text.contains("more info"));
    }

    #[test]
    fn test_strip_markdown_preserves_line_numbers() {
        // Line 1: heading (skipped), Line 2: empty (skipped), Line 3: content
        let content = "# Skip me\n\nHello world";
        let lines = LanguageValidator::strip_markdown(content);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, 3); // 1-indexed line number
        assert_eq!(lines[0].1, "Hello world");
    }

    // ── validate_content ──────────────────────────────────────────────────────

    #[test]
    fn test_validate_content_english_pass() {
        // Enough Latin text to easily pass 80% threshold
        let content = "This is a well written English document with many words that contain Latin characters throughout the entire text body.";
        let validator = LanguageValidator::new();
        let result = validator
            .validate_content(content, "test.md", "English", 80.0)
            .unwrap();
        assert_eq!(result.result, "pass");
        assert!(result.target_percentage >= 80.0);
    }

    #[test]
    fn test_validate_content_skipped_insufficient() {
        // Very short content — fewer than 20 classified chars
        let content = "Hi";
        let validator = LanguageValidator::new();
        let result = validator
            .validate_content(content, "test.md", "English", 80.0)
            .unwrap();
        assert_eq!(result.result, "skipped");
    }

    #[test]
    fn test_validate_content_below_threshold() {
        // Mix of Latin and Hangul: Korean text dominates, so English validation fails
        let content = "안녕하세요 반갑습니다 한국어 텍스트가 많이 포함되어 있습니다 hello world";
        let validator = LanguageValidator::new();
        let result = validator
            .validate_content(content, "test.md", "English", 80.0)
            .unwrap();
        assert_eq!(result.result, "below_threshold");
        assert!(result.target_percentage < 80.0);
    }

    // ── non_target_line_50_percent_rule ──────────────────────────────────────

    #[test]
    fn test_non_target_line_50_percent_rule() {
        // Line 1: all Korean → non-target for English validation
        // Line 3: all English → target
        let content = "안녕하세요 반갑습니다 여기는 한국어 텍스트\nHello this is an English line with many Latin characters";
        let validator = LanguageValidator::new();
        let result = validator
            .validate_content(content, "test.md", "English", 80.0)
            .unwrap();
        // The Korean line should appear in non_target_lines
        assert!(!result.non_target_lines.is_empty());
    }

    // ── threshold_boundary_inclusive ─────────────────────────────────────────

    #[test]
    fn test_threshold_boundary_inclusive() {
        // Craft content where exactly the threshold percentage is target
        // 20 Latin chars, 0 non-Latin classified chars → 100% Latin
        let content = "abcdefghijklmnopqrst"; // exactly 20 Latin chars
        let validator = LanguageValidator::new();

        // Should pass at 100% threshold
        let result = validator
            .validate_content(content, "test.md", "English", 100.0)
            .unwrap();
        assert_eq!(result.result, "pass");
        assert!((result.target_percentage - 100.0).abs() < 0.001);

        // Should fail if threshold > 100 (impossible, but let's test boundary)
        // More practically: test that exactly at threshold is inclusive (pass)
        let result2 = validator
            .validate_content(content, "test.md", "English", 100.0)
            .unwrap();
        assert_eq!(result2.result, "pass");
    }
}
