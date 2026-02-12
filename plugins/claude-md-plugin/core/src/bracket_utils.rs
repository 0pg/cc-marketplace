/// Utility functions for parsing strings while respecting balanced brackets.

/// Split a string by a delimiter, but ignore delimiters inside balanced brackets.
/// Supports <>, (), [], {}
///
/// # Examples
/// ```
/// use claude_md_core::bracket_utils::split_respecting_brackets;
///
/// let result = split_respecting_brackets("Map<string, int>, List<string>", ',');
/// assert_eq!(result, vec!["Map<string, int>", "List<string>"]);
/// ```
pub fn split_respecting_brackets(s: &str, delimiter: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut angle_depth: u32 = 0;   // < >
    let mut paren_depth: u32 = 0;   // ( )
    let mut bracket_depth: u32 = 0; // [ ]
    let mut brace_depth: u32 = 0;   // { }

    for c in s.chars() {
        match c {
            '<' => {
                angle_depth += 1;
                current.push(c);
            }
            '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                current.push(c);
            }
            '(' => {
                paren_depth += 1;
                current.push(c);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(c);
            }
            '[' => {
                bracket_depth += 1;
                current.push(c);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(c);
            }
            '{' => {
                brace_depth += 1;
                current.push(c);
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(c);
            }
            _ if c == delimiter
                && angle_depth == 0
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                result.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Find the byte index of a closing bracket that matches the opening bracket at start_byte_idx.
/// Handles nested brackets of the same type.
/// Returns None if no matching bracket is found.
pub fn find_matching_bracket(s: &str, start_byte_idx: usize, open: char, close: char) -> Option<usize> {
    let mut chars_iter = s[start_byte_idx..].char_indices();
    let (_, first_char) = chars_iter.next()?;
    if first_char != open {
        return None;
    }
    let mut depth: u32 = 1;
    for (offset, c) in chars_iter {
        if c == open { depth += 1; }
        else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(start_byte_idx + offset);
            }
        }
    }
    None
}

/// Extract content between parentheses, respecting nested brackets.
/// Returns (params_content, rest_of_string) or None if malformed.
pub fn extract_parenthesized(s: &str) -> Option<(String, String)> {
    let paren_start = s.find('(')?;
    let paren_end = find_matching_bracket(s, paren_start, '(', ')')?;

    let params = s[paren_start + 1..paren_end].to_string();
    let rest = s[paren_end + 1..].trim().to_string();

    Some((params, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_split() {
        let result = split_respecting_brackets("a, b, c", ',');
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_nested_generic() {
        let result = split_respecting_brackets("Map<string, List<int>>, string", ',');
        assert_eq!(result, vec!["Map<string, List<int>>", "string"]);
    }

    #[test]
    fn test_multiple_bracket_types() {
        let result = split_respecting_brackets("fn(a, b), [x, y], {k: v}", ',');
        assert_eq!(result, vec!["fn(a, b)", "[x, y]", "{k: v}"]);
    }

    #[test]
    fn test_different_delimiter() {
        let result = split_respecting_brackets("a|b<x|y>|c", '|');
        assert_eq!(result, vec!["a", "b<x|y>", "c"]);
    }

    #[test]
    fn test_find_matching_bracket_basic() {
        let s = "foo(bar)";
        assert_eq!(find_matching_bracket(s, 3, '(', ')'), Some(7));
    }

    #[test]
    fn test_find_matching_bracket_nested() {
        let s = "fn(a(b))";
        assert_eq!(find_matching_bracket(s, 2, '(', ')'), Some(7));
    }

    #[test]
    fn test_find_matching_bracket_no_match() {
        let s = "fn(abc";
        assert_eq!(find_matching_bracket(s, 2, '(', ')'), None);
    }

    #[test]
    fn test_find_matching_bracket_not_open() {
        let s = "fn_abc";
        assert_eq!(find_matching_bracket(s, 2, '(', ')'), None);
    }

    #[test]
    fn test_find_matching_bracket_multibyte_korean_prefix() {
        // "한글함수" is 12 bytes (3 bytes per Korean char), '(' is at byte index 12
        let s = "한글함수(param)";
        let paren_start = s.find('(').unwrap();
        assert_eq!(paren_start, 12);
        let result = find_matching_bracket(s, paren_start, '(', ')');
        assert_eq!(result, Some(s.find(')').unwrap()));
    }

    #[test]
    fn test_find_matching_bracket_multibyte_emoji_prefix() {
        // "fn_🎉" has emoji that is 4 bytes, '(' is after it
        let s = "fn_🎉(x)";
        let paren_start = s.find('(').unwrap();
        let result = find_matching_bracket(s, paren_start, '(', ')');
        assert_eq!(result, Some(s.find(')').unwrap()));
    }

    #[test]
    fn test_extract_parenthesized_multibyte_prefix() {
        let s = "한글함수(param)";
        let result = extract_parenthesized(s);
        assert!(result.is_some());
        let (params, rest) = result.unwrap();
        assert_eq!(params, "param");
        assert_eq!(rest, "");
    }

    #[test]
    fn test_extract_parenthesized_emoji_prefix() {
        let s = "fn_🎉(x)";
        let result = extract_parenthesized(s);
        assert!(result.is_some());
        let (params, rest) = result.unwrap();
        assert_eq!(params, "x");
        assert_eq!(rest, "");
    }
}
