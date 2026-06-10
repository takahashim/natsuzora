//! Grammar-based proptest strategies for Natsuzora templates.
//!
//! Generation follows `spec/bnf.md`: an AST (`crate::ast::Node`) is
//! generated against a statically known data environment
//! (`crate::env::Env`), then serialized to template source via
//! `TestCase::from_nodes`. Most generated references resolve to
//! renderable values ("consistent mode"); a weighted minority
//! deliberately produces undefined variables, type errors, reserved
//! words, or shadowing ("fault injection"), so that error
//! classification is also compared between implementations. The other
//! implementation is the oracle, so no expected output is computed
//! here.

use std::collections::HashSet;

use proptest::prelude::*;
use proptest::sample::select;
use proptest::strategy::Union;
use serde_json::Value;

use crate::ast::{Node, Trim, PADS};
use crate::case::TestCase;
use crate::env::Env;

pub const RESERVED: &[&str] = &[
    "if", "unless", "else", "each", "as", "in", "of", "unsecure", "true", "false", "null",
    "include",
];

fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

fn ident() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,5}".prop_filter("reserved word", |s| !is_reserved(s))
}

// ---------------------------------------------------------------------------
// Data generation
// ---------------------------------------------------------------------------

fn json_string() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => "[ -~]{0,8}",
        1 => Just(String::new()),
        1 => Just("0".to_string()),
        1 => Just("<b>&\"'</b>".to_string()),
        1 => Just("日本語テキスト".to_string()),
        1 => Just("line1\nline2".to_string()),
    ]
}

fn json_int() -> impl Strategy<Value = i64> {
    prop_oneof![
        4 => -100i64..=100i64,
        1 => Just(0i64),
        1 => Just(9_007_199_254_740_991i64),
        1 => Just(-9_007_199_254_740_991i64),
    ]
}

fn json_leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        4 => json_string().prop_map(Value::String),
        3 => json_int().prop_map(|i| Value::Number(i.into())),
        1 => any::<bool>().prop_map(Value::Bool),
        1 => Just(Value::Null),
    ]
}

fn json_value(depth: u32) -> BoxedStrategy<Value> {
    if depth == 0 {
        return json_leaf().boxed();
    }
    prop_oneof![
        5 => json_leaf(),
        2 => prop::collection::vec(json_value(depth - 1), 0..3).prop_map(Value::Array),
        2 => prop::collection::btree_map(ident(), json_value(depth - 1), 0..3)
            .prop_map(|m| Value::Object(m.into_iter().collect())),
    ]
    .boxed()
}

/// Root data: a non-empty object with identifier keys.
pub fn root_data() -> impl Strategy<Value = Value> {
    prop::collection::btree_map(ident(), json_value(2), 1..5)
        .prop_map(|m| Value::Object(m.into_iter().collect()))
}

// ---------------------------------------------------------------------------
// Path strategies (consistent mode + fault injection)
// ---------------------------------------------------------------------------

/// Whether a value is a good candidate for an output position.
/// Null is included deliberately: it is *not* stringifiable per spec
/// 3.4, but generating it exercises the `?` modifier paths and the
/// null-vs-error classification in both implementations.
fn output_candidate(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Null)
}

fn pick(items: Vec<String>) -> BoxedStrategy<String> {
    select(items).boxed()
}

fn undefined_name(env: &Env) -> BoxedStrategy<String> {
    let names: HashSet<String> = env.names().into_iter().collect();
    ident()
        .prop_filter("must be undefined", move |n| !names.contains(n))
        .boxed()
}

/// Path for a stringify position (variable / unsecure). Mostly resolves
/// to String/Integer/Null; faults: any value, undefined head, dangling
/// tail, reserved word.
fn output_path(env: &Env) -> BoxedStrategy<String> {
    let paths = env.paths();
    let good: Vec<String> = paths
        .iter()
        .filter(|(_, v)| output_candidate(v))
        .map(|(p, _)| p.clone())
        .collect();
    let all: Vec<String> = paths.iter().map(|(p, _)| p.clone()).collect();

    let mut arms: Vec<(u32, BoxedStrategy<String>)> = Vec::new();
    if !good.is_empty() {
        arms.push((20, pick(good)));
    }
    arms.push((2, pick(all.clone())));
    arms.push((1, undefined_name(env)));
    arms.push((
        1,
        pick(all).prop_map(|p| format!("{p}.zq")).boxed(), // dangling tail
    ));
    arms.push((1, select(RESERVED).prop_map(|s| s.to_string()).boxed()));
    Union::new_weighted(arms).boxed()
}

