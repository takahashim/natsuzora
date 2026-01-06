//! Spec-based integration tests
//!
//! These tests run the shared test cases from natsuzora-spec/tests/
//! to verify compatibility with other implementations.

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TestFile {
    #[allow(dead_code)]
    description: String,
    tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    template: String,
    data: Value,
    expected: Option<String>,
    error: Option<String>,
}

fn run_test_file(filename: &str) {
    let spec_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join(filename);

    if !spec_path.exists() {
        eprintln!("Skipping {}: file not found at {:?}", filename, spec_path);
        return;
    }

    let content = fs::read_to_string(&spec_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));

    let test_file: TestFile = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", filename, e));

    for test in test_file.tests {
        run_single_test(&test, filename);
    }
}

fn run_single_test(test: &TestCase, filename: &str) {
    let result = natsuzora::render(&test.template, test.data.clone());

    match (&result, &test.expected, &test.error) {
        // Success case: output should match expected
        (Ok(output), Some(expected), None) => {
            assert_eq!(
                output, expected,
                "\n[{}] Test '{}' failed:\n  Template: {}\n  Data: {}\n  Expected: {:?}\n  Got: {:?}",
                filename, test.name, test.template, test.data, expected, output
            );
        }
        // Error case: should produce an error
        (Err(_), None, Some(_error_type)) => {
            // Expected error occurred - test passes
        }
        // Unexpected success when error was expected
        (Ok(output), None, Some(error_type)) => {
            panic!(
                "\n[{}] Test '{}' should have failed with {} but succeeded:\n  Template: {}\n  Data: {}\n  Output: {:?}",
                filename, test.name, error_type, test.template, test.data, output
            );
        }
        // Unexpected error when success was expected
        (Err(e), Some(expected), None) => {
            panic!(
                "\n[{}] Test '{}' failed with unexpected error:\n  Template: {}\n  Data: {}\n  Expected: {:?}\n  Error: {}",
                filename, test.name, test.template, test.data, expected, e
            );
        }
        // Invalid test case
        _ => {
            panic!(
                "\n[{}] Test '{}' has invalid configuration (must have either 'expected' or 'error')",
                filename, test.name
            );
        }
    }
}

#[test]
fn test_basic() {
    run_test_file("basic.json");
}

#[test]
fn test_if_block() {
    run_test_file("if_block.json");
}

#[test]
fn test_each_block() {
    run_test_file("each_block.json");
}

#[test]
fn test_unsecure() {
    run_test_file("unsecure.json");
}

#[test]
fn test_truthiness() {
    run_test_file("truthiness.json");
}

#[test]
fn test_stringify() {
    run_test_file("stringify.json");
}

#[test]
fn test_errors() {
    run_test_file("errors.json");
}

// Include tests are skipped for now as they require file system setup
// #[test]
// fn test_include() {
//     run_test_file("include.json");
// }
