//! Parser for Contract notation.
//!
//! Parses human-readable contract notation into Contract types.

use std::collections::BTreeMap;

use crate::types::{Contract, ContractField, ContractModifier, DiffMarker, ScalarType, TypeDef};

/// Error that can occur during parsing.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message} at line {line}, column {column}")]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// Lowercase identifier (field name), including contextual keywords like "type"
    Identifier(String),
    /// Uppercase identifier (type name)
    TypeName(String),
    Colon,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Question,
    Exclamation,
    /// `+` diff marker
    Plus,
    /// `-` diff marker or arrow component
    Minus,
    /// `*` diff marker
    Star,
    /// `->` arrow for type change
    Arrow,
    Comment(String),
    Newline,
    Eof,
}

struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    fn read_comment(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
        self.input[start..self.pos].to_string()
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();

        let Some(c) = self.peek_char() else {
            return Ok(Token::Eof);
        };

        match c {
            '#' => {
                self.advance();
                let comment = self.read_comment();
                Ok(Token::Comment(comment))
            }
            '\n' => {
                self.advance();
                Ok(Token::Newline)
            }
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            '{' => {
                self.advance();
                Ok(Token::OpenBrace)
            }
            '}' => {
                self.advance();
                Ok(Token::CloseBrace)
            }
            '[' => {
                self.advance();
                Ok(Token::OpenBracket)
            }
            ']' => {
                self.advance();
                Ok(Token::CloseBracket)
            }
            '?' => {
                self.advance();
                Ok(Token::Question)
            }
            '!' => {
                self.advance();
                Ok(Token::Exclamation)
            }
            '+' => {
                self.advance();
                Ok(Token::Plus)
            }
            '-' => {
                self.advance();
                // Check for ->
                if self.peek_char() == Some('>') {
                    self.advance();
                    Ok(Token::Arrow)
                } else {
                    Ok(Token::Minus)
                }
            }
            '*' => {
                self.advance();
                Ok(Token::Star)
            }
            c if c.is_ascii_alphabetic() => {
                let ident = self.read_identifier();
                // Distinguish by first character case
                if ident.chars().next().unwrap().is_ascii_uppercase() {
                    Ok(Token::TypeName(ident))
                } else {
                    Ok(Token::Identifier(ident))
                }
            }
            _ => Err(ParseError {
                message: format!("unexpected character '{c}'"),
                line: self.line,
                column: self.column,
            }),
        }
    }
}

/// Parsed contract file with type definitions and root contract.
#[derive(Debug, Clone)]
pub struct ContractFile {
    /// Named type definitions
    pub types: BTreeMap<String, Contract>,
    /// Root contract (fields at the top level)
    pub root: Contract,
}