/// Path for an if/unless condition: any visible value, rarely undefined.
fn condition_path(env: &Env) -> BoxedStrategy<String> {
    let all: Vec<String> = env.paths().iter().map(|(p, _)| p.clone()).collect();
    Union::new_weighted(vec![(15, pick(all)), (1, undefined_name(env))]).boxed()
}

/// Path for an each target: mostly arrays; faults: any value, undefined.
fn each_target(env: &Env) -> BoxedStrategy<String> {
    let paths = env.paths();
    let arrays: Vec<String> = paths
        .iter()
        .filter(|(_, v)| v.is_array())
        .map(|(p, _)| p.clone())
        .collect();
    let all: Vec<String> = paths.iter().map(|(p, _)| p.clone()).collect();

    let mut arms: Vec<(u32, BoxedStrategy<String>)> = Vec::new();
    if !arrays.is_empty() {
        arms.push((15, pick(arrays)));
    }
    arms.push((2, pick(all)));
    arms.push((1, undefined_name(env)));
    Union::new_weighted(arms).boxed()
}

/// each loop variable: mostly fresh; fault: collides with a visible
/// name (ShadowingError).
fn loop_var(env: &Env) -> BoxedStrategy<String> {
    let names: HashSet<String> = env.names().into_iter().collect();
    let names_for_filter = names.clone();
    let fresh = ident()
        .prop_filter("must not shadow", move |n| !names_for_filter.contains(n))
        .boxed();
    let mut arms: Vec<(u32, BoxedStrategy<String>)> = vec![(12, fresh)];
    let visible: Vec<String> = names.into_iter().collect();
    if !visible.is_empty() {
        arms.push((1, pick(visible)));
    }
    Union::new_weighted(arms).boxed()
}

// ---------------------------------------------------------------------------
// Node strategies
// ---------------------------------------------------------------------------

fn trim() -> impl Strategy<Value = Trim> {
    let flag = prop_oneof![3 => Just(false), 1 => Just(true)];
    let flag2 = prop_oneof![3 => Just(false), 1 => Just(true)];
    (flag, flag2).prop_map(|(l, r)| Trim { l, r })
}

fn pad_index() -> impl Strategy<Value = u8> {
    prop_oneof![
        3 => Just(1u8), // single space, the common case
        1 => 0u8..PADS.len() as u8,
    ]
}

fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => "[ -~]{1,10}".prop_map(|s| s),
        2 => Just("  \n  ".to_string()),
        1 => Just("<p>&amp;</p>".to_string()),
        1 => Just("日本語".to_string()),
        1 => "[ \t\n]{1,4}".prop_map(|s| s),
        1 => Just("]}".to_string()),
    ]
    .prop_filter("text must not contain open delimiter", |s| {
        !s.contains("{[")
    })
}

fn comment_node() -> impl Strategy<Value = Node> {
    ("[a-zA-Z0-9_ <>&]{0,10}", trim()).prop_map(|(content, trim)| Node::Comment { content, trim })
}

fn var_modifier() -> impl Strategy<Value = Option<char>> {
    prop_oneof![
        8 => Just(None),
        2 => Just(Some('?')),
        1 => Just(Some('!')),
    ]
}

fn var_node(env: &Env) -> impl Strategy<Value = Node> {
    (
        output_path(env),
        var_modifier(),
        trim(),
        pad_index(),
        pad_index(),
    )
        .prop_map(|(path, modifier, trim, p0, p1)| Node::Var {
            path,
            modifier,
            trim,
            pad: (p0, p1),
        })
}

