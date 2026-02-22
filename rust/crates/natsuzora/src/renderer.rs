//! Renderer for evaluating Natsuzora AST.

use crate::context::Context;
use crate::error::{NatsuzoraError, Result};
use crate::html_escape;
use crate::template_loader::TemplateLoader;
use crate::value::Value;
use natsuzora_ast::{
    AstNode, EachBlock, IfBlock, IncludeNode, Modifier, Template, UnlessBlock, UnsecureNode,
    VariableNode, WhitespaceControl,
};
use std::collections::HashMap;

/// Renderer for evaluating Natsuzora AST
pub struct Renderer<'a> {
    template_loader: Option<&'a mut TemplateLoader>,
}

impl<'a> Renderer<'a> {
    /// Create a new renderer
    pub fn new(template_loader: Option<&'a mut TemplateLoader>) -> Self {
        Self { template_loader }
    }

    /// Render a template with the given data
    pub fn render(&mut self, template: &Template, data: Value) -> Result<String> {
        let mut context = Context::new(data)?;
        self.render_nodes(template.nodes(), &mut context)
    }

    fn render_nodes(&mut self, nodes: &[AstNode], context: &mut Context) -> Result<String> {
        let mut output = String::new();
        let mut pending_trim = false;

        for node in nodes.iter() {
            // Handle whitespace trimming from previous tag's -]}
            if pending_trim {
                if let AstNode::Text(text) = node {
                    let trimmed = trim_leading_whitespace(&text.content);
                    output.push_str(trimmed);
                    pending_trim = false;
                    continue;
                }
                pending_trim = false;
            }

            let (rendered, ws) = self.render_node_with_ws(node, context)?;

            // Handle {[- trim before
            if ws.trim_before && !output.is_empty() {
                output = trim_trailing_whitespace(&output);
            }

            output.push_str(&rendered);

            // Handle -]} trim after
            if ws.trim_after {
                pending_trim = true;
            }
        }

        Ok(output)
    }

    fn render_node_with_ws(
        &mut self,
        node: &AstNode,
        context: &mut Context,
    ) -> Result<(String, WhitespaceControl)> {
        match node {
            AstNode::Text(n) => Ok((n.content.clone(), WhitespaceControl::default())),
            AstNode::Variable(n) => {
                let rendered = self.render_variable(n, context)?;
                Ok((rendered, n.whitespace))
            }
            AstNode::Unsecure(n) => {
                let rendered = self.render_unsecure(n, context)?;
                Ok((rendered, n.whitespace))
            }
            AstNode::If(n) => {
                let rendered = self.render_if(n, context)?;
                // Return the open tag's whitespace for trim_before, close tag for trim_after
                Ok((
                    rendered,
                    WhitespaceControl {
                        trim_before: n.whitespace_open.trim_before,
                        trim_after: n.whitespace_close.trim_after,
                    },
                ))
            }
            AstNode::Unless(n) => {
                let rendered = self.render_unless(n, context)?;
                Ok((
                    rendered,
                    WhitespaceControl {
                        trim_before: n.whitespace_open.trim_before,
                        trim_after: n.whitespace_close.trim_after,
                    },
                ))
            }
            AstNode::Each(n) => {
                let rendered = self.render_each(n, context)?;
                Ok((
                    rendered,
                    WhitespaceControl {
                        trim_before: n.whitespace_open.trim_before,
                        trim_after: n.whitespace_close.trim_after,
                    },
                ))
            }
            AstNode::Include(n) => {
                let rendered = self.render_include(n, context)?;
                Ok((rendered, n.whitespace))
            }
        }
    }

