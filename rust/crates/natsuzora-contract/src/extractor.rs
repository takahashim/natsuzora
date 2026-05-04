//! Contract extraction from templates.
//!
//! This module analyzes template ASTs to extract data contracts.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use bitflags::bitflags;
use natsuzora::ast::{
    AstNode, EachBlock, IfBlock, IncludeLoader, IncludeNode, Modifier, Path, Template,
    UnlessBlock, UnsecureNode, VariableNode,
};
use thiserror::Error;

use crate::types::{Contract, ContractModifier, ScalarType};

/// Errors that can occur during contract extraction.
#[derive(Debug, Error)]
pub enum ContractError {
    #[error("include cycle detected for '{name}'")]
    IncludeCycle { name: String },
    #[error("failed to load include '{name}': {source}")]
    IncludeLoad {
        name: String,
        #[source]
        source: natsuzora::ast::LoaderError,
    },
}

/// Extract a contract from a template.
///
/// This function walks the template AST and infers the data structure
/// required by the template based on variable references, conditionals,
/// and loops.
///
/// # Example
///
/// ```rust,ignore
/// use natsuzora::ast::parse;
/// use natsuzora_contract::extract_contract;
///
/// let template = parse("Hello {[ user.name ]}!").unwrap();
/// let contract = extract_contract(&template, &mut loader).unwrap();
/// // contract describes: { user: { name: scalar } }
/// ```
pub fn extract_contract<L: IncludeLoader>(
    template: &Template,
    loader: &mut L,
) -> Result<Contract, ContractError> {
    let mut analyzer = Analyzer::new(loader);
    analyzer.walk_template(template)?;
    Ok(analyzer.finish())
}

// ============================================================================
// Internal: Constraint Building
// ============================================================================

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct ScalarKind: u8 {
        const STRING = 0b0001;
        const INTEGER = 0b0010;
        const BOOL = 0b0100;
        const NULL = 0b1000;
    }
}

#[derive(Clone)]
struct ConstraintRef(Rc<RefCell<ConstraintNode>>);

#[derive(Clone)]
enum ConstraintNode {
    Any,
    Scalar(ScalarConstraint),
    Object(ObjectShape),
    Array { items: ConstraintRef },
}

#[derive(Clone)]
struct ScalarConstraint {
    kind: ScalarKind,
    allow_empty: bool,
}

#[derive(Clone, Default)]
struct ObjectShape {
    required: BTreeSet<String>,
    properties: BTreeMap<String, ConstraintRef>,
}

impl ConstraintRef {
    fn any() -> Self {
        Self(Rc::new(RefCell::new(ConstraintNode::Any)))
    }

    fn object() -> Self {
        Self(Rc::new(RefCell::new(ConstraintNode::Object(
            ObjectShape::default(),
        ))))
    }

    fn ensure_property(&self, key: &str) -> ConstraintRef {
        if let Some(prop) = self.try_get_property(key) {
            return prop;
        }
        {
            let mut node = self.0.borrow_mut();
            match &mut *node {
                ConstraintNode::Object(obj) => {
                    obj.required.insert(key.to_string());
                    return obj
                        .properties
                        .entry(key.to_string())
                        .or_insert_with(ConstraintRef::any)
                        .clone();
                }
                _ => {
                    *node = ConstraintNode::Object(ObjectShape::default());
                }
            }
        }
        self.ensure_property(key)
    }

    fn try_get_property(&self, key: &str) -> Option<ConstraintRef> {
        let node = self.0.borrow();
        match &*node {
            ConstraintNode::Object(obj) => obj.properties.get(key).cloned(),
            _ => None,
        }
    }

    fn ensure_array_items(&self) -> ConstraintRef {
        {
            let mut node = self.0.borrow_mut();
            match &mut *node {
                ConstraintNode::Array { items } => items.clone(),
                ConstraintNode::Any => {
                    let items = ConstraintRef::object();
                    *node = ConstraintNode::Array {
                        items: items.clone(),
                    };
                    items
                }
                _ => {
                    let items = ConstraintRef::object();
                    *node = ConstraintNode::Array {
                        items: items.clone(),
                    };
                    items
                }
            }
        }
    }

    fn constrain_scalar(&self, spec: ScalarConstraint) {
        let mut node = self.0.borrow_mut();
        match &mut *node {
            ConstraintNode::Any => {
                *node = ConstraintNode::Scalar(spec);
            }
            ConstraintNode::Scalar(existing) => {
                existing.kind &= spec.kind;
                existing.allow_empty &= spec.allow_empty;
            }
            _ => {
                *node = ConstraintNode::Any;
            }
        }
    }

    fn constrain_bool(&self) {
        let mut node = self.0.borrow_mut();
        match &mut *node {
            ConstraintNode::Any => {
                *node = ConstraintNode::Scalar(ScalarConstraint {
                    kind: ScalarKind::BOOL,
                    allow_empty: true,
                });
            }
            ConstraintNode::Scalar(existing) => {
                existing.kind &= ScalarKind::BOOL;
            }
            _ => {}
        }
    }