/// Parsed contract file with diff markers (2-generation support).
#[derive(Debug, Clone)]
pub struct ContractFileWithDiff {
    /// Named type definitions with optional diff markers
    pub types: BTreeMap<String, TypeDef>,
    /// Root fields with optional diff markers
    pub fields: BTreeMap<String, ContractField>,
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    /// Peeked token for lookahead
    peeked: Option<Token>,
    line: usize,
    column: usize,
    /// Type definitions collected during parsing
    types: BTreeMap<String, Contract>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(input);
        let line = lexer.line;
        let column = lexer.column;
        let current = lexer.next_token()?;
        Ok(Self {
            lexer,
            current,
            peeked: None,
            line,
            column,
            types: BTreeMap::new(),
        })
    }

    /// Peek at the next token without consuming it
    fn peek(&mut self) -> Result<&Token, ParseError> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    fn advance(&mut self) -> Result<Token, ParseError> {
        self.line = self.lexer.line;
        self.column = self.lexer.column;
        let next = if let Some(peeked) = self.peeked.take() {
            peeked
        } else {
            self.lexer.next_token()?
        };
        let prev = std::mem::replace(&mut self.current, next);
        Ok(prev)
    }

    fn skip_separators(&mut self) -> Result<(), ParseError> {
        while let Token::Newline | Token::Comment(_) = &self.current {
            self.advance()?;
        }
        Ok(())
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance()? {
            Token::Identifier(name) => Ok(name),
            _ => Err(ParseError {
                message: "expected field name (lowercase identifier)".into(),
                line: self.line,
                column: self.column,
            }),
        }
    }

    fn expect_type_name(&mut self) -> Result<String, ParseError> {
        match self.advance()? {
            Token::TypeName(name) => Ok(name),
            _ => Err(ParseError {
                message: "expected type name (uppercase identifier)".into(),
                line: self.line,
                column: self.column,
            }),
        }
    }

    /// Try to parse a diff marker (+, -, *)
    fn try_parse_diff_marker(&mut self) -> Result<Option<DiffMarker>, ParseError> {
        match &self.current {
            Token::Plus => {
                self.advance()?;
                Ok(Some(DiffMarker::Added))
            }
            Token::Minus => {
                self.advance()?;
                Ok(Some(DiffMarker::Removed))
            }
            Token::Star => {
                self.advance()?;
                Ok(Some(DiffMarker::Changed))
            }
            _ => Ok(None),
        }
    }

    /// Check if current token is "type" identifier followed by a TypeName (type definition)
    fn is_type_definition(&mut self) -> Result<bool, ParseError> {
        if let Token::Identifier(name) = &self.current {
            if name == "type" {
                // Peek at next token to see if it's a TypeName
                let next = self.peek()?;
                return Ok(matches!(next, Token::TypeName(_)));
            }
        }
        Ok(false)
    }

    /// Parse the entire contract file (type definitions + root)
    fn parse_file(&mut self) -> Result<ContractFile, ParseError> {
        self.skip_separators()?;

        // Parse type definitions
        while self.is_type_definition()? {
            self.parse_type_def()?;
            self.skip_separators()?;
        }

        // Parse root contract
        let root = self.parse_object_body()?;

        Ok(ContractFile {
            types: std::mem::take(&mut self.types),
            root,
        })
    }

    /// Parse a type definition: `type Name { ... }`
    fn parse_type_def(&mut self) -> Result<(), ParseError> {
        // Consume 'type' identifier (contextual keyword)
        if !matches!(&self.current, Token::Identifier(name) if name == "type") {
            return Err(ParseError {
                message: "expected 'type' keyword".into(),
                line: self.line,
                column: self.column,
            });
        }
        self.advance()?;

        // Get type name
        let type_name = self.expect_type_name()?;

        // Expect '{'
        if !matches!(self.current, Token::OpenBrace) {
            return Err(ParseError {
                message: "expected '{' after type name".into(),
                line: self.line,
                column: self.column,
            });
        }
        self.advance()?;

        // Parse body
        let contract = self.parse_object_body()?;

        // Expect '}'
        if !matches!(self.current, Token::CloseBrace) {
            return Err(ParseError {
                message: "expected '}'".into(),
                line: self.line,
                column: self.column,
            });
        }
        self.advance()?;

        self.types.insert(type_name, contract);
        Ok(())
    }

    /// Parse the entire contract file with diff markers (2-generation support)
    fn parse_file_with_diff(&mut self) -> Result<ContractFileWithDiff, ParseError> {
        let mut types = BTreeMap::new();
        let mut fields = BTreeMap::new();

        self.skip_separators()?;

        // Parse type definitions and root fields
        loop {
            self.skip_separators()?;
            if matches!(self.current, Token::Eof) {
                break;
            }

            // Check for diff marker
            let marker = self.try_parse_diff_marker()?;

            // Check if this is a type definition (Identifier("type") followed by TypeName)
            if self.is_type_definition()? {
                // Validate marker for type def (* is not allowed)
                if matches!(marker, Some(DiffMarker::Changed)) {
                    return Err(ParseError {
                        message: "* marker is not allowed for type definitions".into(),
                        line: self.line,
                        column: self.column,
                    });
                }

                self.advance()?; // consume 'type'
                let type_name = self.expect_type_name()?;

                if !matches!(self.current, Token::OpenBrace) {
                    return Err(ParseError {
                        message: "expected '{' after type name".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
                self.advance()?;

                let contract = self.parse_object_body()?;

                if !matches!(self.current, Token::CloseBrace) {
                    return Err(ParseError {
                        message: "expected '}'".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
                self.advance()?;

                types.insert(type_name, TypeDef { marker, contract });
            } else if matches!(self.current, Token::Identifier(_)) {
                // This is a field (including fields named "type" when followed by : or {)
                let (name, field) = self.parse_field_with_diff(marker)?;
                fields.insert(name, field);
            } else if marker.is_some() {
                return Err(ParseError {
                    message: "expected field name or 'type' after diff marker".into(),
                    line: self.line,
                    column: self.column,
                });
            } else {
                break;
            }
        }

        Ok(ContractFileWithDiff { types, fields })
    }

    /// Parse a single field with diff marker
    fn parse_field_with_diff(
        &mut self,
        marker: Option<DiffMarker>,
    ) -> Result<(String, ContractField), ParseError> {
        let name = self.expect_identifier()?;

        let (current_type, next_type) = match &self.current {
            Token::Colon => {
                self.advance()?;
                let first_type = self.parse_type()?;

                // Check for type change (->)
                if matches!(self.current, Token::Arrow) {
                    if !matches!(marker, Some(DiffMarker::Changed)) {
                        return Err(ParseError {
                            message: "'->' is only allowed with * marker".into(),
                            line: self.line,
                            column: self.column,
                        });
                    }
                    self.advance()?;
                    let second_type = self.parse_type()?;
                    (first_type, Some(second_type))
                } else {
                    if matches!(marker, Some(DiffMarker::Changed)) {
                        return Err(ParseError {
                            message: "* marker requires '->' for type change".into(),
                            line: self.line,
                            column: self.column,
                        });
                    }
                    (first_type, None)
                }
            }
            Token::OpenBrace => {
                if matches!(marker, Some(DiffMarker::Changed)) {
                    return Err(ParseError {
                        message: "* marker is not allowed for nested objects".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
                self.advance()?;
                let inner = self.parse_object_body()?;
                if !matches!(self.current, Token::CloseBrace) {
                    return Err(ParseError {
                        message: "expected '}'".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
                self.advance()?;
                (inner, None)
            }
            _ => {
                return Err(ParseError {
                    message: "expected ':' or '{'".into(),
                    line: self.line,
                    column: self.column,
                });
            }
        };

        let field = ContractField {
            marker,
            current_type,
            next_type,
        };

        Ok((name, field))
    }

    /// Parse object body (fields without enclosing braces)
    fn parse_object_body(&mut self) -> Result<Contract, ParseError> {
        let mut properties = BTreeMap::new();
        let mut required = Vec::new();

        self.skip_separators()?;

        while !matches!(self.current, Token::Eof | Token::CloseBrace) {
            self.skip_separators()?;
            if matches!(self.current, Token::Eof | Token::CloseBrace) {
                break;
            }

            let name = self.expect_identifier()?;
            required.push(name.clone());

            let contract = match &self.current {
                Token::Colon => {
                    self.advance()?;
                    self.parse_type()?
                }
                Token::OpenBrace => {
                    self.advance()?;
                    let inner = self.parse_object_body()?;
                    if !matches!(self.current, Token::CloseBrace) {
                        return Err(ParseError {
                            message: "expected '}'".into(),
                            line: self.line,
                            column: self.column,
                        });
                    }
                    self.advance()?;
                    inner
                }
                _ => {
                    return Err(ParseError {
                        message: "expected ':' or '{'".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
            };

            properties.insert(name, contract);
            self.skip_separators()?;
        }

        Ok(Contract::Object {
            required,
            properties,
        })
    }

    /// Parse a type (scalar, array, or type reference)
    fn parse_type(&mut self) -> Result<Contract, ParseError> {
        // Check for array type: []type or []{...}
        if matches!(self.current, Token::OpenBracket) {
            self.advance()?;
            if !matches!(self.current, Token::CloseBracket) {
                return Err(ParseError {
                    message: "expected ']'".into(),
                    line: self.line,
                    column: self.column,
                });
            }
            self.advance()?;

            // Parse array item type
            let items = if matches!(self.current, Token::OpenBrace) {
                self.advance()?;
                let inner = self.parse_object_body()?;
                if !matches!(self.current, Token::CloseBrace) {
                    return Err(ParseError {
                        message: "expected '}'".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
                self.advance()?;
                inner
            } else if matches!(self.current, Token::TypeName(_)) {
                self.parse_type_reference()?
            } else {
                self.parse_scalar_type()?
            };

            return Ok(Contract::Array {
                items: Box::new(items),
            });
        }

        // Check for type reference (uppercase)
        if matches!(self.current, Token::TypeName(_)) {
            return self.parse_type_reference();
        }

        self.parse_scalar_type()
    }

    /// Parse a type reference (e.g., `Author`, `Comment`)
    fn parse_type_reference(&mut self) -> Result<Contract, ParseError> {
        let type_name = self.expect_type_name()?;

        // For now, we return a placeholder that will be resolved later
        // In a full implementation, we'd resolve the reference here or later
        Ok(Contract::TypeRef { name: type_name })
    }

    /// Parse a scalar type with optional modifier
    fn parse_scalar_type(&mut self) -> Result<Contract, ParseError> {
        let type_name = self.expect_identifier()?;
        let scalar_type = match type_name.as_str() {
            "string" => ScalarType::String,
            "integer" => ScalarType::Integer,
            "bool" => ScalarType::Bool,
            "scalar" => ScalarType::Scalar,
            _ => {
                return Err(ParseError {
                    message: format!("unknown type '{type_name}' (did you mean to use uppercase for a type reference?)"),
                    line: self.line,
                    column: self.column,
                });
            }
        };

        let modifier = match &self.current {
            Token::Question => {
                self.advance()?;
                ContractModifier::Nullable
            }
            Token::Exclamation => {
                self.advance()?;
                ContractModifier::Required
            }
            _ => ContractModifier::None,
        };

        Ok(Contract::Scalar {
            scalar_type,
            modifier,
        })
    }
}

/// Parse contract notation into a Contract.
/// This is a simplified parse that doesn't support type definitions.
pub fn parse(input: &str) -> Result<Contract, ParseError> {
    let file = parse_file(input)?;
    resolve_types(&file)
}

/// Parse contract notation into a ContractFile with type definitions.
pub fn parse_file(input: &str) -> Result<ContractFile, ParseError> {
    let mut parser = Parser::new(input)?;
    parser.parse_file()
}

/// Parse contract notation with diff markers (2-generation support).
pub fn parse_file_with_diff(input: &str) -> Result<ContractFileWithDiff, ParseError> {
    let mut parser = Parser::new(input)?;
    parser.parse_file_with_diff()
}

/// Resolve type references in a contract file.
pub fn resolve_types(file: &ContractFile) -> Result<Contract, ParseError> {
    resolve_contract(&file.root, &file.types)
}

fn resolve_contract(
    contract: &Contract,
    types: &BTreeMap<String, Contract>,
) -> Result<Contract, ParseError> {
    match contract {
        Contract::TypeRef { name } => {
            let resolved = types.get(name).ok_or_else(|| ParseError {
                message: format!("undefined type '{name}'"),
                line: 0,
                column: 0,
            })?;
            resolve_contract(resolved, types)
        }
        Contract::Object {
            required,
            properties,
        } => {
            let mut resolved_props = BTreeMap::new();
            for (key, value) in properties {
                resolved_props.insert(key.clone(), resolve_contract(value, types)?);
            }
            Ok(Contract::Object {
                required: required.clone(),
                properties: resolved_props,
            })
        }
        Contract::Array { items } => Ok(Contract::Array {
            items: Box::new(resolve_contract(items, types)?),
        }),
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_field() {
        let contract = parse("name: string").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("name"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_nullable_modifier() {
        let contract = parse("name: string?").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("name"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::Nullable,
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_required_modifier() {
        let contract = parse("name: string!").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("name"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::Required,
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_nested_object() {
        let contract = parse(
            r#"
            user {
                name: string!
                age: integer
            }
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                let user = properties.get("user").expect("user");
                match user {
                    Contract::Object { properties, .. } => {
                        assert!(properties.contains_key("name"));
                        assert!(properties.contains_key("age"));
                    }
                    _ => panic!("expected object"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_simple_array() {
        let contract = parse("tags: []string").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                let tags = properties.get("tags").expect("tags");
                match tags {
                    Contract::Array { items } => {
                        assert!(matches!(
                            items.as_ref(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                ..
                            }
                        ));
                    }
                    _ => panic!("expected array"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_object_array() {
        let contract = parse(
            r#"
            items: []{
                title: string!
                count: integer
            }
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                let items = properties.get("items").expect("items");
                match items {
                    Contract::Array { items } => match items.as_ref() {
                        Contract::Object { properties, .. } => {
                            assert!(properties.contains_key("title"));
                            assert!(properties.contains_key("count"));
                        }
                        _ => panic!("expected object in array"),
                    },
                    _ => panic!("expected array"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_bool_type() {
        let contract = parse("isActive: bool").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("isActive"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::Bool,
                        modifier: ContractModifier::None,
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_with_comments() {
        let contract = parse(
            r#"
            # This is a comment
            name: string!
            # Another comment
            age: integer
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("age"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_newline_separated() {
        let contract = parse("x: integer\ny: integer\nz: integer").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("x"));
                assert!(properties.contains_key("y"));
                assert!(properties.contains_key("z"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_with_empty_lines() {
        let contract = parse(
            r#"
            name: string

            age: integer

            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("age"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_type_definition() {
        let contract = parse(
            r#"
            type User {
                name: string!
                age: integer
            }

            user: User
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                let user = properties.get("user").expect("user");
                match user {
                    Contract::Object { properties, .. } => {
                        assert!(properties.contains_key("name"));
                        assert!(properties.contains_key("age"));
                    }
                    _ => panic!("expected object"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_type_reference_array() {
        let contract = parse(
            r#"
            type Comment {
                body: string!
            }

            comments: []Comment
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                let comments = properties.get("comments").expect("comments");
                match comments {
                    Contract::Array { items } => match items.as_ref() {
                        Contract::Object { properties, .. } => {
                            assert!(properties.contains_key("body"));
                        }
                        _ => panic!("expected object in array"),
                    },
                    _ => panic!("expected array"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_type_with_multiple_fields() {
        let contract = parse(
            r#"
            type Point {
                x: integer
                y: integer
            }

            name: string!
            age: integer
            point: Point
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("age"));
                assert!(properties.contains_key("point"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_scalar_type() {
        let contract = parse("value: scalar").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("value"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::Scalar,
                        modifier: ContractModifier::None,
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_integer_type() {
        let contract = parse("count: integer").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("count"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::Integer,
                        modifier: ContractModifier::None,
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_empty_object() {
        let contract = parse("config {}").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                let config = properties.get("config").expect("config");
                match config {
                    Contract::Object { properties, .. } => {
                        assert!(properties.is_empty());
                    }
                    _ => panic!("expected object"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_deeply_nested() {
        let contract = parse(
            r#"
            level1 {
                level2 {
                    level3 {
                        value: string
                    }
                }
            }
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                let l1 = properties.get("level1").expect("level1");
                match l1 {
                    Contract::Object { properties, .. } => {
                        let l2 = properties.get("level2").expect("level2");
                        match l2 {
                            Contract::Object { properties, .. } => {
                                let l3 = properties.get("level3").expect("level3");
                                match l3 {
                                    Contract::Object { properties, .. } => {
                                        assert!(properties.contains_key("value"));
                                    }
                                    _ => panic!("expected object"),
                                }
                            }
                            _ => panic!("expected object"),
                        }
                    }
                    _ => panic!("expected object"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_inline_type() {
        let contract = parse("type Point {\n  x: integer\n  y: integer\n}\norigin: Point").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                let origin = properties.get("origin").expect("origin");
                match origin {
                    Contract::Object { properties, .. } => {
                        assert!(properties.contains_key("x"));
                        assert!(properties.contains_key("y"));
                    }
                    _ => panic!("expected object"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_array_with_modifier() {
        let contract = parse("tags: []string?").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                let tags = properties.get("tags").expect("tags");
                match tags {
                    Contract::Array { items } => {
                        assert!(matches!(
                            items.as_ref(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::Nullable,
                            }
                        ));
                    }
                    _ => panic!("expected array"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn error_unexpected_token() {
        let result = parse("name: @invalid");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("unexpected") || err.message.contains("expected"));
    }

    #[test]
    fn error_unclosed_brace() {
        let result = parse("user { name: string");
        assert!(result.is_err());
    }

    #[test]
    fn error_undefined_type_reference() {
        let result = parse("user: UndefinedType");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("UndefinedType"));
    }

    #[test]
    fn parses_multiple_type_definitions() {
        let contract = parse(
            r#"
            type Author { name: string }
            type Comment {
                body: string
                author: Author
            }

            comments: []Comment
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                let comments = properties.get("comments").expect("comments");
                match comments {
                    Contract::Array { items } => match items.as_ref() {
                        Contract::Object { properties, .. } => {
                            assert!(properties.contains_key("body"));
                            assert!(properties.contains_key("author"));
                        }
                        _ => panic!("expected object"),
                    },
                    _ => panic!("expected array"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_multiline_inline_object() {
        let contract = parse(
            r#"items: []{
    title: string!
    count: integer
}"#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("items"));
            }
            _ => panic!("expected object"),
        }
    }

    // =========================================================================
    // Diff marker tests
    // =========================================================================

    #[test]
    fn parses_added_field() {
        let file = parse_file_with_diff("+ email: string").unwrap();
        assert!(file.fields.contains_key("email"));
        let field = file.fields.get("email").unwrap();
        assert_eq!(field.marker, Some(DiffMarker::Added));
    }

    #[test]
    fn parses_removed_field() {
        let file = parse_file_with_diff("- legacyId: integer").unwrap();
        assert!(file.fields.contains_key("legacyId"));
        let field = file.fields.get("legacyId").unwrap();
        assert_eq!(field.marker, Some(DiffMarker::Removed));
    }

    #[test]
    fn parses_changed_field() {
        let file = parse_file_with_diff("* age: integer -> scalar").unwrap();
        assert!(file.fields.contains_key("age"));
        let field = file.fields.get("age").unwrap();
        assert_eq!(field.marker, Some(DiffMarker::Changed));
        assert!(field.next_type.is_some());
    }

    #[test]
    fn parses_added_type() {
        let file = parse_file_with_diff("+ type NewType {\n  name: string\n}").unwrap();
        assert!(file.types.contains_key("NewType"));
        let type_def = file.types.get("NewType").unwrap();
        assert_eq!(type_def.marker, Some(DiffMarker::Added));
    }

    #[test]
    fn parses_removed_type() {
        let file = parse_file_with_diff("- type OldType {\n  name: string\n}").unwrap();
        assert!(file.types.contains_key("OldType"));
        let type_def = file.types.get("OldType").unwrap();
        assert_eq!(type_def.marker, Some(DiffMarker::Removed));
    }

    #[test]
    fn error_star_on_type() {
        let result = parse_file_with_diff("* type ChangedType {\n  name: string\n}");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("* marker is not allowed"));
    }

    #[test]
    fn error_arrow_without_star() {
        let result = parse_file_with_diff("age: integer -> scalar");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("only allowed with *"));
    }

    #[test]
    fn error_star_without_arrow() {
        let result = parse_file_with_diff("* age: integer");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("requires '->'"));
    }

    #[test]
    fn parses_mixed_diff_markers() {
        let input = r#"
name: string
+ email: string
- legacyId: integer
* age: integer -> scalar
"#;
        let file = parse_file_with_diff(input).unwrap();
        assert_eq!(file.fields.len(), 4);
        assert_eq!(file.fields.get("name").unwrap().marker, None);
        assert_eq!(
            file.fields.get("email").unwrap().marker,
            Some(DiffMarker::Added)
        );
        assert_eq!(
            file.fields.get("legacyId").unwrap().marker,
            Some(DiffMarker::Removed)
        );
        assert_eq!(
            file.fields.get("age").unwrap().marker,
            Some(DiffMarker::Changed)
        );
    }

    // =========================================================================
    // Contextual keyword "type" as field name tests
    // =========================================================================

    #[test]
    fn parses_type_as_field_name_scalar() {
        // "type" followed by ":" should be parsed as a field, not a type definition
        let contract = parse("type: string").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("type"));
                assert!(matches!(
                    properties.get("type"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        ..
                    })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_type_as_field_name_object() {
        // "type" followed by "{" should be parsed as a field with inline object
        let contract = parse("type { name: string }").unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("type"));
                match properties.get("type") {
                    Some(Contract::Object {
                        properties: inner, ..
                    }) => {
                        assert!(inner.contains_key("name"));
                    }
                    _ => panic!("expected nested object"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_type_definition_and_type_field() {
        // Both type definition and field named "type" in the same file
        let contract = parse(
            r#"
            type User {
                name: string
            }

            type: string
            user: User
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("type"));
                assert!(properties.contains_key("user"));
                // Verify "type" field is a scalar
                assert!(matches!(
                    properties.get("type"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        ..
                    })
                ));
                // Verify "user" field is an object (resolved from type definition)
                assert!(matches!(
                    properties.get("user"),
                    Some(Contract::Object { .. })
                ));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn parses_type_field_with_diff_marker() {
        // "type" as field name with diff markers
        let file = parse_file_with_diff("+ type: string").unwrap();
        assert!(file.fields.contains_key("type"));
        let field = file.fields.get("type").unwrap();
        assert_eq!(field.marker, Some(DiffMarker::Added));
    }

    #[test]
    fn parses_builtin_types_as_field_names() {
        // "string", "integer", "bool", "scalar" can all be field names
        let contract = parse(
            r#"
            string: string
            integer: integer
            bool: bool
            scalar: scalar
            "#,
        )
        .unwrap();

        match contract {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("string"));
                assert!(properties.contains_key("integer"));
                assert!(properties.contains_key("bool"));
                assert!(properties.contains_key("scalar"));
            }
            _ => panic!("expected object"),
        }
    }
}
