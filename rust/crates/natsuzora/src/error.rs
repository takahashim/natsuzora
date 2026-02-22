//! Error types for Natsuzora template engine.

use thiserror::Error;

// Re-export Location from natsuzora-ast
pub use natsuzora_ast::Location;

/// All errors that can occur in Natsuzora
#[derive(Error, Debug)]
pub enum NatsuzoraError {
    #[error("Parse error at line {}, column {}: {message}", location.line, location.column)]
    ParseError {
        message: String,
        location: Location,
    },

    #[error("Undefined variable '{name}' at line {}, column {}", location.line, location.column)]
    UndefinedVariable {
        name: String,
        location: Location,
    },

    #[error("Null value error for '{name}' at line {}, column {}", location.line, location.column)]
    NullValueError {
        name: String,
        location: Location,
    },

    #[error("Empty string error for '{name}' at line {}, column {}", location.line, location.column)]
    EmptyStringError {
        name: String,
        location: Location,
    },

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
