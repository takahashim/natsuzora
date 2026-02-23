//! Tree-sitter based AST for Natsuzora templates.

use std::error::Error;
use std::ops::Range;

use thiserror::Error;
use tree_sitter::{Node, Parser, Tree};

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

    fn from_node(node: &Node) -> Self {
        let pos = node.start_position();
        Self {
            line: pos.row + 1,
            column: pos.column + 1,
            byte_offset: node.start_byte(),
        }
    }
}

// ============================================================================
// Whitespace Control
// ============================================================================

/// Whitespace control for tags ({[- and -]}).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WhitespaceControl {
    /// Trim whitespace before this tag (from {[-).
    pub trim_before: bool,
    /// Trim whitespace after this tag (from -]}).
    pub trim_after: bool,
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
    Comment(CommentNode),
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
            AstNode::Comment(n) => n.location,
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
    pub whitespace: WhitespaceControl,
    pub location: Location,
}

/// Unsecure (unescaped) output: {[!unsecure path ]}
#[derive(Debug, Clone)]
pub struct UnsecureNode {
    pub path: Path,
    pub whitespace: WhitespaceControl,
    pub location: Location,
}

/// Comment node: {[% ... ]} - carries whitespace control only, renders to empty.
#[derive(Debug, Clone)]
pub struct CommentNode {
    pub whitespace: WhitespaceControl,
    pub location: Location,
}

/// Conditional block: {[#if condition]} ... {[#else]} ... {[/if]}
#[derive(Debug, Clone)]
pub struct IfBlock {
    pub condition: Path,
    pub then_branch: Vec<AstNode>,
    pub else_branch: Option<Vec<AstNode>>,
    pub whitespace_open: WhitespaceControl,
    pub whitespace_else: Option<WhitespaceControl>,
    pub whitespace_close: WhitespaceControl,
    pub location: Location,
}

/// Inverse conditional block: {[#unless condition]} ... {[/unless]}
#[derive(Debug, Clone)]
pub struct UnlessBlock {
    pub condition: Path,
    pub body: Vec<AstNode>,
    pub whitespace_open: WhitespaceControl,
    pub whitespace_close: WhitespaceControl,
    pub location: Location,
}

/// Loop block: {[#each collection as item]} ... {[/each]}
#[derive(Debug, Clone)]
pub struct EachBlock {
    pub collection: Path,
    pub item_ident: String,
    pub body: Vec<AstNode>,
    pub whitespace_open: WhitespaceControl,
    pub whitespace_close: WhitespaceControl,
    pub location: Location,
}

