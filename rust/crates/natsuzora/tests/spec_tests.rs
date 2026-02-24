//! Integration tests using shared test cases from tests/*.json

use natsuzora::{render, render_with_includes};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct TestSuite {
    #[allow(dead_code)]
    description: String,
    tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    template: String,
    data: serde_json::Value,
    #[serde(default)]
    expected: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    partials: Option<HashMap<String, String>>,
}

fn get_tests_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
}

fn load_test_suite(filename: &str) -> TestSuite {
    let path = get_tests_dir().join(filename);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {}", path.display()));
    serde_json::from_str(&content).unwrap_or_else(|_| panic!("Failed to parse {filename}"))
}

fn setup_partials(partials: &HashMap<String, String>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    for (name, content) in partials {
        let segments: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
        let mut path = dir.path().to_path_buf();
        for (i, seg) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                path.push(format!("_{seg}"));
            } else {
                path.push(seg);
            }
        }
        path.set_extension("ntzr");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create partial dir");
        }
        fs::write(&path, content).expect("Failed to write partial");
    }
    dir
}

fn run_test_case(case: &TestCase) {
    let result = if let Some(partials) = &case.partials {
        let dir = setup_partials(partials);
        render_with_includes(&case.template, case.data.clone(), dir.path())
    } else {
        render(&case.template, case.data.clone())
    };

    if let Some(expected) = &case.expected {
        match result {
            Ok(output) => assert_eq!(
                &output, expected,
                "Test '{}' failed: expected '{}', got '{}'",
                case.name, expected, output
            ),
            Err(e) => panic!(
                "Test '{}' should succeed with '{}', but got error: {:?}",
                case.name, expected, e
            ),
        }
    } else if let Some(error_type) = &case.error {
        match result {
            Ok(output) => panic!(
                "Test '{}' should fail with {}, but succeeded with '{}'",
                case.name, error_type, output
            ),
            Err(e) => {
                let error_name = format!("{e:?}");
                assert!(
                    error_name.contains(error_type) || error_type_matches(&e, error_type),
                    "Test '{}' expected error type '{}', got '{:?}'",
                    case.name,
                    error_type,
                    e
                );
            }
        }
    }
}

fn error_type_matches(e: &natsuzora::NatsuzoraError, expected: &str) -> bool {
    use natsuzora::NatsuzoraError::*;
    match (e, expected) {
        // SyntaxError matches any parse/lexer error (implementation detail)
        (ParseError { .. }, "SyntaxError") => true,
        (ParseError { .. }, "ParseError") => true,
        (ParseError { message, .. }, "LexerError") => {
            // LexerError maps to ParseError with specific patterns
            message.contains("syntax error") || message.contains("identifier")
        }
        (ParseError { message, .. }, "ReservedWordError") => message.contains("reserved word"),
        (UndefinedVariable { .. }, "UndefinedVariable") => true,
        (TypeError { .. }, "TypeError") => true,
        (TypeError { .. }, "NullValueError") => true,
        (TypeError { .. }, "EmptyStringError") => true,
        (ShadowingError { .. }, "ShadowingError") => true,
        (IncludeError { .. }, "IncludeError") => true,
        _ => false,
    }
}

fn run_test_suite(filename: &str, skip_tests: &[&str]) {
    let suite = load_test_suite(filename);
    let mut passed = 0;
    let mut skipped = 0;

    for case in &suite.tests {
        if skip_tests.contains(&case.name.as_str()) {
            skipped += 1;
            continue;
        }
        run_test_case(case);
        passed += 1;
    }

    eprintln!("{filename}: {passed} tests passed, {skipped} skipped");
}

#[test]
fn test_basic() {
    run_test_suite("basic.json", &[]);
}

#[test]
fn test_stringify() {
    run_test_suite("stringify.json", &[]);
}

#[test]
fn test_errors() {
    run_test_suite("errors.json", &[]);
}

#[test]
fn test_if_block() {
    run_test_suite("if_block.json", &[]);
}

#[test]
fn test_each_block() {
    run_test_suite("each_block.json", &[]);
}

#[test]
fn test_truthiness() {
    run_test_suite("truthiness.json", &[]);
}

#[test]
fn test_unsecure() {
    run_test_suite("unsecure.json", &[]);
}

#[test]
fn test_comment() {
    run_test_suite("comment.json", &[]);
}

#[test]
fn test_whitespace_control() {
    run_test_suite("whitespace_control.json", &[]);
}

#[test]
fn test_include() {
    run_test_suite("include.json", &[]);
}

#[test]
fn test_delimiter_escape() {
    run_test_suite("delimiter_escape.json", &[]);
}

#[test]
fn test_unless_block() {
    run_test_suite("unless_block.json", &[]);
}

#[test]
fn test_block_errors() {
    run_test_suite("block_errors.json", &[]);
}

#[test]
fn test_edge_cases() {
    run_test_suite("edge_cases.json", &[]);
}