    fn render_variable(&self, node: &VariableNode, context: &Context) -> Result<String> {
        let location = node.location;
        let path_str = node.path.as_str();
        let value = context.resolve(node.path.segments(), location)?;

        match node.modifier {
            Modifier::None => {
                // v4.0: null causes error without modifier
                if value.is_null() {
                    return Err(NatsuzoraError::NullValueError {
                        name: path_str,
                        location,
                    });
                }
                let str_value = value.stringify()?;
                Ok(html_escape::escape(&str_value))
            }
            Modifier::Nullable => {
                // ? modifier: null outputs empty string
                let str_value = value.stringify_nullable()?;
                Ok(html_escape::escape(&str_value))
            }
            Modifier::Required => {
                // ! modifier: null or empty string causes error
                if value.is_null() {
                    return Err(NatsuzoraError::NullValueError {
                        name: path_str,
                        location,
                    });
                }
                if value.is_empty_string() {
                    return Err(NatsuzoraError::EmptyStringError {
                        name: path_str,
                        location,
                    });
                }
                let str_value = value.stringify()?;
                Ok(html_escape::escape(&str_value))
            }
        }
    }

    fn render_unsecure(&self, node: &UnsecureNode, context: &Context) -> Result<String> {
        let location = node.location;
        let path_str = node.path.as_str();
        let value = context.resolve(node.path.segments(), location)?;

        // Unsecure has no modifier, so null causes error (v4.0)
        if value.is_null() {
            return Err(NatsuzoraError::NullValueError {
                name: path_str,
                location,
            });
        }
        value.stringify()
    }

    fn render_if(&mut self, node: &IfBlock, context: &mut Context) -> Result<String> {
        let location = node.location;
        let value = context.resolve(node.condition.segments(), location)?;

        if value.is_truthy() {
            self.render_nodes(&node.then_branch, context)
        } else if let Some(else_branch) = &node.else_branch {
            self.render_nodes(else_branch, context)
        } else {
            Ok(String::new())
        }
    }

    fn render_unless(&mut self, node: &UnlessBlock, context: &mut Context) -> Result<String> {
        let location = node.location;
        let value = context.resolve(node.condition.segments(), location)?;

        if value.is_truthy() {
            Ok(String::new())
        } else {
            self.render_nodes(&node.body, context)
        }
    }

    fn render_each(&mut self, node: &EachBlock, context: &mut Context) -> Result<String> {
        let location = node.location;
        let len = context.get_array_len(node.collection.segments(), location)?;

        let mut output = String::new();
        for index in 0..len {
            let item = context.get_array_item(node.collection.segments(), index, location)?;

            let mut bindings = HashMap::new();
            bindings.insert(node.item_ident.clone(), item);

            context.push_scope(bindings)?;
            output.push_str(&self.render_nodes(&node.body, context)?);
            context.pop_scope();
        }

        Ok(output)
    }

    fn render_include(&mut self, node: &IncludeNode, context: &mut Context) -> Result<String> {
        let partial = {
            let loader =
                self.template_loader
                    .as_mut()
                    .ok_or_else(|| NatsuzoraError::IncludeError {
                        message: "Template loader not configured for include".to_string(),
                    })?;
            loader.load(&node.name)?
        };

        let mut bindings = HashMap::new();
        for arg in &node.args {
            let value = context.resolve(arg.value.segments(), arg.location)?.clone();
            bindings.insert(arg.name.clone(), value);
        }

        if let Some(loader) = self.template_loader.as_mut() {
            loader.push_include(&node.name);
        }

        context.push_include_scope(bindings);
        let result = self.render_nodes(partial.nodes(), context);
        context.pop_scope();

        if let Some(loader) = self.template_loader.as_mut() {
            loader.pop_include();
        }

        result
    }
}

/// Trim trailing whitespace (spaces and tabs) on the current line.
/// For {[- (left trim): removes whitespace from start of line to tag start.
/// Preserves the newline character before the whitespace.
fn trim_trailing_whitespace(s: &str) -> String {
    s.trim_end_matches(|c: char| c == ' ' || c == '\t')
        .to_string()
}

