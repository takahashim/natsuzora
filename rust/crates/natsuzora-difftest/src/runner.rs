//! In-process execution of a test case with the Rust implementation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use natsuzora::NatsuzoraError;

use crate::case::{Outcome, TestCase};

/// Renders a case with the Rust implementation, normalizing the result.
pub fn run_rust(case: &TestCase) -> Outcome {
    let result = if case.partials.is_empty() {
        natsuzora::render(&case.template, case.data.clone())
    } else {
        let dir = tempfile::tempdir().expect("failed to create tempdir for partials");
        materialize_partials(dir.path(), &case.partials);
        natsuzora::render_with_includes(&case.template, case.data.clone(), dir.path())
    };

    match result {
        Ok(output) => Outcome::Output(output),
        Err(e) => Outcome::Error(canonical_error(&e)),
    }
}

/// Same materialization rule as the shared spec tests and the Ruby
/// worker: "/a/b" -> <root>/a/_b.ntzr
fn materialize_partials(root: &Path, partials: &BTreeMap<String, String>) {
    for (name, content) in partials {
        let segments: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
        let (last, dirs) = segments.split_last().expect("empty partial name");
        let mut path = root.to_path_buf();
        for dir in dirs {
            path.push(dir);
        }
        fs::create_dir_all(&path).expect("failed to create partial directory");
        path.push(format!("_{last}.ntzr"));
        fs::write(&path, content).expect("failed to write partial");
    }
}

/// Maps a Rust-side error to the canonical error type of the difftest
/// protocol. Mirrors `error_type_matches` in the shared spec tests:
/// the Rust implementation reports reserved-word usage as a ParseError
/// whose message contains "reserved word".
pub fn canonical_error(error: &NatsuzoraError) -> String {
    match error {
        NatsuzoraError::ParseError { message, .. } if message.contains("reserved word") => {
            "ReservedWordError".to_string()
        }
        NatsuzoraError::ParseError { .. } => "ParseError".to_string(),
        NatsuzoraError::UndefinedVariable { .. } => "UndefinedVariable".to_string(),
        NatsuzoraError::TypeError { .. } => "TypeError".to_string(),
        NatsuzoraError::IncludeError { .. } => "IncludeError".to_string(),
        NatsuzoraError::ShadowingError { .. } => "ShadowingError".to_string(),
        NatsuzoraError::IoError(e) => format!("Unmapped(IoError: {e})"),
        NatsuzoraError::WithIncludeTrace { source, .. } => canonical_error(source),
    }
}
