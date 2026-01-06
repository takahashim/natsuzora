use thiserror::Error;

/// Source location for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Location {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// All errors that can occur in Natsuzora
#[derive(Error, Debug)]
pub enum NatsuzoraError {
    #[error("Lexer error at {location}: {message}")]
    LexerError { message: String, location: Location },

    #[error("Parse error at {location}: {message}")]
    ParseError { message: String, location: Location },

    #[error("Reserved word '{word}' cannot be used as identifier at {location}")]
    ReservedWordError { word: String, location: Location },

    #[error("Undefined variable: {name}")]
    UndefinedVariable { name: String },

    #[error("Type error: {message}")]
    TypeError { message: String },

    #[error("Include error: {message}")]
    IncludeError { message: String },

    #[error("Shadowing error: cannot shadow existing variable '{name}'")]
    ShadowingError { name: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type alias for Natsuzora operations
pub type Result<T> = std::result::Result<T, NatsuzoraError>;
