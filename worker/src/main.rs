use clap::{arg, command, Parser};
use utils::{error::WorkerError, run_with_runtime};

mod parse;
mod rule;
mod runner;
mod utils;

/// Command-line arguments structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long)]
    config_path: String,

    /// Path to the parameter file
    #[arg(short, long)]
    param_path: String,
}

/// Main entry point of the application.
#[tracing::instrument]
fn main() -> Result<(), WorkerError> {
    let args = Args::parse();
    run_with_runtime(args)
}
