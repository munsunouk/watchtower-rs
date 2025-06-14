use crate::*;

pub mod constants;
pub mod error;
pub mod log;
pub mod setting;

use ethers::{
    abi::Token,
    providers::Http,
    types::{BlockId, BlockNumber, Filter, Log, U64},
};

use std::sync::atomic::Ordering::SeqCst;
use tokio::runtime::Runtime;
use watch_tower_lib::utils::error::ClientError;

use crate::rule::{ContractCall, ContractEvent};

use constants::{ADD_MEMORY_VALUE_ORDER, DEFAULT_MEMORY_VALUE_ORDER};
use error::WorkerError;

pub async fn get_block_token(
    contract_call: &ContractCall<Http>,
    block_number: U64,
) -> Result<Token, ClientError> {
    let token = contract_call
        .get_method_call(BlockId::Number(BlockNumber::Number(block_number)))
        .await?;
    Ok(token)
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

    let logs = contract_event
        .client
        .get_logs(&filter)
        .await
        .map_err(|_| WorkerError::InvalidClient)?;

    Ok(logs)
}

use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;

static TOKIO_THREADS_TOTAL: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));
static TOKIO_THREADS_ALIVE: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));

/// Builds the runtime for the application.
fn build_runtime() -> Result<Runtime, WorkerError> {
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
pub fn run_with_runtime(rule_path: &str) -> Result<(), WorkerError> {
    let runtime = build_runtime()?;

    let result = runtime.block_on(async {
        let runner = Runner::new(rule_path).await?;
        runner.run().await
        // get_result(rule_path).await
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

// async fn get_result(rule_path: &str) -> Result<(), WorkerError> {
//     let config = set_config(CONFIG_PATH);
//     let rule = set_rule(rule_path);

//     let result = parse_result(&config, &rule.script).await.unwrap();

//     let btc_height_msg = "the Difference in the Block Height of BTC is more than 10";

//     if Token::Bool(false) == result {
//         let slack_msg = format!("```{}```", btc_height_msg.to_string());
//         let hashtag = Some("<!here>");

//         let slack_client =
//             SlackClient::new(&config.slack_config.token, &config.slack_config.channel);
//         slack_client
//             .send_alert("Scheduled BTC Height Notification", &slack_msg, hashtag)
//             .await
//             .unwrap();
//     }

//     println!("result: {:?}", result);
//     Ok(())
// }
