//! Token types for the Natsuzora lexer.

use crate::Location;

/// Token types produced by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// Raw text content outside tags.
    Text,
    /// `%` - comment marker
    Percent,
    /// `-` - whitespace control marker
    Dash,
    /// `]}` - closing delimiter
    Close,
    /// `#` - block open marker
    Hash,
    /// `/` - block close marker or include path separator
    Slash,
    /// `!unsecure`
    BangUnsecure,
    /// `!include`
    BangInclude,
    /// `!` - exclamation (modifier)
    Exclamation,
    /// `if`
    KwIf,
    /// `unless`
    KwUnless,
    /// `else`
    KwElse,
    /// `each`
    KwEach,
    /// `as`
    KwAs,
    /// `.` - dot separator
    Dot,
    /// `,` - comma
    Comma,
    /// `=` - equals
    Equal,
    /// `?` - nullable modifier
    Question,
    /// Whitespace (spaces, tabs, newlines) inside tags
    Whitespace,
    /// Identifier: [A-Za-z][A-Za-z0-9_]*
    Ident,
    /// End of file
    Eof,
}

/// A token with its type, value, and location.
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub location: Location,
}

impl Token {
    pub fn new(token_type: TokenType, value: impl Into<String>, location: Location) -> Self {
        Self {
            token_type,
            value: value.into(),
            location,
        }
    }
}
