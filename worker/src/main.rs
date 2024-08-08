use runner::Runner;

mod fetcher;
mod manager;
mod rule;
mod runner;
mod utils;

use anyhow::Result;
use utils::{constants::CONFIG_PATH, set_runtime};

/// Main entry point of the application.
#[tracing::instrument]
fn main() -> Result<()> {
    let runtime = set_runtime().unwrap();

    runtime.block_on(async {
        let runner = Runner::new(CONFIG_PATH).await;
        runner.run().await?;
        Ok(())
    })
}
