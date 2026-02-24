//! Token processor for whitespace control and comment handling.
//!
//! Responsibilities:
//! - Consume DASH tokens and apply trim rules
//! - Consume comment tags entirely
//! - Detect unclosed comments

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
}

impl TokenProcessor {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            result: Vec::new(),
            strip_next_text: false,
        }
    }

    fn process(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut idx = 0;

        while idx < self.tokens.len() {
            let token = &self.tokens[idx];

            if token.token_type == TokenType::Text {
                self.append_text(token.clone());
                idx += 1;
            } else {
                idx = self.process_tag(idx)?;
            }
        }

        Ok(std::mem::take(&mut self.result))
    }

    fn process_tag(&mut self, start_idx: usize) -> Result<usize, ParseError> {
        let close_idx = self.find_close_index(start_idx);
        let tag_tokens: Vec<Token> = match close_idx {
            Some(ci) => self.tokens[start_idx..=ci].to_vec(),
            None => self.tokens[start_idx..].to_vec(),
        };

        self.apply_left_trim(&tag_tokens);
        self.apply_right_trim(&tag_tokens);

        if comment_tag(&tag_tokens) {
            if let Some(ci) = close_idx {
                return Ok(ci + 1);
            }
            let comment = tag_tokens
                .iter()
                .find(|token| token.token_type == TokenType::Percent)
                .unwrap_or(&tag_tokens[0]);
            return Err(ParseError::UnclosedComment {
                line: comment.location.line,
                column: comment.location.column,
            });
        }

        self.emit_tag_tokens(&tag_tokens);
        Ok(match close_idx {
            Some(ci) => ci + 1,
            None => self.tokens.len(),
        })
    }

    fn append_text(&mut self, token: Token) {
        let mut text_value = token.value.clone();

        if self.strip_next_text {
            self.strip_next_text = false;
            text_value = strip_leading_whitespace_if_blank_line(&text_value);
        }

        if text_value.is_empty() {
            return;
        }

        self.result
            .push(Token::new(TokenType::Text, text_value, token.location));
    }

    fn find_close_index(&self, start_idx: usize) -> Option<usize> {
        let mut idx = start_idx;
        while idx < self.tokens.len() {
            if self.tokens[idx].token_type == TokenType::Close {
                return Some(idx);
            }
            idx += 1;
        }
        None
    }

    fn apply_left_trim(&mut self, tag_tokens: &[Token]) {
        let is_left_trim = matches!(tag_tokens.first(), Some(t) if t.token_type == TokenType::Dash);
        if is_left_trim {
            self.strip_trailing_from_last_text_if_blank_line();
        }
    }

    fn apply_right_trim(&mut self, tag_tokens: &[Token]) {
        let close_idx = tag_tokens
            .iter()
            .position(|token| token.token_type == TokenType::Close);
        let is_right_trim =
            matches!(close_idx, Some(ci) if ci > 0 && tag_tokens[ci - 1].token_type == TokenType::Dash);
        if is_right_trim {
            self.strip_next_text = true;
        }
    }

    fn emit_tag_tokens(&mut self, tag_tokens: &[Token]) {
        for token in tag_tokens {
            if token.token_type == TokenType::Dash {
                continue;
            }
            self.result.push(token.clone());
        }
    }

    fn strip_trailing_from_last_text_if_blank_line(&mut self) {
        let last_idx = self
            .result
            .iter()
            .rposition(|token| token.token_type == TokenType::Text);
        let Some(last_idx) = last_idx else {
            return;
        };

        let last_text = &self.result[last_idx];
        let value = &last_text.value;
        let line_start = same_line_start_offset(value);
        let trailing_segment = &value[line_start..];
        if !horizontal_whitespace_only(trailing_segment) {
            return;
        }

        self.result[last_idx] = Token::new(
            TokenType::Text,
            value[..line_start].to_string(),
            last_text.location,
        );
    }
}

fn comment_tag(tag_tokens: &[Token]) -> bool {
    let Some(first) = tag_tokens.first() else {
        return false;
    };

    if first.token_type == TokenType::Percent {
        return true;
    }

    first.token_type == TokenType::Dash
        && tag_tokens
            .get(1)
            .is_some_and(|token| token.token_type == TokenType::Percent)
}

/// Strip leading whitespace/newline only when tag-right side is blank until line end.
fn strip_leading_whitespace_if_blank_line(text: &str) -> String {
    let bytes = text.as_bytes();
    let pos = skip_leading_horizontal_whitespace(bytes);

    if pos >= bytes.len() {
        return String::new();
    }

    let Some(advance) = leading_newline_advance(bytes, pos) else {
        return text.to_string();
    };

    text[(pos + advance)..].to_string()
}

fn same_line_start_offset(value: &str) -> usize {
    match (value.rfind('\n'), value.rfind('\r')) {
        (Some(nl), Some(cr)) => nl.max(cr) + 1,
        (Some(nl), None) => nl + 1,
        (None, Some(cr)) => cr + 1,
        (None, None) => 0,
    }
}

fn horizontal_whitespace_only(segment: &str) -> bool {
    segment.chars().all(|c| c == ' ' || c == '\t')
}

fn skip_leading_horizontal_whitespace(bytes: &[u8]) -> usize {
    let mut idx = 0;
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
    }
    idx
}

fn leading_newline_advance(bytes: &[u8], pos: usize) -> Option<usize> {
    if bytes[pos] == b'\n' {
        return Some(1);
    }
    if bytes[pos] != b'\r' {
        return None;
    }
    if pos + 1 < bytes.len() && bytes[pos + 1] == b'\n' {
        Some(2)
    } else {
        Some(1)
    }
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
        let texts: Vec<&str> = result
            .iter()
            .filter(|token| token.token_type == TokenType::Text)
            .map(|token| token.value.as_str())
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
        let first_text = result
            .iter()
            .find(|token| token.token_type == TokenType::Text)
            .unwrap();
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
        let first_text = result
            .iter()
            .find(|token| token.token_type == TokenType::Text)
            .unwrap();
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
            .find(|token| token.token_type == TokenType::Text)
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
            .find(|token| token.token_type == TokenType::Text)
            .unwrap();
        assert_eq!(last_text.value, "  hello");
    }
}
