use ethers::abi::{Param, ParamType, Token};

use ethers::types::U256;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

use watch_tower_lib::cli::db::postgres::{
    select_assign_data_sync, select_fetched_raw_data_with_filter, PostgresClient, Query, SelectData,
};
use watch_tower_lib::config::{
    BlockchainTargetValue, Configuration, ContractCallTargetValue, ContractConfig,
    ContractEventTargetValue, EVMProvider, RPCConfig, RPCTargetValue,
};
use watch_tower_lib::utils::types::ChainID;
use watch_tower_lib::utils::{
    constants::{
        BOOLEAN_LITERAL_FALSE, BOOLEAN_LITERAL_TRUE, DB_EXPECTED_VALUE_COLUMN, DB_ID_COLUMN,
        DB_RULE_FILTER_COLUMN, DEFAULT_INDEX, LOGIC_OPERATOR_AND, LOGIC_OPERATOR_OR,
        RULE_FILTER_SPLIT_CHAR, RULE_ID_SPLIT_INDEX, VALUE_ID_SPLIT_INDEX,
    },
    error::GeneralError,
    parse_i32_to_usize, parse_string_to_uint, parse_to_abi,
    types::RuleID,
    DbRuleType,
};

use watch_tower_lib::utils::error::IndexType;

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use std::{collections::HashMap, str::FromStr};

use watch_tower_lib::utils::{
    arithmetic_token, compare_token,
    constants::{
        ADDRESS_SPLIT_INDEX, BLOCK_NUMBER_SPLIT_INDEX, CHAIN_SPLIT_INDEX, EVENT_INDEX_SPLIT_INDEX,
        INTERVAL_SPLIT_INDEX, URL_SPLIT_INDEX,
    },
    parse_string_to_i32,
};

use futures::executor::block_on;

use crate::rule::decode_meta_data;
use crate::rule::get::{get, get_eth_balance, get_latest_block_number, ContractParams, RpcParams};
use crate::rule::parse_meta_data;
use crate::rule::store::{assign, eval, SymbolTable, TokenConvert};
use crate::utils::error::WorkerError;

/// # Description
/// This struct represents an evaluation rule.
/// # Fields
/// * `id` - The ID of the rule.
/// * `rule_filter` - The rule filter.
/// * `expected_value` - The expected value.
#[derive(Clone, PartialEq, Debug)]
pub struct EvaluationRule {
    pub id: RuleID,
    pub rule_filter: String,
    pub expected_value: String,
}

impl TryFrom<&PgRow> for EvaluationRule {
    type Error = GeneralError;
    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN))?,
            rule_filter: row.get(DB_RULE_FILTER_COLUMN),
            expected_value: row.get(DB_EXPECTED_VALUE_COLUMN),
        })
    }
}

#[derive(Parser)]
#[grammar = "parse/evaluation.pest"]
pub struct RuleEvaluationParser;

pub type ParsePairFuture<'a> = Pin<Box<dyn Future<Output = Result<Token, GeneralError>> + 'a>>;

pub struct Context<'a> {
    pub config: &'a Configuration,
    pub symbol_table: &'a mut SymbolTable,
    pub variables: &'a mut HashMap<String, ParseResultType>,
}

