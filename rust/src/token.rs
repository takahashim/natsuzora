use crate::error::Location;

/// Token kinds in the Natsuzora template language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Raw text outside of {{ }}
    Text(String),
    /// Identifier or include name (starting with /)
    Ident(String),

    /// Opening delimiter {{
    Open,
    /// Closing delimiter }}
    Close,

    /// Hash symbol #
    Hash,
    /// Slash symbol /
    Slash,
    /// Greater-than symbol >
    Gt,
    /// Equal symbol =
    Equal,
    /// Comma symbol ,
    Comma,
    /// Dot symbol .
    Dot,

    /// Keyword: if
    KwIf,
    /// Keyword: unless
    KwUnless,
    /// Keyword: else
    KwElse,
    /// Keyword: each
    KwEach,
    /// Keyword: as
    KwAs,
    /// Keyword: unsecure
    KwUnsecure,

    /// Whitespace (preserved for required whitespace checks)
    Whitespace(String),

    /// End of file
    Eof,
}

/// A token with its kind and source location
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub location: Location,
}

impl Token {
    pub fn new(kind: TokenKind, location: Location) -> Self {
        Self { kind, location }
    }
}

/// Reserved words that cannot be used as identifiers
pub const RESERVED_WORDS: &[&str] = &[
    "if", "unless", "else", "each", "as", "unsecure", "true", "false", "null", "include",
];

/// Check if a string is a reserved word
pub fn is_reserved_word(s: &str) -> bool {
    RESERVED_WORDS.contains(&s)
}

/// Try to convert a string to a keyword token kind
pub fn to_keyword(s: &str) -> Option<TokenKind> {
    match s {
        "if" => Some(TokenKind::KwIf),
        "unless" => Some(TokenKind::KwUnless),
        "else" => Some(TokenKind::KwElse),
        "each" => Some(TokenKind::KwEach),
        "as" => Some(TokenKind::KwAs),
        "unsecure" => Some(TokenKind::KwUnsecure),
        _ => None,
    }
}
