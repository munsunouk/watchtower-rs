use std::{collections::HashMap, sync::Arc};

use ethers::{
    abi::Token,
    providers::{Http, Provider},
    types::{Address, BlockNumber, Filter, Log, H256, U256, U64},
};
use sentry::ClientInitGuard;
use serde_json::{json, Value};
use tokio_stream::StreamExt;
use watch_tower_lib::{
    cli::{
        eth::{EthClient, ProviderMetadata},
        rpc::RpcClient,
    },
    config::{set_config, Configuration, EVMProvider},
    utils::{
        error::ClientError, parse_i32_to_usize, parse_token_to_i64, types::ChainID, DbRuleType,
        RpcCallType,
    },
};

use crate::{
    rule::{decodes_token, get::GetRequest},
    utils::{
        constants::{CONFIG_PATH, CONTROLLER_NAME},
        error::WorkerError,
        get_block_token, get_event_logs,
        setting::{build_eth_clients, build_rpc_client, build_sentry},
    },
};

use ethers::abi::decode;

use crate::utils::setting::{build_contract_call, build_contract_event, build_rpc_call};
use watch_tower_lib::rule::{
    contract_call::ContractCallRule, contract_event::ContractEventRule, rpc_call::RpcCallRule,
};

use crate::rule::get::GetContext;

pub struct Runner {
    pub config: Configuration,
    pub _sentry_guard: ClientInitGuard,
    pub assign_map: HashMap<String, Token>,
}

impl Runner {
    /// # Description
    /// This function creates a new `Runner` instance.
    /// # Arguments
    /// * `config_path` - A string slice that holds the path to the configuration file.
    ///
    /// # Returns
    ///
    /// A new instance of `Runner`.
    pub async fn new() -> Result<Self, WorkerError> {
        let config = set_config(CONFIG_PATH);

        //Sentry
        let _sentry_guard =
            build_sentry(&config.sentry_config.dsn, &config.sentry_config.environment)?;

        let assign_map = HashMap::new();

        Ok(Self {
            config,
            _sentry_guard,
            assign_map,
        })
    }

    // async fn get_result() -> Result<(), WorkerError> {
    //     let abi = json!([{
    //         "name": "latestRoundData",
    //         "type": "function",
    //         "inputs": [],
    //         "outputs": [
    //             {"name": "roundId", "type": "uint80", "internalType": "uint80"},
    //             {"name": "answer", "type": "int256", "internalType": "int256"},
    //             {"name": "startedAt", "type": "uint256", "internalType": "uint256"},
    //             {"name": "updatedAt", "type": "uint256", "internalType": "uint256"},
    //             {"name": "answeredInRound", "type": "uint80", "internalType": "uint80"}
    //         ]
    //     }]);

    //     runner.assign(
    //         "block_number",
    //         runner.get_latest_block_number(3068).await.unwrap(),
    //     );

    //     let val1: i64 = runner
    //         .get((
    //             3068,
    //             "0x6A74c7356820Dc036d0e43e07eDeaCBeF3DDD882".to_string(),
    //             abi.clone(),
    //             vec![],
    //             "1".to_string(),
    //             runner.eval("block_number").try_into()?,
    //         ))
    //         .await
    //         .try_into()?;

    //     let val2: i64 = runner
    //         .get((
    //             3068,
    //             "0xC60afe0AAfC863ED24B4a8A26D952C581bDAE6b2".to_string(),
    //             abi.clone(),
    //             vec![],
    //             "1".to_string(),
    //             runner.eval("block_number").try_into()?,
    //         ))
    //         .await
    //         .try_into()?;

    //     let val3: i64 = runner
    //         .get((
    //             3068,
    //             "0x40c8BB8036351EF29b41ea8AFEbA76ac2d8A96bF".to_string(),
    //             abi,
    //             vec![],
    //             "1".to_string(),
    //             runner.eval("block_number").try_into()?,
    //         ))
    //         .await
    //         .try_into()?;

    //     let result = (val1 + val2 + val3) / 3; // 방안1

    //     let result = operation((val1 + val2 + val3) / 3); // 방안2

    //     println!("result: {:?}", result);

    //     Ok(())
    // }

    pub fn assign(&mut self, key: &str, value: Token) {
        self.assign_map.insert(key.to_string(), value);
    }

    pub fn eval(&self, key: &str) -> Token {
        self.assign_map.get(key).unwrap().clone()
    }
}
