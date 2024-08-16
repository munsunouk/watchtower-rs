mod fetcher;
mod manager;
mod rule;
mod runner;
mod utils;

use utils::{error::WorkerError, run_with_runtime};

/// Main entry point of the application.
#[tracing::instrument]
fn main() -> Result<(), WorkerError> {
    run_with_runtime()
}