    fn to_contract(&self) -> Contract {
        match &*self.0.borrow() {
            ConstraintNode::Any => Contract::Any,
            ConstraintNode::Scalar(spec) => {
                let scalar_type = if spec.kind == ScalarKind::STRING {
                    ScalarType::String
                } else if spec.kind == ScalarKind::INTEGER {
                    ScalarType::Integer
                } else if spec.kind == ScalarKind::BOOL {
                    ScalarType::Bool
                } else {
                    ScalarType::Scalar
                };

                let modifier = if spec.kind.contains(ScalarKind::NULL) {
                    ContractModifier::Nullable
                } else if !spec.allow_empty {
                    ContractModifier::Required
                } else {
                    ContractModifier::None
                };

                Contract::Scalar {
                    scalar_type,
                    modifier,
                }
            }
            ConstraintNode::Object(obj) => {
                let required = obj.required.iter().cloned().collect();
                let properties = obj
                    .properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_contract()))
                    .collect();
                Contract::Object {
                    required,
                    properties,
                }
            }
            ConstraintNode::Array { items } => Contract::Array {
                items: Box::new(items.to_contract()),
            },
        }
    }
}

// ============================================================================
// Internal: Analyzer
// ============================================================================

struct ScopeFrame {
    name: Option<String>,
    target: ConstraintRef,
}

struct Analyzer<'a, L> {
    loader: &'a mut L,
    include_stack: Vec<String>,
    root: ConstraintRef,
}

impl<'a, L: IncludeLoader> Analyzer<'a, L> {
    fn new(loader: &'a mut L) -> Self {
        Self {
            loader,
            include_stack: Vec::new(),
            root: ConstraintRef::object(),
        }
    }

    fn finish(self) -> Contract {
        self.root.to_contract()
    }

    fn walk_template(&mut self, template: &Template) -> Result<(), ContractError> {
        let mut scopes = vec![ScopeFrame {
            name: None,
            target: self.root.clone(),
        }];
        self.walk_nodes(template.nodes(), &mut scopes)
    }

    fn walk_nodes(
        &mut self,
        nodes: &[AstNode],
        scopes: &mut Vec<ScopeFrame>,
    ) -> Result<(), ContractError> {
        for node in nodes {
            match node {
                AstNode::Text(_) => {}
                AstNode::Variable(var) => {
                    self.constrain_variable(var, scopes);
                }
                AstNode::Unsecure(unsecure) => {
                    self.constrain_unsecure(unsecure, scopes);
                }
                AstNode::If(block) => {
                    self.handle_if(block, scopes)?;
                }
                AstNode::Unless(block) => {
                    self.handle_unless(block, scopes)?;
                }
                AstNode::Each(block) => {
                    self.handle_each(block, scopes)?;
                }
                AstNode::Include(include) => {
                    self.handle_include(include, scopes)?;
                }
            }
        }
        Ok(())
    }

    fn constrain_variable(&self, var: &VariableNode, scopes: &[ScopeFrame]) {
        let node = self.ensure_path(resolve_path(scopes, &var.path));
        let spec = scalar_constraint_for_modifier(var.modifier);
        node.constrain_scalar(spec);
    }

    fn constrain_unsecure(&self, unsecure: &UnsecureNode, scopes: &[ScopeFrame]) {
        let node = self.ensure_path(resolve_path(scopes, &unsecure.path));
        // Unsecure has no modifier, use default (no null allowed)
        let spec = scalar_constraint_for_modifier(Modifier::None);
        node.constrain_scalar(spec);
    }

    fn handle_if(
        &mut self,
        block: &IfBlock,
        scopes: &mut Vec<ScopeFrame>,
    ) -> Result<(), ContractError> {
        // if/unless conditions use truthiness, so we mark them as bool
        let node = self.ensure_path(resolve_path(scopes, &block.condition));
        node.constrain_bool();
        self.walk_nodes(&block.then_branch, scopes)?;
        if let Some(else_branch) = &block.else_branch {
            self.walk_nodes(else_branch, scopes)?;
        }
        Ok(())
    }

    fn handle_unless(
        &mut self,
        block: &UnlessBlock,
        scopes: &mut Vec<ScopeFrame>,
    ) -> Result<(), ContractError> {
        let node = self.ensure_path(resolve_path(scopes, &block.condition));
        node.constrain_bool();
        self.walk_nodes(&block.body, scopes)?;
        Ok(())
    }

    fn ensure_path(&self, (target, remainder): (ConstraintRef, &[String])) -> ConstraintRef {
        let mut current = target;
        for segment in remainder {
            current = current.ensure_property(segment);
        }
        current
    }

