//! Tree-sitter language bindings for the Natsuzora template language.

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_natsuzora() -> Language;
}

/// Returns the tree-sitter [`Language`] for Natsuzora templates.
pub fn language() -> Language {
    unsafe { tree_sitter_natsuzora() }
}

/// Returns the JSON description of the node types.
pub fn node_types_json() -> &'static str {
    include_str!("../../../../tree-sitter/src/node-types.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_loads() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(language()).unwrap();
        let tree = parser.parse("Hello, {[ name ]}!", None).unwrap();
        assert!(!tree.root_node().has_error());
    }
}