fn unsecure_node(env: &Env) -> impl Strategy<Value = Node> {
    (output_path(env), trim()).prop_map(|(path, trim)| Node::Unsecure { path, trim })
}

fn if_node(env: &Env, depth: u32) -> impl Strategy<Value = Node> {
    (
        condition_path(env),
        nodes(env, depth - 1),
        prop::option::of(nodes(env, depth - 1)),
        trim(),
        trim(),
    )
        .prop_map(|(cond, then, els, open, close)| Node::If {
            cond,
            then,
            els,
            open,
            close,
        })
}

fn unless_node(env: &Env, depth: u32) -> impl Strategy<Value = Node> {
    (condition_path(env), nodes(env, depth - 1), trim(), trim()).prop_map(
        |(cond, body, open, close)| Node::Unless {
            cond,
            body,
            open,
            close,
        },
    )
}

fn each_node(env: &Env, depth: u32) -> impl Strategy<Value = Node> {
    let env = env.clone();
    (each_target(&env), loop_var(&env)).prop_flat_map(move |(target, var)| {
        // Bind the loop variable to the first element's value so the
        // body can reference element sub-paths. Heterogeneous arrays
        // may still fail mid-iteration; both implementations must then
        // fail identically.
        let element = match env.resolve(&target) {
            Some(Value::Array(items)) if !items.is_empty() => items[0].clone(),
            _ => Value::Null,
        };
        let body_env = env.with(var.clone(), element);
        let target = target.clone();
        let var = var.clone();
        (nodes(&body_env, depth - 1), trim(), trim()).prop_map(move |(body, open, close)| {
            Node::Each {
                target: target.clone(),
                var: var.clone(),
                body,
                open,
                close,
            }
        })
    })
}

fn include_node(env: &Env, depth: u32) -> impl Strategy<Value = Node> {
    let env = env.clone();
    let all: Vec<String> = env.paths().iter().map(|(p, _)| p.clone()).collect();
    let name = prop::collection::vec(ident(), 1..3)
        .prop_map(|segments| format!("/{}", segments.join("/")));
    let args = prop::collection::btree_map(ident(), select(all), 0..3)
        .prop_map(|m| m.into_iter().collect::<Vec<_>>());

    (name, args, trim()).prop_flat_map(move |(name, args, trim)| {
        // Include arguments are evaluated in the caller scope; the
        // partial sees the caller scope plus the bound arguments
        // (arguments may legally shadow, spec 5.2).
        let mut body_env = env.clone();
        for (key, value_path) in &args {
            let value = env.resolve(value_path).unwrap_or(Value::Null);
            body_env = body_env.with(key.clone(), value);
        }
        let name = name.clone();
        let args = args.clone();
        nodes(&body_env, depth - 1).prop_map(move |body| Node::Include {
            name: name.clone(),
            args: args.clone(),
            body,
            trim,
        })
    })
}

pub fn nodes(env: &Env, depth: u32) -> BoxedStrategy<Vec<Node>> {
    prop::collection::vec(node(env, depth), 0..5).boxed()
}

fn node(env: &Env, depth: u32) -> BoxedStrategy<Node> {
    let mut arms: Vec<(u32, BoxedStrategy<Node>)> = vec![
        (4, text().prop_map(Node::Text).boxed()),
        (4, var_node(env).boxed()),
        (1, Just(Node::DelimEscape).boxed()),
        (1, comment_node().boxed()),
        (1, unsecure_node(env).boxed()),
    ];
    if depth > 0 {
        arms.push((2, if_node(env, depth).boxed()));
        arms.push((1, unless_node(env, depth).boxed()));
        arms.push((2, each_node(env, depth).boxed()));
        arms.push((1, include_node(env, depth).boxed()));
    }
    Union::new_weighted(arms).boxed()
}

// ---------------------------------------------------------------------------
// Top-level case strategy
// ---------------------------------------------------------------------------

pub fn test_case() -> impl Strategy<Value = TestCase> {
    root_data().prop_flat_map(|data| {
        let env = Env::from_root(&data);
        nodes(&env, 3).prop_map(move |ns| TestCase::from_nodes(&ns, data.clone()))
    })
}
