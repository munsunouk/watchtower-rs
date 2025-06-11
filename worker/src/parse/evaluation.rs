use ethers::abi::{ParamType, Token};

use ethers::types::U256;
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, Row};

use watch_tower_lib::cli::slack::SlackNotifier;
use watch_tower_lib::config::{
    set_param_config, BlockchainTargetValue, Configuration, ContractCallTargetValue,
    ContractConfig, ContractEventTargetValue, EVMProvider, NotificationCallTargetValue,
    NotificationConfig, RPCTargetValue,
};
use watch_tower_lib::utils::types::ChainID;
use watch_tower_lib::utils::{
    constants::{
        BOOLEAN_LITERAL_FALSE, BOOLEAN_LITERAL_TRUE, DB_EXPECTED_VALUE_COLUMN, DB_ID_COLUMN,
        DB_RULE_FILTER_COLUMN, DEFAULT_INDEX, LOGIC_OPERATOR_AND, LOGIC_OPERATOR_OR,
    },
    error::GeneralError,
    parse_i32_to_usize, parse_string_to_uint, parse_to_abi,
    types::RuleID,
};

use watch_tower_lib::utils::error::IndexType;

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use std::{collections::HashMap, str::FromStr};

use watch_tower_lib::utils::{arithmetic_token, compare_token};

use crate::rule::decode_meta_data;
use crate::rule::get::{get, get_eth_balance, get_latest_block, get_latest_block_number};
use crate::rule::store::{assign, check_store_value, eval, SymbolTable};
use crate::utils::constants::{CONFIG_PATH, PARAM_CONFIG_PATH};
use watch_tower_lib::utils::types::GeneralToken;

use hex;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::time::{SystemTime, UNIX_EPOCH};

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

