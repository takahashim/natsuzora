use crate::ast::{
    EachBlockNode, IfBlockNode, IncludeNode, Node, Template, TextNode, UnlessBlockNode,
    UnsecureBlockNode, VariableNode,
};
use crate::context::Context;
use crate::error::{NatsuzoraError, Result};
use crate::html_escape;
use crate::template_loader::TemplateLoader;
use crate::value::Value;
use std::collections::HashMap;

/// Renderer for evaluating Natsuzora AST
pub struct Renderer<'a> {
    ast: &'a Template,
    template_loader: Option<&'a mut TemplateLoader>,
    escape_enabled: bool,
}

impl<'a> Renderer<'a> {
    /// Create a new renderer for the given AST
    pub fn new(ast: &'a Template, template_loader: Option<&'a mut TemplateLoader>) -> Self {
        Self {
            ast,
            template_loader,
            escape_enabled: true,
        }
    }

    /// Render the template with the given data
    pub fn render(&mut self, data: Value) -> Result<String> {
        let mut context = Context::new(data)?;
        // Copy the reference to avoid borrow conflict with &mut self
        let ast = self.ast;
        self.render_nodes(&ast.nodes, &mut context)
    }

    fn render_nodes(&mut self, nodes: &[Node], context: &mut Context) -> Result<String> {
        let mut output = String::new();
        for node in nodes {
            output.push_str(&self.render_node(node, context)?);
        }
        Ok(output)
    }

    fn render_node(&mut self, node: &Node, context: &mut Context) -> Result<String> {
        match node {
            Node::Text(n) => self.render_text(n),
            Node::Variable(n) => self.render_variable(n, context),
            Node::IfBlock(n) => self.render_if(n, context),
            Node::UnlessBlock(n) => self.render_unless(n, context),
            Node::EachBlock(n) => self.render_each(n, context),
            Node::UnsecureBlock(n) => self.render_unsecure(n, context),
            Node::Include(n) => self.render_include(n, context),
        }
    }

    fn render_text(&self, node: &TextNode) -> Result<String> {
        Ok(node.content.clone())
    }

    fn render_variable(&self, node: &VariableNode, context: &Context) -> Result<String> {
        let value = context.resolve(&node.path)?;
        let str_value = value.stringify()?;
        Ok(if self.escape_enabled {
            html_escape::escape(&str_value)
        } else {
            str_value
        })
    }

    fn render_if(&mut self, node: &IfBlockNode, context: &mut Context) -> Result<String> {
        let value = context.resolve(&node.condition.path)?;
        if value.is_truthy() {
            self.render_nodes(&node.then_nodes, context)
        } else if let Some(else_nodes) = &node.else_nodes {
            self.render_nodes(else_nodes, context)
        } else {
            Ok(String::new())
        }
    }

    fn render_unless(&mut self, node: &UnlessBlockNode, context: &mut Context) -> Result<String> {
        let value = context.resolve(&node.condition.path)?;
        if value.is_truthy() {
            Ok(String::new())
        } else {
            self.render_nodes(&node.body_nodes, context)
        }
    }

    fn render_each(&mut self, node: &EachBlockNode, context: &mut Context) -> Result<String> {
        let collection = context.resolve(&node.collection.path)?;
        // Clone the array to break the borrow chain - we need to mutate context in the loop
        let array = collection.as_array()?.clone();

        let mut output = String::new();
        for (index, item) in array.into_iter().enumerate() {
            let mut bindings = HashMap::new();
            bindings.insert(node.item_name.clone(), item);
            if let Some(index_name) = &node.index_name {
                bindings.insert(index_name.clone(), Value::Integer(index as i64));
            }

            context.push_scope(bindings)?;
            output.push_str(&self.render_nodes(&node.body_nodes, context)?);
            context.pop_scope();
        }

        Ok(output)
    }

    fn render_unsecure(
        &mut self,
        node: &UnsecureBlockNode,
        context: &mut Context,
    ) -> Result<String> {
        let prev_escape = self.escape_enabled;
        self.escape_enabled = false;
        let result = self.render_nodes(&node.nodes, context);
        self.escape_enabled = prev_escape;
        result
    }

