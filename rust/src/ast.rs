use crate::error::Location;

/// Root node representing a parsed template
#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Node>,
    pub location: Location,
}

/// All possible AST node types
#[derive(Debug, Clone)]
pub enum Node {
    Text(TextNode),
    Variable(VariableNode),
    IfBlock(IfBlockNode),
    UnlessBlock(UnlessBlockNode),
    EachBlock(EachBlockNode),
    UnsecureBlock(UnsecureBlockNode),
    Include(IncludeNode),
}

/// Raw text content
#[derive(Debug, Clone)]
pub struct TextNode {
    pub content: String,
    pub location: Location,
}

/// Variable reference with dot-separated path
#[derive(Debug, Clone)]
pub struct VariableNode {
    pub path: Vec<String>,
    pub location: Location,
}

/// Conditional block: {{#if condition}} ... {{#else}} ... {{/if}}
#[derive(Debug, Clone)]
pub struct IfBlockNode {
    pub condition: VariableNode,
    pub then_nodes: Vec<Node>,
    pub else_nodes: Option<Vec<Node>>,
    pub location: Location,
}

/// Inverse conditional block: {{#unless condition}} ... {{/unless}}
#[derive(Debug, Clone)]
pub struct UnlessBlockNode {
    pub condition: VariableNode,
    pub body_nodes: Vec<Node>,
    pub location: Location,
}

/// Loop block: {{#each collection as item}} ... {{/each}}
/// or {{#each collection as item, index}} ... {{/each}}
#[derive(Debug, Clone)]
pub struct EachBlockNode {
    pub collection: VariableNode,
    pub item_name: String,
    pub index_name: Option<String>,
    pub body_nodes: Vec<Node>,
    pub location: Location,
}

/// Unsecure block for raw HTML output: {{#unsecure}} ... {{/unsecure}}
#[derive(Debug, Clone)]
pub struct UnsecureBlockNode {
    pub nodes: Vec<Node>,
    pub location: Location,
}

/// Include directive: {{> /path/to/partial key=value}}
#[derive(Debug, Clone)]
pub struct IncludeNode {
    pub name: String,
    pub args: Vec<(String, VariableNode)>,
    pub location: Location,
}
