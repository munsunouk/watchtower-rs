use std::collections::HashMap;

use ethers::{
    abi::{decode, Param, Token},
    providers::Http,
    types::{BlockId, BlockNumber, H256, U256},
};
use serde_json::Value;
use tokio_stream::StreamExt;
use watch_tower_lib::{
    cli::{eth::EthClient, rpc::RpcClient},
    config::{set_config, EVMProvider},
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, rpc_call::RpcCallRule,
    },
    utils::{
        parse_i32_to_usize, parse_string_to_address, parse_token_to_i64, parse_u256_to_u64,
        types::ChainID, DbRuleType, RpcCallType,
    },
};

use crate::{
    rule::decodes_token,
    utils::{
        constants::CONFIG_PATH,
        error::WorkerError,
        get_block_token, get_event_logs,
        setting::{
            build_contract_call, build_contract_event, build_eth_clients, build_rpc_call,
            build_rpc_client,
        },
    },
};

// Define parameter types
pub struct ContractParams {
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub params: Vec<Option<Token>>,
    pub target_index: String,
    pub target_block_number: U256,
}

pub struct RpcParams {
    pub url: String,
    pub call_type: String,
    pub method_type: String,
    pub api_body: Option<Value>,
    pub api_query: Option<Value>,
    pub target_index: String,
}

pub struct ContractEventParams {
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub event_index: i32,
    pub target_index: String,
    pub target_block_number: U256,
}

pub enum GetRequest {
    Contract(ContractParams),
    Rpc(RpcParams),
    ContractEvent(ContractEventParams),
}

impl From<(i32, String, Value, Vec<Option<Token>>, String, U256)> for GetRequest {
    fn from(tuple: (i32, String, Value, Vec<Option<Token>>, String, U256)) -> Self {
        GetRequest::Contract(ContractParams {
            chain_id: tuple.0,
            address: tuple.1,
            abi: tuple.2,
            params: tuple.3,
            target_index: tuple.4,
            target_block_number: tuple.5,
        })
    }
}

impl From<(String, String, String, Option<Value>, Option<Value>, String)> for GetRequest {
    fn from(tuple: (String, String, String, Option<Value>, Option<Value>, String)) -> Self {
        GetRequest::Rpc(RpcParams {
            url: tuple.0,
            call_type: tuple.1,
            method_type: tuple.2,
            api_body: tuple.3,
            api_query: tuple.4,
            target_index: tuple.5,
        })
    }
}

impl From<(i32, String, Value, i32, String, U256)> for GetRequest {
    fn from(tuple: (i32, String, Value, i32, String, U256)) -> Self {
        GetRequest::ContractEvent(ContractEventParams {
            chain_id: tuple.0,
            address: tuple.1,
            abi: tuple.2,
            event_index: tuple.3,
            target_index: tuple.4,
            target_block_number: tuple.5,
        })
    }
}

pub struct GetContext {
    pub eth_clients: HashMap<ChainID, EthClient<Http>>,
    pub rpc_client: RpcClient,
    pub evm_providers: Vec<EVMProvider>,
}

impl GetContext {
    pub fn new(evm_providers: Vec<EVMProvider>) -> Result<Self, WorkerError> {
        let eth_clients = build_eth_clients(&evm_providers);
        let rpc_client = build_rpc_client()?;

        Ok(Self {
            eth_clients,
            rpc_client,
            evm_providers,
        })
    }

    pub async fn raw_get<P: Into<GetRequest>>(&self, params: P) -> Token {
        match params.into() {
            GetRequest::Contract(params) => self
                .get_contract_call(
                    params.chain_id,
                    params.address,
                    params.abi,
                    params.params,
                    params.target_index,
                    params.target_block_number,
                )
                .await
                .unwrap(),
            GetRequest::Rpc(params) => self
                .get_rpc_call(
                    params.url,
                    params.call_type,
                    params.method_type,
                    params.api_body,
                    params.api_query,
                    params.target_index,
                )
                .await
                .unwrap(),
            GetRequest::ContractEvent(params) => self
                .get_contract_event(
                    params.chain_id,
                    params.address,
                    params.abi,
                    params.event_index,
                    params.target_index,
                    params.target_block_number,
                )
                .await
                .unwrap(),
        }
    }