    fn render_include(&mut self, node: &IncludeNode, context: &mut Context) -> Result<String> {
        // First, check if we have a template loader and load the partial
        let partial_ast = {
            let loader =
                self.template_loader
                    .as_mut()
                    .ok_or_else(|| NatsuzoraError::IncludeError {
                        message: "Template loader not configured for include".to_string(),
                    })?;
            loader.load(&node.name)?
        };

        // Resolve include arguments
        let mut bindings = HashMap::new();
        for (key, var) in &node.args {
            let value = context.resolve(&var.path)?.clone();
            bindings.insert(key.clone(), value);
        }

        // Push include onto stack
        if let Some(loader) = self.template_loader.as_mut() {
            loader.push_include(&node.name);
        }

        context.push_include_scope(bindings);
        let result = self.render_nodes(&partial_ast.nodes, context);
        context.pop_scope();

        // Pop include from stack
        if let Some(loader) = self.template_loader.as_mut() {
            loader.pop_include();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use serde_json::json;

    fn render(source: &str, data: serde_json::Value) -> Result<String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        let value = Value::from_json(data)?;
        let mut renderer = Renderer::new(&ast, None);
        renderer.render(value)
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
    fn test_render_each_with_index() {
        let result = render(
            "{[#each items as item, idx]}{[ idx ]}:{[ item ]},{[/each]}",
            json!({"items": ["a", "b"]}),
        )
        .unwrap();
        assert_eq!(result, "0:a,1:b,");
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
    fn test_render_unless_null() {
        let result = render("{[#unless x]}shown{[/unless]}", json!({"x": null})).unwrap();
        assert_eq!(result, "shown");
    }

    #[test]
    fn test_render_unless_zero() {
        let result = render("{[#unless x]}shown{[/unless]}", json!({"x": 0})).unwrap();
        assert_eq!(result, "shown");
    }

    #[test]
    fn test_render_unless_empty_string() {
        let result = render("{[#unless x]}shown{[/unless]}", json!({"x": ""})).unwrap();
        assert_eq!(result, "shown");
    }

    #[test]
    fn test_render_unless_non_zero() {
        let result = render("{[#unless x]}shown{[/unless]}", json!({"x": 1})).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_unsecure() {
        let result = render(
            "{[#unsecure]}{[ html ]}{[/unsecure]}",
            json!({"html": "<b>bold</b>"}),
        )
        .unwrap();
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
    fn test_truthiness_empty_string() {
        let result = render("{[#if value]}yes{[#else]}no{[/if]}", json!({"value": ""})).unwrap();
        assert_eq!(result, "no");
    }

    #[test]
    fn test_truthiness_zero() {
        let result = render("{[#if value]}yes{[#else]}no{[/if]}", json!({"value": 0})).unwrap();
        assert_eq!(result, "no");
    }

    #[test]
    fn test_truthiness_empty_array() {
        let result = render("{[#if value]}yes{[#else]}no{[/if]}", json!({"value": []})).unwrap();
        assert_eq!(result, "no");
    }

    #[test]
    fn test_stringify_null() {
        let result = render("[{[ value ]}]", json!({"value": null})).unwrap();
        assert_eq!(result, "[]");
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

    #[test]
    fn test_comment_with_spaces() {
        let result = render("Hello {[! comment ]} World", json!({})).unwrap();
        assert_eq!(result, "Hello  World");
    }

    #[test]
    fn test_multiline_comment() {
        let result = render("Hello{[! this is\na multi-line\ncomment ]}World", json!({})).unwrap();
        assert_eq!(result, "HelloWorld");
    }

    #[test]
    fn test_comment_between_variables() {
        let result = render("{[ a ]}{[! ignored ]}{[ b ]}", json!({"a": "1", "b": "2"})).unwrap();
        assert_eq!(result, "12");
    }

    #[test]
    fn test_comment_inside_block() {
        let result = render("{[#if x]}{[! comment ]}yes{[/if]}", json!({"x": true})).unwrap();
        assert_eq!(result, "yes");
    }

    #[test]
    fn test_whitespace_control_strip_before() {
        let result = render("line1\n  {[- name ]}", json!({"name": "Alice"})).unwrap();
        assert_eq!(result, "line1\nAlice");
    }

    #[test]
    fn test_whitespace_control_strip_after() {
        let result = render("{[ name -]}\nnext", json!({"name": "Alice"})).unwrap();
        assert_eq!(result, "Alicenext");
    }

    #[test]
    fn test_whitespace_control_both_sides() {
        let result = render("before\n  {[- name -]}\nafter", json!({"name": "Alice"})).unwrap();
        assert_eq!(result, "before\nAliceafter");
    }

    #[test]
    fn test_whitespace_control_with_each() {
        let template =
            "<ul>\n  {[-#each items as item-]}\n  <li>{[ item ]}</li>\n  {[-/each-]}\n</ul>";
        let result = render(template, json!({"items": ["a", "b"]})).unwrap();
        assert_eq!(result, "<ul>\n  <li>a</li>\n  <li>b</li>\n</ul>");
    }

    #[test]
    fn test_whitespace_control_with_if() {
        let template = "{[-#if x-]}\nyes\n{[-/if-]}\n";
        let result = render(template, json!({"x": true})).unwrap();
        assert_eq!(result, "yes\n");
    }
}
