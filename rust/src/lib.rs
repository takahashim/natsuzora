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

// Public modules - part of the API
pub mod ast;
pub mod error;
pub mod template_loader;
pub mod value;

// Internal implementation modules
mod context;
mod html_escape;
mod lexer;
mod parser;
mod renderer;
mod token;
mod validator;

pub use ast::Template;
pub use error::{NatsuzoraError, Result};
pub use template_loader::TemplateLoader;
pub use value::Value;

use lexer::Lexer;
use parser::Parser;
use renderer::Renderer;
use std::path::Path;

/// Main template struct for parsing once and rendering multiple times
pub struct Natsuzora {
    ast: Template,
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
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        Ok(Self {
            ast,
            include_root: None,
        })
    }

    /// Parse a template with include support
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tmpl = natsuzora::Natsuzora::parse_with_includes(
    ///     "{[> /components/header]}",
    ///     "templates/shared",
    /// ).unwrap();
    /// ```
    pub fn parse_with_includes(source: &str, include_root: impl AsRef<Path>) -> Result<Self> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        Ok(Self {
            ast,
            include_root: Some(include_root.as_ref().to_path_buf()),
        })
    }

    /// Render the template with the given JSON data
    pub fn render(&self, data: serde_json::Value) -> Result<String> {
        let value = Value::from_json(data)?;
        let mut loader = self
            .include_root
            .as_ref()
            .map(|p| TemplateLoader::new(p))
            .transpose()?;
        let mut renderer = Renderer::new(&self.ast, loader.as_mut());
        renderer.render(value)
    }

    /// Get a reference to the parsed AST
    pub fn ast(&self) -> &Template {
        &self.ast
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
///     "{[> /components/header]}",
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
    fn test_render_simple() {
        let result = render("Hello, {[ name ]}!", json!({"name": "World"})).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_template_reuse() {
        let tmpl = Natsuzora::parse("Hello, {[ name ]}!").unwrap();

        let result1 = tmpl.render(json!({"name": "Alice"})).unwrap();
        assert_eq!(result1, "Hello, Alice!");

        let result2 = tmpl.render(json!({"name": "Bob"})).unwrap();
        assert_eq!(result2, "Hello, Bob!");
    }

    #[test]
    fn test_html_escaping() {
        let result = render("{[ html ]}", json!({"html": "<b>bold</b>"})).unwrap();
        assert_eq!(result, "&lt;b&gt;bold&lt;/b&gt;");
    }

    #[test]
    fn test_unsecure_block() {
        let result = render(
            "{[#unsecure]}{[ html ]}{[/unsecure]}",
            json!({"html": "<b>bold</b>"}),
        )
        .unwrap();
        assert_eq!(result, "<b>bold</b>");
    }

    #[test]
    fn test_if_block() {
        let result = render("{[#if show]}visible{[/if]}", json!({"show": true})).unwrap();
        assert_eq!(result, "visible");
    }

    #[test]
    fn test_each_block() {
        let result = render(
            "{[#each items as item]}{[ item ]}{[/each]}",
            json!({"items": ["a", "b", "c"]}),
        )
        .unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn test_pagination_example() {
        let template = r#"{[#each pagination.pages as page]}{[#if page.current]}[{[ page.num ]}]{[#else]}{[ page.num ]}{[/if]}{[/each]}"#;
        let data = json!({
            "pagination": {
                "pages": [
                    {"num": 1, "current": false},
                    {"num": 2, "current": true},
                    {"num": 3, "current": false}
                ]
            }
        });
        let result = render(template, data).unwrap();
        assert_eq!(result, "1[2]3");
    }
}
