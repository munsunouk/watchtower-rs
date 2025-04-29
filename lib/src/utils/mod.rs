pub mod constants;
pub mod error;
pub mod types;

use crate::{
    cli::db::postgres::PostgresClient,
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, rpc_call::RpcCallRule,
    },
    utils::error::GeneralError,
};

use abi::ParamType;
use constants::{
    ADDRESS_COMPARATOR_TYPE, BOOL_COMPARATOR_TYPE, BYTES_COMPARATOR_TYPE, COMPARATOR_EQUAL,
    COMPARATOR_GREATER, COMPARATOR_GREATER_EQUAL, COMPARATOR_LESS, COMPARATOR_LESS_EQUAL,
    COMPARATOR_NOT_EQUAL, CONTRACT_CALL, CONTRACT_CALL_BLOCK_LOG, CONTRACT_CALL_BLOCK_LOG_TYPE,
    CONTRACT_CALL_LOG, CONTRACT_CALL_LOG_TYPE, CONTRACT_CALL_RULE, CONTRACT_CALL_RULE_TYPE,
    CONTRACT_EVENT, CONTRACT_EVENT_BLOCK_LOG, CONTRACT_EVENT_BLOCK_LOG_TYPE, CONTRACT_EVENT_LOG,
    CONTRACT_EVENT_LOG_TYPE, CONTRACT_EVENT_RULE, CONTRACT_EVENT_RULE_TYPE, EVALUATION_RULE,
    EVALUATION_RULE_TYPE, FIXED_BYTES_COMPARATOR_TYPE, INT_ARITHMETIC_TYPE, INT_COMPARATOR_TYPE,
    OPERATOR_ADD, OPERATOR_DIV, OPERATOR_MUL, OPERATOR_SUB, RPC_CALL, RPC_CALL_LOG,
    RPC_CALL_LOG_TYPE, RPC_CALL_RULE, RPC_CALL_RULE_TYPE, STRING_COMPARATOR_TYPE,
    UINT_ARITHMETIC_TYPE, UINT_COMPARATOR_TYPE,
};
use reqwest::Method;
use serde_json::{from_str, Value};
use sqlx::types::Json;

use std::str::FromStr;

use ethers::{
    abi::{Abi, Int, Token, Uint},
    prelude::*,
    utils::hex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcCallType {
    Body,
    Query,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DbRuleType {
    ContractCall,
    ContractEvent,
    RpcCall,
    Evaluation,
    ContractCallLog,
    ContractEventLog,
    RpcCallLog,
    ContractCallBlockLog,
    ContractEventBlockLog,
}

impl FromStr for DbRuleType {
    type Err = GeneralError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            CONTRACT_CALL_RULE_TYPE | CONTRACT_CALL | CONTRACT_CALL_RULE => Ok(Self::ContractCall),
            CONTRACT_EVENT_RULE_TYPE | CONTRACT_EVENT | CONTRACT_EVENT_RULE => {
                Ok(Self::ContractEvent)
            }
            RPC_CALL_RULE_TYPE | RPC_CALL | RPC_CALL_RULE => Ok(Self::RpcCall),
            EVALUATION_RULE_TYPE | EVALUATION_RULE => Ok(Self::Evaluation),
            CONTRACT_CALL_LOG_TYPE | CONTRACT_CALL_LOG => Ok(Self::ContractCallLog),
            CONTRACT_EVENT_LOG_TYPE | CONTRACT_EVENT_LOG => Ok(Self::ContractEventLog),
            RPC_CALL_LOG_TYPE | RPC_CALL_LOG => Ok(Self::RpcCallLog),
            CONTRACT_CALL_BLOCK_LOG_TYPE | CONTRACT_CALL_BLOCK_LOG => {
                Ok(Self::ContractCallBlockLog)
            }
            CONTRACT_EVENT_BLOCK_LOG_TYPE | CONTRACT_EVENT_BLOCK_LOG => {
                Ok(Self::ContractEventBlockLog)
            }
            _ => Err(GeneralError::InvalidRuleDecode(
                "Invalid rule type".to_string(),
            )),
        }
    }
}

