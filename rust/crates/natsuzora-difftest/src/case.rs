//! Test case and outcome types shared by the runner and the worker client.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One differential test case: a template, its input data, and the
/// partial templates referenced by `{[!include ...]}` tags.
///
/// Construct generated cases via `TestCase::from_nodes` (see
/// `crate::ast`); `Deserialize` is for replaying saved cases.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestCase {
    pub template: String,
    pub data: serde_json::Value,
    #[serde(default)]
    pub partials: BTreeMap<String, String>,
}

/// Result of rendering a case with one implementation, normalized for
/// comparison: either the output string or a canonical error type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Output(String),
    Error(String),
}
