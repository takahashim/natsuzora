//! Ad-hoc reproduction tool: reads a TestCase as JSON from stdin,
//! prints the outcome of both implementations.
//!
//! cargo run -p natsuzora-difftest --example repro <<'EOF'
//! {"template": "...", "data": {...}, "partials": {...}}
//! EOF

use std::io::Read;

use natsuzora_difftest::{runner, worker, TestCase};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let case: TestCase = serde_json::from_str(&input).expect("invalid TestCase JSON");

    println!("rust: {:?}", runner::run_rust(&case));
    println!("ruby: {:?}", worker::run_ruby(&case));
}
