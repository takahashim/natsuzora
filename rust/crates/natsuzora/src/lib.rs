//! Natsuzora - A minimal, display-only template language
//!
//! Natsuzora is a template language designed for static HTML generation with:
//! - Safety-first design with HTML escaping by default
//! - Deterministic evaluation with no side effects
//! - Simple syntax for conditionals, loops, and includes
//!
//! # Example
//!
//! ```rust
//! use serde_json::json;
//!
//! let result = natsuzora::render(
//!     "Hello, {[ name ]}!",
//!     json!({"name": "World"}),
//! ).unwrap();
//!
//! assert_eq!(result, "Hello, World!");
//! ```

// Public modules
pub mod context;
pub mod error;
pub mod html_escape;
pub mod renderer;
pub mod template_loader;
pub mod value;

pub use error::{NatsuzoraError, Result};
pub use natsuzora_ast::{IncludeLoader, LoaderError, Location, Modifier, ParseError, Template};
pub use renderer::Renderer;
pub use template_loader::TemplateLoader;
pub use value::Value;

use std::path::Path;

/// Main template struct for parsing once and rendering multiple times
pub struct Natsuzora {
    template: Template,
    include_root: Option<std::path::PathBuf>,
}

impl Natsuzora {
    /// Parse a template source string
    ///
    /// # Example
    ///
    /// ```rust
    /// use serde_json::json;
    ///
    /// let tmpl = natsuzora::Natsuzora::parse("Hello, {[ name ]}!").unwrap();
    /// let result = tmpl.render(json!({"name": "Alice"})).unwrap();
    /// assert_eq!(result, "Hello, Alice!");
    /// ```
    pub fn parse(source: &str) -> Result<Self> {
        let template = natsuzora_ast::parse(source).map_err(|e| NatsuzoraError::ParseError {
            message: e.to_string(),
            location: Location::default(),
        })?;
        Ok(Self {
            template,
            include_root: None,
        })
    }

    /// Parse a template with include support
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tmpl = natsuzora::Natsuzora::parse_with_includes(
    ///     "{[!include /components/header]}",
    ///     "templates/shared",
    /// ).unwrap();
    /// ```
    pub fn parse_with_includes(source: &str, include_root: impl AsRef<Path>) -> Result<Self> {
        let template = natsuzora_ast::parse(source).map_err(|e| NatsuzoraError::ParseError {
            message: e.to_string(),
            location: Location::default(),
        })?;
        Ok(Self {
            template,
            include_root: Some(include_root.as_ref().to_path_buf()),
        })
    }

    /// Render the template with the given JSON data
    pub fn render(&self, data: serde_json::Value) -> Result<String> {
        let value = Value::from_json(data)?;
        let mut loader = self
            .include_root
            .as_ref()
            .map(TemplateLoader::new)
            .transpose()?;
        let mut renderer = Renderer::new(loader.as_mut());
        renderer.render(&self.template, value)
    }

    /// Get a reference to the parsed template
    pub fn template(&self) -> &Template {
        &self.template
    }
}

/// Convenience function: parse and render in one call
///
/// # Example
///
/// ```rust
/// use serde_json::json;
///
/// let result = natsuzora::render(
///     "Hello, {[ name ]}!",
///     json!({"name": "World"}),
/// ).unwrap();
///
/// assert_eq!(result, "Hello, World!");
/// ```
pub fn render(source: &str, data: serde_json::Value) -> Result<String> {
    Natsuzora::parse(source)?.render(data)
}

/// Convenience function: parse and render with include support
///
/// # Example
///
/// ```rust,ignore
/// use serde_json::json;
///
/// let result = natsuzora::render_with_includes(
///     "{[!include /components/header]}",
///     json!({}),
///     "templates/shared",
/// ).unwrap();
/// ```
pub fn render_with_includes(
    source: &str,
    data: serde_json::Value,
    include_root: impl AsRef<Path>,
) -> Result<String> {
    Natsuzora::parse_with_includes(source, include_root)?.render(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_template_reuse() {
        let tmpl = Natsuzora::parse("Hello, {[ name ]}!").unwrap();

        let result1 = tmpl.render(json!({"name": "Alice"})).unwrap();
        assert_eq!(result1, "Hello, Alice!");

        let result2 = tmpl.render(json!({"name": "Bob"})).unwrap();
        assert_eq!(result2, "Hello, Bob!");
    }
}