impl DbRuleType {
    pub fn to_str(&self) -> &str {
        match self {
            Self::ContractCall => CONTRACT_CALL_RULE,
            Self::ContractEvent => CONTRACT_EVENT_RULE,
            Self::RpcCall => RPC_CALL_RULE,
            Self::Evaluation => EVALUATION_RULE,
            Self::ContractCallLog => CONTRACT_CALL_LOG,
            Self::ContractEventLog => CONTRACT_EVENT_LOG,
            Self::RpcCallLog => RPC_CALL_LOG,
            Self::ContractCallBlockLog => CONTRACT_CALL_BLOCK_LOG,
            Self::ContractEventBlockLog => CONTRACT_EVENT_BLOCK_LOG,
        }
    }

    pub fn to_wrapped_str(&self) -> Result<String, GeneralError> {
        match self {
            Self::ContractCall => Ok("contractcall".to_string()),
            Self::ContractEvent => Ok("contractevent".to_string()),
            Self::RpcCall => Ok("rpccall".to_string()),
            _ => Err(GeneralError::InvalidRuleDecode(format!(
                "Invalid rule type: {:?}",
                self
            ))),
        }
    }
}

/// Converts an i32 to usize.
///
/// # Arguments
///
/// * `input` - An i32 value.
///
/// # Returns
///
/// A usize value.
pub fn parse_i32_to_usize(input: i32) -> Result<usize, GeneralError> {
    input
        .try_into()
        .map_err(|_| GeneralError::InvalidTypeConvert)
}

pub fn parse_u256_to_u64(input: U256) -> U64 {
    U64::from(input.as_u64())
}

pub fn parse_string_to_u64(input: String) -> Result<U64, GeneralError> {
    input
        .parse::<U64>()
        .map_err(|_| GeneralError::InvalidTypeConvert)
}

pub fn parse_i64_to_u64(input: i64) -> U64 {
    U64::from(input)
}

pub fn parse_string_to_method(input: String) -> Method {
    Method::from_bytes(input.as_bytes()).unwrap_or(Method::POST)
}

pub fn parse_string_to_rpc_call_type(input: String) -> Result<RpcCallType, GeneralError> {
    match input.as_str() {
        "body" => Ok(RpcCallType::Body),
        "query" => Ok(RpcCallType::Query),
        _ => Err(GeneralError::InvalidTypeConvert),
    }
}

pub fn parse_json_to_value(input: Json<Value>) -> Result<Value, GeneralError> {
    serde_json::to_value(input).map_err(|_| GeneralError::InvalidTypeConvert)
}

/// Parses a JSON value into an ABI.
///
/// # Arguments
///
/// * `input` - A JSON value representing the ABI.
///
/// # Returns
///
/// A Result containing either an `Abi` instance or a `GeneralError`.
pub fn parse_to_abi(input: Value) -> Result<Abi, GeneralError> {
    from_str(&input.to_string()).map_err(|_| GeneralError::InvalidTypeABI)
}

/// Parses a string into a uint.
///
/// # Arguments
///
/// * `input` - A string representing the uint.
///
/// # Returns
///
/// A `Uint` instance.
pub fn parse_string_to_uint(input: String) -> Result<Uint, GeneralError> {
    U256::from_dec_str(&input).map_err(|_| GeneralError::InvalidTypeConvertError(input))
}

pub fn parse_token_to_i64(token: Token) -> Result<i64, GeneralError> {
    match token {
        Token::Uint(value) => Ok(value.as_u64() as i64),
        Token::Int(value) => Ok(value.as_u64() as i64),
        _ => Err(GeneralError::InvalidTypeConvert),
    }
}

pub fn parse_string_to_address(input: String) -> Result<Address, GeneralError> {
    input
        .parse::<Address>()
        .map_err(|_| GeneralError::InvalidTypeConvertError(input))
}