/// Include directive: {[!include /path key=value ]}
#[derive(Debug, Clone)]
pub struct IncludeNode {
    pub name: String,
    pub args: Vec<IncludeArg>,
    pub whitespace: WhitespaceControl,
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
    #[error("tree-sitter parsing failed")]
    ParserInit(#[source] tree_sitter::LanguageError),

    #[error("template contains syntax errors at line {line}, column {column}")]
    SyntaxError {
        line: usize,
        column: usize,
        byte_range: Range<usize>,
    },

    #[error("unexpected tree node '{kind}' at line {line}, column {column}")]
    UnexpectedNode {
        kind: String,
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

    #[error("invalid utf-8 in source")]
    InvalidUtf8(#[from] std::str::Utf8Error),
}

/// Reserved words that cannot be used as identifiers.
const RESERVED_WORDS: &[&str] = &[
    "if", "unless", "else", "each", "as", "unsecure", "true", "false", "null", "include", "in", "of",
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
// Parsing
// ============================================================================

/// Parse a template source string into an AST.
pub fn parse(source: &str) -> Result<Template, ParseError> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_natsuzora::language())
        .map_err(ParseError::ParserInit)?;
    let tree = parser.parse(source, None).unwrap();
    if tree.root_node().has_error() {
        let (location, byte_range) = locate_error(&tree);
        return Err(ParseError::SyntaxError {
            line: location.line,
            column: location.column,
            byte_range,
        });
    }
    build_template(tree, source)
}

fn locate_error(tree: &Tree) -> (Location, Range<usize>) {
    fn find_error_recursive(node: Node) -> Option<Node> {
        if node.is_error() || node.is_missing() {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(error_node) = find_error_recursive(child) {
                return Some(error_node);
            }
        }
        None
    }

    if let Some(error_node) = find_error_recursive(tree.root_node()) {
        (Location::from_node(&error_node), error_node.byte_range())
    } else {
        let root = tree.root_node();
        (Location::from_node(&root), root.byte_range())
    }
}

fn build_template(tree: Tree, source: &str) -> Result<Template, ParseError> {
    let root = tree.root_node();
    let location = Location::from_node(&root);
    let mut cursor = root.walk();
    let mut nodes = Vec::new();
    for child in root.named_children(&mut cursor) {
        if let Some(node) = parse_node(child, source)? {
            nodes.push(node);
        }
    }
    Ok(Template::new(nodes, location))
}

fn parse_node(node: Node, source: &str) -> Result<Option<AstNode>, ParseError> {
    let location = Location::from_node(&node);
    Ok(match node.kind() {
        "text" => Some(AstNode::Text(TextNode {
            content: node.utf8_text(source.as_bytes())?.to_string(),
            location,
        })),
        "delimiter_escape" => Some(AstNode::Text(TextNode {
            content: "{[".to_string(),
            location,
        })),
        "variable" => Some(AstNode::Variable(parse_variable_node(node, source)?)),
        "unsecure_output" => Some(AstNode::Unsecure(parse_unsecure_node(node, source)?)),
        "if_block" => Some(AstNode::If(parse_if_block(node, source)?)),
        "unless_block" => Some(AstNode::Unless(parse_unless_block(node, source)?)),
        "each_block" => Some(AstNode::Each(parse_each_block(node, source)?)),
        "include" => Some(AstNode::Include(parse_include(node, source)?)),
        "comment" => {
            let text = node.utf8_text(source.as_bytes())?;
            let trim_before = text.starts_with("{[-");
            let trim_after = text.ends_with("-]}");
            Some(AstNode::Comment(CommentNode {
                whitespace: WhitespaceControl {
                    trim_before,
                    trim_after,
                },
                location,
            }))
        }
        other => {
            return Err(ParseError::UnexpectedNode {
                kind: other.to_string(),
                line: location.line,
                column: location.column,
            })
        }
    })
}

fn parse_variable_node(node: Node, source: &str) -> Result<VariableNode, ParseError> {
    let location = Location::from_node(&node);
    let path_node = child_by_kind(node, "path").ok_or_else(|| ParseError::UnexpectedNode {
        kind: node.kind().to_string(),
        line: location.line,
        column: location.column,
    })?;
    let modifier = child_by_kind(node, "modifier")
        .map(|m| parse_modifier(m, source))
        .transpose()?
        .unwrap_or(Modifier::None);
    let whitespace = parse_whitespace_control(node, source)?;
    let path = parse_path(path_node, source)?;

    Ok(VariableNode {
        path,
        modifier,
        whitespace,
        location,
    })
}

fn parse_unsecure_node(node: Node, source: &str) -> Result<UnsecureNode, ParseError> {
    let location = Location::from_node(&node);
    let path_node = child_by_kind(node, "path").ok_or_else(|| ParseError::UnexpectedNode {
        kind: node.kind().to_string(),
        line: location.line,
        column: location.column,
    })?;
    let whitespace = parse_whitespace_control(node, source)?;
    let path = parse_path(path_node, source)?;

    Ok(UnsecureNode {
        path,
        whitespace,
        location,
    })
}

fn parse_if_block(node: Node, source: &str) -> Result<IfBlock, ParseError> {
    let location = Location::from_node(&node);
    let mut cursor = node.walk();
    let mut condition = None;
    let mut then_branch = Vec::new();
    let mut else_branch = None;
    let mut whitespace_open = WhitespaceControl::default();
    let mut whitespace_else = None;
    let mut whitespace_close = WhitespaceControl::default();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "if_open" => {
                let path_node =
                    child_by_kind(child, "path").ok_or_else(|| ParseError::UnexpectedNode {
                        kind: child.kind().to_string(),
                        line: Location::from_node(&child).line,
                        column: Location::from_node(&child).column,
                    })?;
                condition = Some(parse_path(path_node, source)?);
                whitespace_open = parse_whitespace_control(child, source)?;
            }
            "else_clause" => {
                let (ws_else, nodes) = parse_else_clause(child, source)?;
                whitespace_else = Some(ws_else);
                else_branch = Some(nodes);
            }
            "if_close" => {
                whitespace_close = parse_whitespace_control(child, source)?;
            }
            _ => {
                if let Some(node) = parse_node(child, source)? {
                    then_branch.push(node);
                }
            }
        }
    }

