use std::collections::HashMap;

use ethers::{
    abi::{decode, ParamType, Token},
    providers::Http,
    types::{BlockId, BlockNumber, H256, U256},
};
use serde_json::Value;
use tokio_stream::StreamExt;
use watch_tower_lib::{
    cli::{eth::EthClient, rpc::RpcClient},
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, rpc_call::RpcCallRule,
    },
    utils::{
        parse_i32_to_usize, parse_string_to_address, parse_u256_to_u64,
        types::{ChainID, GeneralToken},
        DbTable, RpcCallType,
    },
};

use std::slice::from_ref;

use crate::{
    option_or_err,
    rule::{
        convert_target_index_to_indices, convert_value_to_param_type, convert_value_to_token,
        decodes_token,
    },
    utils::{
        config::{Configuration, EVMProvider},
        error::WorkerError,
        get_block_token, get_event_logs,
        log::TraceLog,
        setting::{
            build_contract_call, build_contract_event, build_eth_clients, build_rpc_call,
            build_rpc_client,
        },
    },
};

pub struct RpcCallParams {
    pub url: String,
    pub url_token: Option<String>,
    pub call_type: String,
    pub method_type: String,
    pub api_body: Option<Value>,
    pub api_query: Option<Value>,
    pub target_index: String,
}

// Define parameter types
pub struct ContractParams {
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub params: Vec<Option<GeneralToken>>,
    pub target_index: String,
    pub target_block_number: U256,
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
    Rpc(RpcCallParams),
    ContractEvent(ContractEventParams),
}

