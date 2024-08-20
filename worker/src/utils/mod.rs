pub mod config;
pub mod constants;
pub mod data;
pub mod error;
pub mod msg;
pub mod traits;

use std::sync::atomic::Ordering::SeqCst;
use tokio::runtime::Runtime;

use crate::runner::Runner;

use self::{
    constants::{ADD_MEMORY_VALUE_ORDER, CONFIG_PATH, DEFAULT_MEMORY_VALUE_ORDER},
    error::WorkerError,
};

use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;

static TOKIO_THREADS_TOTAL: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));
static TOKIO_THREADS_ALIVE: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));

/// Sets the runtime for the application.
fn set_runtime() -> Result<Runtime, WorkerError> {
    tokio::runtime::Builder::new_multi_thread()
        .on_thread_start(|| {
            TOKIO_THREADS_ALIVE.fetch_add(ADD_MEMORY_VALUE_ORDER, SeqCst);
            TOKIO_THREADS_TOTAL.fetch_add(ADD_MEMORY_VALUE_ORDER, SeqCst);
        })
        .on_thread_stop(|| {
            TOKIO_THREADS_ALIVE.fetch_sub(ADD_MEMORY_VALUE_ORDER, SeqCst);
        })
        .enable_all()
        .build()
        .map_err(|_| WorkerError::InvalidRuntime)
}

/// Runs the application with the runtime.
pub fn run_with_runtime() -> Result<(), WorkerError> {
    let runtime = set_runtime()?;

    let result = runtime.block_on(async {
        let runner = Runner::new(CONFIG_PATH).await?;
        runner.run().await
    });

    drop(runtime);

    result.map_err(|err| WorkerError::GeneralShutdown(err.to_string()))
}
