//! Differential testing harness between the Rust and Ruby Natsuzora
//! implementations (see `spec/difftest.md`).
//!
//! The driver lives here: proptest strategies generate template + data
//! cases, the Rust implementation is called in-process, and the Ruby
//! implementation is reached through a persistent JSONL worker process.

pub mod ast;
pub mod case;
pub mod env;
pub mod generator;
pub mod runner;
pub mod worker;

pub use case::{Outcome, TestCase};
