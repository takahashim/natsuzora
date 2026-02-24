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

impl TokenType {
    /// Returns the fixed source literal for token types that have one.
    ///
    /// Tokens whose surface text varies by instance (e.g. identifiers,
    /// whitespace, text) return `None` and should use `Token.value`.
    pub fn literal(self) -> Option<&'static str> {
        match self {
            TokenType::Percent => Some("%"),
            TokenType::Dash => Some("-"),
            TokenType::Close => Some("]}"),
            TokenType::Hash => Some("#"),
            TokenType::Slash => Some("/"),
            TokenType::BangUnsecure => Some("!unsecure"),
            TokenType::BangInclude => Some("!include"),
            TokenType::Exclamation => Some("!"),
            TokenType::KwIf => Some("if"),
            TokenType::KwUnless => Some("unless"),
            TokenType::KwElse => Some("else"),
            TokenType::KwEach => Some("each"),
            TokenType::KwAs => Some("as"),
            TokenType::Dot => Some("."),
            TokenType::Comma => Some(","),
            TokenType::Equal => Some("="),
            TokenType::Question => Some("?"),
            TokenType::Text | TokenType::Whitespace | TokenType::Ident | TokenType::Eof => None,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::TokenType;

    #[test]
    fn all_fixed_literal_variants() {
        let cases: Vec<(TokenType, &str)> = vec![
            (TokenType::Percent, "%"),
            (TokenType::Dash, "-"),
            (TokenType::Close, "]}"),
            (TokenType::Hash, "#"),
            (TokenType::Slash, "/"),
            (TokenType::BangUnsecure, "!unsecure"),
            (TokenType::BangInclude, "!include"),
            (TokenType::Exclamation, "!"),
            (TokenType::KwIf, "if"),
            (TokenType::KwUnless, "unless"),
            (TokenType::KwElse, "else"),
            (TokenType::KwEach, "each"),
            (TokenType::KwAs, "as"),
            (TokenType::Dot, "."),
            (TokenType::Comma, ","),
            (TokenType::Equal, "="),
            (TokenType::Question, "?"),
        ];
        for (variant, expected) in cases {
            assert_eq!(
                variant.literal(),
                Some(expected),
                "{:?} should return Some({:?})",
                variant,
                expected
            );
        }
    }

    #[test]
    fn all_dynamic_variants_return_none() {
        let dynamic = vec![
            TokenType::Text,
            TokenType::Whitespace,
            TokenType::Ident,
            TokenType::Eof,
        ];
        for variant in dynamic {
            assert_eq!(
                variant.literal(),
                None,
                "{:?} should return None",
                variant
            );
        }
    }
}
