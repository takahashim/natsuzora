//! Renderer for evaluating Natsuzora AST.
//!
//! Since TokenProcessor handles whitespace control before parsing,
//! the renderer simply evaluates the AST without any whitespace trimming logic.

use crate::context::Context;
use crate::error::{NatsuzoraError, Result};
use crate::html_escape;
use crate::template_loader::TemplateLoader;
use crate::value::Value;
use natsuzora_ast::{
    AstNode, EachBlock, IfBlock, IncludeNode, Modifier, Template, UnlessBlock, UnsecureNode,
    VariableNode,
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

        for node in nodes {
            match node {
                AstNode::Text(n) => output.push_str(&n.content),
                AstNode::Variable(n) => output.push_str(&self.render_variable(n, context)?),
                AstNode::Unsecure(n) => output.push_str(&self.render_unsecure(n, context)?),
                AstNode::If(n) => output.push_str(&self.render_if(n, context)?),
                AstNode::Unless(n) => output.push_str(&self.render_unless(n, context)?),
                AstNode::Each(n) => output.push_str(&self.render_each(n, context)?),
                AstNode::Include(n) => output.push_str(&self.render_include(n, context)?),
            }
        }

        Ok(output)
    }

    fn render_variable(&self, node: &VariableNode, context: &Context) -> Result<String> {
        let location = node.location;
        let value = context.resolve(node.path.segments(), location)?;

        let str_value = match node.modifier {
            Modifier::None => value.stringify()?,
            Modifier::Nullable => value.stringify_nullable()?,
            Modifier::Required => value.stringify_required()?,
        };
        Ok(html_escape::escape(&str_value))
    }

    fn render_unsecure(&self, node: &UnsecureNode, context: &Context) -> Result<String> {
        let location = node.location;
        let value = context.resolve(node.path.segments(), location)?;
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
            let iteration = self.render_nodes(&node.body, context)?;
            context.pop_scope();

            output.push_str(&iteration);
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

    #[test]
    fn test_null_without_modifier_error() {
        let result = render("{[ value ]}", json!({"value": null}));
        assert!(matches!(result, Err(NatsuzoraError::TypeError { .. })));
    }

    #[test]
    fn test_nullable_modifier() {
        let result = render("{[ value? ]}", json!({"value": null})).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_required_modifier_null_error() {
        let result = render("{[ value! ]}", json!({"value": null}));
        assert!(matches!(result, Err(NatsuzoraError::TypeError { .. })));
    }

    #[test]
    fn test_required_modifier_empty_string_error() {
        let result = render("{[ value! ]}", json!({"value": ""}));
        assert!(matches!(result, Err(NatsuzoraError::TypeError { .. })));
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
        let result = render("Hello{[% comment ]}World", json!({})).unwrap();
        assert_eq!(result, "HelloWorld");
    }
}