/// Trim leading whitespace and optional newline
fn trim_leading_whitespace(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut pos = 0;

    // Skip optional newline first
    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos += 1;
    } else if pos < bytes.len() && bytes[pos] == b'\r' {
        pos += 1;
        if pos < bytes.len() && bytes[pos] == b'\n' {
            pos += 1;
        }
    }

    // Then skip spaces/tabs
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }

    // Return the remaining string
    &s[pos..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Location;
    use serde_json::json;

    fn render(source: &str, data: serde_json::Value) -> Result<String> {
        let template = natsuzora_ast::parse(source).map_err(|e| NatsuzoraError::ParseError {
            message: e.to_string(),
            location: Location::default(),
        })?;
        let value = Value::from_json(data)?;
        let mut renderer = Renderer::new(None);
        renderer.render(&template, value)
    }

    #[test]
    fn test_render_text() {
        let result = render("Hello, world!", json!({})).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_render_variable() {
        let result = render("Hello, {[ name ]}!", json!({"name": "Alice"})).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_render_variable_escaped() {
        let result = render(
            "{[ html ]}",
            json!({"html": "<script>alert('xss')</script>"}),
        )
        .unwrap();
        assert_eq!(result, "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
    }

    #[test]
    fn test_render_if_true() {
        let result = render("{[#if visible]}Hello{[/if]}", json!({"visible": true})).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_render_if_false() {
        let result = render("{[#if visible]}Hello{[/if]}", json!({"visible": false})).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_if_else() {
        let result = render(
            "{[#if visible]}Yes{[#else]}No{[/if]}",
            json!({"visible": false}),
        )
        .unwrap();
        assert_eq!(result, "No");
    }

    #[test]
    fn test_render_each() {
        let result = render(
            "{[#each items as item]}{[ item ]}{[/each]}",
            json!({"items": ["a", "b", "c"]}),
        )
        .unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn test_render_unless_false() {
        let result = render(
            "{[#unless hidden]}visible{[/unless]}",
            json!({"hidden": false}),
        )
        .unwrap();
        assert_eq!(result, "visible");
    }

    #[test]
    fn test_render_unless_true() {
        let result = render(
            "{[#unless hidden]}visible{[/unless]}",
            json!({"hidden": true}),
        )
        .unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_unsecure() {
        let result = render("{[!unsecure html]}", json!({"html": "<b>bold</b>"})).unwrap();
        assert_eq!(result, "<b>bold</b>");
    }

    #[test]
    fn test_render_path() {
        let result = render(
            "{[ user.profile.name ]}",
            json!({"user": {"profile": {"name": "Alice"}}}),
        )
        .unwrap();
        assert_eq!(result, "Alice");
    }

    // v4.0 Modifier tests
    #[test]
    fn test_null_without_modifier_error() {
        let result = render("{[ value ]}", json!({"value": null}));
        assert!(matches!(result, Err(NatsuzoraError::NullValueError { .. })));
    }

    #[test]
    fn test_nullable_modifier() {
        let result = render("{[ value? ]}", json!({"value": null})).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_required_modifier_null_error() {
        let result = render("{[ value! ]}", json!({"value": null}));
        assert!(matches!(result, Err(NatsuzoraError::NullValueError { .. })));
    }

    #[test]
    fn test_required_modifier_empty_string_error() {
        let result = render("{[ value! ]}", json!({"value": ""}));
        assert!(matches!(
            result,
            Err(NatsuzoraError::EmptyStringError { .. })
        ));
    }

    #[test]
    fn test_required_modifier_with_value() {
        let result = render("{[ value! ]}", json!({"value": "hello"})).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_stringify_integer() {
        let result = render("[{[ value ]}]", json!({"value": 42})).unwrap();
        assert_eq!(result, "[42]");
    }

    #[test]
    fn test_stringify_boolean_error() {
        let result = render("{[ value ]}", json!({"value": true}));
        assert!(result.is_err());
    }

    #[test]
    fn test_comment_ignored() {
        let result = render("Hello{[! comment ]}World", json!({})).unwrap();
        assert_eq!(result, "HelloWorld");
    }
}
