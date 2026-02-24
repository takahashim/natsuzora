//! Hand-written lexer for Natsuzora templates.
//!
//! Two-mode state machine:
//! - Text mode: accumulates raw text until `{[` delimiter
//! - Tag mode: tokenizes operators, keywords, identifiers inside `{[` ... `]}`
//!
//! Escape: `{[{]}` → `{[` (processed inline as text)

use crate::token::{Token, TokenType};
use crate::{Location, ParseError};

/// Tokenize a source string into a sequence of tokens.
pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}

struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
    in_tag: bool,
}

impl<'a> Lexer<'a> {
    const TAG_OPEN: &'static [u8] = b"{[";
    const TAG_OPEN_ESCAPE: &'static [u8] = b"{[{]}";

    fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            in_tag: false,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        while self.pos < self.source.len() {
            if self.in_tag {
                self.tokenize_tag(&mut tokens)?;
            } else {
                self.tokenize_text(&mut tokens);
            }
        }

        // Add EOF token
        let eof_loc = Location::new(self.line, self.col, self.pos);
        tokens.push(Token::new(TokenType::Eof, "", eof_loc));

        Ok(tokens)
    }

    /// Tokenize text mode: accumulate text until `{[` delimiter.
    fn tokenize_text(&mut self, tokens: &mut Vec<Token>) {
        let start_loc = Location::new(self.line, self.col, self.pos);
        let mut text = String::new();

        while self.pos < self.source.len() {
            if self.looking_at(Self::TAG_OPEN) {
                // Check for escape sequence: {[{]}
                if self.looking_at(Self::TAG_OPEN_ESCAPE) {
                    text.push_str("{[");
                    self.advance_n(Self::TAG_OPEN_ESCAPE.len()); // skip {[{]}
                    continue;
                }

                // Found tag open delimiter
                break;
            }

            let ch = self.source[self.pos];
            text.push(ch as char);
            self.advance_one();
        }

        if !text.is_empty() {
            tokens.push(Token::new(TokenType::Text, text, start_loc));
        }

        // Enter tag mode if we found {[
        if self.looking_at(Self::TAG_OPEN) {
            self.advance_n(Self::TAG_OPEN.len()); // skip {[
            self.in_tag = true;
        }
    }

    /// Tokenize tag mode: operators, keywords, identifiers.
    fn tokenize_tag(&mut self, tokens: &mut Vec<Token>) -> Result<(), ParseError> {
        // Skip nothing — we process one token at a time
        if self.pos >= self.source.len() {
            return Ok(());
        }

        let loc = Location::new(self.line, self.col, self.pos);
        let ch = self.source[self.pos];

        match ch {
            // Closing delimiter ]}
            b']' if self.looking_at_token(TokenType::Close) => {
                self.emit_fixed(tokens, TokenType::Close, loc);
                self.in_tag = false;
            }

            b'%' => {
                self.emit_fixed(tokens, TokenType::Percent, loc);
            }

            b'-' => {
                self.emit_fixed(tokens, TokenType::Dash, loc);
            }

            b'#' => {
                self.emit_fixed(tokens, TokenType::Hash, loc);
            }

            b'/' => {
                self.emit_fixed(tokens, TokenType::Slash, loc);
            }

            b'!' => {
                self.tokenize_bang(tokens, loc);
            }

            b'.' => {
                self.emit_fixed(tokens, TokenType::Dot, loc);
            }

            b',' => {
                self.emit_fixed(tokens, TokenType::Comma, loc);
            }

            b'=' => {
                self.emit_fixed(tokens, TokenType::Equal, loc);
            }

            b'?' => {
                self.emit_fixed(tokens, TokenType::Question, loc);
            }

            // Whitespace
            b' ' | b'\t' | b'\r' | b'\n' => {
                let start = self.pos;
                while self.pos < self.source.len() {
                    match self.source[self.pos] {
                        b' ' | b'\t' | b'\r' | b'\n' => self.advance_one(),
                        _ => break,
                    }
                }
                let ws_text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or(" ");
                tokens.push(Token::new(TokenType::Whitespace, ws_text, loc));
            }

            // Identifier or keyword
            b'A'..=b'Z' | b'a'..=b'z' => {
                let start = self.pos;
                while self.pos < self.source.len() && self.is_ident_continue_at(self.pos) {
                    self.advance_one();
                }
                let ident = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("");
                let token_type = match ident {
                    "if" => TokenType::KwIf,
                    "unless" => TokenType::KwUnless,
                    "else" => TokenType::KwElse,
                    "each" => TokenType::KwEach,
                    "as" => TokenType::KwAs,
                    _ => TokenType::Ident,
                };
                tokens.push(Token::new(token_type, ident, loc));
            }

            _ => {
                return Err(ParseError::SyntaxError {
                    line: loc.line,
                    column: loc.column,
                    byte_range: self.pos..self.pos + 1,
                });
            }
        }

        Ok(())
    }

    /// Check if the source at current position starts with the given bytes.
    fn looking_at(&self, pattern: &[u8]) -> bool {
        self.source[self.pos..].starts_with(pattern)
    }

    /// Return the fixed literal for a token type.
    fn token_literal(token_type: TokenType) -> &'static str {
        token_type
            .literal()
            .expect("token type must have a fixed literal")
    }

    /// Check whether current position starts with the given fixed token literal.
    fn looking_at_token(&self, token_type: TokenType) -> bool {
        self.looking_at(Self::token_literal(token_type).as_bytes())
    }

    /// Emit a token with fixed literal text and advance by its byte length.
    fn emit_fixed(&mut self, tokens: &mut Vec<Token>, token_type: TokenType, loc: Location) {
        let literal = Self::token_literal(token_type);
        tokens.push(Token::new(token_type, literal, loc));
        self.advance_n(literal.len());
    }

    /// Tokenize `!`, `!unsecure`, `!include` using longest keyword match.
    fn tokenize_bang(&mut self, tokens: &mut Vec<Token>, loc: Location) {
        for token_type in [TokenType::BangUnsecure, TokenType::BangInclude] {
            let literal = Self::token_literal(token_type);
            if self.looking_at(literal.as_bytes())
                && !self.is_ident_continue_at(self.pos + literal.len())
            {
                self.emit_fixed(tokens, token_type, loc);
                return;
            }
        }
        self.emit_fixed(tokens, TokenType::Exclamation, loc);
    }

    /// Check if byte at given position is a valid identifier continuation character.
    fn is_ident_continue_at(&self, pos: usize) -> bool {
        if pos >= self.source.len() {
            return false;
        }
        matches!(self.source[pos], b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
    }

    /// Advance position by one byte, updating line/column tracking.
    fn advance_one(&mut self) {
        if self.pos < self.source.len() {
            if self.source[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    /// Advance position by n bytes, updating line/column tracking.
    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(tokens: &[Token]) -> Vec<TokenType> {
        tokens.iter().map(|t| t.token_type).collect()
    }

    #[test]
    fn test_plain_text() {
        let tokens = tokenize("Hello, World!").unwrap();
        assert_eq!(types(&tokens), vec![TokenType::Text, TokenType::Eof]);
        assert_eq!(tokens[0].value, "Hello, World!");
    }

    #[test]
    fn test_variable() {
        let tokens = tokenize("{[ name ]}").unwrap();
        assert_eq!(
            types(&tokens),
            vec![
                TokenType::Whitespace,
                TokenType::Ident,
                TokenType::Whitespace,
                TokenType::Close,
                TokenType::Eof,
            ]
        );
        assert_eq!(tokens[1].value, "name");
    }

    #[test]
    fn test_escape_sequence() {
        let tokens = tokenize("a{[{]}b").unwrap();
        assert_eq!(types(&tokens), vec![TokenType::Text, TokenType::Eof]);
        assert_eq!(tokens[0].value, "a{[b");
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize("{[#if x]}y{[/if]}").unwrap();
        let tt: Vec<_> = types(&tokens);
        assert!(tt.contains(&TokenType::Hash));
        assert!(tt.contains(&TokenType::KwIf));
    }

    #[test]
    fn test_dash_whitespace_control() {
        let tokens = tokenize("{[- name -]}").unwrap();
        assert!(types(&tokens).contains(&TokenType::Dash));
    }

    #[test]
    fn test_bang_unsecure() {
        let tokens = tokenize("{[!unsecure html]}").unwrap();
        assert!(types(&tokens).contains(&TokenType::BangUnsecure));
    }

    #[test]
    fn test_bang_include() {
        let tokens = tokenize("{[!include /path]}").unwrap();
        assert!(types(&tokens).contains(&TokenType::BangInclude));
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("{[% this is a comment ]}").unwrap();
        assert!(types(&tokens).contains(&TokenType::Percent));
    }
}