    pub async fn get_rpc_call(
        &self,
        url: String,
        call_type: String,
        method_type: String,
        api_body: Option<Value>,
        api_query: Option<Value>,
        target_index: String,
    ) -> Result<Token, WorkerError> {
        let rule = RpcCallRule::new(
            url,
            call_type,
            method_type,
            api_body,
            api_query,
            target_index,
        )
        .map_err(|err| WorkerError::InvalidMessage)?;

        let rpc_call = build_rpc_call(self.rpc_client.clone(), rule.clone());

        let (token, param_type) = if rpc_call.rule.call_type == RpcCallType::Body {
            rpc_call.fetch_api_call_with_body().await?
        } else {
            rpc_call.fetch_api_call_with_query().await?
        };

        let token = decodes_token(&token, &param_type, &rule.target_index)
            .map_err(|err| WorkerError::InvalidMessage)?;

        println!("token: {:?}", token);

        Ok(token)
    }

    pub async fn get_contract_call(
        &self,
        chain_id: i32,
        address: String,
        abi: Value,
        params: Vec<Option<Token>>,
        target_index: String,
        target_block_number: U256,
    ) -> Result<Token, WorkerError> {
        let rule = ContractCallRule::new(
            chain_id,
            address,
            abi,
            params.clone(),
            target_index,
            target_block_number,
        )
        .unwrap();

        let target_block_number = rule.target_block_number.clone();
        let contract_call = build_contract_call(
            self.eth_clients.get(&rule.chain_id).unwrap().clone(),
            rule.clone(),
        );

        let output_param_type = match contract_call.get_output_param_type() {
            Ok(param_type) => param_type,
            Err(e) => {
                WorkerError::InvalidParamType(DbRuleType::ContractCall, e.to_string()).log();
                return Err(e);
            }
        };

        let raw_token = get_block_token(&contract_call, target_block_number)
            .await
            .unwrap();

        let token = decodes_token(
            &raw_token,
            &output_param_type,
            &contract_call.rule.target_index,
        )
        .unwrap();

        println!(
            "block_number: {:?}, token: {:?}",
            target_block_number, token
        );

        Ok(token)
    }

    pub async fn get_contract_event(
        &self,
        chain_id: i32,
        address: String,
        abi: Value,
        event_index: i32,
        target_index: String,
        target_block_number: U256,
    ) -> Result<Token, WorkerError> {
        let rule = ContractEventRule::new(
            chain_id,
            address,
            abi,
            event_index,
            target_index,
            target_block_number,
        )
        .map_err(|_| WorkerError::InvalidMessage)?;

        let contract_event = build_contract_event(
            self.eth_clients.get(&rule.chain_id).unwrap().clone(),
            rule.clone(),
        );

        let input_param_type = match contract_event.get_raw_input_param_type() {
            Ok(param_type) => param_type,
            Err(e) => {
                WorkerError::InvalidParamType(DbRuleType::ContractEvent, e.to_string()).log();
                return Err(WorkerError::InvalidMessage);
            }
        };

        let parsing_input_param_type = match contract_event.get_input_param_type() {
            Ok(param_type) => param_type,
            Err(e) => {
                WorkerError::InvalidParamType(DbRuleType::ContractEvent, e.to_string()).log();
                return Err(WorkerError::InvalidMessage);
            }
        };

        let logs = get_event_logs(&contract_event, rule.target_block_number).await?;

        let mut stream = tokio_stream::iter(logs);

        // let mut vec_token = Vec::new();
        let mut token = Token::Bool(false);

        while let Some(log) = stream.next().await {
            match contract_event.is_target_event(
                log.topics
                    .get(contract_event.rule.event_index)
                    .unwrap_or(&H256::zero()),
            ) {
                Ok(is_target) => {
                    if !is_target {
                        continue;
                    }

                    let raw_token = match decode(&[input_param_type.clone()], &log.data) {
                        Ok(tokens) => Token::Tuple(tokens),
                        Err(e) => {
                            WorkerError::InvalidMessage.log();
                            continue;
                        }
                    };

                    let decoded_token = decodes_token(
                        &raw_token,
                        &parsing_input_param_type,
                        &contract_event.rule.target_index,
                    )
                    .map_err(|err| WorkerError::InvalidMessage)?;

                    // vec_token.push(decoded_token);
                    token = decoded_token;
                }
                Err(e) => {
                    WorkerError::InvalidMessage.log();
                    return Err(WorkerError::InvalidMessage);
                }
            }
        }

        // println!(
        //     "block_number: {:?}, token: {:?}",
        //     target_block_number, vec_token
        // );

        println!(
            "block_number: {:?}, token: {:?}",
            target_block_number, token
        );

        // Ok(Token::Array(vec_token))
        Ok(token)
    }

