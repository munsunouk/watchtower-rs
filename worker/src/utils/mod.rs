pub mod config;
pub mod constants;
pub mod error;
pub mod msg;
pub mod traits;

use std::sync::atomic::Ordering::SeqCst;

use crate::runner::Runner;

use self::{
    constants::{
        CONFIG_PATH, ADD_MEMORY_VALUE_ORDER, TOKIO_THREADS_ALIVE, TOKIO_THREADS_TOTAL,
    },
    error::WorkerError,
};

/// Sets the runtime for the application.
fn set_runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
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
}

/// Runs the application with the runtime.
pub fn run_with_runtime() -> Result<(), WorkerError> {
    let runtime = set_runtime().unwrap();

    let result = runtime.block_on(async {
        let runner = Runner::new(CONFIG_PATH).await;
        runner.run().await
    });

    drop(runtime);

    result.map_err(|err| WorkerError::GeneralShutdown(err.to_string()))
}
