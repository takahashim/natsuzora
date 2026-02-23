//! Token processor for whitespace control and comment handling.
//!
//! Mirrors Ruby's `TokenProcessor` exactly:
//! - DASH tokens strip adjacent TEXT whitespace
//! - PERCENT...CLOSE sequences are consumed (comments)
//! - Unclosed comments are detected

use crate::token::{Token, TokenType};
use crate::ParseError;

/// Process tokens: handle whitespace control and strip comments.
pub fn process(tokens: Vec<Token>) -> Result<Vec<Token>, ParseError> {
    let mut processor = TokenProcessor::new(tokens);
    processor.process()
}

struct TokenProcessor {
    tokens: Vec<Token>,
    result: Vec<Token>,
    strip_next_text: bool,
    in_comment: bool,
    comment_start_line: usize,
    comment_start_col: usize,
}

impl TokenProcessor {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            result: Vec::new(),
            strip_next_text: false,
            in_comment: false,
            comment_start_line: 0,
            comment_start_col: 0,
        }
    }

    fn process(&mut self) -> Result<Vec<Token>, ParseError> {
        // Take ownership of tokens to iterate
        let tokens = std::mem::take(&mut self.tokens);

        for token in tokens {
            if self.in_comment {
                self.handle_comment_content(&token);
                continue;
            }

            match token.token_type {
                TokenType::Percent => self.start_comment(&token),
                TokenType::Dash => self.handle_dash(),
                TokenType::Close => self.handle_close(token),
                TokenType::Text => self.handle_text(token),
                _ => self.result.push(token),
            }
        }

        if self.in_comment {
            return Err(ParseError::UnclosedComment {
                line: self.comment_start_line,
                column: self.comment_start_col,
            });
        }

        Ok(std::mem::take(&mut self.result))
    }

    fn handle_dash(&mut self) {
        // Strip trailing whitespace from previous TEXT
        self.strip_trailing_from_last_text();
        // Set flag to strip next TEXT (after CLOSE)
        self.strip_next_text = true;
    }

    fn handle_close(&mut self, token: Token) {
        self.result.push(token);
        // strip_next_text remains set for the next TEXT token
    }

    fn handle_text(&mut self, token: Token) {
        let mut text_value = token.value.clone();

        if self.strip_next_text {
            self.strip_next_text = false;
            text_value = strip_leading_whitespace_and_newline(&text_value);
        }

        // Only add non-empty text tokens
        if text_value.is_empty() {
            return;
        }

        self.result.push(Token::new(
            TokenType::Text,
            text_value,
            token.location,
        ));
    }

    fn strip_trailing_from_last_text(&mut self) {
        if self.result.is_empty() {
            return;
        }

        // Find the last TEXT token
        let last_idx = self
            .result
            .iter()
            .rposition(|t| t.token_type == TokenType::Text);
        if let Some(idx) = last_idx {
            let last_text = &self.result[idx];
            let stripped = last_text.value.trim_end_matches(|c: char| c == ' ' || c == '\t');
            self.result[idx] = Token::new(
                TokenType::Text,
                stripped.to_string(),
                last_text.location,
            );
        }
    }

    fn start_comment(&mut self, token: &Token) {
        self.in_comment = true;
        self.comment_start_line = token.location.line;
        self.comment_start_col = token.location.column;
    }

    fn handle_comment_content(&mut self, token: &Token) {
        if token.token_type == TokenType::Close {
            self.in_comment = false;
        }
        // All tokens inside comment are ignored
    }
}

/// Strip leading whitespace and optional newline.
/// Matches Ruby: `text.sub(/\A[ \t]*\n?/, '')`
fn strip_leading_whitespace_and_newline(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut pos = 0;

    // Skip spaces/tabs
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }

    // Skip optional newline
    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos += 1;
    } else if pos < bytes.len() && bytes[pos] == b'\r' {
        pos += 1;
        if pos < bytes.len() && bytes[pos] == b'\n' {
            pos += 1;
        }
    }

    text[pos..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;

    fn text_token(value: &str) -> Token {
        Token::new(TokenType::Text, value, Location::new(1, 1, 0))
    }

    fn tag_token(tt: TokenType, value: &str) -> Token {
        Token::new(tt, value, Location::new(1, 1, 0))
    }

    #[test]
    fn test_strip_leading_whitespace_and_newline() {
        assert_eq!(strip_leading_whitespace_and_newline("  \nhello"), "hello");
        assert_eq!(strip_leading_whitespace_and_newline("\thello"), "hello");
        assert_eq!(strip_leading_whitespace_and_newline("hello"), "hello");
        assert_eq!(strip_leading_whitespace_and_newline("  hello"), "hello");
        assert_eq!(
            strip_leading_whitespace_and_newline("\n  hello"),
            "  hello"
        );
    }

    #[test]
    fn test_comment_stripping() {
        let tokens = vec![
            text_token("hello"),
            tag_token(TokenType::Percent, "%"),
            tag_token(TokenType::Whitespace, " "),
            tag_token(TokenType::Ident, "comment"),
            tag_token(TokenType::Whitespace, " "),
            tag_token(TokenType::Close, "]}"),
            text_token("world"),
        ];
        let result = process(tokens).unwrap();
        // PERCENT...CLOSE is consumed, leaving hello + world
        let texts: Vec<&str> = result
            .iter()
            .filter(|t| t.token_type == TokenType::Text)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(texts, vec!["hello", "world"]);
    }

    #[test]
    fn test_unclosed_comment() {
        let tokens = vec![
            tag_token(TokenType::Percent, "%"),
            tag_token(TokenType::Ident, "no_close"),
        ];
        let result = process(tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_dash_strips_trailing() {
        let tokens = vec![
            text_token("hello  "),
            tag_token(TokenType::Dash, "-"),
            tag_token(TokenType::Ident, "name"),
            tag_token(TokenType::Close, "]}"),
        ];
        let result = process(tokens).unwrap();
        let first_text = result.iter().find(|t| t.token_type == TokenType::Text).unwrap();
        assert_eq!(first_text.value, "hello");
    }

    #[test]
    fn test_dash_strips_leading_next() {
        let tokens = vec![
            tag_token(TokenType::Ident, "name"),
            tag_token(TokenType::Dash, "-"),
            tag_token(TokenType::Close, "]}"),
            text_token("  \nhello"),
        ];
        let result = process(tokens).unwrap();
        let last_text = result
            .iter()
            .rev()
            .find(|t| t.token_type == TokenType::Text)
            .unwrap();
        assert_eq!(last_text.value, "hello");
    }
}
