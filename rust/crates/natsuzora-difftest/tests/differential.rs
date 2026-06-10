//! Differential property test: the Rust and Ruby implementations must
//! produce identical output (or the same canonical error type) for
//! every generated case. See `spec/difftest.md`.

use natsuzora_difftest::generator::test_case;
use natsuzora_difftest::runner::run_rust;
use natsuzora_difftest::worker::run_ruby;
use proptest::prelude::*;

proptest! {
    // 512 cases per run (spec/difftest.md); override with PROPTEST_CASES.
    #![proptest_config(ProptestConfig {
        cases: 512,
        .. ProptestConfig::default()
    })]

    #[test]
    fn ruby_and_rust_agree(case in test_case()) {
        let rust = run_rust(&case);
        let ruby = run_ruby(&case);
        prop_assert_eq!(
            &rust,
            &ruby,
            "implementations diverged\ncase: {}",
            serde_json::to_string_pretty(&case).unwrap()
        );
    }
}
