use crate::{
    runner::Runner,
    utils::constants::{MINUTES_PER_HOUR, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE},
    *,
};

pub mod config;
pub mod constants;
pub mod error;
pub mod log;
pub mod macros;
pub mod setting;

use cron::Schedule;
use ethers::{
    abi::Token,
    providers::Http,
    types::{BlockId, BlockNumber, Filter, Log, U64},
};

use std::{str::FromStr, sync::atomic::Ordering::SeqCst};
use tokio::runtime::Runtime;

use crate::rule::{ContractCall, ContractEvent};

use constants::{ADD_MEMORY_VALUE_ORDER, DEFAULT_MEMORY_VALUE_ORDER};
use error::WorkerError;

pub async fn get_block_token(
    contract_call: &ContractCall<Http>,
    block_number: &U64,
) -> Result<Token, WorkerError> {
    contract_call
        .get_method_call(BlockId::Number(BlockNumber::Number(*block_number)))
        .await
}

/// # Description
/// This function fetches events from the given block range.
/// # Arguments
/// * `from` - The start block number.
/// * `to` - The end block number.
/// # Returns
/// * `Result<Vec<Log>, WorkerError>` - The logs.
pub async fn get_event_logs(
    contract_event: &ContractEvent<Http>,
    block_number: U64,
) -> Result<Vec<Log>, WorkerError> {
    let filter = Filter::new()
        .from_block(BlockNumber::from(block_number))
        .to_block(BlockNumber::from(block_number))
        .address(contract_event.rule.address);

    Ok(contract_event.client.get_logs(&filter).await?)
}

use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;

static TOKIO_THREADS_TOTAL: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));
static TOKIO_THREADS_ALIVE: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));

/// Builds the runtime for the application.
fn build_runtime() -> Result<Runtime, WorkerError> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .on_thread_start(|| {
            TOKIO_THREADS_ALIVE.fetch_add(ADD_MEMORY_VALUE_ORDER, SeqCst);
            TOKIO_THREADS_TOTAL.fetch_add(ADD_MEMORY_VALUE_ORDER, SeqCst);
        })
        .on_thread_stop(|| {
            TOKIO_THREADS_ALIVE.fetch_sub(ADD_MEMORY_VALUE_ORDER, SeqCst);
        })
        .enable_all()
        .build()?)
}

/// Runs the application with the runtime.
pub fn run_with_runtime(args: Args) -> Result<(), WorkerError> {
    let runtime = build_runtime()?;

    let result = runtime.block_on(async {
        let mut runner = Runner::new(args).await?;
        runner.run().await
    });

    drop(runtime);

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            let worker_err = WorkerError::GeneralShutdown(err.to_string());
            worker_err.log();
            Err(worker_err)
        }
    }
}

/// # Description
/// This function sets a cron schedule based on the check interval.
/// # Arguments
///
/// * `check_interval` - The interval in seconds.
///
/// # Returns
///
/// A `Schedule` instance.
pub fn set_schedule(check_interval: i32) -> Result<Schedule, WorkerError> {
    let format_schedule = if check_interval < SECONDS_PER_MINUTE {
        // Less than 1 minute: use seconds
        format!("*/{} * * * * *", check_interval)
    } else if check_interval < SECONDS_PER_HOUR {
        // Less than 1 hour: use minutes
        let minutes = check_interval / MINUTES_PER_HOUR;
        format!("0 */{} * * * *", minutes)
    } else if check_interval < SECONDS_PER_DAY {
        // Less than 1 day: use hours
        let hours = check_interval / SECONDS_PER_HOUR;
        format!("0 0 */{} * * *", hours)
    } else {
        // 1 day or more: use days
        let days = check_interval / SECONDS_PER_DAY;
        format!("0 0 0 */{} * *", days)
    };

    Ok(Schedule::from_str(&format_schedule)?)
}