/// Parses a string into an int.
///
/// # Arguments
///
/// * `input` - A string representing the int.
///
/// # Returns
///
/// An `Int` instance.
pub fn parse_string_to_int(input: String) -> Result<Int, GeneralError> {
    input
        .parse::<Int>()
        .map_err(|_| GeneralError::InvalidTypeConvertError(input))
}

/// Parses a string into a bool.
///
/// # Arguments
///
/// * `input` - A string representing the bool.
///
/// # Returns
///
/// A `bool` instance.
pub fn parse_string_to_bool(input: String) -> Result<bool, GeneralError> {
    input
        .parse::<bool>()
        .map_err(|_| GeneralError::InvalidTypeConvertError(input))
}

/// Parses a string into an i32.
///
/// # Arguments
///
/// * `input` - A string representing the i32.
///
/// # Returns
///
/// An `i32` instance.
pub fn parse_string_to_i32(input: String) -> Result<i32, GeneralError> {
    input
        .parse::<i32>()
        .map_err(|_| GeneralError::InvalidTypeConvertError(input))
}

/// Parses a string into an Ethereum address.
///
/// # Arguments
///
/// * `input` - A string representing the address.
///
/// # Returns
///
/// A Result containing either an `Address` or a `GeneralError`.
pub fn parse_to_address(input: String) -> Result<Address, GeneralError> {
    input
        .parse::<Address>()
        .map_err(|_| GeneralError::InvalidTypeConvert)
}

/// # Description
/// This function compares a token with an expected value based on a comparator.
/// # Arguments
///
/// * `token` - The token to compare.
/// * `expected_value` - The expected value as a string.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// An optional string containing the value if the comparison is true.
pub fn compare_token(left: &Token, right: &Token, comparator: &str) -> Option<Token> {
    match (left, right) {
        (Token::Uint(value), Token::Uint(expected_value))
        | (Token::Uint(value), Token::Int(expected_value)) => {
            if !check_type_comparator(left, right, comparator) {
                return None;
            }

            parse_compare(value, expected_value, comparator)
        }
        (Token::Int(value), Token::Int(expected_value))
        | (Token::Int(value), Token::Uint(expected_value)) => {
            if !check_type_comparator(left, right, comparator) {
                return None;
            }
            parse_compare(value, expected_value, comparator)
        }
        (Token::Bool(value), Token::Bool(expected_value)) => {
            if !check_type_comparator(left, right, comparator) {
                return None;
            }

            parse_compare(value, expected_value, comparator)
        }
        (Token::String(value), Token::String(expected_value)) => {
            if !check_type_comparator(left, right, comparator) {
                return None;
            }
            parse_compare(value, expected_value, comparator)
        }
        (Token::Address(value), Token::Address(expected_value)) => {
            if !check_type_comparator(left, right, comparator) {
                return None;
            }

            parse_compare(value, expected_value, comparator)
        }
        (Token::Bytes(value), Token::Bytes(expected_value))
        | (Token::FixedBytes(value), Token::FixedBytes(expected_value)) => {
            if !check_type_comparator(left, right, comparator) {
                return None;
            }

            let parsing_value = hex::encode(value);
            let parsing_expected_value = hex::encode(expected_value);

            parse_compare(&parsing_value, &parsing_expected_value, comparator)
        }
        _ => None,
    }
}

/// # Description
/// This function performs arithmetic operations on tokens.
/// # Arguments
///
/// * `left` - The left token.
/// * `right` - The right token.
/// * `operator` - The operator string.
///
pub fn arithmetic_token(left: &Token, right: &Token, operator: &str) -> Option<Token> {
    match (left, right) {
        (Token::Uint(value), Token::Uint(expected_value))
        | (Token::Uint(value), Token::Int(expected_value)) => {
            if !check_type_arithmetic(left, right, operator) {
                return None;
            }
            parse_arithmetic(value, expected_value, operator)
        }
        (Token::Int(value), Token::Int(expected_value))
        | (Token::Int(value), Token::Uint(expected_value)) => {
            if !check_type_arithmetic(left, right, operator) {
                return None;
            }

            parse_arithmetic(value, expected_value, operator)
        }
        _ => None,
    }
}

