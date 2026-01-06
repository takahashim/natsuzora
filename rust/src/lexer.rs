use crate::error::{Location, NatsuzoraError, Result};
use crate::token::{to_keyword, Token, TokenKind};

const OPEN: &str = "{[";
const CLOSE: &str = "]}";

/// Lexer for tokenizing Natsuzora template source
pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    inside_tag: bool,
    at_tag_start: bool,
    after_gt: bool,
    strip_after_close: bool,
}

impl Lexer {
    /// Create a new lexer for the given source
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            inside_tag: false,
            at_tag_start: false,
            after_gt: false,
            strip_after_close: false,
        }
    }

    /// Tokenize the source and return a vector of tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.eof() {
            if self.inside_tag {
                self.tokenize_inside_tag(&mut tokens)?;
            } else {
                self.tokenize_text(&mut tokens)?;
            }
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            Location::new(self.line, self.column),
        ));
        Ok(tokens)
    }

    fn tokenize_text(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        // If previous close had -}}, skip leading whitespace and newline
        if self.strip_after_close {
            self.strip_after_close = false;
            self.skip_leading_whitespace_and_newline();
        }

        let start_line = self.line;
        let start_column = self.column;
        let mut text = String::new();

        while !self.eof() && !self.match_str(OPEN) {
            text.push(self.advance());
        }

        if !text.is_empty() {
            tokens.push(Token::new(
                TokenKind::Text(text),
                Location::new(start_line, start_column),
            ));
        }

        if !self.eof() {
            self.consume_open(tokens)?;
        }

        Ok(())
    }

    fn skip_leading_whitespace_and_newline(&mut self) {
        // Check if everything up to the first newline is whitespace
        let mut lookahead = 0;
        while self.pos + lookahead < self.chars.len() {
            let c = self.chars[self.pos + lookahead];
            if c == '\n' {
                break;
            }
            if c != ' ' && c != '\t' && c != '\r' {
                return; // Non-whitespace found, don't strip
            }
            lookahead += 1;
        }

        // Skip the whitespace
        for _ in 0..lookahead {
            self.advance();
        }
        // Skip the newline if present
        if self.current_char() == Some('\n') {
            self.advance();
        }
    }

    fn consume_open(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let start_line = self.line;
        let start_column = self.column;
        self.advance(); // {
        self.advance(); // [

        // Check for comment: {[! ... ]}
        if self.current_char() == Some('!') {
            return self.skip_comment(start_line, start_column);
        }

        // Check for delimiter escape: {[{]}
        if self.current_char() == Some('{') {
            self.advance(); // {
            if !self.match_str(CLOSE) {
                return Err(NatsuzoraError::LexerError {
                    message: "Expected ']}' after '{[{'".to_string(),
                    location: Location::new(self.line, self.column),
                });
            }
            self.advance(); // ]
            self.advance(); // }
            tokens.push(Token::new(
                TokenKind::Text("{[".to_string()),
                Location::new(start_line, start_column),
            ));
            return Ok(());
        }

        // Check for whitespace control: {[- ... ]}
        if self.current_char() == Some('-') {
            self.advance(); // -
            Self::strip_trailing_whitespace_from_last_text(tokens);
        }

        tokens.push(Token::new(
            TokenKind::Open,
            Location::new(start_line, start_column),
        ));
        self.inside_tag = true;
        self.at_tag_start = true;
        Ok(())
    }

    fn strip_trailing_whitespace_from_last_text(tokens: &mut Vec<Token>) {
        // Find the last TEXT token
        let last_text_idx = tokens
            .iter()
            .rposition(|t| matches!(t.kind, TokenKind::Text(_)));
        let Some(idx) = last_text_idx else { return };

        let token = &tokens[idx];
        let TokenKind::Text(ref value) = token.kind else {
            return;
        };

        // Find the last newline
        if let Some(newline_pos) = value.rfind('\n') {
            // Check if everything after the newline is whitespace
            let suffix = &value[newline_pos + 1..];
            if !suffix.chars().all(|c| c == ' ' || c == '\t') {
                return;
            }

            // Strip trailing whitespace (keep the newline)
            let new_value = value[..=newline_pos].to_string();
            let location = token.location;
            tokens[idx] = Token::new(TokenKind::Text(new_value), location);
        } else {
            // No newline - check if entire value is whitespace
            if !value.chars().all(|c| c == ' ' || c == '\t') {
                return;
            }

            // Remove the token entirely
            tokens.remove(idx);
        }
    }

    fn skip_comment(&mut self, start_line: usize, start_column: usize) -> Result<()> {
        self.advance(); // !

        // Skip until }}
        while !self.eof() && !self.match_str(CLOSE) {
            self.advance();
        }

        if self.eof() {
            return Err(NatsuzoraError::LexerError {
                message: "Unclosed comment".to_string(),
                location: Location::new(start_line, start_column),
            });
        }

        self.advance(); // }
        self.advance(); // }
                        // Comment is completely ignored - no token emitted
        Ok(())
    }

    fn tokenize_inside_tag(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        self.check_no_whitespace_before_special_chars()?;
        self.skip_whitespace_with_token(tokens);
        self.at_tag_start = false;

        if self.eof() {
            return Ok(());
        }

        if self.match_str(CLOSE) {
            self.consume_close(tokens);
            return Ok(());
        }

        // Check for whitespace control: -]}
        if self.current_char() == Some('-') && self.peek_char() == Some(']') {
            self.advance(); // -
            self.strip_after_close = true;
            self.consume_close(tokens);
            return Ok(());
        }

        match self.current_char() {
            Some('#') => self.add_single_char_token(tokens, TokenKind::Hash),
            Some('/') => {
                if self.after_gt {
                    self.tokenize_include_name(tokens)?;
                    self.after_gt = false;
                } else {
                    self.add_single_char_token(tokens, TokenKind::Slash);
                }
            }
            Some('>') => {
                self.add_single_char_token(tokens, TokenKind::Gt);
                self.after_gt = true;
            }
            Some('=') => self.add_single_char_token(tokens, TokenKind::Equal),
            Some(',') => self.add_single_char_token(tokens, TokenKind::Comma),
            Some('.') => self.add_single_char_token(tokens, TokenKind::Dot),
            Some(c) => {
                self.after_gt = false;
                self.tokenize_identifier_or_name(tokens, c)?;
            }
            None => {}
        }

        Ok(())
    }

    fn consume_close(&mut self, tokens: &mut Vec<Token>) {
        let start_line = self.line;
        let start_column = self.column;
        self.advance(); // }
        self.advance(); // }
        tokens.push(Token::new(
            TokenKind::Close,
            Location::new(start_line, start_column),
        ));
        self.inside_tag = false;
    }

    fn tokenize_identifier_or_name(&mut self, tokens: &mut Vec<Token>, c: char) -> Result<()> {
        if c == '/' {
            self.tokenize_include_name(tokens)
        } else if Self::is_ident_start(c) {
            self.tokenize_identifier(tokens)
        } else {
            Err(NatsuzoraError::LexerError {
                message: format!("Unexpected character: '{}'", c),
                location: Location::new(self.line, self.column),
            })
        }
    }

    fn tokenize_identifier(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        while let Some(c) = self.current_char() {
            if Self::is_ident_cont(c) {
                value.push(self.advance());
            } else {
                break;
            }
        }

        let kind = to_keyword(&value).unwrap_or(TokenKind::Ident(value));
        tokens.push(Token::new(kind, Location::new(start_line, start_column)));
        Ok(())
    }

    fn tokenize_include_name(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        // Consume leading /
        value.push(self.advance());

        loop {
            match self.current_char() {
                Some(c) if Self::is_name_seg_char(c) => {
                    value.push(self.advance());
                }
                Some('/') if self.peek_char().map_or(false, Self::is_name_seg_char) => {
                    value.push(self.advance());
                }
                _ => break,
            }
        }

        tokens.push(Token::new(
            TokenKind::Ident(value),
            Location::new(start_line, start_column),
        ));
        Ok(())
    }

    fn check_no_whitespace_before_special_chars(&self) -> Result<()> {
        // Only check at the start of tag content (right after {[ or {[-)
        if !self.at_tag_start {
            return Ok(());
        }

        if !self.current_char().map_or(false, Self::is_whitespace) {
            return Ok(());
        }

        // Look ahead to find first non-whitespace character
        let mut lookahead = self.pos;
        while lookahead < self.chars.len() && Self::is_whitespace(self.chars[lookahead]) {
            lookahead += 1;
        }

        if lookahead < self.chars.len() {
            let next_char = self.chars[lookahead];
            if next_char == '#' || next_char == '/' || next_char == '>' {
                return Err(NatsuzoraError::LexerError {
                    message: format!(
                        "Whitespace not allowed before '{}' after tag open",
                        next_char
                    ),
                    location: Location::new(self.line, self.column),
                });
            }
        }

        Ok(())
    }

    fn skip_whitespace_with_token(&mut self, tokens: &mut Vec<Token>) {
        if !self.current_char().map_or(false, Self::is_whitespace) {
            return;
        }

        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        while let Some(c) = self.current_char() {
            if Self::is_whitespace(c) {
                value.push(self.advance());
            } else {
                break;
            }
        }

        tokens.push(Token::new(
            TokenKind::Whitespace(value),
            Location::new(start_line, start_column),
        ));
    }

    fn add_single_char_token(&mut self, tokens: &mut Vec<Token>, kind: TokenKind) {
        let location = Location::new(self.line, self.column);
        self.advance();
        tokens.push(Token::new(kind, location));
    }

    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn current_char(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    fn match_str(&self, s: &str) -> bool {
        // Zero-allocation string matching using iterators
        let remaining = &self.chars[self.pos..];
        if remaining.len() < s.chars().count() {
            return false;
        }
        s.chars().zip(remaining.iter()).all(|(a, b)| a == *b)
    }

    fn is_ident_start(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn is_ident_cont(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn is_name_seg_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn is_whitespace(c: char) -> bool {
        matches!(c, ' ' | '\t' | '\r' | '\n')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let mut lexer = Lexer::new("Hello, world!");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::Text(s) if s == "Hello, world!"));
        assert!(matches!(tokens[1].kind, TokenKind::Eof));
    }

    #[test]
    fn test_simple_variable() {
        let mut lexer = Lexer::new("{[ name ]}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Open));
        assert!(matches!(&tokens[1].kind, TokenKind::Whitespace(_)));
        assert!(matches!(&tokens[2].kind, TokenKind::Ident(s) if s == "name"));
        assert!(matches!(&tokens[3].kind, TokenKind::Whitespace(_)));
        assert!(matches!(tokens[4].kind, TokenKind::Close));
    }

    #[test]
    fn test_if_keyword() {
        let mut lexer = Lexer::new("{[#if condition]}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Open));
        assert!(matches!(tokens[1].kind, TokenKind::Hash));
        assert!(matches!(tokens[2].kind, TokenKind::KwIf));
    }

    #[test]
    fn test_include_name() {
        let mut lexer = Lexer::new("{[> /components/card]}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Open));
        assert!(matches!(tokens[1].kind, TokenKind::Gt));
        assert!(matches!(&tokens[3].kind, TokenKind::Ident(s) if s == "/components/card"));
    }

    #[test]
    fn test_path_with_dots() {
        let mut lexer = Lexer::new("{[ user.profile.name ]}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[2].kind, TokenKind::Ident(s) if s == "user"));
        assert!(matches!(tokens[3].kind, TokenKind::Dot));
        assert!(matches!(&tokens[4].kind, TokenKind::Ident(s) if s == "profile"));
        assert!(matches!(tokens[5].kind, TokenKind::Dot));
        assert!(matches!(&tokens[6].kind, TokenKind::Ident(s) if s == "name"));
    }

    #[test]
    fn test_comment_skipped() {
        let mut lexer = Lexer::new("{[! this is a comment ]}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_comment_preserves_surrounding_text() {
        let mut lexer = Lexer::new("before{[! comment ]}after");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0].kind, TokenKind::Text(s) if s == "before"));
        assert!(matches!(&tokens[1].kind, TokenKind::Text(s) if s == "after"));
        assert!(matches!(tokens[2].kind, TokenKind::Eof));
    }

    #[test]
    fn test_comment_without_spaces() {
        let mut lexer = Lexer::new("{[!comment]}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_multiline_comment() {
        let mut lexer = Lexer::new("{[! multi\nline\ncomment ]}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_unclosed_comment_error() {
        let mut lexer = Lexer::new("{[! unclosed");
        let result = lexer.tokenize();
        assert!(result.is_err());
        if let Err(NatsuzoraError::LexerError { message, .. }) = result {
            assert!(message.contains("Unclosed comment"));
        } else {
            panic!("Expected LexerError");
        }
    }

    #[test]
    fn test_whitespace_control_strip_before() {
        let mut lexer = Lexer::new("line1\n  {[- name ]}");
        let tokens = lexer.tokenize().unwrap();
        let text = tokens.iter().find(|t| matches!(t.kind, TokenKind::Text(_)));
        assert!(matches!(&text.unwrap().kind, TokenKind::Text(s) if s == "line1\n"));
    }

    #[test]
    fn test_whitespace_control_strip_after() {
        let mut lexer = Lexer::new("{[ name -]}\nnext");
        let tokens = lexer.tokenize().unwrap();
        let texts: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Text(_)))
            .collect();
        assert_eq!(texts.len(), 1);
        assert!(matches!(&texts[0].kind, TokenKind::Text(s) if s == "next"));
    }

    #[test]
    fn test_whitespace_control_both_sides() {
        let mut lexer = Lexer::new("before\n  {[- name -]}\nafter");
        let tokens = lexer.tokenize().unwrap();
        let texts: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Text(_)))
            .collect();
        assert_eq!(texts.len(), 2);
        assert!(matches!(&texts[0].kind, TokenKind::Text(s) if s == "before\n"));
        assert!(matches!(&texts[1].kind, TokenKind::Text(s) if s == "after"));
    }

    #[test]
    fn test_whitespace_control_no_strip_with_content_before() {
        let mut lexer = Lexer::new("text {[- name ]}");
        let tokens = lexer.tokenize().unwrap();
        let text = tokens.iter().find(|t| matches!(t.kind, TokenKind::Text(_)));
        assert!(matches!(&text.unwrap().kind, TokenKind::Text(s) if s == "text "));
    }

    #[test]
    fn test_whitespace_control_no_strip_with_content_after() {
        let mut lexer = Lexer::new("{[ name -]} more\nnext");
        let tokens = lexer.tokenize().unwrap();
        let texts: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Text(_)))
            .collect();
        assert_eq!(texts.len(), 1);
        assert!(matches!(&texts[0].kind, TokenKind::Text(s) if s == " more\nnext"));
    }

    #[test]
    fn test_whitespace_control_with_block_keywords() {
        let mut lexer = Lexer::new("{[-#if x -]}");
        let tokens = lexer.tokenize().unwrap();
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Hash)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwIf)));
    }

    #[test]
    fn test_delimiter_escape_basic() {
        let mut lexer = Lexer::new("{[{]}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::Text(s) if s == "{["));
        assert!(matches!(tokens[1].kind, TokenKind::Eof));
    }

    #[test]
    fn test_delimiter_escape_with_surrounding_text() {
        let mut lexer = Lexer::new("Template syntax: {[{]} name ]}");
        let tokens = lexer.tokenize().unwrap();
        let texts: Vec<_> = tokens
            .iter()
            .filter_map(|t| {
                if let TokenKind::Text(s) = &t.kind {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts, vec!["Template syntax: ", "{[", " name ]}"]);
    }

    #[test]
    fn test_delimiter_escape_multiple() {
        let mut lexer = Lexer::new("{[{]} and {[{]}");
        let tokens = lexer.tokenize().unwrap();
        let texts: Vec<_> = tokens
            .iter()
            .filter_map(|t| {
                if let TokenKind::Text(s) = &t.kind {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts, vec!["{[", " and ", "{["]);
    }

    #[test]
    fn test_delimiter_escape_followed_by_variable() {
        let mut lexer = Lexer::new("{[{]}{[ name ]}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Text(s) if s == "{["));
        assert!(matches!(tokens[1].kind, TokenKind::Open));
        assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::Ident(s) if s == "name")));
    }

    #[test]
    fn test_delimiter_escape_incomplete_error() {
        let mut lexer = Lexer::new("{[{");
        let result = lexer.tokenize();
        assert!(result.is_err());
        if let Err(NatsuzoraError::LexerError { message, .. }) = result {
            assert!(message.contains("Expected ']}' after '{[{'"));
        } else {
            panic!("Expected LexerError");
        }
    }

    #[test]
    fn test_delimiter_escape_without_close_error() {
        let mut lexer = Lexer::new("{[{ more text");
        let result = lexer.tokenize();
        assert!(result.is_err());
        if let Err(NatsuzoraError::LexerError { message, .. }) = result {
            assert!(message.contains("Expected ']}' after '{[{'"));
        } else {
            panic!("Expected LexerError");
        }
    }
}
