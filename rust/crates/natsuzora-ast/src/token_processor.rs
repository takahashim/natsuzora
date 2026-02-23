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
    in_tag: bool,
    tag_token_count: usize,
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
            in_tag: false,
            tag_token_count: 0,
        }
    }

    fn process(&mut self) -> Result<Vec<Token>, ParseError> {
        // Take ownership of tokens to iterate
        let tokens = std::mem::take(&mut self.tokens);

        let mut idx = 0;
        while idx < tokens.len() {
            let token = &tokens[idx];
            let next_token = tokens.get(idx + 1);

            if self.in_comment {
                self.handle_comment_content(token, next_token);
                idx += 1;
                continue;
            }

            match token.token_type {
                TokenType::Percent => {
                    self.start_tag_if_needed();
                    self.start_comment(token);
                    self.tag_token_count += 1;
                }
                TokenType::Dash => {
                    self.start_tag_if_needed();
                    self.handle_dash(next_token);
                    self.tag_token_count += 1;
                }
                TokenType::Close => {
                    self.handle_close(token.clone());
                    self.in_tag = false;
                    self.tag_token_count = 0;
                }
                TokenType::Text => {
                    self.handle_text(token.clone());
                    self.in_tag = false;
                    self.tag_token_count = 0;
                }
                _ => {
                    self.start_tag_if_needed();
                    self.result.push(token.clone());
                    self.tag_token_count += 1;
                }
            }

            idx += 1;
        }

        if self.in_comment {
            return Err(ParseError::UnclosedComment {
                line: self.comment_start_line,
                column: self.comment_start_col,
            });
        }

        Ok(std::mem::take(&mut self.result))
    }

    fn start_tag_if_needed(&mut self) {
        if !self.in_tag {
            self.in_tag = true;
            self.tag_token_count = 0;
        }
    }

    fn handle_dash(&mut self, next_token: Option<&Token>) {
        if self.left_trim_dash() {
            self.strip_trailing_from_last_text_if_blank_line();
        }
        if self.right_trim_dash(next_token) {
            self.strip_next_text = true;
        }
    }

    fn left_trim_dash(&self) -> bool {
        self.tag_token_count == 0
    }

    fn right_trim_dash(&self, next_token: Option<&Token>) -> bool {
        matches!(next_token, Some(t) if t.token_type == TokenType::Close)
    }

    fn handle_close(&mut self, token: Token) {
        self.result.push(token);
    }

    fn handle_text(&mut self, token: Token) {
        let mut text_value = token.value.clone();

        if self.strip_next_text {
            self.strip_next_text = false;
            text_value = strip_leading_whitespace_if_blank_line(&text_value);
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

    fn strip_trailing_from_last_text_if_blank_line(&mut self) {
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
            let value = &last_text.value;
            let line_start = match (value.rfind('\n'), value.rfind('\r')) {
                (Some(nl), Some(cr)) => nl.max(cr) + 1,
                (Some(nl), None) => nl + 1,
                (None, Some(cr)) => cr + 1,
                (None, None) => 0,
            };
            let trailing_segment = &value[line_start..];
            if !trailing_segment.chars().all(|c| c == ' ' || c == '\t') {
                return;
            }

            let stripped = value[..line_start].to_string();
            self.result[idx] = Token::new(
                TokenType::Text,
                stripped,
                last_text.location,
            );
        }
    }

    fn start_comment(&mut self, token: &Token) {
        self.in_comment = true;
        self.comment_start_line = token.location.line;
        self.comment_start_col = token.location.column;
    }

    fn handle_comment_content(&mut self, token: &Token, next_token: Option<&Token>) {
        if token.token_type == TokenType::Dash && self.right_trim_dash(next_token) {
            self.strip_next_text = true;
        }

        if token.token_type == TokenType::Close {
            self.in_comment = false;
            self.in_tag = false;
            self.tag_token_count = 0;
        }
        // All tokens inside comment are ignored
    }
}

/// Strip leading whitespace/newline only when tag-right side is blank until line end.
fn strip_leading_whitespace_if_blank_line(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut pos = 0;

    // Skip spaces/tabs
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }

    if pos >= bytes.len() {
        return String::new();
    }

    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos += 1;
    } else if pos < bytes.len() && bytes[pos] == b'\r' {
        pos += 1;
        if pos < bytes.len() && bytes[pos] == b'\n' {
            pos += 1;
        }
    } else {
        return text.to_string();
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
    fn test_strip_leading_whitespace_if_blank_line() {
        assert_eq!(strip_leading_whitespace_if_blank_line("  \nhello"), "hello");
        assert_eq!(strip_leading_whitespace_if_blank_line("\thello"), "\thello");
        assert_eq!(strip_leading_whitespace_if_blank_line("hello"), "hello");
        assert_eq!(strip_leading_whitespace_if_blank_line("  hello"), "  hello");
        assert_eq!(
            strip_leading_whitespace_if_blank_line("\n  hello"),
            "  hello"
        );
        assert_eq!(strip_leading_whitespace_if_blank_line("   "), "");
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
            text_token("hello\n  "),
            tag_token(TokenType::Dash, "-"),
            tag_token(TokenType::Ident, "name"),
            tag_token(TokenType::Close, "]}"),
        ];
        let result = process(tokens).unwrap();
        let first_text = result.iter().find(|t| t.token_type == TokenType::Text).unwrap();
        assert_eq!(first_text.value, "hello\n");
    }

    #[test]
    fn test_dash_does_not_strip_trailing_when_not_blank_line() {
        let tokens = vec![
            text_token("hello  "),
            tag_token(TokenType::Dash, "-"),
            tag_token(TokenType::Ident, "name"),
            tag_token(TokenType::Close, "]}"),
        ];
        let result = process(tokens).unwrap();
        let first_text = result.iter().find(|t| t.token_type == TokenType::Text).unwrap();
        assert_eq!(first_text.value, "hello  ");
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

    #[test]
    fn test_dash_does_not_strip_leading_next_when_not_blank_line() {
        let tokens = vec![
            tag_token(TokenType::Ident, "name"),
            tag_token(TokenType::Dash, "-"),
            tag_token(TokenType::Close, "]}"),
            text_token("  hello"),
        ];
        let result = process(tokens).unwrap();
        let last_text = result
            .iter()
            .rev()
            .find(|t| t.token_type == TokenType::Text)
            .unwrap();
        assert_eq!(last_text.value, "  hello");
    }
}