/// # Description
/// This function checks if the type comparator is valid.
/// # Arguments
///
/// * `value` - The token to check.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// A boolean indicating if the type comparator is valid.
pub fn check_type_comparator(left: &Token, right: &Token, comparator: &str) -> bool {
    match (left, right) {
        (Token::Uint(_), Token::Uint(_)) | (Token::Uint(_), Token::Int(_)) => {
            UINT_COMPARATOR_TYPE.contains(&comparator)
        }
        (Token::Int(_), Token::Int(_)) | (Token::Int(_), Token::Uint(_)) => {
            INT_COMPARATOR_TYPE.contains(&comparator)
        }
        (Token::Address(_), Token::Address(_)) => ADDRESS_COMPARATOR_TYPE.contains(&comparator),
        (Token::Bool(_), Token::Bool(_)) => BOOL_COMPARATOR_TYPE.contains(&comparator),
        (Token::Bytes(_), Token::Bytes(_)) => BYTES_COMPARATOR_TYPE.contains(&comparator),
        (Token::FixedBytes(_), Token::FixedBytes(_)) => {
            FIXED_BYTES_COMPARATOR_TYPE.contains(&comparator)
        }
        (Token::String(_), Token::String(_)) => STRING_COMPARATOR_TYPE.contains(&comparator),
        _ => false,
    }
}

/// # Description
/// This function compares two values based on a comparator.
/// # Arguments
///
/// * `value` - The value to compare.
/// * `expected_value` - The expected value.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// An optional string containing the value if the comparison is true.
pub fn parse_compare<T: PartialOrd>(
    value: &T,
    expected_value: &T,
    comparator: &str,
) -> Option<Token> {
    match comparator {
        COMPARATOR_EQUAL => Some(Token::Bool(value == expected_value)),
        COMPARATOR_GREATER => Some(Token::Bool(value > expected_value)),
        COMPARATOR_GREATER_EQUAL => Some(Token::Bool(value >= expected_value)),
        COMPARATOR_LESS => Some(Token::Bool(value < expected_value)),
        COMPARATOR_LESS_EQUAL => Some(Token::Bool(value <= expected_value)),
        COMPARATOR_NOT_EQUAL => Some(Token::Bool(value != expected_value)),
        _ => None,
    }
}

/// # Description
/// This function checks if the type arithmetic is valid.
/// # Arguments
///
/// * `left` - The left token.
/// * `right` - The right token.
/// * `operator` - The operator string.
///
pub fn check_type_arithmetic(left: &Token, right: &Token, operator: &str) -> bool {
    match (left, right) {
        (Token::Uint(_), Token::Uint(_)) | (Token::Uint(_), Token::Int(_)) => {
            UINT_ARITHMETIC_TYPE.contains(&operator)
        }
        (Token::Int(_), Token::Int(_)) | (Token::Int(_), Token::Uint(_)) => {
            INT_ARITHMETIC_TYPE.contains(&operator)
        }
        _ => false,
    }
}

/// # Description
/// This function parses arithmetic values.
/// # Arguments
///
/// * `value` - The value to parse.
/// * `expected_value` - The expected value.
/// * `operator` - The operator string.
///
/// # Returns
///
/// An optional token containing the parsed value.
pub fn parse_arithmetic(value: &U256, expected_value: &U256, operator: &str) -> Option<Token> {
    match operator {
        OPERATOR_ADD => value.checked_add(*expected_value).map(Token::Uint),
        OPERATOR_SUB => value.checked_sub(*expected_value).map(Token::Uint),
        OPERATOR_MUL => value.checked_mul(*expected_value).map(Token::Uint),
        OPERATOR_DIV => value.checked_div(*expected_value).map(Token::Uint),
        _ => None,
    }
}

