//! Hand-written Lexer + TokenProcessor + Parser for Natsuzora templates.
//!
//! Pipeline: source → Lexer → Token[] → TokenProcessor → Token[] → Parser → AST

mod lexer;
mod parser;
mod token;
mod token_processor;

use std::error::Error;
use std::ops::Range;

use thiserror::Error;

// ============================================================================
// Location
// ============================================================================

/// Location in source code (1-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

impl Location {
    pub fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
        }
    }
}

// ============================================================================
// AST Nodes
// ============================================================================

/// A parsed template consisting of the linear list of nodes.
#[derive(Debug, Clone)]
pub struct Template {
    nodes: Vec<AstNode>,
    location: Location,
}

impl Template {
    pub fn new(nodes: Vec<AstNode>, location: Location) -> Self {
        Self { nodes, location }
    }

    pub fn nodes(&self) -> &[AstNode] {
        &self.nodes
    }

    pub fn location(&self) -> Location {
        self.location
    }
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Text(TextNode),
    Variable(VariableNode),
    Unsecure(UnsecureNode),
    If(IfBlock),
    Unless(UnlessBlock),
    Each(EachBlock),
    Include(IncludeNode),
}

impl AstNode {
    pub fn location(&self) -> Location {
        match self {
            AstNode::Text(n) => n.location,
            AstNode::Variable(n) => n.location,
            AstNode::Unsecure(n) => n.location,
            AstNode::If(n) => n.location,
            AstNode::Unless(n) => n.location,
            AstNode::Each(n) => n.location,
            AstNode::Include(n) => n.location,
        }
    }
}

/// Raw text content.
#[derive(Debug, Clone)]
pub struct TextNode {
    pub content: String,
    pub location: Location,
}

/// Variable output: {[ path ]} or {[ path? ]} or {[ path! ]}
#[derive(Debug, Clone)]
pub struct VariableNode {
    pub path: Path,
    pub modifier: Modifier,
    pub location: Location,
}

/// Unsecure (unescaped) output: {[!unsecure path ]}
#[derive(Debug, Clone)]
pub struct UnsecureNode {
    pub path: Path,
    pub location: Location,
}

/// Conditional block: {[#if condition]} ... {[#else]} ... {[/if]}
#[derive(Debug, Clone)]
pub struct IfBlock {
    pub condition: Path,
    pub then_branch: Vec<AstNode>,
    pub else_branch: Option<Vec<AstNode>>,
    pub location: Location,
}

/// Inverse conditional block: {[#unless condition]} ... {[/unless]}
#[derive(Debug, Clone)]
pub struct UnlessBlock {
    pub condition: Path,
    pub body: Vec<AstNode>,
    pub location: Location,
}

/// Loop block: {[#each collection as item]} ... {[/each]}
#[derive(Debug, Clone)]
pub struct EachBlock {
    pub collection: Path,
    pub item_ident: String,
    pub body: Vec<AstNode>,
    pub location: Location,
}

/// Include directive: {[!include /path key=value ]}
#[derive(Debug, Clone)]
pub struct IncludeNode {
    pub name: String,
    pub args: Vec<IncludeArg>,
    pub location: Location,
}

/// Include argument: key=value
#[derive(Debug, Clone)]
pub struct IncludeArg {
    pub name: String,
    pub value: Path,
    pub location: Location,
}

/// Variable modifier for null/empty handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Modifier {
    /// No modifier: null causes error.
    #[default]
    None,
    /// `?` modifier: null outputs empty string.
    Nullable,
    /// `!` modifier: null or empty string causes error.
    Required,
}

/// A dot-separated path (e.g., user.profile.name).
#[derive(Debug, Clone)]
pub struct Path {
    segments: Vec<String>,
    location: Location,
}

