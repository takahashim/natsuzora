//! Integration tests using shared test cases from tests/*.json

use natsuzora::render;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct TestSuite {
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
    let content = fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()));
    serde_json::from_str(&content).expect(&format!("Failed to parse {}", filename))
}

fn run_test_case(case: &TestCase) {
    let result = render(&case.template, case.data.clone());

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
                let error_name = format!("{:?}", e);
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
        (ParseError { message, .. }, "ParseError") => true,
        (ParseError { message, .. }, "LexerError") => {
            // LexerError maps to ParseError with specific patterns
            message.contains("syntax error") || message.contains("identifier")
        }
        (ParseError { message, .. }, "ReservedWordError") => {
            message.contains("reserved word")
        }
        (UndefinedVariable { .. }, "UndefinedVariable") => true,
        (TypeError { .. }, "TypeError") => true,
        (NullValueError { .. }, "NullValueError") => true,
        (EmptyStringError { .. }, "EmptyStringError") => true,
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

    eprintln!(
        "{}: {} tests passed, {} skipped",
        filename, passed, skipped
    );
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

// Include tests are skipped - they require fixture files
// #[test]
// fn test_include() {
//     run_test_suite("include.json", &[]);
// }