    fn handle_each(
        &mut self,
        block: &EachBlock,
        scopes: &mut Vec<ScopeFrame>,
    ) -> Result<(), ContractError> {
        let collection = self.ensure_path(resolve_path(scopes, &block.collection));
        let items = collection.ensure_array_items();
        scopes.push(ScopeFrame {
            name: Some(block.item_ident.clone()),
            target: items.clone(),
        });
        self.walk_nodes(&block.body, scopes)?;
        scopes.pop();
        Ok(())
    }

    fn handle_include(
        &mut self,
        include: &IncludeNode,
        scopes: &mut Vec<ScopeFrame>,
    ) -> Result<(), ContractError> {
        if self.include_stack.contains(&include.name) {
            return Err(ContractError::IncludeCycle {
                name: include.name.clone(),
            });
        }
        let template =
            self.loader
                .load(&include.name)
                .map_err(|source| ContractError::IncludeLoad {
                    name: include.name.clone(),
                    source,
                })?;

        let mut arg_frames = Vec::new();
        for arg in &include.args {
            let resolved = self.ensure_path(resolve_path(scopes, &arg.value));
            arg_frames.push(ScopeFrame {
                name: Some(arg.name.clone()),
                target: resolved,
            });
        }

        self.include_stack.push(include.name.clone());
        for frame in &arg_frames {
            scopes.push(ScopeFrame {
                name: frame.name.clone(),
                target: frame.target.clone(),
            });
        }
        self.walk_nodes(template.nodes(), scopes)?;
        for _ in &arg_frames {
            scopes.pop();
        }
        self.include_stack.pop();
        Ok(())
    }
}

fn resolve_path<'a>(scopes: &'a [ScopeFrame], path: &'a Path) -> (ConstraintRef, &'a [String]) {
    for frame in scopes.iter().rev() {
        if let Some(name) = &frame.name {
            if let Some(rest) = strip_prefix(path.segments(), name) {
                return (frame.target.clone(), rest);
            }
        } else {
            return (frame.target.clone(), path.segments());
        }
    }
    unreachable!("root scope missing")
}

fn strip_prefix<'a>(segments: &'a [String], name: &str) -> Option<&'a [String]> {
    if segments.first().map(|s| s == name).unwrap_or(false) {
        Some(&segments[1..])
    } else {
        None
    }
}

fn scalar_constraint_for_modifier(modifier: Modifier) -> ScalarConstraint {
    let mut kind = ScalarKind::STRING | ScalarKind::INTEGER;
    let allow_empty = !matches!(modifier, Modifier::Required);
    if matches!(modifier, Modifier::Nullable) {
        kind |= ScalarKind::NULL;
    }
    ScalarConstraint { kind, allow_empty }
}

#[cfg(test)]
mod tests {
    use super::*;
    use natsuzora::ast::LoaderError;

    struct TestLoader;

    impl IncludeLoader for TestLoader {
        fn load(&mut self, _: &str) -> Result<Template, LoaderError> {
            unreachable!("no include expected")
        }
    }

    #[test]
    fn extracts_simple_object_contract() {
        let template = natsuzora::ast::parse("Hello {[ user.name ]}!").unwrap();
        let mut loader = TestLoader;
        let contract = extract_contract(&template, &mut loader).unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                let user = properties.get("user").expect("user property");
                match user {
                    Contract::Object { properties, .. } => {
                        assert!(matches!(
                            properties.get("name"),
                            Some(Contract::Scalar {
                                scalar_type: ScalarType::Scalar,
                                ..
                            })
                        ));
                    }
                    _ => panic!("user should be object"),
                }
            }
            _ => panic!("root should be object"),
        }
    }

    #[test]
    fn extracts_bool_for_if_condition() {
        let template = natsuzora::ast::parse("{[#if visible]}Hello{[/if]}").unwrap();
        let mut loader = TestLoader;
        let contract = extract_contract(&template, &mut loader).unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                assert!(matches!(
                    properties.get("visible"),
                    Some(Contract::Scalar {
                        scalar_type: ScalarType::Bool,
                        ..
                    })
                ));
            }
            _ => panic!("root should be object"),
        }
    }

    #[test]
    fn extracts_array_for_each() {
        let template =
            natsuzora::ast::parse("{[#each items as item]}{[ item.name ]}{[/each]}").unwrap();
        let mut loader = TestLoader;
        let contract = extract_contract(&template, &mut loader).unwrap();
        match contract {
            Contract::Object { properties, .. } => {
                match properties.get("items") {
                    Some(Contract::Array { items }) => {
                        match items.as_ref() {
                            Contract::Object { properties, .. } => {
                                assert!(properties.contains_key("name"));
                            }
                            _ => panic!("items should contain objects"),
                        }
                    }
                    _ => panic!("items should be array"),
                }
            }
            _ => panic!("root should be object"),
        }
    }
}