impl Path {
    pub fn new(segments: Vec<String>, location: Location) -> Self {
        Self { segments, location }
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn location(&self) -> Location {
        self.location
    }

    /// Returns the path as a dot-separated string.
    pub fn as_str(&self) -> String {
        self.segments.join(".")
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("template contains syntax errors at line {line}, column {column}")]
    SyntaxError {
        line: usize,
        column: usize,
        byte_range: Range<usize>,
    },

    #[error("{message} at line {line}, column {column}")]
    UnexpectedToken {
        message: String,
        line: usize,
        column: usize,
    },

    #[error("reserved word '{word}' cannot be used as identifier at line {line}, column {column}")]
    ReservedWord {
        word: String,
        line: usize,
        column: usize,
    },

    #[error("identifier cannot start with underscore: '{name}' at line {line}, column {column}")]
    InvalidIdentifier {
        name: String,
        line: usize,
        column: usize,
    },

    #[error("unclosed comment at line {line}, column {column}")]
    UnclosedComment { line: usize, column: usize },
}

/// Reserved words that cannot be used as identifiers.
const RESERVED_WORDS: &[&str] = &[
    "if", "unless", "else", "each", "as", "unsecure", "true", "false", "null", "include", "in",
    "of",
];

/// Check if a word is reserved.
fn is_reserved_word(word: &str) -> bool {
    RESERVED_WORDS.contains(&word)
}

/// Validate an identifier (not reserved, not starting with underscore).
fn validate_identifier(name: &str, location: Location) -> Result<(), ParseError> {
    if is_reserved_word(name) {
        return Err(ParseError::ReservedWord {
            word: name.to_string(),
            line: location.line,
            column: location.column,
        });
    }
    if name.starts_with('_') {
        return Err(ParseError::InvalidIdentifier {
            name: name.to_string(),
            line: location.line,
            column: location.column,
        });
    }
    Ok(())
}

// ============================================================================
// Parsing (public API)
// ============================================================================

/// Parse a template source string into an AST.
pub fn parse(source: &str) -> Result<Template, ParseError> {
    let tokens = lexer::tokenize(source)?;
    let processed = token_processor::process(tokens)?;
    parser::parse(processed)
}

// ============================================================================
// Include Loader
// ============================================================================

/// Error type for include loading operations.
pub type LoaderError = Box<dyn Error + Send + Sync>;

/// Trait for loading included templates.
pub trait IncludeLoader {
    /// Load a template by name.
    fn load(&mut self, name: &str) -> Result<Template, LoaderError>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_text() {
        let template = parse("Hello, World!").unwrap();
        assert_eq!(template.nodes().len(), 1);
        match &template.nodes()[0] {
            AstNode::Text(t) => assert_eq!(t.content, "Hello, World!"),
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn parse_variable_with_location() {
        let template = parse("Hello, {[ user.name ]}!").unwrap();
        assert_eq!(template.nodes().len(), 3);
        match &template.nodes()[1] {
            AstNode::Variable(v) => {
                assert_eq!(v.path.segments(), &["user", "name"]);
                assert_eq!(v.location.line, 1);
                // Location points to the first identifier (after {[ and whitespace)
                assert_eq!(v.location.column, 11);
            }
            _ => panic!("expected variable node"),
        }
    }

    #[test]
    fn parse_variable_with_modifier() {
        let template = parse("{[ name? ]}").unwrap();
        match &template.nodes()[0] {
            AstNode::Variable(v) => {
                assert_eq!(v.modifier, Modifier::Nullable);
            }
            _ => panic!("expected variable node"),
        }

        let template = parse("{[ name! ]}").unwrap();
        match &template.nodes()[0] {
            AstNode::Variable(v) => {
                assert_eq!(v.modifier, Modifier::Required);
            }
            _ => panic!("expected variable node"),
        }
    }

    #[test]
    fn parse_if_block_with_else() {
        let template = parse("{[#if show]}yes{[#else]}no{[/if]}").unwrap();
        assert_eq!(template.nodes().len(), 1);
        match &template.nodes()[0] {
            AstNode::If(block) => {
                assert_eq!(block.condition.segments(), &["show"]);
                assert_eq!(block.then_branch.len(), 1);
                assert!(block.else_branch.is_some());
            }
            _ => panic!("expected if block"),
        }
    }

    #[test]
    fn parse_each_block() {
        let template = parse("{[#each items as item]}{[ item.name ]}{[/each]}").unwrap();
        match &template.nodes()[0] {
            AstNode::Each(block) => {
                assert_eq!(block.collection.segments(), &["items"]);
                assert_eq!(block.item_ident, "item");
                assert_eq!(block.body.len(), 1);
            }
            _ => panic!("expected each block"),
        }
    }

    #[test]
    fn parse_include() {
        let template = parse("{[!include /shared/header title=page.title]}").unwrap();
        match &template.nodes()[0] {
            AstNode::Include(inc) => {
                assert_eq!(inc.name, "/shared/header");
                assert_eq!(inc.args.len(), 1);
                assert_eq!(inc.args[0].name, "title");
                assert_eq!(inc.args[0].value.segments(), &["page", "title"]);
            }
            _ => panic!("expected include node"),
        }
    }

    #[test]
    fn parse_delimiter_escape() {
        let template = parse("literal: {[{]}").unwrap();
        assert_eq!(template.nodes().len(), 1);
        match &template.nodes()[0] {
            AstNode::Text(t) => assert_eq!(t.content, "literal: {["),
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn error_location() {
        let result = parse("{[ invalid.. ]}");
        assert!(result.is_err());
    }
}
