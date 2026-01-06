/// Escape HTML special characters: & < > " '
pub fn escape(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(c),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_ampersand() {
        assert_eq!(escape("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_less_than() {
        assert_eq!(escape("a < b"), "a &lt; b");
    }

    #[test]
    fn test_escape_greater_than() {
        assert_eq!(escape("a > b"), "a &gt; b");
    }

    #[test]
    fn test_escape_double_quote() {
        assert_eq!(escape("a \"b\" c"), "a &quot;b&quot; c");
    }

    #[test]
    fn test_escape_single_quote() {
        assert_eq!(escape("a 'b' c"), "a &#39;b&#39; c");
    }

    #[test]
    fn test_escape_multiple() {
        assert_eq!(
            escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_no_escape_needed() {
        assert_eq!(escape("Hello, world!"), "Hello, world!");
    }
}