    Ok(IfBlock {
        condition: condition.ok_or_else(|| ParseError::UnexpectedNode {
            kind: "if_block".to_string(),
            line: location.line,
            column: location.column,
        })?,
        then_branch,
        else_branch,
        whitespace_open,
        whitespace_else,
        whitespace_close,
        location,
    })
}

fn parse_else_clause(
    node: Node,
    source: &str,
) -> Result<(WhitespaceControl, Vec<AstNode>), ParseError> {
    let mut cursor = node.walk();
    let mut nodes = Vec::new();
    let mut ws = WhitespaceControl::default();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "else_open" {
            ws = parse_whitespace_control(child, source)?;
            continue;
        }
        if let Some(node) = parse_node(child, source)? {
            nodes.push(node);
        }
    }
    Ok((ws, nodes))
}

fn parse_unless_block(node: Node, source: &str) -> Result<UnlessBlock, ParseError> {
    let location = Location::from_node(&node);
    let mut cursor = node.walk();
    let mut condition = None;
    let mut body = Vec::new();
    let mut whitespace_open = WhitespaceControl::default();
    let mut whitespace_close = WhitespaceControl::default();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "unless_open" => {
                let path_node =
                    child_by_kind(child, "path").ok_or_else(|| ParseError::UnexpectedNode {
                        kind: child.kind().to_string(),
                        line: Location::from_node(&child).line,
                        column: Location::from_node(&child).column,
                    })?;
                condition = Some(parse_path(path_node, source)?);
                whitespace_open = parse_whitespace_control(child, source)?;
            }
            "unless_close" => {
                whitespace_close = parse_whitespace_control(child, source)?;
            }
            _ => {
                if let Some(node) = parse_node(child, source)? {
                    body.push(node);
                }
            }
        }
    }

    Ok(UnlessBlock {
        condition: condition.ok_or_else(|| ParseError::UnexpectedNode {
            kind: "unless_block".to_string(),
            line: location.line,
            column: location.column,
        })?,
        body,
        whitespace_open,
        whitespace_close,
        location,
    })
}

fn parse_each_block(node: Node, source: &str) -> Result<EachBlock, ParseError> {
    let location = Location::from_node(&node);
    let mut cursor = node.walk();
    let mut header = None;
    let mut body = Vec::new();
    let mut whitespace_open = WhitespaceControl::default();
    let mut whitespace_close = WhitespaceControl::default();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "each_open" => {
                header = Some(parse_each_open(child, source)?);
                whitespace_open = parse_whitespace_control(child, source)?;
            }
            "each_close" => {
                whitespace_close = parse_whitespace_control(child, source)?;
            }
            _ => {
                if let Some(node) = parse_node(child, source)? {
                    body.push(node);
                }
            }
        }
    }

    let (collection, item_ident) = header.ok_or_else(|| ParseError::UnexpectedNode {
        kind: "each_block".to_string(),
        line: location.line,
        column: location.column,
    })?;

    Ok(EachBlock {
        collection,
        item_ident,
        body,
        whitespace_open,
        whitespace_close,
        location,
    })
}

fn parse_each_open(node: Node, source: &str) -> Result<(Path, String), ParseError> {
    let location = Location::from_node(&node);
    let path_node = child_by_kind(node, "path").ok_or_else(|| ParseError::UnexpectedNode {
        kind: node.kind().to_string(),
        line: location.line,
        column: location.column,
    })?;
    let ident_node = child_by_kind(node, "identifier").ok_or_else(|| ParseError::UnexpectedNode {
        kind: node.kind().to_string(),
        line: location.line,
        column: location.column,
    })?;
    let ident_location = Location::from_node(&ident_node);
    let item_ident = ident_node.utf8_text(source.as_bytes())?.to_string();
    validate_identifier(&item_ident, ident_location)?;
    Ok((parse_path(path_node, source)?, item_ident))
}