impl From<(i32, String, Value, Vec<Option<GeneralToken>>, String, U256)> for GetRequest {
    fn from(tuple: (i32, String, Value, Vec<Option<GeneralToken>>, String, U256)) -> Self {
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

impl
    From<(
        String,
        Option<String>,
        String,
        String,
        Option<Value>,
        Option<Value>,
        String,
    )> for GetRequest
{
    fn from(
        tuple: (
            String,
            Option<String>,
            String,
            String,
            Option<Value>,
            Option<Value>,
            String,
        ),
    ) -> Self {
        GetRequest::Rpc(RpcCallParams::from(tuple))
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
}

impl GetContext {
    pub fn new(evm_providers: &[EVMProvider]) -> Result<Self, WorkerError> {
        let eth_clients = build_eth_clients(evm_providers);
        let rpc_client = build_rpc_client()?;

        Ok(Self {
            eth_clients,
            rpc_client,
        })
    }

    pub async fn raw_get<P: Into<GetRequest>>(
        &self,
        params: P,
    ) -> Result<GeneralToken, WorkerError> {
        match params.into() {
            GetRequest::Contract(params) => {
                self.get_contract_call(
                    params.chain_id,
                    &params.address,
                    params.abi,
                    params.params,
                    params.target_index,
                    &params.target_block_number,
                )
                .await
            }
            GetRequest::Rpc(params) => self.get_rpc_call(params).await,
            GetRequest::ContractEvent(params) => {
                self.get_contract_event(
                    params.chain_id,
                    &params.address,
                    params.abi,
                    params.event_index,
                    params.target_index,
                    &params.target_block_number,
                )
                .await
            }
        }
    }

    pub async fn get_rpc_call(&self, params: RpcCallParams) -> Result<GeneralToken, WorkerError> {
        let rule = RpcCallRule::new(
            params.url,
            params.url_token,
            params.call_type,
            params.method_type,
            params.api_body,
            params.api_query,
            params.target_index,
        )?;

        let rpc_call = build_rpc_call(&self.rpc_client, &rule);

        let (status, body) = if rpc_call.rule.call_type == RpcCallType::Body {
            rpc_call.fetch_api_call_with_body().await?
        } else {
            rpc_call.fetch_api_call_with_query().await?
        };

        let status_token = Token::Uint(U256::from(status.as_u64()));
        let body_token = convert_value_to_token(&body)?;
        let body_param_type = convert_value_to_param_type(&body)?;

        let mut param_type = ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);
        let mut tokens = Token::Tuple(vec![status_token, body_token]);
        let mut key_store = Vec::new();

        let (mut indices, mut foreach_positions) =
            convert_target_index_to_indices(&rule.target_index, Some(&body), Some(&mut key_store))?;

        let token = decodes_token(
            &mut tokens,
            &mut param_type,
            &mut indices,
            &mut foreach_positions,
        )?;

        let result: GeneralToken = token.try_into()?;
        TraceLog::TokenOutput(result.clone()).debug();
        Ok(result)
    }

    pub async fn get_contract_call(
        &self,
        chain_id: i32,
        address: &str,
        abi: Value,
        params: Vec<Option<GeneralToken>>,
        target_index: String,
        target_block_number: &U256,
    ) -> Result<GeneralToken, WorkerError> {
        let params: Result<Vec<_>, _> = params
            .into_iter()
            .map(|p| p.map(|t| t.to_eth_token()).transpose())
            .collect();
        let params = params?;

        let rule = ContractCallRule::new(
            chain_id,
            address,
            abi,
            &params,
            target_index,
            target_block_number,
        )?;

        let contract_call =
            build_contract_call(option_or_err!(self.eth_clients.get(&rule.chain_id)), &rule);

        let mut output_param_type = match contract_call.get_output_param_type() {
            Ok(param_type) => param_type,
            Err(e) => {
                WorkerError::InvalidParamType(DbTable::Rule, e.to_string()).log();
                return Err(e);
            }
        };

        let mut raw_token = get_block_token(&contract_call, &rule.target_block_number).await?;

        let (mut indices, mut foreach_positions) =
            convert_target_index_to_indices(&rule.target_index, None, None)?;

        let token = decodes_token(
            &mut raw_token,
            &mut output_param_type,
            &mut indices,
            &mut foreach_positions,
        )?;

        let result: GeneralToken = token.try_into()?;
        TraceLog::TokenOutput(result.clone()).debug();
        Ok(result)
    }

    pub async fn get_contract_event(
        &self,
        chain_id: i32,
        address: &str,
        abi: Value,
        event_index: i32,
        target_index: String,
        target_block_number: &U256,
    ) -> Result<GeneralToken, WorkerError> {
        let rule = ContractEventRule::new(
            chain_id,
            address,
            abi,
            event_index,
            target_index,
            target_block_number,
        )?;

        let contract_event =
            build_contract_event(option_or_err!(self.eth_clients.get(&rule.chain_id)), &rule);

        let input_param_type = match contract_event.get_raw_input_param_type() {
            Ok(param_type) => param_type,
            Err(e) => {
                WorkerError::InvalidParamType(DbTable::Rule, e.to_string()).log();
                return Err(WorkerError::InvalidMessage);
            }
        };

        let mut parsing_input_param_type = match contract_event.get_input_param_type() {
            Ok(param_type) => param_type,
            Err(e) => {
                WorkerError::InvalidParamType(DbTable::Rule, e.to_string()).log();
                return Err(WorkerError::InvalidMessage);
            }
        };

        let logs = get_event_logs(&contract_event, rule.target_block_number).await?;

        let (mut indices, mut foreach_positions) =
            convert_target_index_to_indices(&rule.target_index, None, None)?;

        let mut stream = tokio_stream::iter(logs);

        let mut vec_token = Vec::new();

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

                    let mut raw_token = match decode(from_ref(&input_param_type), &log.data) {
                        Ok(tokens) => Token::Tuple(tokens),
                        Err(_e) => {
                            WorkerError::InvalidMessage.log();
                            continue;
                        }
                    };

                    let decoded_token = decodes_token(
                        &mut raw_token,
                        &mut parsing_input_param_type,
                        &mut indices,
                        &mut foreach_positions,
                    )?;

                    vec_token.push(decoded_token);
                }
                Err(_e) => {
                    WorkerError::InvalidMessage.log();
                    return Err(WorkerError::InvalidMessage);
                }
            }
        }

