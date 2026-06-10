//! Statically known visible bindings: a scope model mirroring the
//! language's name resolution (root → each loop vars → include args),
//! used to generate paths that resolve — or deliberately don't.

use std::collections::HashSet;

use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Env {
    /// Visible bindings, innermost last (shadowing resolution order).
    bindings: Vec<(String, Value)>,
}

impl Env {
    pub fn from_root(root: &Value) -> Env {
        let bindings = match root {
            Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => Vec::new(),
        };
        Env { bindings }
    }

    pub(crate) fn with(&self, name: String, value: Value) -> Env {
        let mut env = self.clone();
        env.bindings.push((name, value));
        env
    }

    pub(crate) fn names(&self) -> Vec<String> {
        self.bindings.iter().map(|(n, _)| n.clone()).collect()
    }

    /// All visible dotted paths with the value they resolve to,
    /// including intermediate objects.
    pub(crate) fn paths(&self) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for (name, value) in self.bindings.iter().rev() {
            if !seen.insert(name.clone()) {
                continue; // shadowed by an inner binding
            }
            collect_paths(name, value, &mut out, 0);
        }
        out
    }

    pub(crate) fn resolve(&self, path: &str) -> Option<Value> {
        let mut parts = path.split('.');
        let head = parts.next()?;
        let mut current = self
            .bindings
            .iter()
            .rev()
            .find(|(name, _)| name == head)
            .map(|(_, value)| value.clone())?;
        for part in parts {
            current = current.get(part)?.clone();
        }
        Some(current)
    }
}

fn collect_paths(prefix: &str, value: &Value, out: &mut Vec<(String, Value)>, depth: u32) {
    out.push((prefix.to_string(), value.clone()));
    if depth >= 4 {
        return;
    }
    if let Value::Object(map) = value {
        for (key, child) in map {
            collect_paths(&format!("{prefix}.{key}"), child, out, depth + 1);
        }
    }
}
