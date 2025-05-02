use ethers::abi::{ParamType, Token};

use ethers::types::U256;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

use watch_tower_lib::cli::db::postgres::{
    select_assign_data_sync, select_fetched_raw_data_with_filter, PostgresClient, Query, SelectData,
};
use watch_tower_lib::config::{
    BlockchainTargetValue, Configuration, ContractConfig, ContractTargetValue, EVMProvider,
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

use crate::rule::get::{get, get_eth_balance, get_latest_block_number, ContractParams};
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
                let mut inner = pair.into_inner();

                let mut result = Token::Bool(true);

                if let Some(first) = inner.next() {
                    result = parse_pair(first, context).await?;

                    context.variables.insert(
                        "function_params".to_string(),
                        ParseResultType::Token(result.clone()),
                    );
                }

                Ok(result)
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

            Rule::call_stmt => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
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
                                ParseResultType::Token(token) => {
                                    if token.type_check(&ParamType::Uint(256)) {
                                        token.clone().into_uint().unwrap()
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
                    ParseResultType::ABI(abi) => abi.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let params = match context.variables.get("method_params").unwrap() {
                    ParseResultType::Array(params) => params.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };
                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

                let target_block_number = match context.variables.get("function_params").unwrap() {
                    ParseResultType::Token(token) => {
                        if token.type_check(&ParamType::Uint(256)) {
                            token.clone().into_uint().unwrap()
                        } else {
                            return Err(GeneralError::InvalidTypeConvert);
                        }
                    }
                    _ => return Err(GeneralError::InvalidTypeConvert),
                };

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
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::service => {
                let service = pair.as_str();

                for contract_config in context.config.contract_config.clone() {
                    let ContractConfig {
                        service: parsed_service,
                        path,
                        ..
                    } = contract_config;

                    if service == parsed_service {
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
                            .insert("abi".to_string(), ParseResultType::ABI(abi));
                        context.variables.insert(
                            "service".to_string(),
                            ParseResultType::String(service.to_string()),
                        );
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::contract => {
                let contract_str = pair.as_str();

                for contract_config in context.config.contract_config.clone() {
                    let ContractConfig {
                        service: parsed_service,
                        contract,
                        address,
                        ..
                    } = contract_config;

                    if let Some(ParseResultType::String(service)) = context.variables.get("service")
                    {
                        if *service == parsed_service && contract_str == contract {
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
                let result_clone = result.clone();
                context.variables.insert(
                    "identifier".to_string(),
                    ParseResultType::Token(result_clone),
                );
                Ok(result)
            }

            Rule::call_target => {
                let call_target_str = pair.as_str();

                for call_target in context.config.call_target.clone() {
                    let ContractTargetValue {
                        name,
                        params,
                        target_index,
                    } = call_target;

                    if call_target_str == name {
                        context
                            .variables
                            .insert("method_params".to_string(), ParseResultType::Array(params));
                        context.variables.insert(
                            "target_index".to_string(),
                            ParseResultType::String(target_index),
                        );
                    }
                }

                Ok(Token::Bool(true))
            }

            Rule::blockchain_target => {
                let blockchain_target_str = pair.as_str();

                for blockchain_target in context.config.blockchain_target.clone() {
                    let BlockchainTargetValue {
                        name,
                        params,
                        metadata,
                    } = blockchain_target;

                    if blockchain_target_str == name {
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

            Rule::contract_property => {
                let inner = pair.into_inner();

                let mut result = Token::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::blockchain_property => {
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
    ABI(Value),
    ChainID(ChainID),
    Array(Vec<String>),
    Token(Token),
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
    symbol_table: &mut SymbolTable,
    program_input: &str,
) -> Result<Token, GeneralError> {
    let pairs = match RuleEvaluationParser::parse(Rule::program, program_input) {
        Ok(pairs) => pairs,
        Err(_) => {
            return Err(GeneralError::InvalidRuleDecode(program_input.to_string()));
        }
    };
    let mut result = Token::Bool(false);
    let mut context = Context {
        config,
        symbol_table,
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
        // 1) 유동성 조회 후 저장
        let test_input = "
        bifrostBN = Bifrost.LatestBlock(); 
        ChainlinkBTC = Bifrost.ChainlinkOracle.BTC.LatestPrice(bifrostBN); 
        BifnetBTC = Bifrost.BifnetOracle.BTC.LatestPrice(bifrostBN -1); 
        BifaggBTC = Bifrost.Bifagg.BTC.LatestPrice(bifrostBN -2); 
        result = (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3;";

        let pairs = RuleEvaluationParser::parse(Rule::program, test_input).unwrap();
        println!("pairs: {:?}", pairs);
    }

    #[tokio::test]
    async fn test_parse_result() {
        let test_input = "
        bifrostBN = Bifrost.LatestBlock(); 

        ChainlinkBTC = Bifrost.ChainlinkOracle.BTC.LatestPrice(bifrostBN); 
        BifnetBTC = Bifrost.BifnetOracle.BTC.LatestPrice(bifrostBN - 1); 
        BifaggBTC = Bifrost.Bifagg.BTC.LatestPrice(bifrostBN -2); 

        (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3;";

        let mut symbol_table = SymbolTable::new();
        let config = setup();

        let result = parse_result(&config, &mut symbol_table, test_input)
            .await
            .unwrap();

        println!("result: {:?}", result);
    }
}