fn parse_include(node: Node, source: &str) -> Result<IncludeNode, ParseError> {
    let location = Location::from_node(&node);
    let mut cursor = node.walk();
    let mut name = None;
    let mut args = Vec::new();
    let whitespace = parse_whitespace_control(node, source)?;

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "include_name" => {
                let name_text = child.utf8_text(source.as_bytes())?;
                let seg_location = Location::from_node(&child);
                // Validate each segment in the include path
                for seg_name in name_text.split('/').filter(|s| !s.is_empty()) {
                    if seg_name.starts_with('_') {
                        return Err(ParseError::InvalidIdentifier {
                            name: seg_name.to_string(),
                            line: seg_location.line,
                            column: seg_location.column,
                        });
                    }
                }
                name = Some(name_text.to_string());
            }
            "include_args" => {
                let mut arg_cursor = child.walk();
                for arg in child.named_children(&mut arg_cursor) {
                    if arg.kind() == "include_arg" {
                        let arg_location = Location::from_node(&arg);
                        let key_node =
                            arg.named_child(0)
                                .ok_or_else(|| ParseError::UnexpectedNode {
                                    kind: arg.kind().to_string(),
                                    line: arg_location.line,
                                    column: arg_location.column,
                                })?;
                        let path_node =
                            arg.named_child(1)
                                .ok_or_else(|| ParseError::UnexpectedNode {
                                    kind: arg.kind().to_string(),
                                    line: arg_location.line,
                                    column: arg_location.column,
                                })?;
                        let key_location = Location::from_node(&key_node);
                        let key_name = key_node.utf8_text(source.as_bytes())?.to_string();
                        validate_identifier(&key_name, key_location)?;
                        args.push(IncludeArg {
                            name: key_name,
                            value: parse_path(path_node, source)?,
                            location: arg_location,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(IncludeNode {
        name: name.ok_or_else(|| ParseError::UnexpectedNode {
            kind: "include".to_string(),
            line: location.line,
            column: location.column,
        })?,
        args,
        whitespace,
        location,
    })
}

fn parse_path(node: Node, source: &str) -> Result<Path, ParseError> {
    let location = Location::from_node(&node);
    let mut cursor = node.walk();
    let mut segments = Vec::new();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let ident_location = Location::from_node(&child);
            let ident = child.utf8_text(source.as_bytes())?.to_string();
            validate_identifier(&ident, ident_location)?;
            segments.push(ident);
        }
    }

    if segments.is_empty() {
        return Err(ParseError::UnexpectedNode {
            kind: "path".to_string(),
            line: location.line,
            column: location.column,
        });
    }

    Ok(Path::new(segments, location))
}

fn parse_whitespace_control(node: Node, source: &str) -> Result<WhitespaceControl, ParseError> {
    let mut trim_before = false;
    let mut trim_after = false;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "tag_open" => {
                let text = child.utf8_text(source.as_bytes())?;
                trim_before = text == "{[-";
            }
            "tag_close" => {
                let text = child.utf8_text(source.as_bytes())?;
                trim_after = text == "-]}";
            }
            _ => {}
        }
    }

    Ok(WhitespaceControl {
        trim_before,
        trim_after,
    })
}

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

fn parse_modifier(node: Node, source: &str) -> Result<Modifier, ParseError> {
    let location = Location::from_node(&node);
    let text = node.utf8_text(source.as_bytes())?;
    match text {
        "?" => Ok(Modifier::Nullable),
        "!" => Ok(Modifier::Required),
        _ => Err(ParseError::UnexpectedNode {
            kind: format!("modifier({text})"),
            line: location.line,
            column: location.column,
        }),
    }
}

// ============================================================================
// Include Loader
// ============================================================================

/// Error type for include loading operations.
pub type LoaderError = Box<dyn Error + Send + Sync>;

/// Trait for loading included templates.
///
/// Implementations of this trait are responsible for:
/// - Resolving template names to file paths
/// - Reading and parsing template files
/// - Caching loaded templates (optional)
/// - Detecting circular includes (optional)
pub trait IncludeLoader {
    /// Load a template by name.
    ///
    /// The `name` parameter is the include path as written in the template,
    /// e.g., `/components/header` for `{[!include /components/header]}`.
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
                assert_eq!(v.location.column, 8);
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
    fn parse_whitespace_control() {
        let template = parse("{[- name -]}").unwrap();
        match &template.nodes()[0] {
            AstNode::Variable(v) => {
                assert!(v.whitespace.trim_before);
                assert!(v.whitespace.trim_after);
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
        assert_eq!(template.nodes().len(), 2);
        match &template.nodes()[1] {
            AstNode::Text(t) => assert_eq!(t.content, "{["),
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn error_location() {
        let result = parse("{[ invalid.. ]}");
        assert!(result.is_err());
        if let Err(ParseError::SyntaxError { line, column, .. }) = result {
            assert_eq!(line, 1);
            assert!(column > 0);
        }
    }
}