        let result: GeneralToken = Token::Array(vec_token).try_into()?;
        TraceLog::TokenOutput(result.clone()).debug();
        Ok(result)
    }

    pub async fn get_latest_block_number(
        &self,
        chain_id: i32,
    ) -> Result<GeneralToken, WorkerError> {
        let chain_id = parse_i32_to_usize(chain_id)? as ChainID;

        let client = option_or_err!(self.eth_clients.get(&chain_id));
        let block_number = client.get_latest_block_number().await?;

        let result: GeneralToken = Token::Uint(U256::from(block_number.as_u64())).try_into()?;
        TraceLog::TokenOutput(result.clone()).debug();
        Ok(result)
    }

    pub async fn get_latest_block(
        &self,
        chain_id: i32,
        target: String,
    ) -> Result<GeneralToken, WorkerError> {
        let chain_id = parse_i32_to_usize(chain_id)? as ChainID;

        let client = option_or_err!(self.eth_clients.get(&chain_id));
        let block = client.get_latest_block().await?;

        match target.as_str() {
            "timestamp" => {
                let result: GeneralToken = Token::Uint(block.timestamp).try_into()?;
                TraceLog::TokenOutput(result.clone()).debug();
                Ok(result)
            }
            "number" => {
                let result: GeneralToken =
                    Token::Uint(U256::from(option_or_err!(block.number).as_u64())).try_into()?;
                TraceLog::TokenOutput(result.clone()).debug();
                Ok(result)
            }
            "hash" => Ok(Token::String(option_or_err!(block.hash).to_string()).try_into()?),
            _ => Err(WorkerError::InvalidMessage),
        }
    }

    pub async fn get_eth_balance(
        &self,
        chain_id: i32,
        address: &str,
        block_number: &U256,
    ) -> Result<GeneralToken, WorkerError> {
        let chain_id = parse_i32_to_usize(chain_id)? as ChainID;

        let block_number = BlockId::Number(BlockNumber::Number(parse_u256_to_u64(block_number)));

        let address = parse_string_to_address(address)?;

        let client = option_or_err!(self.eth_clients.get(&chain_id));
        let balance = client.get_balance(address, block_number).await?;

        let result: GeneralToken = Token::Uint(balance).try_into()?;
        TraceLog::TokenOutput(result.clone()).debug();
        Ok(result)
    }
}

pub async fn get<P>(config: &Configuration, params: P) -> Result<GeneralToken, WorkerError>
where
    P: Into<GetRequest>,
{
    let get_context = GetContext::new(&config.evm_providers)?;
    get_context.raw_get(params).await
}

pub async fn get_latest_block_number(
    config: &Configuration,
    chain_id: i32,
) -> Result<GeneralToken, WorkerError> {
    let get_context = GetContext::new(&config.evm_providers)?;

    get_context.get_latest_block_number(chain_id).await
}

pub async fn get_latest_block(
    config: &Configuration,
    chain_id: i32,
    target: String,
) -> Result<GeneralToken, WorkerError> {
    let get_context = GetContext::new(&config.evm_providers)?;
    get_context.get_latest_block(chain_id, target).await
}

pub async fn get_eth_balance(
    config: &Configuration,
    chain_id: i32,
    address: &str,
    block_number: &U256,
) -> Result<GeneralToken, WorkerError> {
    let get_context = GetContext::new(&config.evm_providers)?;
    get_context
        .get_eth_balance(chain_id, address, block_number)
        .await
}

impl
    From<(
        String,
        Option<String>,
        String,
        String,
        Option<Value>,
        Option<Value>,
        String,
    )> for RpcCallParams
{
    fn from(
        tuple: (
            String,
            Option<String>,
            String,
            String,
            Option<Value>,
            Option<Value>,
            String,
        ),
    ) -> Self {
        let (url, url_token, call_type, method_type, api_body, api_query, target_index) = tuple;
        Self {
            url,
            url_token,
            call_type,
            method_type,
            api_body,
            api_query,
            target_index,
        }
    }
}

impl From<(i32, String, Value, Vec<Option<GeneralToken>>, String, U256)> for ContractParams {
    fn from(tuple: (i32, String, Value, Vec<Option<GeneralToken>>, String, U256)) -> Self {
        let (chain_id, address, abi, params, target_index, target_block_number) = tuple;
        Self {
            chain_id,
            address,
            abi,
            params,
            target_index,
            target_block_number,
        }
    }
}

impl From<(i32, String, Value, i32, String, U256)> for ContractEventParams {
    fn from(tuple: (i32, String, Value, i32, String, U256)) -> Self {
        let (chain_id, address, abi, event_index, target_index, target_block_number) = tuple;
        Self {
            chain_id,
            address,
            abi,
            event_index,
            target_index,
            target_block_number,
        }
    }
}
