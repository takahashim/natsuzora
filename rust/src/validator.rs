//! Centralized validation functions for Natsuzora templates

use crate::error::{Location, NatsuzoraError, Result};
use crate::token::is_reserved_word;

/// Validate an identifier (variable name, each binding, include argument key)
///
/// Rules:
/// - Cannot be a reserved word (if, unless, each, as, unsecure, true, false, null, include)
/// - Cannot start with '_' (reserved for internal use)
/// - Cannot contain '@' (reserved for future use)
pub fn validate_identifier(name: &str, location: &Location) -> Result<()> {
    if is_reserved_word(name) {
        return Err(NatsuzoraError::ReservedWordError {
            word: name.to_string(),
            location: *location,
        });
    }
    if name.starts_with('_') {
        return Err(NatsuzoraError::ParseError {
            message: format!("Identifier cannot start with '_': {}", name),
            location: *location,
        });
    }
    if name.contains('@') {
        return Err(NatsuzoraError::ParseError {
            message: format!("Identifier cannot contain '@': {}", name),
            location: *location,
        });
    }
    Ok(())
}

/// Validate an include name at parse time
///
/// Rules:
/// - Must start with '/'
/// - Must have at least one segment after '/'
/// - Cannot contain '..' (path traversal)
/// - Cannot contain '//' (double slash)
/// - Cannot contain ':' (Windows drive letters)
pub fn validate_include_name_syntax(name: &str, location: &Location) -> Result<()> {
    if !name.starts_with('/') {
        return Err(NatsuzoraError::ParseError {
            message: "Include name must start with '/'".to_string(),
            location: *location,
        });
    }
    if name == "/" {
        return Err(NatsuzoraError::ParseError {
            message: "Include name must have at least one segment".to_string(),
            location: *location,
        });
    }
    if name.contains("..") {
        return Err(NatsuzoraError::ParseError {
            message: "Include name cannot contain '..'".to_string(),
            location: *location,
        });
    }
    if name.contains("//") {
        return Err(NatsuzoraError::ParseError {
            message: "Include name cannot contain '//'".to_string(),
            location: *location,
        });
    }
    if name.contains(':') {
        return Err(NatsuzoraError::ParseError {
            message: "Include name cannot contain ':'".to_string(),
            location: *location,
        });
    }
    Ok(())
}

/// Validate an include name at load time
///
/// Additional rules beyond parse-time validation:
/// - Cannot contain '\' (Windows path separator)
pub fn validate_include_name_runtime(name: &str) -> Result<()> {
    if !name.starts_with('/') {
        return Err(NatsuzoraError::IncludeError {
            message: format!("Include name must start with '/': {}", name),
        });
    }
    if name.contains("..") || name.contains("//") {
        return Err(NatsuzoraError::IncludeError {
            message: format!("Invalid include name: {}", name),
        });
    }
    if name.contains('\\') || name.contains(':') {
        return Err(NatsuzoraError::IncludeError {
            message: format!("Invalid include name: {}", name),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> Location {
        Location::new(1, 1)
    }

    mod identifier_validation {
        use super::*;

        #[test]
        fn accepts_valid_identifiers() {
            assert!(validate_identifier("name", &loc()).is_ok());
            assert!(validate_identifier("userName", &loc()).is_ok());
            assert!(validate_identifier("user_name", &loc()).is_ok());
            assert!(validate_identifier("item123", &loc()).is_ok());
        }

        #[test]
        fn rejects_reserved_words() {
            assert!(matches!(
                validate_identifier("if", &loc()),
                Err(NatsuzoraError::ReservedWordError { .. })
            ));
            assert!(matches!(
                validate_identifier("each", &loc()),
                Err(NatsuzoraError::ReservedWordError { .. })
            ));
            assert!(matches!(
                validate_identifier("true", &loc()),
                Err(NatsuzoraError::ReservedWordError { .. })
            ));
            assert!(matches!(
                validate_identifier("null", &loc()),
                Err(NatsuzoraError::ReservedWordError { .. })
            ));
        }

        #[test]
        fn rejects_underscore_prefix() {
            assert!(matches!(
                validate_identifier("_private", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
            assert!(matches!(
                validate_identifier("__dunder", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
        }

        #[test]
        fn rejects_at_symbol() {
            assert!(matches!(
                validate_identifier("user@name", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
            assert!(matches!(
                validate_identifier("@special", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
        }
    }

    mod include_name_syntax_validation {
        use super::*;

        #[test]
        fn accepts_valid_include_names() {
            assert!(validate_include_name_syntax("/greeting", &loc()).is_ok());
            assert!(validate_include_name_syntax("/components/card", &loc()).is_ok());
            assert!(validate_include_name_syntax("/a/b/c/d", &loc()).is_ok());
        }

        #[test]
        fn rejects_without_leading_slash() {
            assert!(matches!(
                validate_include_name_syntax("greeting", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
        }

        #[test]
        fn rejects_path_traversal() {
            assert!(matches!(
                validate_include_name_syntax("/path/../traversal", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
        }

        #[test]
        fn rejects_double_slash() {
            assert!(matches!(
                validate_include_name_syntax("/path//double", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
        }

        #[test]
        fn rejects_colon() {
            assert!(matches!(
                validate_include_name_syntax("/c:/windows/path", &loc()),
                Err(NatsuzoraError::ParseError { .. })
            ));
        }
    }

    mod include_name_runtime_validation {
        use super::*;

        #[test]
        fn accepts_valid_include_names() {
            assert!(validate_include_name_runtime("/greeting").is_ok());
            assert!(validate_include_name_runtime("/components/card").is_ok());
        }

        #[test]
        fn rejects_backslash() {
            assert!(matches!(
                validate_include_name_runtime("/path\\with\\backslash"),
                Err(NatsuzoraError::IncludeError { .. })
            ));
        }
    }
}
