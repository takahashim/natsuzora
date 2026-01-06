use crate::ast::{
    EachBlockNode, IfBlockNode, IncludeNode, Node, Template, TextNode, UnlessBlockNode,
    UnsecureBlockNode, VariableNode,
};
use crate::error::{Location, NatsuzoraError, Result};
use crate::token::{Token, TokenKind};
use crate::validator;

/// Recursive descent parser for Natsuzora templates
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a new parser from a token stream
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse the token stream into an AST
    pub fn parse(&mut self) -> Result<Template> {
        let nodes = self.parse_nodes()?;
        Ok(Template {
            nodes,
            location: Location::new(1, 1),
        })
    }

    fn parse_nodes(&mut self) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while !self.is_eof() && !self.block_close(None)? {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<Node> {
        match &self.current_token().kind {
            TokenKind::Text(_) => self.parse_text(),
            TokenKind::Open => self.parse_mustache(),
            _ => self.unexpected_token(None),
        }
    }

    fn parse_text(&mut self) -> Result<Node> {
        let token = self.consume_text()?;
        match token.kind {
            TokenKind::Text(content) => Ok(Node::Text(TextNode {
                content,
                location: token.location,
            })),
            _ => Err(NatsuzoraError::ParseError {
                message: format!("Internal error: expected text token, got {:?}", token.kind),
                location: token.location,
            }),
        }
    }

    fn parse_mustache(&mut self) -> Result<Node> {
        let open_token = self.consume(TokenKind::Open)?;

        // Check for illegal whitespace before special characters (#, /, >)
        if matches!(self.current_token().kind, TokenKind::Whitespace(_)) {
            let saved_pos = self.pos;
            self.skip_whitespace();
            let is_special = matches!(
                self.current_token().kind,
                TokenKind::Hash | TokenKind::Slash | TokenKind::Gt
            );
            if is_special {
                return Err(NatsuzoraError::ParseError {
                    message: format!(
                        "Whitespace not allowed after '{{[' before '{:?}'",
                        self.current_token().kind
                    ),
                    location: open_token.location,
                });
            }
            self.pos = saved_pos;
        }

        self.skip_whitespace();

        match &self.current_token().kind {
            TokenKind::Hash => self.parse_block_open(),
            TokenKind::Slash => self.unexpected_token(Some("Unexpected block close")),
            TokenKind::Gt => self.parse_include(),
            _ => self.parse_variable_node(),
        }
    }

    fn parse_block_open(&mut self) -> Result<Node> {
        self.consume(TokenKind::Hash)?;
        self.skip_whitespace();

        match &self.current_token().kind {
            TokenKind::KwIf => self.parse_if_block(),
            TokenKind::KwUnless => self.parse_unless_block(),
            TokenKind::KwEach => self.parse_each_block(),
            TokenKind::KwUnsecure => self.parse_unsecure_block(),
            TokenKind::KwElse => self.unexpected_token(Some("Unexpected 'else' without 'if'")),
            _ => self.unexpected_token(None),
        }
    }

    fn parse_if_block(&mut self) -> Result<Node> {
        let token = self.consume(TokenKind::KwIf)?;
        let location = token.location;

        self.consume_required_whitespace()?;
        let condition = self.parse_path()?;
        self.skip_whitespace();
        self.consume(TokenKind::Close)?;

        let then_nodes = self.parse_if_body()?;
        let else_nodes = if self.else_open()? {
            self.consume_else()?;
            Some(self.parse_if_body()?)
        } else {
            None
        };

        self.consume_block_close(Some(TokenKind::KwIf))?;

        Ok(Node::IfBlock(IfBlockNode {
            condition,
            then_nodes,
            else_nodes,
            location,
        }))
    }

    fn parse_if_body(&mut self) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while !self.block_close(Some(TokenKind::KwIf))? && !self.else_open()? {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_unless_block(&mut self) -> Result<Node> {
        let token = self.consume(TokenKind::KwUnless)?;
        let location = token.location;

        self.consume_required_whitespace()?;
        let condition = self.parse_path()?;
        self.skip_whitespace();
        self.consume(TokenKind::Close)?;

        let body_nodes = self.parse_unless_body()?;

        self.consume_block_close(Some(TokenKind::KwUnless))?;

        Ok(Node::UnlessBlock(UnlessBlockNode {
            condition,
            body_nodes,
            location,
        }))
    }

    fn parse_unless_body(&mut self) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while !self.block_close(Some(TokenKind::KwUnless))? {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn else_open(&mut self) -> Result<bool> {
        if !matches!(self.current_token().kind, TokenKind::Open) {
            return Ok(false);
        }

        let saved_pos = self.pos;
        self.advance_token(); // open

        // If there's whitespace after {[, it cannot be {[#else]}
        if matches!(self.current_token().kind, TokenKind::Whitespace(_)) {
            self.pos = saved_pos;
            return Ok(false);
        }

        let result = if matches!(self.current_token().kind, TokenKind::Hash) {
            self.advance_token(); // hash
            self.skip_whitespace();
            matches!(self.current_token().kind, TokenKind::KwElse)
        } else {
            false
        };

        self.pos = saved_pos;
        Ok(result)
    }

    fn consume_else(&mut self) -> Result<()> {
        self.consume(TokenKind::Open)?;
        self.skip_whitespace();
        self.consume(TokenKind::Hash)?;
        self.skip_whitespace();
        self.consume(TokenKind::KwElse)?;
        self.skip_whitespace();
        self.consume(TokenKind::Close)?;
        Ok(())
    }

    fn parse_each_block(&mut self) -> Result<Node> {
        let token = self.consume(TokenKind::KwEach)?;
        let location = token.location;

        self.consume_required_whitespace()?;
        let collection = self.parse_path()?;
        self.consume_required_whitespace()?;
        self.consume(TokenKind::KwAs)?;
        self.consume_required_whitespace()?;
        let item_name = self.parse_identifier_with_validation()?;

        self.skip_whitespace();
        let index_name = if matches!(self.current_token().kind, TokenKind::Comma) {
            self.consume(TokenKind::Comma)?;
            self.skip_whitespace();
            let idx = self.parse_identifier_with_validation()?;

            if item_name == idx {
                return Err(NatsuzoraError::ParseError {
                    message: format!("Item and index cannot have the same name: '{}'", item_name),
                    location,
                });
            }

            Some(idx)
        } else {
            None
        };

        self.skip_whitespace();
        self.consume(TokenKind::Close)?;

        let body_nodes = self.parse_each_body()?;

        self.consume_block_close(Some(TokenKind::KwEach))?;

        Ok(Node::EachBlock(EachBlockNode {
            collection,
            item_name,
            index_name,
            body_nodes,
            location,
        }))
    }

    fn parse_each_body(&mut self) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while !self.block_close(Some(TokenKind::KwEach))? {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_unsecure_block(&mut self) -> Result<Node> {
        let token = self.consume(TokenKind::KwUnsecure)?;
        let location = token.location;

        self.skip_whitespace();
        self.consume(TokenKind::Close)?;

        let mut body_nodes = Vec::new();
        while !self.block_close(Some(TokenKind::KwUnsecure))? {
            body_nodes.push(self.parse_node()?);
        }

        self.consume_block_close(Some(TokenKind::KwUnsecure))?;

        Ok(Node::UnsecureBlock(UnsecureBlockNode {
            nodes: body_nodes,
            location,
        }))
    }

    fn parse_include(&mut self) -> Result<Node> {
        let token = self.consume(TokenKind::Gt)?;
        let location = token.location;

        self.skip_whitespace();
        let name = self.parse_include_name()?;
        let args = self.parse_include_args()?;
        self.skip_whitespace();
        self.consume(TokenKind::Close)?;

        Ok(Node::Include(IncludeNode {
            name,
            args,
            location,
        }))
    }

    fn parse_include_name(&mut self) -> Result<String> {
        let token = self.current_token().clone();
        let name = match &token.kind {
            TokenKind::Ident(s) if s.starts_with('/') => s.clone(),
            _ => {
                return Err(NatsuzoraError::ParseError {
                    message: "Include name must start with '/'".to_string(),
                    location: token.location,
                });
            }
        };

        validator::validate_include_name_syntax(&name, &token.location)?;
        self.advance_token();
        Ok(name)
    }

    fn parse_include_args(&mut self) -> Result<Vec<(String, VariableNode)>> {
        let mut args = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        while matches!(self.current_token().kind, TokenKind::Whitespace(_)) {
            self.skip_whitespace();

            // Check if it's an identifier (not starting with /)
            let is_arg = match &self.current_token().kind {
                TokenKind::Ident(s) => !s.starts_with('/'),
                _ => false,
            };

            if !is_arg {
                break;
            }

            let key_token = self.consume_ident()?;
            let key = match key_token.kind {
                TokenKind::Ident(s) => s,
                _ => {
                    return Err(NatsuzoraError::ParseError {
                        message: format!(
                            "Internal error: expected identifier, got {:?}",
                            key_token.kind
                        ),
                        location: key_token.location,
                    });
                }
            };

            validator::validate_identifier(&key, &key_token.location)?;

            if seen_keys.contains(&key) {
                return Err(NatsuzoraError::ParseError {
                    message: format!("Duplicate include argument: {}", key),
                    location: key_token.location,
                });
            }
            seen_keys.insert(key.clone());

            self.skip_whitespace();
            self.consume(TokenKind::Equal)?;
            self.skip_whitespace();
            let value = self.parse_path()?;
            args.push((key, value));
        }

        Ok(args)
    }

    fn parse_variable_node(&mut self) -> Result<Node> {
        let path = self.parse_path()?;
        self.skip_whitespace();
        self.consume(TokenKind::Close)?;
        Ok(Node::Variable(path))
    }

    fn parse_path(&mut self) -> Result<VariableNode> {
        let first_token = self.current_token().clone();
        let mut segments = vec![self.parse_identifier_with_validation()?];

        while matches!(self.current_token().kind, TokenKind::Dot) {
            self.consume(TokenKind::Dot)?;
            segments.push(self.parse_identifier_with_validation()?);
        }

        Ok(VariableNode {
            path: segments,
            location: first_token.location,
        })
    }

    fn parse_identifier_with_validation(&mut self) -> Result<String> {
        let token = self.current_token().clone();

        // Check if it's a keyword (reserved word used as identifier)
        if self.is_keyword_token(&token) {
            self.advance_token();
            let word = self.keyword_to_string(&token.kind);
            return Err(NatsuzoraError::ReservedWordError {
                word,
                location: token.location,
            });
        }

        let token = self.consume_ident()?;
        let name = match &token.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => {
                return Err(NatsuzoraError::ParseError {
                    message: format!("Internal error: expected identifier, got {:?}", token.kind),
                    location: token.location,
                });
            }
        };

        validator::validate_identifier(&name, &token.location)?;
        Ok(name)
    }

    fn is_keyword_token(&self, token: &Token) -> bool {
        matches!(
            token.kind,
            TokenKind::KwIf
                | TokenKind::KwUnless
                | TokenKind::KwElse
                | TokenKind::KwEach
                | TokenKind::KwAs
                | TokenKind::KwUnsecure
        )
    }

    fn keyword_to_string(&self, kind: &TokenKind) -> String {
        match kind {
            TokenKind::KwIf => "if".to_string(),
            TokenKind::KwUnless => "unless".to_string(),
            TokenKind::KwElse => "else".to_string(),
            TokenKind::KwEach => "each".to_string(),
            TokenKind::KwAs => "as".to_string(),
            TokenKind::KwUnsecure => "unsecure".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn block_close(&mut self, keyword: Option<TokenKind>) -> Result<bool> {
        if !matches!(self.current_token().kind, TokenKind::Open) {
            return Ok(false);
        }

        let saved_pos = self.pos;
        self.advance_token(); // open

        // If there's whitespace after {[, it cannot be a block close
        if matches!(self.current_token().kind, TokenKind::Whitespace(_)) {
            self.pos = saved_pos;
            return Ok(false);
        }

        let result = if matches!(self.current_token().kind, TokenKind::Slash) {
            if let Some(kw) = keyword {
                self.advance_token(); // slash
                self.skip_whitespace();
                self.current_token().kind == kw
            } else {
                true
            }
        } else {
            false
        };

        self.pos = saved_pos;
        Ok(result)
    }

    fn consume_block_close(&mut self, keyword: Option<TokenKind>) -> Result<()> {
        self.consume(TokenKind::Open)?;
        self.skip_whitespace();
        self.consume(TokenKind::Slash)?;
        self.skip_whitespace();
        if let Some(kw) = keyword {
            self.consume(kw)?;
        }
        self.skip_whitespace();
        self.consume(TokenKind::Close)?;
        Ok(())
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn is_eof(&self) -> bool {
        matches!(self.current_token().kind, TokenKind::Eof)
    }

    fn advance_token(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token> {
        let token = self.current_token().clone();
        if std::mem::discriminant(&token.kind) != std::mem::discriminant(&expected) {
            return Err(NatsuzoraError::ParseError {
                message: format!("Expected {:?}, got {:?}", expected, token.kind),
                location: token.location,
            });
        }
        self.advance_token();
        Ok(token)
    }

    fn consume_text(&mut self) -> Result<Token> {
        let token = self.current_token().clone();
        if !matches!(token.kind, TokenKind::Text(_)) {
            return Err(NatsuzoraError::ParseError {
                message: format!("Expected text, got {:?}", token.kind),
                location: token.location,
            });
        }
        self.advance_token();
        Ok(token)
    }

    fn consume_ident(&mut self) -> Result<Token> {
        let token = self.current_token().clone();
        if !matches!(token.kind, TokenKind::Ident(_)) {
            return Err(NatsuzoraError::ParseError {
                message: format!("Expected identifier, got {:?}", token.kind),
                location: token.location,
            });
        }
        self.advance_token();
        Ok(token)
    }

    fn consume_required_whitespace(&mut self) -> Result<()> {
        if !matches!(self.current_token().kind, TokenKind::Whitespace(_)) {
            return Err(NatsuzoraError::ParseError {
                message: "Expected whitespace".to_string(),
                location: self.current_token().location,
            });
        }
        self.skip_whitespace();
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.current_token().kind, TokenKind::Whitespace(_)) {
            self.advance_token();
        }
    }

    fn unexpected_token<T>(&self, message: Option<&str>) -> Result<T> {
        let token = self.current_token();
        let msg = match message {
            Some(m) => format!("{}: {:?}", m, token.kind),
            None => format!("Unexpected token: {:?}", token.kind),
        };
        Err(NatsuzoraError::ParseError {
            message: msg,
            location: token.location,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Template> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_parse_text() {
        let template = parse("Hello, world!").unwrap();
        assert_eq!(template.nodes.len(), 1);
        if let Node::Text(node) = &template.nodes[0] {
            assert_eq!(node.content, "Hello, world!");
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_parse_variable() {
        let template = parse("{[ name ]}").unwrap();
        assert_eq!(template.nodes.len(), 1);
        if let Node::Variable(node) = &template.nodes[0] {
            assert_eq!(node.path, vec!["name"]);
        } else {
            panic!("Expected Variable node");
        }
    }

    #[test]
    fn test_parse_path() {
        let template = parse("{[ user.profile.name ]}").unwrap();
        if let Node::Variable(node) = &template.nodes[0] {
            assert_eq!(node.path, vec!["user", "profile", "name"]);
        } else {
            panic!("Expected Variable node");
        }
    }

    #[test]
    fn test_parse_if_block() {
        let template = parse("{[#if visible]}Hello{[/if]}").unwrap();
        if let Node::IfBlock(node) = &template.nodes[0] {
            assert_eq!(node.condition.path, vec!["visible"]);
            assert_eq!(node.then_nodes.len(), 1);
            assert!(node.else_nodes.is_none());
        } else {
            panic!("Expected IfBlock node");
        }
    }

    #[test]
    fn test_parse_if_else_block() {
        let template = parse("{[#if visible]}Yes{[#else]}No{[/if]}").unwrap();
        if let Node::IfBlock(node) = &template.nodes[0] {
            assert!(node.else_nodes.is_some());
            assert_eq!(node.else_nodes.as_ref().unwrap().len(), 1);
        } else {
            panic!("Expected IfBlock node");
        }
    }

    #[test]
    fn test_parse_unless_block() {
        let template = parse("{[#unless hidden]}visible{[/unless]}").unwrap();
        if let Node::UnlessBlock(node) = &template.nodes[0] {
            assert_eq!(node.condition.path, vec!["hidden"]);
            assert_eq!(node.body_nodes.len(), 1);
        } else {
            panic!("Expected UnlessBlock node");
        }
    }

    #[test]
    fn test_parse_nested_unless_blocks() {
        let template = parse("{[#unless a]}{[#unless b]}inner{[/unless]}{[/unless]}").unwrap();
        if let Node::UnlessBlock(outer) = &template.nodes[0] {
            assert_eq!(outer.body_nodes.len(), 1);
            if let Node::UnlessBlock(inner) = &outer.body_nodes[0] {
                assert_eq!(inner.condition.path, vec!["b"]);
            } else {
                panic!("Expected inner UnlessBlock node");
            }
        } else {
            panic!("Expected outer UnlessBlock node");
        }
    }

    #[test]
    fn test_parse_each_block() {
        let template = parse("{[#each items as item]}{[ item ]}{[/each]}").unwrap();
        if let Node::EachBlock(node) = &template.nodes[0] {
            assert_eq!(node.collection.path, vec!["items"]);
            assert_eq!(node.item_name, "item");
            assert!(node.index_name.is_none());
        } else {
            panic!("Expected EachBlock node");
        }
    }

    #[test]
    fn test_parse_each_with_index() {
        let template = parse("{[#each items as item, idx]}{[ idx ]}{[/each]}").unwrap();
        if let Node::EachBlock(node) = &template.nodes[0] {
            assert_eq!(node.index_name, Some("idx".to_string()));
        } else {
            panic!("Expected EachBlock node");
        }
    }

    #[test]
    fn test_reserved_word_error() {
        let result = parse("{[ if ]}");
        assert!(result.is_err());
        if let Err(NatsuzoraError::ReservedWordError { word, .. }) = result {
            assert_eq!(word, "if");
        } else {
            panic!("Expected ReservedWordError");
        }
    }
}
