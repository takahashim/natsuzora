//! Recursive descent parser for Natsuzora templates.
//!
//! Mirrors Ruby's `Parser` class. Consumes a processed token stream
//! (after TokenProcessor) and produces an AST.

use crate::token::{Token, TokenType};
use crate::{
    validate_identifier, AstNode, EachBlock, IfBlock, IncludeArg, IncludeNode, Location, Modifier,
    ParseError, Path, Template, TextNode, UnlessBlock, UnsecureNode, VariableNode,
};

/// Parse a processed token stream into an AST Template.
pub fn parse(tokens: Vec<Token>) -> Result<Template, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse(&mut self) -> Result<Template, ParseError> {
        let nodes = self.parse_nodes()?;
        Ok(Template::new(nodes, Location::new(1, 1, 0)))
    }

    fn parse_nodes(&mut self) -> Result<Vec<AstNode>, ParseError> {
        let mut nodes = Vec::new();
        while self.current_type() != TokenType::Eof && !self.is_block_close(None) {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<AstNode, ParseError> {
        match self.current_type() {
            TokenType::Text => {
                let node = self.parse_text()?;
                Ok(node)
            }
            TokenType::Hash
            | TokenType::Slash
            | TokenType::BangUnsecure
            | TokenType::BangInclude
            | TokenType::Ident
            | TokenType::KwIf
            | TokenType::KwUnless
            | TokenType::KwElse
            | TokenType::KwEach
            | TokenType::KwAs
            | TokenType::Whitespace
            | TokenType::Question
            | TokenType::Exclamation
            | TokenType::Dot
            | TokenType::Equal
            | TokenType::Comma => self.parse_tag_content(),
            _ => self.unexpected_token(None),
        }
    }

    fn parse_text(&mut self) -> Result<AstNode, ParseError> {
        let token = self.consume(TokenType::Text)?;
        Ok(AstNode::Text(TextNode {
            content: token.value,
            location: token.location,
        }))
    }

    fn parse_tag_content(&mut self) -> Result<AstNode, ParseError> {
        if self.current_type() == TokenType::Whitespace {
            self.check_no_whitespace_before_special()?;
        }

        self.skip_whitespace();

        match self.current_type() {
            TokenType::Hash => self.parse_block_open(),
            TokenType::Slash => self.unexpected_token(Some("Unexpected block close")),
            TokenType::BangUnsecure => self.parse_unsecure_output(),
            TokenType::BangInclude => self.parse_include(),
            _ => self.parse_variable_node(),
        }
    }

    fn check_no_whitespace_before_special(&mut self) -> Result<(), ParseError> {
        let ws_loc = self.current_location();
        let saved_pos = self.pos;
        self.skip_whitespace();
        let special = matches!(
            self.current_type(),
            TokenType::Hash | TokenType::Slash | TokenType::BangUnsecure | TokenType::BangInclude
        );
        self.pos = saved_pos;
        if special {
            return Err(ParseError::UnexpectedToken {
                message: format!(
                    "Whitespace not allowed before '{}' after tag open",
                    self.peek_after_whitespace_value()
                ),
                line: ws_loc.line,
                column: ws_loc.column,
            });
        }
        Ok(())
    }

    fn peek_after_whitespace_value(&self) -> String {
        let mut p = self.pos;
        while p < self.tokens.len() && self.tokens[p].token_type == TokenType::Whitespace {
            p += 1;
        }
        if p < self.tokens.len() {
            self.tokens[p].value.clone()
        } else {
            String::new()
        }
    }

    fn parse_block_open(&mut self) -> Result<AstNode, ParseError> {
        self.consume(TokenType::Hash)?;
        self.skip_whitespace();

        match self.current_type() {
            TokenType::KwIf => self.parse_if_block(),
            TokenType::KwUnless => self.parse_unless_block(),
            TokenType::KwEach => self.parse_each_block(),
            TokenType::KwElse => {
                self.unexpected_token(Some("Unexpected 'else' without 'if'"))
            }
            _ => self.unexpected_token(None),
        }
    }

    fn parse_if_block(&mut self) -> Result<AstNode, ParseError> {
        let kw_token = self.consume(TokenType::KwIf)?;
        let location = kw_token.location;

        self.consume_required_whitespace()?;
        let condition = self.parse_path()?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;

        let then_nodes = self.parse_if_body()?;
        let mut else_nodes = None;

        if self.is_else_open() {
            self.consume_else()?;
            else_nodes = Some(self.parse_if_body()?);
        }

        self.consume_block_close(TokenType::KwIf)?;

        Ok(AstNode::If(IfBlock {
            condition,
            then_branch: then_nodes,
            else_branch: else_nodes,
            location,
        }))
    }

    fn parse_if_body(&mut self) -> Result<Vec<AstNode>, ParseError> {
        let mut nodes = Vec::new();
        while !self.is_block_close(Some(TokenType::KwIf)) && !self.is_else_open() {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_unless_block(&mut self) -> Result<AstNode, ParseError> {
        let kw_token = self.consume(TokenType::KwUnless)?;
        let location = kw_token.location;

        self.consume_required_whitespace()?;
        let condition = self.parse_path()?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;

        let body = self.parse_unless_body()?;
        self.consume_block_close(TokenType::KwUnless)?;

        Ok(AstNode::Unless(UnlessBlock {
            condition,
            body,
            location,
        }))
    }

    fn parse_unless_body(&mut self) -> Result<Vec<AstNode>, ParseError> {
        let mut nodes = Vec::new();
        while !self.is_block_close(Some(TokenType::KwUnless)) {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_each_block(&mut self) -> Result<AstNode, ParseError> {
        let kw_token = self.consume(TokenType::KwEach)?;
        let location = kw_token.location;

        self.consume_required_whitespace()?;
        let collection = self.parse_path()?;
        self.consume_required_whitespace()?;
        self.consume(TokenType::KwAs)?;
        self.consume_required_whitespace()?;
        let item_name = self.parse_identifier_with_validation()?;

        self.skip_whitespace();
        self.consume(TokenType::Close)?;

        let body = self.parse_each_body()?;
        self.consume_block_close(TokenType::KwEach)?;

        Ok(AstNode::Each(EachBlock {
            collection,
            item_ident: item_name,
            body,
            location,
        }))
    }

    fn parse_each_body(&mut self) -> Result<Vec<AstNode>, ParseError> {
        let mut nodes = Vec::new();
        while !self.is_block_close(Some(TokenType::KwEach)) {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_unsecure_output(&mut self) -> Result<AstNode, ParseError> {
        let token = self.consume(TokenType::BangUnsecure)?;
        let location = token.location;

        self.consume_required_whitespace()?;
        let path = self.parse_path()?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;

        Ok(AstNode::Unsecure(UnsecureNode { path, location }))
    }

    fn parse_include(&mut self) -> Result<AstNode, ParseError> {
        let token = self.consume(TokenType::BangInclude)?;
        let location = token.location;

        self.consume_required_whitespace()?;
        let name = self.parse_include_name()?;
        let args = self.parse_include_args()?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;

        Ok(AstNode::Include(IncludeNode {
            name,
            args,
            location,
        }))
    }

    fn parse_include_name(&mut self) -> Result<String, ParseError> {
        let first_loc = self.current_location();
        if self.current_type() != TokenType::Slash {
            return Err(ParseError::UnexpectedToken {
                message: "Include name must start with '/'".to_string(),
                line: first_loc.line,
                column: first_loc.column,
            });
        }

        let mut path = String::new();
        path.push_str(&self.parse_include_segment()?);
        while self.current_type() == TokenType::Slash {
            path.push_str(&self.parse_include_segment()?);
        }

        // Validate: no underscore-started segments
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            if seg.starts_with('_') {
                return Err(ParseError::InvalidIdentifier {
                    name: seg.to_string(),
                    line: first_loc.line,
                    column: first_loc.column,
                });
            }
        }

        Ok(path)
    }

    fn parse_include_segment(&mut self) -> Result<String, ParseError> {
        self.consume(TokenType::Slash)?;

        let loc = self.current_location();
        if self.current_type() != TokenType::Ident {
            return Err(ParseError::UnexpectedToken {
                message: "Expected identifier after /".to_string(),
                line: loc.line,
                column: loc.column,
            });
        }

        let ident_token = self.consume(TokenType::Ident)?;
        Ok(format!("/{}", ident_token.value))
    }

    fn parse_include_args(&mut self) -> Result<Vec<IncludeArg>, ParseError> {
        let mut args = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        while self.current_type() == TokenType::Whitespace {
            self.skip_whitespace();
            if self.current_type() != TokenType::Ident {
                break;
            }

            let arg = self.parse_include_arg()?;
            if seen_keys.contains(&arg.name) {
                return Err(ParseError::UnexpectedToken {
                    message: format!("Duplicate include argument: {}", arg.name),
                    line: arg.location.line,
                    column: arg.location.column,
                });
            }
            seen_keys.insert(arg.name.clone());
            args.push(arg);
        }

        Ok(args)
    }

    fn parse_include_arg(&mut self) -> Result<IncludeArg, ParseError> {
        let key_token = self.consume(TokenType::Ident)?;
        let key_loc = key_token.location;
        validate_identifier(&key_token.value, key_loc)?;

        self.skip_whitespace();
        self.consume(TokenType::Equal)?;
        self.skip_whitespace();
        let value = self.parse_path()?;

        Ok(IncludeArg {
            name: key_token.value,
            value,
            location: key_loc,
        })
    }

    fn parse_variable_node(&mut self) -> Result<AstNode, ParseError> {
        let path = self.parse_path_with_modifier()?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;
        Ok(path)
    }

    fn parse_path_with_modifier(&mut self) -> Result<AstNode, ParseError> {
        let first_loc = self.current_location();
        let segments = self.parse_path_segments()?;

        let modifier = match self.current_type() {
            TokenType::Question => {
                self.advance();
                Modifier::Nullable
            }
            TokenType::Exclamation => {
                self.advance();
                Modifier::Required
            }
            _ => Modifier::None,
        };

        Ok(AstNode::Variable(VariableNode {
            path: Path::new(segments, first_loc),
            modifier,
            location: first_loc,
        }))
    }

    fn parse_path(&mut self) -> Result<Path, ParseError> {
        let first_loc = self.current_location();
        let segments = self.parse_path_segments()?;
        Ok(Path::new(segments, first_loc))
    }

    fn parse_path_segments(&mut self) -> Result<Vec<String>, ParseError> {
        let mut segments = vec![self.parse_identifier_with_validation()?];

        while self.current_type() == TokenType::Dot {
            self.consume(TokenType::Dot)?;
            segments.push(self.parse_identifier_with_validation()?);
        }

        Ok(segments)
    }

    fn parse_identifier_with_validation(&mut self) -> Result<String, ParseError> {
        let loc = self.current_location();

        // Check if it's a keyword token used as identifier
        if self.is_keyword_token() {
            let token = self.current_token().unwrap();
            let word = token.value.clone();
            self.advance();
            return Err(ParseError::ReservedWord {
                word,
                line: loc.line,
                column: loc.column,
            });
        }

        let token = self.consume(TokenType::Ident)?;
        validate_identifier(&token.value, loc)?;
        Ok(token.value)
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn is_keyword_token(&self) -> bool {
        matches!(
            self.current_type(),
            TokenType::KwIf
                | TokenType::KwUnless
                | TokenType::KwElse
                | TokenType::KwEach
                | TokenType::KwAs
        )
    }

    fn is_else_open(&self) -> bool {
        if self.current_type() != TokenType::Hash {
            return false;
        }

        let saved = self.pos;
        let mut p = saved + 1;
        // Skip whitespace
        while p < self.tokens.len() && self.tokens[p].token_type == TokenType::Whitespace {
            p += 1;
        }
        p < self.tokens.len() && self.tokens[p].token_type == TokenType::KwElse
    }

    fn consume_else(&mut self) -> Result<(), ParseError> {
        self.consume(TokenType::Hash)?;
        self.skip_whitespace();
        self.consume(TokenType::KwElse)?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;
        Ok(())
    }

    fn is_block_close(&self, keyword: Option<TokenType>) -> bool {
        if self.current_type() != TokenType::Slash {
            return false;
        }

        let keyword = match keyword {
            Some(kw) => kw,
            None => return true,
        };

        let mut p = self.pos + 1;
        // Skip whitespace
        while p < self.tokens.len() && self.tokens[p].token_type == TokenType::Whitespace {
            p += 1;
        }
        p < self.tokens.len() && self.tokens[p].token_type == keyword
    }

    fn consume_block_close(&mut self, keyword: TokenType) -> Result<(), ParseError> {
        self.consume(TokenType::Slash)?;
        self.skip_whitespace();
        self.consume(keyword)?;
        self.skip_whitespace();
        self.consume(TokenType::Close)?;
        Ok(())
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn current_type(&self) -> TokenType {
        self.current_token()
            .map(|t| t.token_type)
            .unwrap_or(TokenType::Eof)
    }

    fn current_location(&self) -> Location {
        self.current_token()
            .map(|t| t.location)
            .unwrap_or_default()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn consume(&mut self, expected: TokenType) -> Result<Token, ParseError> {
        let token = self.current_token().cloned();
        match token {
            Some(t) if t.token_type == expected => {
                self.advance();
                Ok(t)
            }
            Some(t) => Err(ParseError::UnexpectedToken {
                message: format!("Expected {:?}, got {:?}", expected, t.token_type),
                line: t.location.line,
                column: t.location.column,
            }),
            None => Err(ParseError::UnexpectedToken {
                message: format!("Expected {:?}, got end of input", expected),
                line: 0,
                column: 0,
            }),
        }
    }

    fn consume_required_whitespace(&mut self) -> Result<(), ParseError> {
        if self.current_type() != TokenType::Whitespace {
            let loc = self.current_location();
            return Err(ParseError::UnexpectedToken {
                message: "Expected whitespace".to_string(),
                line: loc.line,
                column: loc.column,
            });
        }
        self.skip_whitespace();
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while self.current_type() == TokenType::Whitespace {
            self.advance();
        }
    }

    fn unexpected_token<T>(&self, message: Option<&str>) -> Result<T, ParseError> {
        let loc = self.current_location();
        let msg = match (message, self.current_token()) {
            (Some(m), Some(t)) => format!("{}: {:?}", m, t.token_type),
            (Some(m), None) => m.to_string(),
            (None, Some(t)) => format!("Unexpected token: {:?}", t.token_type),
            (None, None) => "Unexpected end of input".to_string(),
        };
        Err(ParseError::UnexpectedToken {
            message: msg,
            line: loc.line,
            column: loc.column,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;

    #[test]
    fn test_parse_simple_variable() {
        let tmpl = parse("{[ name ]}").unwrap();
        assert_eq!(tmpl.nodes().len(), 1);
    }

    #[test]
    fn test_parse_if_else() {
        let tmpl = parse("{[#if x]}a{[#else]}b{[/if]}").unwrap();
        assert_eq!(tmpl.nodes().len(), 1);
    }

    #[test]
    fn test_parse_nested_blocks() {
        let tmpl = parse("{[#each items as item]}{[#if item.show]}{[ item.name ]}{[/if]}{[/each]}")
            .unwrap();
        assert_eq!(tmpl.nodes().len(), 1);
    }

    #[test]
    fn test_reserved_word_error() {
        let result = parse("{[ if ]}");
        assert!(result.is_err());
    }

    #[test]
    fn test_underscore_identifier_error() {
        let result = parse("{[ _private ]}");
        assert!(result.is_err());
    }
}
