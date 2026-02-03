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
}
