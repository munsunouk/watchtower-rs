pub mod constants;
pub mod error;
pub mod types;

use crate::{
    cli::db::postgres::PostgresClient,
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, parse_token_to_string,
        rpc_call::RpcCallRule,
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
    EVALUATION_RULE_TYPE, FIXED_BYTES_COMPARATOR_TYPE, FLOAT_ARITHMETIC_TYPE,
    FLOAT_COMPARATOR_TYPE, INT_ARITHMETIC_TYPE, INT_COMPARATOR_TYPE, OPERATOR_ADD, OPERATOR_DIV,
    OPERATOR_MUL, OPERATOR_SUB, RPC_CALL, RPC_CALL_LOG, RPC_CALL_LOG_TYPE, RPC_CALL_RULE,
    RPC_CALL_RULE_TYPE, STRING_ARITHMETIC_TYPE, STRING_COMPARATOR_TYPE, UINT_ARITHMETIC_TYPE,
    UINT_COMPARATOR_TYPE,
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
use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::utils::types::GeneralToken;

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
        .map_err(|_| GeneralError::InvalidTypeConvertError(input.to_string()))
}

pub fn parse_u256_to_u64(input: U256) -> U64 {
    U64::from(input.as_u64())
}

pub fn parse_string_to_u64(input: String) -> Result<U64, GeneralError> {
    input
        .parse::<U64>()
        .map_err(|_| GeneralError::InvalidTypeConvertError(input))
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
        _ => Err(GeneralError::InvalidTypeConvertError(input)),
    }
}

