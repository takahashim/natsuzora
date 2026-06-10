//! Template AST for generated cases: node definitions, source
//! serialization, and partial collection.
//!
//! `TestCase::from_nodes` is the only way to turn a node tree into a
//! case: the template source and the partials map must be derived from
//! the same tree, so the pair of walks is encapsulated here and the
//! individual walks stay private.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::case::TestCase;

/// Whitespace-control flags (`{[-` / `-]}`) for one tag.
#[derive(Clone, Copy, Debug)]
pub struct Trim {
    pub l: bool,
    pub r: bool,
}

/// Padding variants for positions where the grammar allows `WS?`.
pub(crate) const PADS: &[&str] = &["", " ", "  ", "\t", "\n"];

#[derive(Clone, Debug)]
pub enum Node {
    Text(String),
    DelimEscape,
    Comment {
        content: String,
        trim: Trim,
    },
    Var {
        path: String,
        modifier: Option<char>,
        trim: Trim,
        pad: (u8, u8),
    },
    Unsecure {
        path: String,
        trim: Trim,
    },
    If {
        cond: String,
        then: Vec<Node>,
        els: Option<Vec<Node>>,
        open: Trim,
        close: Trim,
    },
    Unless {
        cond: String,
        body: Vec<Node>,
        open: Trim,
        close: Trim,
    },
    Each {
        target: String,
        var: String,
        body: Vec<Node>,
        open: Trim,
        close: Trim,
    },
    Include {
        name: String,
        args: Vec<(String, String)>,
        body: Vec<Node>,
        trim: Trim,
    },
}

impl TestCase {
    /// Builds a case from a generated node tree, deriving the template
    /// source and every include body (as partials) from the same tree.
    pub fn from_nodes(nodes: &[Node], data: Value) -> TestCase {
        let mut partials = BTreeMap::new();
        collect_partials(nodes, &mut partials);
        TestCase {
            template: to_source(nodes),
            data,
            partials,
        }
    }
}

fn dash(flag: bool) -> &'static str {
    if flag {
        "-"
    } else {
        ""
    }
}

fn to_source(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        write_node(node, &mut out);
    }
    out
}

fn write_node(node: &Node, out: &mut String) {
    match node {
        Node::Text(text) => out.push_str(text),
        Node::DelimEscape => out.push_str("{[{]}"),
        Node::Comment { content, trim } => {
            out.push_str(&format!(
                "{{[{}%{}{}]}}",
                dash(trim.l),
                content,
                dash(trim.r)
            ));
        }
        Node::Var {
            path,
            modifier,
            trim,
            pad,
        } => {
            let p0 = PADS[pad.0 as usize % PADS.len()];
            let p1 = PADS[pad.1 as usize % PADS.len()];
            let modifier = modifier.map(String::from).unwrap_or_default();
            out.push_str(&format!(
                "{{[{}{p0}{path}{modifier}{p1}{}]}}",
                dash(trim.l),
                dash(trim.r)
            ));
        }
        Node::Unsecure { path, trim } => {
            out.push_str(&format!(
                "{{[{}!unsecure {path} {}]}}",
                dash(trim.l),
                dash(trim.r)
            ));
        }
        Node::If {
            cond,
            then,
            els,
            open,
            close,
        } => {
            out.push_str(&format!(
                "{{[{}#if {cond} {}]}}",
                dash(open.l),
                dash(open.r)
            ));
            for n in then {
                write_node(n, out);
            }
            if let Some(els) = els {
                out.push_str("{[#else]}");
                for n in els {
                    write_node(n, out);
                }
            }
            out.push_str(&format!("{{[{}/if{}]}}", dash(close.l), dash(close.r)));
        }
        Node::Unless {
            cond,
            body,
            open,
            close,
        } => {
            out.push_str(&format!(
                "{{[{}#unless {cond} {}]}}",
                dash(open.l),
                dash(open.r)
            ));
            for n in body {
                write_node(n, out);
            }
            out.push_str(&format!("{{[{}/unless{}]}}", dash(close.l), dash(close.r)));
        }
        Node::Each {
            target,
            var,
            body,
            open,
            close,
        } => {
            out.push_str(&format!(
                "{{[{}#each {target} as {var} {}]}}",
                dash(open.l),
                dash(open.r)
            ));
            for n in body {
                write_node(n, out);
            }
            out.push_str(&format!("{{[{}/each{}]}}", dash(close.l), dash(close.r)));
        }
        Node::Include {
            name,
            args,
            body: _,
            trim,
        } => {
            let mut tag = format!("{{[{}!include {name}", dash(trim.l));
            for (key, value) in args {
                tag.push_str(&format!(" {key}={value}"));
            }
            tag.push_str(&format!(" {}]}}", dash(trim.r)));
            out.push_str(&tag);
        }
    }
}

/// Collects every include body in the tree as partial source, keyed by
/// include name. Duplicate names overwrite each other; both
/// implementations still see identical inputs, so cases stay valid.
fn collect_partials(nodes: &[Node], out: &mut BTreeMap<String, String>) {
    for node in nodes {
        match node {
            Node::If { then, els, .. } => {
                collect_partials(then, out);
                if let Some(els) = els {
                    collect_partials(els, out);
                }
            }
            Node::Unless { body, .. } | Node::Each { body, .. } => collect_partials(body, out),
            Node::Include { name, body, .. } => {
                out.insert(name.clone(), to_source(body));
                collect_partials(body, out);
            }
            _ => {}
        }
    }
}
