//! Rust 側で `natsuzora/tests/contract/*.json` の共有テストを消費する。
//! Ruby 側 (`natsuzora/ruby/spec/contract/shared_tests_spec.rb`) と同じ JSON
//! ファイルを読み、両言語で同じ結果になることで実装間ドリフトを検知する。

use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use natsuzora_contract::{parse, validate};

#[derive(Debug, Deserialize)]
struct TestSuite {
    #[allow(dead_code)]
    description: String,
    tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    schema: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    should_parse: Option<bool>,
    #[serde(default)]
    valid: Option<bool>,
}

fn shared_tests_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // crates/natsuzora-contract/Cargo.toml から ../../tests/contract/ を見る
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("..")
        .join("tests")
        .join("contract")
}

fn load_suite(name: &str) -> TestSuite {
    let path = shared_tests_dir().join(name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e))
}

#[test]
fn parser_basic_shared_tests() {
    let suite = load_suite("parser_basic.json");
    let mut failures: Vec<String> = Vec::new();

    for tc in &suite.tests {
        let should_parse = tc.should_parse.expect("should_parse missing");
        let result = parse(&tc.schema);
        match (should_parse, result.is_ok()) {
            (true, true) | (false, false) => { /* expected */ }
            (true, false) => {
                failures.push(format!(
                    "{}: expected to parse, but failed: {:?}",
                    tc.name,
                    result.err()
                ));
            }
            (false, true) => {
                failures.push(format!(
                    "{}: expected to fail to parse, but parsed",
                    tc.name
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "parser failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn validator_basic_shared_tests() {
    let suite = load_suite("validator_basic.json");
    let mut failures: Vec<String> = Vec::new();

    for tc in &suite.tests {
        let valid_expected = tc.valid.expect("valid missing");
        let data = tc.data.as_ref().expect("data missing");

        let contract = match parse(&tc.schema) {
            Ok(c) => c,
            Err(e) => {
                failures.push(format!("{}: schema failed to parse: {:?}", tc.name, e));
                continue;
            }
        };

        let result = validate(&contract, data);
        match (valid_expected, result.is_ok()) {
            (true, true) | (false, false) => { /* expected */ }
            (true, false) => {
                failures.push(format!(
                    "{}: expected valid, but error: {:?}",
                    tc.name,
                    result.err()
                ));
            }
            (false, true) => {
                failures.push(format!("{}: expected invalid, but validated", tc.name));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "validator failures:\n{}",
        failures.join("\n")
    );
}
