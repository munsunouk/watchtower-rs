mod fetcher;
mod manager;
mod rule;
mod runner;
mod utils;

use clap::{arg, command, Parser};
use utils::{error::WorkerError, run_with_runtime};

/// Command-line arguments structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long)]
    config_path: String,
}

/// Main entry point of the application.
#[tracing::instrument]
fn main() -> Result<(), WorkerError> {
    let args = Args::parse();
    run_with_runtime(&args.config_path)
}