pub type ParsePairFuture<'a> =
    Pin<Box<dyn Future<Output = Result<GeneralToken, GeneralError>> + 'a>>;

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
pub fn parse_pair<'a>(
    config: &'a Configuration,
    program_rule: &'a watch_tower_lib::config::Rule,
    pair: Pair<'a, Rule>,
    context: &'a mut Context,
) -> ParsePairFuture<'a> {
    Box::pin(async move {
        match pair.as_rule() {
            Rule::Program => {
                let inner = pair.into_inner();

                let mut result = GeneralToken::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::ExprStmt => {
                let inner = pair.into_inner();

                let mut result = GeneralToken::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                Ok(result)
            }

            Rule::AssignmentStmt => {
                let mut inner = pair.into_inner();

                let identifer = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let result = parse_pair(config, program_rule, first, context).await?;

                assign(
                    context.symbol_table,
                    identifer.as_str().to_string(),
                    result.clone(),
                );
                context.variables.clear();

                let result = GeneralToken::Bool(false);

                Ok(result)
            }

            // operation level parsing
            Rule::Expr => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let mut result = parse_pair(config, program_rule, first, context).await?;

                while let Some(op) = inner.next() {
                    let next = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                    let right = parse_pair(config, program_rule, next, context).await?;

                    result = match op.as_str() {
                        LOGIC_OPERATOR_AND => {
                            if result.type_check(&ParamType::Bool)
                                && right.type_check(&ParamType::Bool)
                            {
                                GeneralToken::Bool(
                                    result.clone().into_bool().ok_or(
                                        GeneralError::InvalidTypeConvertError(format!(
                                            "Expected bool, got {:?}",
                                            result
                                        )),
                                    )? && right.clone().into_bool().ok_or(
                                        GeneralError::InvalidTypeConvertError(format!(
                                            "Expected bool, got {:?}",
                                            right
                                        )),
                                    )?,
                                )
                            } else {
                                return Err(GeneralError::InvalidTypeConvertError(format!(
                                    "Expected bool, got {:?}",
                                    result
                                )));
                            }
                        }
                        LOGIC_OPERATOR_OR => {
                            if result.type_check(&ParamType::Bool)
                                && right.type_check(&ParamType::Bool)
                            {
                                GeneralToken::Bool(
                                    result.clone().into_bool().ok_or(
                                        GeneralError::InvalidTypeConvertError(format!(
                                            "Expected bool, got {:?}",
                                            result
                                        )),
                                    )? || right.clone().into_bool().ok_or(
                                        GeneralError::InvalidTypeConvertError(format!(
                                            "Expected bool, got {:?}",
                                            right
                                        )),
                                    )?,
                                )
                            } else {
                                return Err(GeneralError::InvalidTypeConvertError(format!(
                                    "Expected bool, got {:?}",
                                    result
                                )));
                            }
                        }
                        _ => return Err(GeneralError::InvalidOperator(op.as_str().to_string())),
                    };
                }
                Ok(result)
            }

            Rule::Condition => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                let result = parse_pair(config, program_rule, first, context).await?;
                Ok(result)
            }

            Rule::When => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                let condition = parse_pair(config, program_rule, first, context).await?;

                let then_expr = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                let else_expr = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                let result = if condition.clone().into_bool().ok_or(
                    GeneralError::InvalidTypeConvertError(format!(
                        "Expected bool, got {:?}",
                        condition
                    )),
                )? {
                    parse_pair(config, program_rule, then_expr, context).await?
                } else {
                    parse_pair(config, program_rule, else_expr, context).await?
                };

                Ok(result)
            }

            Rule::Operation => {
                let mut inner = pair.into_inner();
                let left = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))
                    .map(|p| parse_pair(config, program_rule, p, context))?;
                let left = left.await?;

                if let Some(op) = inner.next() {
                    let right = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))
                        .map(|p| parse_pair(config, program_rule, p, context))?;
                    let right = right.await?;

                    let err_msg = format!("{:?}, {:?}, {}", left, right, op.as_str().to_string());

                    compare_token(&left, &right, op.as_str())
                        .ok_or(GeneralError::InvalidOperator(err_msg))
                } else {
                    Ok(left)
                }
            }

            Rule::Term => {
                let mut inner = pair.into_inner();
                let mut result =
                    parse_pair(config, program_rule, inner.next().unwrap(), context).await?;

                while let Some(op) = inner.next() {
                    let next = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                    let right = parse_pair(config, program_rule, next, context).await?;

                    result = arithmetic_token(&result, &right, op.as_str()).unwrap();
                }

                Ok(result)
            }

            Rule::Factor => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let mut result = parse_pair(config, program_rule, first, context).await?;

                while let Some(op) = inner.next() {
                    let next = inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                    let right = parse_pair(config, program_rule, next, context).await?;

                    result = arithmetic_token(&result, &right, op.as_str()).unwrap();
                }
                Ok(result)
            }

            Rule::Params => {
                let mut inner = pair.into_inner().peekable();
                let mut params_vec = Vec::new();

                if inner.peek().is_some() {
                    for unwrapped_pair in inner {
                        let result =
                            parse_pair(config, program_rule, unwrapped_pair, context).await?;
                        params_vec.push(Some(result.clone()));
                    }
                }

                context.variables.insert(
                    "function_params".to_string(),
                    ParseResultType::ArrayParam(params_vec),
                );

                Ok(GeneralToken::Bool(false))
            }

            Rule::Primary => {
                let mut inner = pair.into_inner();
                let first = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                parse_pair(config, program_rule, first, context).await
            }

            Rule::boolean_literal => match pair.as_str() {
                BOOLEAN_LITERAL_TRUE => Ok(GeneralToken::Bool(true)),
                BOOLEAN_LITERAL_FALSE => Ok(GeneralToken::Bool(false)),
                _ => Err(GeneralError::InvalidOperator(pair.as_str().to_string())),
            },

            Rule::Number => Ok(GeneralToken::Uint(parse_string_to_uint(
                pair.as_str().to_string(),
            )?)),

            Rule::StringLiteral => {
                let string = pair.as_str();

                let string = string.replace("'", "");

                Ok(GeneralToken::String(string.to_string()))
            }

            Rule::Address => {
                let address = pair.as_str();
                Ok(GeneralToken::Address(
                    ethers::types::Address::from_str(address).map_err(|_| {
                        GeneralError::InvalidRuleDecode(format!("Invalid address: {}", address))
                    })?,
                ))
            }

            Rule::CallStmt => {
                let inner = pair.into_inner();

                let mut result = GeneralToken::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                if let Some(_meta_data) = context.variables.get("meta_data") {
                    result = decode_meta_data(&result, context.variables)?;
                }

                context.variables.clear();

                Ok(result)
            }

            Rule::NotificationCallExpr => {
                let inner = pair.into_inner();
                let mut result = GeneralToken::Bool(false);

                for unwrapped_pair in inner {
                    result = parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                let notification = match context.variables.get("notification") {
                    Some(ParseResultType::String(notification)) => notification.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for notification".to_string(),
                        ))
                    }
                };

                let key = match context.variables.get("key") {
                    Some(ParseResultType::String(key)) => key.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for key".to_string(),
                        ))
                    }
                };

                let param_config = set_param_config(PARAM_CONFIG_PATH);

                let param_nessesary = match context.variables.get("param_nessesary") {
                    Some(ParseResultType::Array(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected Array for param_nessesary".to_string(),
                        ))
                    }
                };

                let function_params = match context.variables.get("function_params") {
                    Some(ParseResultType::ArrayParam(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ArrayParam for function_params".to_string(),
                        ))
                    }
                };

                let mut slack_client = None;
                let mut channel_name = String::new();

                for (param_nessery, function_param) in
                    param_nessesary.iter().zip(function_params.iter())
                {
                    match param_nessery.as_str() {
                        "Channel" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(channel) = token {
                                    channel_name = channel.clone();
                                    for channel_config in param_config.channel_config.iter() {
                                        if (channel_config.name == *channel)
                                            && (notification == "Slack")
                                        {
                                            slack_client =
                                                Some(SlackNotifier::new(&key, &channel_config.id));
                                        }
                                    }
                                }
                            }
                        }
                        "Message" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(message) = token {
                                    if notification == "Slack" {
                                        if let Some(client) = &slack_client {
                                            // Check if similar message was sent in last 3 hours for this rule and channel
                                            if !check_recent_notification(
                                                &program_rule.name,
                                                &channel_name,
                                            ) {
                                                client.send_message(&message).await.unwrap();
                                                // Log the notification with channel info
                                                if let Err(e) = log_notification(
                                                    &program_rule.name,
                                                    &channel_name,
                                                    &message,
                                                ) {
                                                    eprintln!("Failed to log notification: {}", e);
                                                }
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(result)
            }

            Rule::ChainFunctionCallExpr => {
                let inner = pair.into_inner();

                let result;

                for unwrapped_pair in inner {
                    parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                let param_config = set_param_config(PARAM_CONFIG_PATH);

                let chain_id = match context.variables.get("chain_id").unwrap() {
                    ParseResultType::ChainID(id) => *id as i32,
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ChainID for chain_id".to_string(),
                        ))
                    }
                };

                let blockchain = match context.variables.get("blockchain").unwrap() {
                    ParseResultType::String(blockchain) => blockchain.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for blockchain".to_string(),
                        ))
                    }
                };

                let param_nessesary = match context.variables.get("param_nessesary") {
                    Some(ParseResultType::Array(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected Array for param_nessesary".to_string(),
                        ))
                    }
                };

                let function_params = match context.variables.get("function_params") {
                    Some(ParseResultType::ArrayParam(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ArrayParam for function_params".to_string(),
                        ))
                    }
                };

                let mut target_block_number = get_latest_block_number(config, chain_id)
                    .await
                    .into_uint()
                    .unwrap();

                let mut address: Option<String> = None;

                for (param_nessery, function_param) in
                    param_nessesary.iter().zip(function_params.iter())
                {
                    match param_nessery.as_str() {
                        "BlockNumber" => {
                            if let Some(token) = function_param {
                                if token.type_check(&ParamType::Uint(256)) {
                                    target_block_number = token.clone().into_uint().unwrap();
                                }
                            }
                        }
                        "Balance" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(balance_name) = token {
                                    for balance in param_config.balance_config.iter() {
                                        if (balance.name == *balance_name)
                                            && (balance.blockchain == *blockchain)
                                        {
                                            address = Some(balance.address.clone());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(GeneralError::InvalidTypeConvertError(
                                "Invalid token type".to_string(),
                            ))
                        }
                    }
                }

                match context.variables.get("name") {
                    Some(ParseResultType::String(name)) => match name.as_str() {
                        "LatestBlock" => {
                            result = get_latest_block(config, chain_id, "number".to_string()).await;
                        }
                        "LatestTimestamp" => {
                            result =
                                get_latest_block(config, chain_id, "timestamp".to_string()).await;
                        }
                        "Balance" => {
                            result = get_eth_balance(
                                config,
                                chain_id,
                                address.unwrap(),
                                target_block_number,
                            )
                            .await;
                        }
                        _ => {
                            return Err(GeneralError::InvalidTypeConvertError(
                                "Invalid token type".to_string(),
                            ))
                        }
                    },
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for name".to_string(),
                        ))
                    }
                }

                Ok(result)
            }

            Rule::RpcFunctionCallExpr => {
                let inner = pair.into_inner();

                for unwrapped_pair in inner {
                    parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                let param_config = set_param_config(PARAM_CONFIG_PATH);

                let call_type = match context.variables.get("call_type").unwrap() {
                    ParseResultType::String(call_type) => call_type.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for call_type".to_string(),
                        ))
                    }
                };

                let method_type = match context.variables.get("method_type").unwrap() {
                    ParseResultType::String(method_type) => method_type.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for method_type".to_string(),
                        ))
                    }
                };

                let param_nessesary = match context.variables.get("param_nessesary") {
                    Some(ParseResultType::Array(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected Array for param_nessesary".to_string(),
                        ))
                    }
                };

                let function_params = match context.variables.get("function_params") {
                    Some(ParseResultType::ArrayParam(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ArrayParam for function_params".to_string(),
                        ))
                    }
                };

                let mut url: Option<String> = None;
                let mut url_token: Option<String> = None;

                let mut api_body = if let Some(ParseResultType::JSON(api_body)) =
                    context.variables.get("api_body")
                {
                    Some(api_body.clone())
                } else {
                    None
                };

                let mut api_query = if let Some(ParseResultType::JSON(api_query)) =
                    context.variables.get("api_query")
                {
                    Some(api_query.clone())
                } else {
                    None
                };

                for (param_nessery, function_param) in
                    param_nessesary.iter().zip(function_params.iter())
                {
                    match param_nessery.as_str() {
                        "Url" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(url_name) = token {
                                    for url_config in param_config.url_config.iter() {
                                        if url_config.name == *url_name {
                                            url = Some(url_config.url.clone());

                                            if let Some(token_str) = &url_config.token {
                                                url_token = Some(token_str.clone());
                                            }

                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        "VaultAddress" => {
                            if let Some(token) = function_param {
                                if token.type_check(&ParamType::Array(Box::new(ParamType::String)))
                                {
                                    if let GeneralToken::Array(tokens) = token {
                                        let strings: Result<Vec<String>, GeneralError> = tokens
                                            .iter()
                                            .map(|t| match t {
                                                GeneralToken::String(s) => Ok(s.clone()),
                                                _ => Err(GeneralError::InvalidTypeConvertError(
                                                    format!("Expected String, got {:?}", t),
                                                )),
                                            })
                                            .collect();
                                        let joined_string = strings?.join("|");
                                        api_query = Some(json!({
                                            "active": joined_string
                                        }));
                                    }
                                } else if token.type_check(&ParamType::String) {
                                    if let GeneralToken::String(single_string) = token {
                                        api_query = Some(json!({
                                            "active": single_string
                                        }));
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(GeneralError::InvalidTypeConvertError(
                                "Invalid token type".to_string(),
                            ))
                        }
                    }
                }

                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for target_index".to_string(),
                        ))
                    }
                };

                let result = get(
                    config,
                    (
                        url.unwrap(),
                        url_token,
                        call_type,
                        method_type,
                        api_body,
                        api_query,
                        target_index,
                    ),
                )
                .await;

                Ok(result)
            }

            Rule::ContractMethodCallExpr => {
                let inner = pair.into_inner();

                for unwrapped_pair in inner {
                    parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                let param_config = set_param_config(PARAM_CONFIG_PATH);

                let chain_id = match context.variables.get("chain_id").unwrap() {
                    ParseResultType::ChainID(id) => *id as i32,
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ChainID for chain_id".to_string(),
                        ))
                    }
                };
                let address = match context.variables.get("address").unwrap() {
                    ParseResultType::String(addr) => addr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for address".to_string(),
                        ))
                    }
                };
                let abi = match context.variables.get("abi").unwrap() {
                    ParseResultType::JSON(abi) => abi.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected JSON for abi".to_string(),
                        ))
                    }
                };

                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for target_index".to_string(),
                        ))
                    }
                };

                let mut target_block_number = get_latest_block_number(config, chain_id)
                    .await
                    .into_uint()
                    .unwrap();

                let param_nessesary = match context.variables.get("param_nessesary") {
                    Some(ParseResultType::Array(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected Array for param_nessesary".to_string(),
                        ))
                    }
                };

                let function_params = match context.variables.get("function_params") {
                    Some(ParseResultType::ArrayParam(arr)) => arr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ArrayParam for function_params".to_string(),
                        ))
                    }
                };

                let mut params = match context.variables.get("method_params").unwrap() {
                    ParseResultType::ArrayParam(params) => params.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ArrayParam for method_params".to_string(),
                        ))
                    }
                };

                let available_contract = if let Some(ParseResultType::String(available_contract)) =
                    context.variables.get("available_contract")
                {
                    Some(available_contract.clone())
                } else {
                    None
                };

                let available_contract = if let Some(ParseResultType::String(available_contract)) =
                    context.variables.get("available_contract")
                {
                    Some(available_contract.clone())
                } else {
                    None
                };

                for (param_nessery, function_param) in
                    param_nessesary.iter().zip(function_params.iter())
                {
                    match param_nessery.as_str() {
                        "BlockNumber" => {
                            if let Some(token) = function_param {
                                if token.type_check(&ParamType::Uint(256)) {
                                    target_block_number = token.clone().into_uint().unwrap();
                                }
                            }
                        }
                        "Pool" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(pool_name) = token {
                                    for pool in param_config.pool_config.iter() {
                                        if pool.name == *pool_name {
                                            params.push(Some(GeneralToken::Address(
                                                pool.address.parse().unwrap(),
                                            )));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        "OID" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(oid_name) = token {
                                    for oid in param_config.oid_config.iter() {
                                        if oid.name == *oid_name {
                                            // Convert to bytes32
                                            let bytes =
                                                hex::decode(&oid.address[2..]).map_err(|_| {
                                                    GeneralError::InvalidTypeConvertError(format!(
                                                        "Failed to parse bytes32: {}",
                                                        oid.address
                                                    ))
                                                })?;
                                            params.push(Some(GeneralToken::FixedBytes(bytes)));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        "Validator" => {
                            if let Some(token) = function_param {
                                if let GeneralToken::String(validator_name) = token {
                                    for validator in param_config.validator_config.iter() {
                                        if validator.name == *validator_name {
                                            if available_contract.as_deref() == Some("State") {
                                                params.push(Some(GeneralToken::Address(
                                                    validator.address.parse().unwrap(),
                                                )));
                                            } else if available_contract.as_deref()
                                                == Some("Candidate")
                                            {
                                                params.push(Some(GeneralToken::Address(
                                                    validator.controller_address.parse().unwrap(),
                                                )));
                                            }

                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            if let Some(token) = function_param {
                                params.push(Some(token.clone()));
                            }
                        }
                    }
                }

                let result = get(
                    config,
                    (
                        chain_id,
                        address,
                        abi,
                        params,
                        target_index,
                        target_block_number,
                    ),
                )
                .await;

                Ok(result)
            }

            Rule::EventCallExpr => {
                let inner = pair.into_inner();

                for unwrapped_pair in inner {
                    parse_pair(config, program_rule, unwrapped_pair, context).await?;
                }

                let chain_id = match context.variables.get("chain_id").unwrap() {
                    ParseResultType::ChainID(id) => *id as i32,
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected ChainID for chain_id".to_string(),
                        ))
                    }
                };
                let address = match context.variables.get("address").unwrap() {
                    ParseResultType::String(addr) => addr.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for address".to_string(),
                        ))
                    }
                };
                let abi = match context.variables.get("abi").unwrap() {
                    ParseResultType::JSON(abi) => abi.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected JSON for abi".to_string(),
                        ))
                    }
                };
                let event_index = match context.variables.get("event_index").unwrap() {
                    ParseResultType::EventIndex(idx) => idx.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected EventIndex for event_index".to_string(),
                        ))
                    }
                };
                let target_index = match context.variables.get("target_index").unwrap() {
                    ParseResultType::String(idx) => idx.clone(),
                    _ => {
                        return Err(GeneralError::InvalidTypeConvertError(
                            "Expected String for target_index".to_string(),
                        ))
                    }
                };

                let mut target_block_number = get_latest_block_number(config, chain_id)
                    .await
                    .into_uint()
                    .unwrap();

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

                let result = get(
                    config,
                    (
                        chain_id,
                        address,
                        abi,
                        event_index,
                        target_index,
                        target_block_number,
                    ),
                )
                .await;

                Ok(result)
            }

            Rule::Chain => {
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

                Ok(GeneralToken::Bool(false))
            }

            Rule::Service => {
                let service = pair.as_str();

                if let Some(_) = context.variables.get("chain_id") {
                    for contract_config in context.config.contract_config.clone() {
                        let ContractConfig {
                            service: parsed_service,
                            blockchain,
                            contract,
                            ..
                        } = contract_config;

                        if *service == parsed_service
                            && *blockchain
                                == *match context.variables.get("blockchain").unwrap() {
                                    ParseResultType::String(s) => s,
                                    _ => {
                                        return Err(GeneralError::InvalidTypeConvertError(
                                            "Expected String for blockchain".to_string(),
                                        ))
                                    }
                                }
                        {
                            context.variables.insert(
                                "service".to_string(),
                                ParseResultType::String(service.to_string()),
                            );

                            context
                                .variables
                                .insert("contract".to_string(), ParseResultType::String(contract));
                        }
                    }
                } else {
                }

                Ok(GeneralToken::Bool(false))
            }

            Rule::Notification => {
                let notification = pair.as_str();

                for notification_config in context.config.notification_config.clone() {
                    let NotificationConfig { service, key } = notification_config;

                    if notification == service {
                        context.variables.insert(
                            "notification".to_string(),
                            ParseResultType::String(service.to_string()),
                        );

                        context
                            .variables
                            .insert("key".to_string(), ParseResultType::String(key.to_string()));
                    }
                }

                Ok(GeneralToken::Bool(false))
            }

            Rule::Contract => {
                let contract_str = pair.as_str();
                let mut found = false;

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
                            // Get the config file's directory
                            let config_dir = std::path::Path::new(CONFIG_PATH).parent().unwrap();
                            // Resolve the ABI path relative to the config file
                            let abi_path = config_dir.join(path);
                            let abi_content =
                                fs::read_to_string(abi_path.clone()).map_err(|_e| {
                                    GeneralError::InvalidTypeConvertError(format!(
                                        "Failed to read ABI file: {}",
                                        &abi_path.display()
                                    ))
                                })?;

                            let abi = parse_abi_text(&abi_content)?;

                            context
                                .variables
                                .insert("abi".to_string(), ParseResultType::JSON(abi));

                            context
                                .variables
                                .insert("address".to_string(), ParseResultType::String(address));

                            found = false;
                            break;
                        }
                    }
                }

                Ok(GeneralToken::Bool(found))
            }

            Rule::Identifier => {
                let identifier = pair.as_str();

                let check_store_value = check_store_value(context.symbol_table, identifier);

                let result = if check_store_value == GeneralToken::Bool(true) {
                    eval(context.symbol_table, identifier)
                } else {
                    GeneralToken::String(identifier.to_string())
                };

                if let Some(ParseResultType::HashMap(existing)) =
                    context.variables.get("identifier")
                {
                    let mut new_hashmap = existing.clone();
                    new_hashmap.insert(
                        identifier.to_string(),
                        ParseResultType::GeneralToken(result.clone()),
                    );
                    context.variables.insert(
                        "identifier".to_string(),
                        ParseResultType::HashMap(new_hashmap),
                    );
                } else {
                    let mut new_hashmap = HashMap::new();
                    new_hashmap.insert(
                        identifier.to_string(),
                        ParseResultType::GeneralToken(result.clone()),
                    );
                    context.variables.insert(
                        "identifier".to_string(),
                        ParseResultType::HashMap(new_hashmap),
                    );
                }

                Ok(result)
            }

            Rule::RpcFunctionName => {
                let rpc_call_target_str = pair.as_str();

                for rpc_call_target in context.config.rpc_call_target.clone() {
                    let RPCTargetValue {
                        name,
                        meta_data,
                        call_type,
                        method_type,
                        api_body,
                        api_query,
                        target_index,
                        param_nessesary,
                    } = rpc_call_target;

                    if rpc_call_target_str == name {
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

                        context
                            .variables
                            .insert("call_type".to_string(), ParseResultType::String(call_type));
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

                        context.variables.insert(
                            "target_index".to_string(),
                            ParseResultType::String(target_index),
                        );

                        context
                            .variables
                            .insert("meta_data".to_string(), ParseResultType::String(meta_data));

                        context.variables.insert(
                            "param_nessesary".to_string(),
                            ParseResultType::Array(param_nessesary),
                        );
                    }
                }

                Ok(GeneralToken::Bool(false))
            }

            Rule::ContractMethodName => {
                let contract_call_target_str = pair.as_str();

                for contract_call_target in context.config.contract_call_target.clone() {
                    let ContractCallTargetValue {
                        name,
                        params,
                        target_index,
                        param_nessesary,
                        available_contract,
                    } = contract_call_target;

                    if contract_call_target_str == name {
                        let should_insert = if let Some(available_contract) = available_contract {
                            if let Some(ParseResultType::String(contract)) =
                                context.variables.get("contract")
                            {
                                if available_contract == *contract {
                                    context.variables.insert(
                                        "available_contract".to_string(),
                                        ParseResultType::String(available_contract),
                                    );
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            true
                        };

                        if should_insert {
                            context.variables.insert(
                                "method_params".to_string(),
                                ParseResultType::ArrayParam(params.clone()),
                            );
                            context.variables.insert(
                                "target_index".to_string(),
                                ParseResultType::String(target_index),
                            );
                            context.variables.insert(
                                "param_nessesary".to_string(),
                                ParseResultType::Array(param_nessesary),
                            );

                            break;
                        }
                    }
                }

                Ok(GeneralToken::Bool(false))
            }

            Rule::EventName => {
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

                Ok(GeneralToken::Bool(false))
            }

            Rule::NotificationFunctionName => {
                let notification_call_target_str = pair.as_str();

                for notification_call_target in context.config.notification_call_target.clone() {
                    let NotificationCallTargetValue {
                        name,
                        params,
                        param_nessesary,
                    } = notification_call_target;

                    if notification_call_target_str == name {
                        context.variables.insert(
                            "name".to_string(),
                            ParseResultType::ArrayParam(params.clone()),
                        );

                        context.variables.insert(
                            "param_nessesary".to_string(),
                            ParseResultType::Array(param_nessesary),
                        );
                    }
                }

                Ok(GeneralToken::Bool(false))
            }

            Rule::ChainFunctionName => {
                let blockchain_call_target_str = pair.as_str();

                for blockchain_call_target in context.config.blockchain_call_target.clone() {
                    let BlockchainTargetValue {
                        name,
                        param_nessesary,
                        ..
                    } = blockchain_call_target;

                    if blockchain_call_target_str == name {
                        context
                            .variables
                            .insert("name".to_string(), ParseResultType::String(name));

                        context.variables.insert(
                            "param_nessesary".to_string(),
                            ParseResultType::Array(param_nessesary),
                        );
                    }
                }

                Ok(GeneralToken::Bool(false))
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
pub fn _check_function_length(abi_text: &str) -> Result<bool, GeneralError> {
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
pub fn _check_event_length(abi_text: &str) -> Result<bool, GeneralError> {
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
    _Number(U256),
    _Bool(bool),
    JSON(Value),
    ChainID(ChainID),
    Array(Vec<String>),
    ArrayParam(Vec<Option<GeneralToken>>),
    GeneralToken(GeneralToken),
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
    program_rule: &watch_tower_lib::config::Rule,
) -> Result<GeneralToken, GeneralError> {
    let mut symbol_table = SymbolTable::new();

    let pairs = match RuleEvaluationParser::parse(Rule::Program, &program_rule.script) {
        Ok(pairs) => pairs,
        Err(_) => {
            return Err(GeneralError::InvalidRuleDecode(
                program_rule.script.to_string(),
            ));
        }
    };
    let mut result = GeneralToken::Bool(false);
    let mut context = Context {
        config,
        symbol_table: &mut symbol_table,
        variables: &mut HashMap::new(),
    };
    for pair in pairs {
        result = parse_pair(config, program_rule, pair, &mut context).await?;
    }
    Ok(result)
}

/// Log notification to file
fn log_notification(rule_name: &str, channel: &str, message: &str) -> std::io::Result<()> {
    let log_dir = "./service/log";
    let log_path = format!("{}/notification.log", log_dir);

    // Create directory if it doesn't exist
    create_dir_all(log_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    writeln!(
        file,
        "[Notification] ({}) [{}] {} {}",
        rule_name, channel, timestamp, message
    )?;
    Ok(())
}

/// Check if a similar message was sent in the last 3 hours
fn check_recent_notification(rule_name: &str, channel: &str) -> bool {
    let log_path = "./service/log/notification.log";
    if let Ok(file) = File::open(log_path) {
        let reader = BufReader::new(file);
        let current_time = Local::now();

        // 3 hours in seconds
        let three_hours = chrono::Duration::hours(3);

        for line in reader.lines().flatten() {
            if line.contains(&format!("({})", rule_name))
                && line.contains(&format!("[{}]", channel))
            {
                // Extract timestamp from log line - format: [Notification] (rule_name) [channel] YYYY-MM-DD HH:MM:SS
                let parts: Vec<&str> = line.split("] ").collect();
                if parts.len() >= 2 {
                    // Get the last part which contains the timestamp and message
                    let last_part = parts.last().unwrap();
                    let timestamp_str = last_part
                        .split_whitespace()
                        .take(2)
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if let Ok(log_time) =
                        NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                    {
                        let log_time = Local.from_local_datetime(&log_time).unwrap();
                        if current_time.signed_duration_since(log_time) < three_hours {
                            return true;
                        }
                    } else {
                        println!("failed parse log");
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {

    use watch_tower_lib::config::{set_rule, set_test_config};

    use super::*;

    fn setup() -> Configuration {
        set_test_config("/Users/munseon-ug/rust/watchtower/worker/config.yaml")
    }

    #[test]
    fn test_new_parse_rule() {
        let test_input: &'static str = "
        active_status = 1;
        bifrostBN = Bifrost_Testnet.LatestBlock();
        arbitrumBN = Arbitrum_Sepolia.LatestBlock();
        ";

        let pairs = RuleEvaluationParser::parse(Rule::Program, test_input).unwrap();
        println!("pairs: {:?}", pairs);
    }
}
