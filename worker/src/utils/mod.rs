pub mod config;
pub mod constants;
pub mod msg;
pub mod traits;

use std::sync::atomic::Ordering::SeqCst;

use self::constants::{TOKIO_THREADS_ALIVE, TOKIO_THREADS_TOTAL};

pub fn set_runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .on_thread_start(|| {
            TOKIO_THREADS_ALIVE.fetch_add(1, SeqCst);
            TOKIO_THREADS_TOTAL.fetch_add(1, SeqCst);
        })
        .on_thread_stop(|| {
            TOKIO_THREADS_ALIVE.fetch_sub(1, SeqCst);
        })
        .enable_all()
        .build()
}