pub fn convert_hex_token(str_param: &str) -> Result<Token, GeneralError> {
    if str_param.starts_with("0x") {
        let trimmed_hex = str_param.strip_prefix("0x").unwrap_or(str_param);
        match trimmed_hex.len() {
            40 => Ok(Token::Address(Address::from_str(str_param).map_err(
                |_| GeneralError::InvalidTypeConvertError(str_param.to_string()),
            )?)),
            _ => Ok(Token::Uint(hex_to_eth_amount(str_param)?)),
        }
    } else {
        Ok(Token::String(str_param.to_string()))
    }
}

pub fn convert_hex_param(str_param: &str) -> Result<ParamType, GeneralError> {
    if str_param.starts_with("0x") {
        let trimmed_hex = str_param.strip_prefix("0x").unwrap_or(str_param);
        match trimmed_hex.len() {
            40 => Ok(ParamType::Address),
            _ => Ok(ParamType::Uint(256)),
        }
    } else {
        Ok(ParamType::String)
    }
}

pub fn hex_to_eth_amount(hex: &str) -> Result<U256, GeneralError> {
    U256::from_str_radix(&hex[2..], 16).map_err(|_| GeneralError::InvalidTypeConvert)
}

/// # Description
/// This function loads RPC call rules from the database.
/// # Arguments
/// * `db_client` - A reference to the Postgres client.
/// # Returns
///
/// A vector of `RpcCallRule`.
pub async fn load_rpc_call_rules(
    db_client: &PostgresClient,
) -> Result<Vec<RpcCallRule>, GeneralError> {
    let result = db_client
        .select_table(DbRuleType::RpcCall)
        .await
        .map_err(|e| GeneralError::InvalidDatabase(e.to_string()))?;

    let rpc_calls: Vec<RpcCallRule> = result
        .iter()
        .map(|row| row.try_into())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rpc_calls)
}

/// # Description
/// This function loads contract call rules from the database.
/// # Arguments
/// * `db_client` - A reference to the Postgres client.
/// # Returns
///
/// A vector of `ContractCallRule`.
pub async fn load_contract_call_rules(
    db_client: &PostgresClient,
) -> Result<Vec<ContractCallRule>, GeneralError> {
    let result = db_client
        .select_table(DbRuleType::ContractCall)
        .await
        .map_err(|e| GeneralError::InvalidDatabase(e.to_string()))?;

    let contract_calls: Vec<ContractCallRule> = result
        .iter()
        .map(|row| row.try_into())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(contract_calls)
}

/// # Description
/// This function loads contract event rules from the database.
/// # Arguments
/// * `db_client` - A reference to the Postgres client.
/// # Returns
///
/// A vector of `ContractEventRule`.
pub async fn load_contract_event_rules(
    db_client: &PostgresClient,
) -> Result<Vec<ContractEventRule>, GeneralError> {
    let result = db_client
        .select_table(DbRuleType::ContractEvent)
        .await
        .map_err(|e| GeneralError::InvalidDatabase(e.to_string()))?;

    let contract_events: Vec<ContractEventRule> = result
        .iter()
        .map(|row| row.try_into())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(contract_events)
}

// /// # Description
// /// This function loads evaluations from the database.
// /// # Arguments
// /// * `db_client` - A reference to the Postgres client.
// ///
// /// # Returns
// ///
// /// A vector of `EvaluationRule`.
// pub async fn load_evaluations(
//     db_client: &PostgresClient,
// ) -> Result<Vec<EvaluationRule>, GeneralError> {
//     let result = db_client
//         .select_table(DbRuleType::Evaluation)
//         .await
//         .map_err(|e| GeneralError::InvalidDatabase(e.to_string()))?;

//     result
//         .iter()
//         .map(|row| {
//             EvaluationRule::try_from(row)
//                 .map_err(|e| GeneralError::InvalidTypeConvertError(e.to_string()))
//         })
//         .collect::<Result<Vec<_>, _>>()
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_parse_string_to_method() {
        let method_type = "GET";

        let result = parse_string_to_method(method_type.to_string());

        println!("{}", result);
    }
}
