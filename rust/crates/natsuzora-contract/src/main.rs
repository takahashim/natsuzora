//! CLI for Subaru schema language.
//!
//! Validate data and extract schemas from Natsuzora templates.

#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "cli")]
fn main() -> anyhow::Result<()> {
    cli::main()
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI feature is not enabled. Build with --features cli");
    std::process::exit(1);
}
