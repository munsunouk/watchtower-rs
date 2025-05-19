#[macro_use]
use crate::*;

pub mod constants;
pub mod error;
pub mod log;
pub mod msg;
pub mod setting;
pub mod traits;
pub mod types;
use constants::CONFIG_PATH;
use ethers::{
    abi::Token,
    providers::Http,
    types::{Address, BlockId, BlockNumber, Filter, Log, U256, U64},
};
use serde_json::{json, Value};
use std::convert::TryFrom;
use std::sync::atomic::Ordering::SeqCst;
use tokio::runtime::Runtime;
use watch_tower_lib::{
    config::set_config,
    utils::{error::ClientError, parse_token_to_i64},
};

use crate::{
    parse::parse_result,
    rule::{
        get::{get, get_latest_block_number},
        store::{assign, eval, SymbolTable, TokenConvert},
        ContractCall, ContractEvent,
    },
};

use self::{
    constants::{ADD_MEMORY_VALUE_ORDER, DEFAULT_MEMORY_VALUE_ORDER},
    error::WorkerError,
};

// impl TryFrom<Token> for i64 {
//     type Error = WorkerError;

//     fn try_from(token: Token) -> Result<Self, Self::Error> {
//         match token {
//             Token::Int(int_val) => int_val.try_into().map_err(|e| {
//                 WorkerError::InvalidTypeConvertError(format!(
//                     "Failed to convert Token::Int ({}) to i64: {}",
//                     int_val, e
//                 ))
//             }),
//             Token::Uint(uint_val) => uint_val.try_into().map_err(|e| {
//                 WorkerError::InvalidTypeConvertError(format!(
//                     "Failed to convert Token::Uint ({}) to i64: {}",
//                     uint_val, e
//                 ))
//             }),
//             _ => Err(WorkerError::InvalidTypeConvertError(format!(
//                 "Token {:?} cannot be converted to i64",
//                 token
//             ))),
//         }
//     }
// }

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
pub fn run_with_runtime() -> Result<(), WorkerError> {
    let runtime = build_runtime()?;

    let result = runtime.block_on(async { get_result().await });

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

async fn get_result() -> Result<(), WorkerError> {
    let config = set_config(CONFIG_PATH);

    // let input = "
    //     bifrostBN = Bifrost.LatestBlock();

    //     ChainlinkBTC = Bifrost.ChainlinkOracle.BTC.LatestPrice(bifrostBN);
    //     BifnetBTC = Bifrost.BifnetOracle.BTC.LatestPrice(bifrostBN - 1);
    //     BifaggBTC = Bifrost.Bifagg.BTC.LatestPrice(bifrostBN -2);

    //     (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > ChainlinkBTC || (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > BifnetBTC || (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > BifaggBTC;
    //     ";

    let input = "
        bifrostBN = Bifrost.LatestBlock(); 
        eth_address = 0x51c9abb01e2ef6495daafc56778b499e8d3992ff;

        Bifrost.EthBalance(eth_address, bifrostBN);
        ";

    // (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3;
    // (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > ChainlinkBTC;
    // (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > ChainlinkBTC || (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > BifnetBTC || (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3 > BifaggBTC;

    let result = parse_result(&config, input).await.unwrap();

    println!("result: {:?}", result);

    Ok(())
}