pub fn parse_json_to_value(input: Json<Value>) -> Result<Value, GeneralError> {
    serde_json::to_value(input)
        .map_err(|_| GeneralError::InvalidTypeConvertError("JSON conversion failed".to_string()))
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

pub fn parse_f64_to_uint(input: f64) -> Result<Uint, GeneralError> {
    U256::from_dec_str(&input.to_string())
        .map_err(|_| GeneralError::InvalidTypeConvertError(input.to_string()))
}

pub fn parse_token_to_i64(token: Token) -> Result<i64, GeneralError> {
    match token {
        Token::Uint(value) => Ok(value.as_u64() as i64),
        Token::Int(value) => Ok(value.as_u64() as i64),
        _ => Err(GeneralError::InvalidTypeConvertError(format!(
            "{:?}",
            token
        ))),
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

pub fn parse_string_to_float(input: String) -> Result<f64, GeneralError> {
    input
        .parse::<f64>()
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
        .map_err(|_| GeneralError::InvalidTypeConvertError(input))
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
pub fn compare_token(
    left: &GeneralToken,
    right: &GeneralToken,
    comparator: &str,
) -> Option<GeneralToken> {
    if !check_type_comparator(left, right, comparator) {
        return None;
    }
    match (left, right) {
        (GeneralToken::Uint(value), GeneralToken::Uint(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Uint(value), GeneralToken::Int(expected_value)) => {
            let left_big = BigInt::from_str(&value.to_string()).ok()?;
            parse_compare(&left_big, expected_value, comparator)
        }
        (GeneralToken::Int(value), GeneralToken::Int(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Int(value), GeneralToken::Uint(expected_value)) => {
            let right_big = BigInt::from_str(&expected_value.to_string()).ok()?;
            parse_compare(value, &right_big, comparator)
        }
        (GeneralToken::Float(value), GeneralToken::Float(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Float(value), GeneralToken::Int(expected_value)) => {
            let right_float = expected_value.to_f64().unwrap_or(0.0);
            parse_compare(value, &right_float, comparator)
        }
        (GeneralToken::Int(value), GeneralToken::Float(expected_value)) => {
            let left_float = value.to_f64().unwrap_or(0.0);
            parse_compare(&left_float, expected_value, comparator)
        }
        (GeneralToken::Float(value), GeneralToken::Uint(expected_value)) => {
            let right_float = expected_value.to_string().parse::<f64>().unwrap_or(0.0);
            parse_compare(value, &right_float, comparator)
        }
        (GeneralToken::Uint(value), GeneralToken::Float(expected_value)) => {
            let left_float = value.to_string().parse::<f64>().unwrap_or(0.0);
            parse_compare(&left_float, expected_value, comparator)
        }
        (GeneralToken::Bool(value), GeneralToken::Bool(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::String(value), GeneralToken::String(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Address(value), GeneralToken::Address(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Bytes(value), GeneralToken::Bytes(expected_value))
        | (GeneralToken::FixedBytes(value), GeneralToken::FixedBytes(expected_value)) => {
            let parsing_value = hex::encode(value);
            let parsing_expected_value = hex::encode(expected_value);
            parse_compare(&parsing_value, &parsing_expected_value, comparator)
        }
        _ => None,
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
) -> Option<GeneralToken> {
    match comparator {
        COMPARATOR_EQUAL => Some(GeneralToken::Bool(value == expected_value)),
        COMPARATOR_GREATER => Some(GeneralToken::Bool(value > expected_value)),
        COMPARATOR_GREATER_EQUAL => Some(GeneralToken::Bool(value >= expected_value)),
        COMPARATOR_LESS => Some(GeneralToken::Bool(value < expected_value)),
        COMPARATOR_LESS_EQUAL => Some(GeneralToken::Bool(value <= expected_value)),
        COMPARATOR_NOT_EQUAL => Some(GeneralToken::Bool(value != expected_value)),
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
pub fn check_type_comparator(left: &GeneralToken, right: &GeneralToken, comparator: &str) -> bool {
    match (left, right) {
        (GeneralToken::Uint(_), GeneralToken::Uint(_)) => {
            UINT_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Int(_), GeneralToken::Int(_))
        | (GeneralToken::Uint(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Uint(_)) => {
            INT_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Float(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Uint(_))
        | (GeneralToken::Uint(_), GeneralToken::Float(_)) => {
            FLOAT_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Address(_), GeneralToken::Address(_)) => {
            ADDRESS_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Bool(_), GeneralToken::Bool(_)) => {
            BOOL_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Bytes(_), GeneralToken::Bytes(_)) => {
            BYTES_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::FixedBytes(_), GeneralToken::FixedBytes(_)) => {
            FIXED_BYTES_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::String(_), GeneralToken::String(_)) => {
            STRING_COMPARATOR_TYPE.contains(&comparator)
        }
        _ => false,
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
pub fn arithmetic_token(
    left: &GeneralToken,
    right: &GeneralToken,
    operator: &str,
) -> Option<GeneralToken> {
    if !check_type_arithmetic(left, right, operator) {
        return None;
    }
    match (left, right) {
        (GeneralToken::Uint(value), GeneralToken::Uint(expected_value)) => {
            parse_u256_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::Uint(value), GeneralToken::Int(expected_value)) => {
            let left_big = BigInt::from_str(&value.to_string()).ok()?;
            parse_bigint_arithmetic(&left_big, expected_value, operator)
        }
        (GeneralToken::Int(value), GeneralToken::Int(expected_value)) => {
            parse_bigint_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::Int(value), GeneralToken::Uint(expected_value)) => {
            let right_big = BigInt::from_str(&expected_value.to_string()).ok()?;
            parse_bigint_arithmetic(value, &right_big, operator)
        }
        (GeneralToken::Float(value), GeneralToken::Float(expected_value)) => {
            parse_float_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::Float(value), GeneralToken::Int(expected_value)) => {
            let right_float = expected_value.to_f64().unwrap_or(0.0);
            parse_float_arithmetic(value, &right_float, operator)
        }
        (GeneralToken::Float(value), GeneralToken::Uint(expected_value)) => {
            let right_float = expected_value.to_string().parse::<f64>().unwrap_or(0.0);
            parse_float_arithmetic(value, &right_float, operator)
        }
        (GeneralToken::Int(value), GeneralToken::Float(expected_value)) => {
            let left_float = value.to_f64().unwrap_or(0.0);
            parse_float_arithmetic(&left_float, expected_value, operator)
        }
        (GeneralToken::Uint(value), GeneralToken::Float(expected_value)) => {
            let left_float = value.to_string().parse::<f64>().unwrap_or(0.0);
            parse_float_arithmetic(&left_float, expected_value, operator)
        }
        (GeneralToken::String(value), GeneralToken::String(expected_value)) => {
            parse_string_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::String(value), other) | (other, GeneralToken::String(value)) => {
            let other_str = other.clone().into_string().unwrap_or_default();
            parse_string_arithmetic(value, &other_str, operator)
        }
        _ => None,
    }
}

pub fn format_float_to_4_decimal(value: f64) -> f64 {
    (value * 10000.0).trunc() / 10000.0
}

fn parse_float_arithmetic(
    value: &f64,
    expected_value: &f64,
    operator: &str,
) -> Option<GeneralToken> {
    match operator {
        OPERATOR_ADD => Some(GeneralToken::Float(value + expected_value)),
        OPERATOR_SUB => Some(GeneralToken::Float(value - expected_value)),
        OPERATOR_MUL => Some(GeneralToken::Float(value * expected_value)),
        OPERATOR_DIV => Some(GeneralToken::Float(value / expected_value)),
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
pub fn check_type_arithmetic(left: &GeneralToken, right: &GeneralToken, operator: &str) -> bool {
    match (left, right) {
        (GeneralToken::Uint(_), GeneralToken::Uint(_)) => UINT_ARITHMETIC_TYPE.contains(&operator),
        (GeneralToken::Int(_), GeneralToken::Int(_))
        | (GeneralToken::Uint(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Uint(_)) => INT_ARITHMETIC_TYPE.contains(&operator),
        (GeneralToken::Float(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Uint(_))
        | (GeneralToken::Uint(_), GeneralToken::Float(_)) => {
            FLOAT_ARITHMETIC_TYPE.contains(&operator)
        }
        (GeneralToken::String(_), _) | (_, GeneralToken::String(_)) => {
            STRING_ARITHMETIC_TYPE.contains(&operator)
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
pub fn parse_bigint_arithmetic(
    value: &BigInt,
    expected_value: &BigInt,
    operator: &str,
) -> Option<GeneralToken> {
    match operator {
        OPERATOR_ADD => Some(GeneralToken::Int(value + expected_value)),
        OPERATOR_SUB => Some(GeneralToken::Int(value - expected_value)),
        OPERATOR_MUL => Some(GeneralToken::Int(value * expected_value)),
        OPERATOR_DIV => {
            let result = value.to_f64().unwrap_or(0.0) / expected_value.to_f64().unwrap_or(1.0);
            Some(GeneralToken::Float(result))
        }
        _ => None,
    }
}

pub fn parse_u256_arithmetic(
    value: &U256,
    expected_value: &U256,
    operator: &str,
) -> Option<GeneralToken> {
    match operator {
        OPERATOR_ADD => value.checked_add(*expected_value).map(GeneralToken::Uint),
        OPERATOR_SUB => value.checked_sub(*expected_value).map(GeneralToken::Uint),
        OPERATOR_MUL => value.checked_mul(*expected_value).map(GeneralToken::Uint),
        OPERATOR_DIV => value.checked_div(*expected_value).map(GeneralToken::Uint),
        _ => None,
    }
}

/// # Description
/// This function parses string arithmetic operations.
/// # Arguments
///
/// * `value` - The string value to parse.
/// * `expected_value` - The expected value to concatenate with.
/// * `operator` - The operator string.
///
/// # Returns
///
/// An optional token containing the parsed value.
pub fn parse_string_arithmetic(
    value: &str,
    expected_value: &str,
    operator: &str,
) -> Option<GeneralToken> {
    match operator {
        OPERATOR_ADD => Some(GeneralToken::String(value.to_string() + expected_value)),
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
        Ok(ParamType::Address)
    } else {
        Ok(ParamType::String)
    }
}

pub fn hex_to_eth_amount(hex: &str) -> Result<U256, GeneralError> {
    U256::from_str_radix(&hex[2..], 16)
        .map_err(|_| GeneralError::InvalidTypeConvertError(hex.to_string()))
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
