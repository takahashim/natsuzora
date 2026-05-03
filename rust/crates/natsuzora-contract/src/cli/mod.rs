mod commands;
mod loader;
mod project;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub(super) enum OutputFormat {
    /// Human-readable contract notation
    Contract,
    /// JSON format
    Json,
}

#[derive(Parser, Debug)]
#[command(
    name = "natsuzora-contract",
    about = "Natsuzora contract notation - validate data and extract schemas"
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract a contract from a template
    Extract(ExtractArgs),
    /// Check a template against a contract (reports violations)
    Check(CheckArgs),
    /// Validate JSON data against a contract
    Validate(ValidateArgs),
    /// Parse a contract file and re-output it
    Parse(ParseArgs),
    /// Extract contracts from templates and sync with .ntzc files
    Sync(SyncArgs),
    /// Apply diff markers to promote contracts to next generation
    Apply(ApplyArgs),
}

#[derive(Parser, Debug)]
pub(super) struct ExtractArgs {
    /// Template file (.ntzr) to extract a contract from
    pub template: PathBuf,

    /// Root directory for include partials
    #[arg(long)]
    pub include_root: Option<PathBuf>,

    /// Output format
    #[arg(long, short, value_enum, default_value = "contract")]
    pub format: OutputFormat,

    /// Write output to a file instead of stdout
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Validate JSON data against the extracted contract
    #[arg(long)]
    pub data: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub(super) struct CheckArgs {
    /// Template file (.ntzr) or project directory
    pub path: Option<PathBuf>,

    /// Contract file (single-file mode only)
    #[arg(long)]
    pub contract: Option<PathBuf>,

    /// Root directory for include partials
    #[arg(long)]
    pub include_root: Option<PathBuf>,

    /// Templates directory (batch mode)
    #[arg(long, short = 'T')]
    pub templates_dir: Option<PathBuf>,

    /// Contracts directory (batch mode)
    #[arg(long, short = 'C')]
    pub contracts_dir: Option<PathBuf>,

    /// Check only a specific template, e.g. "cards/show" (batch mode)
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Parser, Debug)]
pub(super) struct ValidateArgs {
    /// Contract file (.ntzc) to validate against
    pub contract: PathBuf,

    /// JSON data file to validate
    #[arg(long)]
    pub data: PathBuf,
}

#[derive(Parser, Debug)]
pub(super) struct ParseArgs {
    /// Contract file (.ntzc) to parse
    pub contract: PathBuf,

    /// Output format
    #[arg(long, short, value_enum, default_value = "contract")]
    pub format: OutputFormat,

    /// Write output to a file instead of stdout
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub(super) struct SyncArgs {
    /// Project directory
    pub path: Option<PathBuf>,

    /// Directory containing .ntzr template files
    #[arg(long, short = 'T')]
    pub templates_dir: Option<PathBuf>,

    /// Directory containing .ntzc contract files
    #[arg(long, short = 'C')]
    pub contracts_dir: Option<PathBuf>,

    /// Root directory for include partials (default: same as templates_dir)
    #[arg(long)]
    pub include_root: Option<PathBuf>,

    /// Sync only a specific template (e.g. "cards/show")
    #[arg(long)]
    pub name: Option<String>,

    /// Show diff without writing files
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub(super) struct ApplyArgs {
    /// Project directory
    pub path: Option<PathBuf>,

    /// Directory containing .ntzc contract files
    #[arg(long, short = 'C')]
    pub contracts_dir: Option<PathBuf>,

    /// Apply only a specific contract (e.g. "cards/show")
    #[arg(long)]
    pub name: Option<String>,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Extract(a) => commands::run_extract(a),
        Commands::Check(a) => commands::run_check(a),
        Commands::Validate(a) => commands::run_validate(a),
        Commands::Parse(a) => commands::run_parse(a),
        Commands::Sync(a) => commands::run_sync(a),
        Commands::Apply(a) => commands::run_apply(a),
    }
}
