use clap::Parser;
use runner::Runner;
use utils::{error::WorkerError, run_with_runtime};

mod parse;
mod rule;
mod runner;
mod utils;

/// Main entry point of the application.
#[tracing::instrument]
fn main() -> Result<(), WorkerError> {
    run_with_runtime()
}