    pub async fn get_latest_block_number(&self, chain_id: i32) -> Result<Token, WorkerError> {
        let chain_id =
            parse_i32_to_usize(chain_id).map_err(|e| WorkerError::InvalidMessage)? as ChainID;

        let client = self.eth_clients.get(&chain_id).unwrap();
        let block_number = client
            .get_latest_block_number()
            .await
            .map_err(|err| WorkerError::InvalidMessage)?;

        Ok(Token::Uint(U256::from(block_number.as_u64())))
    }

    pub async fn get_eth_balance(
        &self,
        chain_id: i32,
        address: String,
        block_number: U256,
    ) -> Result<Token, WorkerError> {
        let chain_id =
            parse_i32_to_usize(chain_id).map_err(|e| WorkerError::InvalidMessage)? as ChainID;

        let block_number = BlockId::Number(BlockNumber::Number(parse_u256_to_u64(block_number)));

        let address = parse_string_to_address(address).map_err(|e| WorkerError::InvalidMessage)?;

        let client = self.eth_clients.get(&chain_id).unwrap();
        let balance = client
            .get_balance(address, block_number)
            .await
            .map_err(|err| WorkerError::InvalidMessage)?;

        Ok(Token::Uint(balance))
    }
}

pub trait TokenConvertible: Sized {
    fn from_token(token: Token) -> Self;
}

impl TokenConvertible for U256 {
    fn from_token(token: Token) -> Self {
        match token {
            Token::Uint(v) => v,
            Token::Int(v) => v,
            _ => panic!("Cannot convert token to U256"),
        }
    }
}

impl TokenConvertible for bool {
    fn from_token(token: Token) -> Self {
        match token {
            Token::Bool(v) => v,
            _ => panic!("Cannot convert token to bool"),
        }
    }
}

impl TokenConvertible for String {
    fn from_token(token: Token) -> Self {
        match token {
            Token::String(v) => v,
            _ => panic!("Cannot convert token to String"),
        }
    }
}

impl TokenConvertible for Vec<Token> {
    fn from_token(token: Token) -> Self {
        match token {
            Token::Array(v) => v,
            _ => panic!("Cannot convert token to Vec<Token>"),
        }
    }
}

pub async fn get<P>(params: P) -> Token
where
    P: Into<GetRequest>,
{
    let config = set_config(CONFIG_PATH);
    let get_context = GetContext::new(config.evm_providers.clone()).unwrap();
    get_context.raw_get(params).await
}

pub async fn get_latest_block_number(chain_id: i32) -> Token {
    let config = set_config(CONFIG_PATH);
    let get_context = GetContext::new(config.evm_providers.clone()).unwrap();
    get_context.get_latest_block_number(chain_id).await.unwrap()
}

pub async fn get_eth_balance(chain_id: i32, address: String, block_number: U256) -> Token {
    let config = set_config(CONFIG_PATH);
    let get_context = GetContext::new(config.evm_providers.clone()).unwrap();
    get_context
        .get_eth_balance(chain_id, address, block_number)
        .await
        .unwrap()
}