/// ParseValues is the function to parse the values.
/// # Description
/// This function parses the values of the rule.
/// # Arguments
/// * `pair` - The pair.
/// * `rule_type` - The rule type.
/// * `rule_name` - The rule name.
/// # Returns
/// * `(HashMap<(String, String), String>, Vec<Token>)` - The rule values and values.
pub fn parse_pair<'a>(pair: Pair<'a, Rule>, context: &'a mut Context) -> ParsePairFuture<'a> {
    Box::pin(async move {
        match pair.as_rule() {
            Rule::program => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(true);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::expression_stmt => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(true);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::assignment_stmt => {
                let mut inner = pair.into_inner();

                let identifer = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let result = parse_pair(first, context).await?;

                assign(
                    context.symbol_table,
                    identifer.as_str().to_string(),
                    result.clone(),
                );
                context.variables.clear();

                let result = Token::Bool(true);

                Ok(result)
            }

            // operation level parsing
            Rule::expression => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let mut result = parse_pair(first, context).await?;

                while let Some(op) = inner.next() {
                    let next = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                    let right = parse_pair(next, context).await?;

                    result = match op.as_str() {
                        LOGIC_OPERATOR_AND => {
                            if result.type_check(&ParamType::Bool)
                                && right.type_check(&ParamType::Bool)
                            {
                                Token::Bool(
                                    result.into_bool().ok_or(GeneralError::InvalidTypeConvert)?
                                        && right
                                            .into_bool()
                                            .ok_or(GeneralError::InvalidTypeConvert)?,
                                )
                            } else {
                                return Err(GeneralError::InvalidTypeConvert);
                            }
                        }
                        LOGIC_OPERATOR_OR => {
                            if result.type_check(&ParamType::Bool)
                                && right.type_check(&ParamType::Bool)
                            {
                                Token::Bool(
                                    result.into_bool().ok_or(GeneralError::InvalidTypeConvert)?
                                        || right
                                            .into_bool()
                                            .ok_or(GeneralError::InvalidTypeConvert)?,
                                )
                            } else {
                                return Err(GeneralError::InvalidTypeConvert);
                            }
                        }
                        _ => return Err(GeneralError::InvalidOperator(op.as_str().to_string())),
                    };
                }
                Ok(result)
            }

            Rule::operation => {
                let mut inner = pair.into_inner();
                let left = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))
                    .map(|p| parse_pair(p, context))?;
                let left = left.await?;

                if let Some(op) = inner.next() {
                    let right = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))
                        .map(|p| parse_pair(p, context))?;
                    let right = right.await?;

                    let err_msg = format!("{:?}, {:?}, {}", left, right, op.as_str().to_string());

                    compare_token(&left, &right, op.as_str())
                        .ok_or(GeneralError::InvalidOperator(err_msg))
                } else {
                    Ok(left)
                }
            }

            Rule::term => {
                let mut inner = pair.into_inner();
                let mut result = parse_pair(inner.next().unwrap(), context).await?;

                while let Some(op) = inner.next() {
                    let next = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                    let right = parse_pair(next, context).await?;

                    result = arithmetic_token(&result, &right, op.as_str())
                        .unwrap_or_else(|| Token::Uint(U256::from(0)));
                }

                Ok(result)
            }

            Rule::factor => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let mut result = parse_pair(first, context).await?;

                while let Some(op) = inner.next() {
                    let next = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                    let right = parse_pair(next, context).await?;

                    result = arithmetic_token(&result, &right, op.as_str())
                        .unwrap_or_else(|| Token::Uint(U256::from(0)));
                }
                Ok(result)
            }

            Rule::params => {
                let mut inner = pair.into_inner().peekable();
                let mut params_vec = Vec::new();

                if inner.peek().is_some() {
                    for unwrapped_pair in inner {
                        let result = parse_pair(unwrapped_pair, context).await?;
                        params_vec.push(Some(result.clone()));
                    }
                }

                context.variables.insert(
                    "function_params".to_string(),
                    ParseResultType::ArrayParam(params_vec),
                );

                Ok(Token::Bool(true))
            }

            Rule::primary => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                parse_pair(first, context).await
            }

            Rule::boolean_literal => match pair.as_str() {
                BOOLEAN_LITERAL_TRUE => Ok(Token::Bool(true)),
                BOOLEAN_LITERAL_FALSE => Ok(Token::Bool(false)),
                _ => Err(GeneralError::InvalidOperator(pair.as_str().to_string())),
            },

            Rule::number => Ok(Token::Uint(parse_string_to_uint(
                pair.as_str().to_string(),
            )?)),

            Rule::hex_address => {
                let address = pair.as_str();
                Ok(Token::Address(
                    ethers::types::Address::from_str(address).map_err(|_| {
                        GeneralError::InvalidRuleDecode(format!("Invalid address: {}", address))
                    })?,
                ))
            }

            Rule::call_stmt => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                if let Some(_meta_data) = context.variables.get("meta_data") {
                    result = decode_meta_data(&result, context.variables)?;
                }

                context.variables.clear();

                Ok(result)
            }

            Rule::blockchain_call => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    parse_pair(unwrapped_pair, context).await?;
                }

                let chain_id = match context.variables.get("chain_id").unwrap() {
                    ParseResultType::ChainID(id) => *id as i32,
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                if let Some(ParseResultType::String(name)) = context.variables.get("name") {
                    if name == "LatestBlock" {
                        result = get_latest_block_number(chain_id).await;
                    } else if name == "EthBalance" {
                        let address = match context.variables.get("address").unwrap() {
                            ParseResultType::String(addr) => addr.clone(),
                            _ => return Err(GeneralError::InvalidTypeConvert),
                        };

                        let target_block_number =
                            match context.variables.get("function_params").unwrap() {
                                ParseResultType::ArrayParam(params) => {
                                    if let Some(Some(token)) = params.first() {
                                        if token.type_check(&ParamType::Uint(256)) {
                                            token.clone().into_uint().unwrap()
                                        } else {
                                            return Err(GeneralError::InvalidTypeConvert);
                                        }
                                    } else {
                                        return Err(GeneralError::InvalidTypeConvert);
                                    }
                                }
                                _ => return Err(GeneralError::InvalidTypeConvert),
                            };

                        result = get_eth_balance(chain_id, address, target_block_number).await;
                    }
                }

                Ok(result)
            }

            Rule::rpc_call => {
                let inner = pair.into_inner();

                for unwrapped_pair in inner {
                    parse_pair(unwrapped_pair, context).await?;
                }

                let url = match context.variables.get("url").unwrap() {
                    ParseResultType::String(url) => url.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                let call_type = match context.variables.get("call_type").unwrap() {
                    ParseResultType::String(call_type) => call_type.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                let method_type = match context.variables.get("method_type").unwrap() {
                    ParseResultType::String(method_type) => method_type.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                if context.variables.get("meta_data").is_some() {
                    let meta_data = parse_meta_data(context.variables)?;

                    if let ParseResultType::HashMap(meta_data) = meta_data {
                        if let Some(ParseResultType::JSON(meta_api_query)) =
                            meta_data.get("api_query")
                        {
                            if let Some(ParseResultType::JSON(existing_api_query)) =
                                context.variables.get("api_query")
                            {
                                let mut merged = existing_api_query.clone();
                                if let Some(obj) = merged.as_object_mut() {
                                    if let Some(meta_obj) = meta_api_query.as_object() {
                                        for (key, value) in meta_obj {
                                            obj.insert(key.clone(), value.clone());
                                        }
                                    }
                                }
                                context
                                    .variables
                                    .insert("api_query".to_string(), ParseResultType::JSON(merged));
                            } else {
                                context.variables.insert(
                                    "api_query".to_string(),
                                    ParseResultType::JSON(meta_api_query.clone()),
                                );
                            }
                        }

                        if let Some(ParseResultType::JSON(meta_api_body)) =
                            meta_data.get("api_body")
                        {
                            if let Some(ParseResultType::JSON(existing_api_body)) =
                                context.variables.get("api_body")
                            {
                                let mut merged = existing_api_body.clone();
                                if let Some(obj) = merged.as_object_mut() {
                                    if let Some(meta_obj) = meta_api_body.as_object() {
                                        for (key, value) in meta_obj {
                                            obj.insert(key.clone(), value.clone());
                                        }
                                    }
                                }
                                context
                                    .variables
                                    .insert("api_body".to_string(), ParseResultType::JSON(merged));
                            } else {
                                context.variables.insert(
                                    "api_body".to_string(),
                                    ParseResultType::JSON(meta_api_body.clone()),
                                );
                            }
                        }
                    }
                }

                let api_body = if let Some(ParseResultType::JSON(api_body)) =
                    context.variables.get("api_body")
                {
                    Some(api_body.clone())
                } else {
                    None
                };

                let api_query = if let Some(ParseResultType::JSON(api_query)) =
                    context.variables.get("api_query")
                {
                    Some(api_query.clone())
                } else {
                    None
                };

                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                let result = get((
                    url,
                    call_type,
                    method_type,
                    api_body,
                    api_query,
                    target_index,
                ))
                .await;

                Ok(result)
            }

            Rule::contract_call => {
                let inner = pair.into_inner();

                for unwrapped_pair in inner {
                    parse_pair(unwrapped_pair, context).await?;
                }

                let chain_id = match context.variables.get("chain_id").unwrap() {
                    ParseResultType::ChainID(id) => *id as i32,
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let address = match context.variables.get("address").unwrap() {
                    ParseResultType::String(addr) => addr.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let abi = match context.variables.get("abi").unwrap() {
                    ParseResultType::JSON(abi) => abi.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let mut params = match context.variables.get("method_params").unwrap() {
                    ParseResultType::ArrayParam(params) => params.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                let mut target_block_number =
                    get_latest_block_number(chain_id).await.into_uint().unwrap();

                if let Some(ParseResultType::HashMap(identifier)) =
                    context.variables.get("identifier")
                {
                    for (key, value) in identifier.iter() {
                        if key.contains("BlockNumber") {
                            if let ParseResultType::Token(token) = value {
                                if token.type_check(&ParamType::Uint(256)) {
                                    target_block_number = token.clone().into_uint().unwrap();
                                }
                            }
                        } else if key.contains("MethodParams") {
                            if let ParseResultType::Token(token) = value {
                                params.push(Some(token.clone()));
                            }
                        }
                    }
                }

                let result = get((
                    chain_id,
                    address,
                    abi,
                    params,
                    target_index,
                    target_block_number,
                ))
                .await;

                Ok(result)
            }

            Rule::contract_event => {
                let inner = pair.into_inner();

                for unwrapped_pair in inner {
                    parse_pair(unwrapped_pair, context).await?;
                }

                let chain_id = match context.variables.get("chain_id").unwrap() {
                    ParseResultType::ChainID(id) => *id as i32,
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let address = match context.variables.get("address").unwrap() {
                    ParseResultType::String(addr) => addr.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let abi = match context.variables.get("abi").unwrap() {
                    ParseResultType::JSON(abi) => abi.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let event_index = match context.variables.get("event_index").unwrap() {
                    ParseResultType::EventIndex(idx) => idx.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                let mut target_block_number =
                    get_latest_block_number(chain_id).await.into_uint().unwrap();

                if let Some(ParseResultType::HashMap(identifier)) =
                    context.variables.get("identifier")
                {
                    for (key, value) in identifier.iter() {
                        if key.contains("BlockNumber") {
                            if let ParseResultType::Token(token) = value {
                                if token.type_check(&ParamType::Uint(256)) {
                                    target_block_number = token.clone().into_uint().unwrap();
                                }
                            }
                        }
                    }
                }

                let result = get((
                    chain_id,
                    address,
                    abi,
                    event_index,
                    target_index,
                    target_block_number,
                ))
                .await;

                Ok(result)
            }

            Rule::blockchain => {
                let blockchain = pair.as_str();

                for provider in context.config.evm_providers.clone() {
                    let EVMProvider {
                        name,
                        provider: _,
                        id,
                    } = provider;

                    if name == blockchain {
                        context
                            .variables
                            .insert("chain_id".to_string(), ParseResultType::ChainID(id));
                        context
                            .variables
                            .insert("blockchain".to_string(), ParseResultType::String(name));
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::service => {
                let service = pair.as_str();

                if let Some(_) = context.variables.get("chain_id") {
                    for contract_config in context.config.contract_config.clone() {
                        let ContractConfig {
                            service: parsed_service,
                            blockchain,
                            path,
                            ..
                        } = contract_config;

                        if *service == parsed_service
                            && *blockchain
                                == *match context.variables.get("blockchain").unwrap() {
                                    ParseResultType::String(s) => s,
                                    _ => return Err(GeneralError::InvalidTypeConvert),
                                }
                        {
                            context.variables.insert(
                                "service".to_string(),
                                ParseResultType::String(service.to_string()),
                            );
                        }
                    }
                } else {
                    for rpc_config in context.config.rpc_config.clone() {
                        let RPCConfig {
                            name,
                            url,
                            call_type,
                            method_type,
                            api_body,
                            api_query,
                        } = rpc_config;

                        let mut value_api_body: Option<Value> = None;
                        let mut value_api_query: Option<Value> = None;

                        if let Some(api_body) = api_body {
                            value_api_body =
                                Some(serde_json::from_str(&api_body).map_err(|e| {
                                    GeneralError::InvalidTypeConvertError(format!(
                                        "Failed to parse JSON: {},{}",
                                        api_body, e
                                    ))
                                })?);
                        }

                        if let Some(api_query) = api_query {
                            value_api_query = serde_json::from_str(&api_query).map_err(|e| {
                                GeneralError::InvalidTypeConvertError(format!(
                                    "Failed to parse JSON: {},{}",
                                    api_query, e
                                ))
                            })?;
                        }

                        if name == service {
                            context
                                .variables
                                .insert("url".to_string(), ParseResultType::String(url));
                            context.variables.insert(
                                "call_type".to_string(),
                                ParseResultType::String(call_type),
                            );
                            context.variables.insert(
                                "method_type".to_string(),
                                ParseResultType::String(method_type),
                            );

                            if let Some(value_api_body) = value_api_body {
                                context.variables.insert(
                                    "api_body".to_string(),
                                    ParseResultType::JSON(value_api_body),
                                );
                            }

                            if let Some(value_api_query) = value_api_query {
                                context.variables.insert(
                                    "api_query".to_string(),
                                    ParseResultType::JSON(value_api_query),
                                );
                            }
                        }
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::contract => {
                let contract_str = pair.as_str();

                for contract_config in context.config.contract_config.clone() {
                    let ContractConfig {
                        service: parsed_service,
                        blockchain: parsed_blockchain,
                        contract,
                        address,
                        path,
                        ..
                    } = contract_config;

                    if let (
                        Some(ParseResultType::String(service)),
                        Some(ParseResultType::String(blockchain)),
                    ) = (
                        context.variables.get("service"),
                        context.variables.get("blockchain"),
                    ) {
                        if *service == parsed_service
                            && contract_str == contract
                            && *blockchain == parsed_blockchain
                        {
                            let abi_content = fs::read_to_string(path).map_err(|e| {
                                GeneralError::InvalidTypeConvertError(format!(
                                    "Failed to read ABI file: {}",
                                    e
                                ))
                            })?;
                            let abi: Value = serde_json::from_str(&abi_content).map_err(|e| {
                                GeneralError::InvalidTypeConvertError(format!(
                                    "Failed to parse ABI: {}",
                                    e
                                ))
                            })?;

                            context
                                .variables
                                .insert("abi".to_string(), ParseResultType::JSON(abi));

                            context
                                .variables
                                .insert("address".to_string(), ParseResultType::String(address));
                        }
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::identifier => {
                let identifier = pair.as_str();
                let result = eval(context.symbol_table, identifier);

                if let Some(ParseResultType::HashMap(existing)) =
                    context.variables.get("identifier")
                {
                    let mut new_hashmap = existing.clone();
                    new_hashmap.insert(
                        identifier.to_string(),
                        ParseResultType::Token(result.clone()),
                    );
                    context.variables.insert(
                        "identifier".to_string(),
                        ParseResultType::HashMap(new_hashmap),
                    );
                } else {
                    let mut new_hashmap = HashMap::new();
                    new_hashmap.insert(
                        identifier.to_string(),
                        ParseResultType::Token(result.clone()),
                    );
                    context.variables.insert(
                        "identifier".to_string(),
                        ParseResultType::HashMap(new_hashmap),
                    );
                }

                Ok(result)
            }

            Rule::rpc_call_target => {
                let rpc_call_target_str = pair.as_str();

                for rpc_call_target in context.config.rpc_call_target.clone() {
                    let RPCTargetValue {
                        name,
                        meta_data,
                        target_index,
                    } = rpc_call_target;

                    if rpc_call_target_str == name {
                        context.variables.insert(
                            "target_index".to_string(),
                            ParseResultType::String(target_index),
                        );

                        context
                            .variables
                            .insert("meta_data".to_string(), ParseResultType::String(meta_data));
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::contract_call_target => {
                let contract_call_target_str = pair.as_str();

                for contract_call_target in context.config.contract_call_target.clone() {
                    let ContractCallTargetValue {
                        name,
                        params,
                        target_index,
                    } = contract_call_target;

                    if contract_call_target_str == name {
                        context.variables.insert(
                            "method_params".to_string(),
                            ParseResultType::ArrayParam(params.clone()),
                        );
                        context.variables.insert(
                            "target_index".to_string(),
                            ParseResultType::String(target_index),
                        );
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::contract_event_target => {
                let contract_event_target_str = pair.as_str();

                for contract_event_target in context.config.contract_event_target.clone() {
                    let ContractEventTargetValue {
                        name,
                        event_index,
                        target_index,
                    } = contract_event_target;

                    if contract_event_target_str == name {
                        context.variables.insert(
                            "event_index".to_string(),
                            ParseResultType::EventIndex(event_index),
                        );
                        context.variables.insert(
                            "target_index".to_string(),
                            ParseResultType::String(target_index),
                        );
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::blockchain_call_target => {
                let blockchain_call_target_str = pair.as_str();

                for blockchain_call_target in context.config.blockchain_call_target.clone() {
                    let BlockchainTargetValue {
                        name,
                        params,
                        metadata,
                    } = blockchain_call_target;

                    if blockchain_call_target_str == name {
                        context
                            .variables
                            .insert("name".to_string(), ParseResultType::String(name));

                        if let Some(metadata) = metadata {
                            context.variables.insert(
                                "address".to_string(),
                                ParseResultType::String(metadata.address),
                            );
                        }
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::rpc_call_property => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::contract_call_property => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::contract_event_property => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::blockchain_call_property => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            _ => Err(GeneralError::InvalidRuleDecode(format!(
                "Unexpected rule: {:?}",
                pair.as_rule()
            ))),
        }
    })
}

/// CheckFunctionLength is the function to check the function length.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<bool, GeneralError>` - The result.
pub fn check_function_length(abi_text: &str) -> Result<bool, GeneralError> {
    let abi_value = parse_abi_text(abi_text)?;

    let abi = parse_to_abi(abi_value)?;

    let function_count = abi.functions().count();

    if function_count != 1 {
        return Ok(false);
    }
    Ok(true)
}

/// CheckEventLength is the function to check the event length.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<bool, GeneralError>` - The result.
pub fn check_event_length(abi_text: &str) -> Result<bool, GeneralError> {
    let abi_value = parse_abi_text(abi_text)?;

    let abi = parse_to_abi(abi_value)?;

    let event_count = abi.events().count();

    if event_count != 1 {
        return Ok(false);
    }

    Ok(true)
}

/// ParseABIText is the function to parse the ABI text.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<Value, GeneralError>` - The result.
pub fn parse_abi_text(abi_text: &str) -> Result<Value, GeneralError> {
    serde_json::from_str(abi_text).map_err(|_| GeneralError::InvalidTypeABI)
}

#[derive(Debug, Clone)]
pub enum ParseResultType {
    String(String),
    Number(U256),
    Bool(bool),
    JSON(Value),
    ChainID(ChainID),
    Array(Vec<String>),
    ArrayParam(Vec<Option<Token>>),
    Token(Token),
    EventIndex(i32),
    HashMap(HashMap<String, ParseResultType>),
}

/// # Description
/// This function evaluates a rule filter.
/// # Arguments
/// * `rule_filter` - The rule filter.
/// * `values` - The values.
/// # Returns
/// A `Result` struct.
pub async fn parse_result(
    config: &Configuration,
    program_input: &str,
) -> Result<Token, GeneralError> {
    let mut symbol_table = SymbolTable::new();

    let pairs = match RuleEvaluationParser::parse(Rule::program, program_input) {
        Ok(pairs) => pairs,
        Err(_) => {
            return Err(GeneralError::InvalidRuleDecode(program_input.to_string()));
        }
    };
    let mut result = Token::Bool(false);
    let mut context = Context {
        config,
        symbol_table: &mut symbol_table,
        variables: &mut HashMap::new(),
    };
    for pair in pairs {
        result = parse_pair(pair, &mut context).await?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {

    use ethers::{abi::Int, types::U256};
    use watch_tower_lib::{config::set_config, utils::parse_compare};

    use crate::utils::constants::CONFIG_PATH;

    use super::*;

    fn setup() -> Configuration {
        set_config("/Users/munseon-ug/rust/watchtower/worker/config.yaml")
    }

    #[test]

    fn test_new_parse_rule() {
        let test_input = "
        24454853;
        ";

        let pairs = RuleEvaluationParser::parse(Rule::program, test_input).unwrap();
        println!("pairs: {:?}", pairs);
    }

    #[tokio::test]
    async fn test_parse_result() {
        // let test_input = "
        //  bifrost_BlockNumber = 24454853;
        //  Bifrost.CCCP.Socket.BridgeAmount(bifrost_BlockNumber);
        // ";

        let test_input = "
         bifrost_BlockNumber = Bifrost.LatestBlock();
         Bifrost.BIFI.LendingPool.BtcUSD_deposit_liquidity(bifrost_BlockNumber);
        ";

        // target 들 매개변수로 변경
        // 유저 보다 먼저 확인하는 작업
        // logger, test case 추가
        // 스마트 라우터 공통
        // let test_input = "
        //  bifrostBN = Bifrost.LatestBlock();

        //  reserve0 = Bifrost.Everdex.Poolinfo.BtcUSD/USDC_BtcUSD_Liquidity(bifrostBN);
        //  reserve1 = Bifrost.Everdex.Poolinfo.BtcUSD/USDC_USDC_Liquidity(bifrostBN);
        //  reserve0 * 100 / (reserve1 * 1000000000000);
        // ";

        // let test_input = "
        //  bifrostBN = Bifrost.LatestBlock();
        //  reserve0 = Bifrost.Everdex.     .stBFC/BFC_stBFC_Liquidity(bifrostBN);
        //  reserve1 = Bifrost.Everdex.SmartRouter.stBFC/BFC_BFC_Liquidity(bifrostBN);

        //  reserve0 * 100 / reserve1;
        // ";

        // let test_input = "
        //  bifrost_BlockNumber = Bifrost.LatestBlock();
        //  totalSupply = Bifrost.BRP.TotalSupply.TotalSupply(bifrost_BlockNumber);
        //  currentRound_MethodParams = Bifrost.BRP.CurrentRound.CurrentRound(bifrost_BlockNumber);
        //  Bifrost.BRP.VaultAddress.VaultAddress(bifrost_BlockNumber, currentRound_MethodParams);
        // ";

        //주석
        // symbol_table parse_result 안
        // let test_input = "

        // kaiaBN = Kaia.LatestBlock();
        // bifrostBN = Bifrost.LatestBlock();

        // bifrostChainlinkBTC = Bifrost.ChainlinkOracle.BTC.LatestPrice(bifrostBN);
        // kaiaChainlinkBTC = Kaia.ChainlinkOracle.BTC.LatestPrice(kaiaBN);

        // (bifrostChainlinkBTC + kaiaChainlinkBTC) / 2;";

        let config = setup();

        let result = parse_result(&config, test_input).await.unwrap();

        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_parse_result1() {
        let test_input = "

         stBFC/BFC_MethodParams = 0x7FD303FCA8c485955700CA7B5f71068878e8EDBa;
         reserve0 = Bifrost.Everdex.SmartRouter.Liquidity(stBFC/BFC_MethodParams);
         reserve1 = Bifrost.Everdex.SmartRouter.stBFC/BFC_BFC_Liquidity();

         reserve0 * 100 / reserve1 > 95;
        ";

        let config = setup();

        let result = parse_result(&config, test_input).await.unwrap();

        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_parse_result2() {
        let test_input = "
         bifrost_BlockNumber = Bifrost.LatestBlock();
         liquidity = Bifrost.BIFI.LendingPool.Liquidity(bifrost_BlockNumber, stBFC|BFC);
         liquidity > 5000000000000000000000000;
        ";

        let config = setup();

        let result = parse_result(&config, test_input).await.unwrap();

        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_parse_result3() {
        let test_input = "
         bifrost_BlockNumber = 24454853;
         amount = Bifrost.CCCP.Socket.BridgeAmount(bifrost_BlockNumber);
         amount > 9500;
        ";

        let config = setup();

        let result = parse_result(&config, test_input).await.unwrap();

        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_parse_result4() {
        let test_input = "
         bifrost_BlockNumber = Bifrost.LatestBlock();

         currentRound_MethodParams = Bifrost.BRP.CurrentRound.CurrentRound(bifrost_BlockNumber);
         BRPVaultAddress = Bifrost.BRP.VaultAddress.VaultAddress(bifrost_BlockNumber, currentRound_MethodParams);
         systemVaultAddress = Bifrost.BRP.Registration.SystemVault(bifrost_BlockNumber, currentRound_MethodParams);

         vaultBalance = BRP.VaultBalance(BRPVaultAddress);
         systemvaultBalance = BRP.VaultBalance(systemVaultAddress);
         btcLiq = vaultBalance + systemvaultBalance;

         totalSupply = Bifrost.BRP.TotalSupply.TotalSupply(bifrost_BlockNumber);
         unifiedBTCLiq = totalSupply - 1110100;
         unifiedBTCLiq - btcLiq > 1000000;
         
        ";

        let config = setup();

        let result = parse_result(&config, test_input).await.unwrap();

        println!("result: {:?}", result);
    }
}
