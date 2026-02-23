//! Error types for Natsuzora template engine.

use thiserror::Error;

// Re-export Location from natsuzora-ast
pub use natsuzora_ast::Location;

/// All errors that can occur in Natsuzora
#[derive(Error, Debug)]
pub enum NatsuzoraError {
    #[error("Parse error at line {}, column {}: {message}", location.line, location.column)]
    ParseError { message: String, location: Location },

    #[error("{message}")]
    UndefinedVariable { message: String, location: Location },

    #[error("Type error: {message}")]
    TypeError { message: String },

    #[error("Include error: {message}")]
    IncludeError { message: String },

    #[error(
        "Shadowing error: cannot shadow existing variable '{name}' (already defined in {origin})"
    )]
    ShadowingError { name: String, origin: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type alias for Natsuzora operations
pub type Result<T> = std::result::Result<T, NatsuzoraError>;
