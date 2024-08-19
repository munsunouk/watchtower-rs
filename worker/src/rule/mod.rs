/// Module for contract call rules and related functionality.
pub mod contract_call;
/// Module for contract event rules and related functionality.
pub mod contract_event;
/// Module for RPC call rules and related functionality.
pub mod rpc_call;

pub use contract_call::ContractCall;
pub use contract_event::ContractEvent;
pub use rpc_call::RpcCall;

use cron::Schedule;
use serde_json::from_str;
use serde_json::Value;
use std::iter::zip;
use std::str::FromStr;
use std::sync::Arc;

use ethers::{
    abi::{Abi, Int, ParamType, Token, Uint},
    prelude::*,
    utils::hex,
};

use crate::utils::constants::ADDRESS_COMPARATOR_TYPE;
use crate::utils::constants::BOOL_COMPARATOR_TYPE;
use crate::utils::constants::BYTES_COMPARATOR_TYPE;
use crate::utils::constants::DEFAULT_INDEX;
use crate::utils::constants::DEFAULT_PARAM_VALUE;
use crate::utils::constants::FILTER_INDEX;
use crate::utils::constants::FILTER_INDEX_SPLIT_CHAR;
use crate::utils::constants::FILTER_VALUE;
use crate::utils::constants::FILTER_VALUE_SPLIT_CHAR;
use crate::utils::constants::FIXED_BYTES_COMPARATOR_TYPE;
use crate::utils::constants::INT_COMPARATOR_TYPE;
use crate::utils::constants::STRING_COMPARATOR_TYPE;
use crate::utils::constants::UINT_COMPARATOR_TYPE;
use crate::utils::error::IndexType;
use crate::utils::error::WorkerError;

/// Parses a JSON value into an ABI.
///
/// # Arguments
///
/// * `input` - A JSON value representing the ABI.
///
/// # Returns
///
/// An `Abi` instance.
pub fn parse_to_abi(input: Value) -> Abi {
    from_str(&input.to_string()).expect(&WorkerError::InvalidTypeABI.to_string())
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
pub fn parse_string_to_uint(input: String) -> Result<Uint, WorkerError> {
    input
        .parse::<Uint>()
        .map_err(|_| WorkerError::InvalidTypeConvertError(input))
}

pub fn parse_string_to_address(input: String) -> Result<Address, WorkerError> {
    input
        .parse::<Address>()
        .map_err(|_| WorkerError::InvalidTypeConvertError(input))
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
pub fn parse_string_to_int(input: String) -> Result<Int, WorkerError> {
    input
        .parse::<Int>()
        .map_err(|_| WorkerError::InvalidTypeConvertError(input))
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
pub fn parse_string_to_bool(input: String) -> Result<bool, WorkerError> {
    input
        .parse::<bool>()
        .map_err(|_| WorkerError::InvalidTypeConvertError(input))
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
pub fn parse_i32_to_usize(input: i32) -> usize {
    input
        .try_into()
        .expect(&WorkerError::InvalidTypeConvert.to_string())
}

/// Parses a string into an Ethereum address.
///
/// # Arguments
///
/// * `input` - A string representing the address.
///
/// # Returns
///
/// An `Address` instance.
pub fn parse_to_address(input: String) -> Address {
    input
        .parse::<Address>()
        .expect(&WorkerError::InvalidTypeConvert.to_string())
}

/// Sets a cron schedule based on the check interval.
///
/// # Arguments
///
/// * `check_interval` - The interval in seconds.
///
/// # Returns
///
/// A `Schedule` instance.
pub fn set_schedule(check_interval: usize) -> Result<Schedule, WorkerError> {
    let format_schedule = format!("{} * * * * *", check_interval);

    Schedule::from_str(&format_schedule)
        .map_err(|_| WorkerError::InvalidTypeConvertError(format_schedule))
}

/// Creates a new Ethereum contract instance.
///
/// # Arguments
///
/// * `address` - The address of the contract.
/// * `abi` - The ABI of the contract.
/// * `provider` - The provider for the contract.
///
/// # Returns
///
/// A `Contract` instance.
pub fn create_contracts<T: JsonRpcClient>(
    address: &Address,
    abi: &Abi,
    providers: Vec<Arc<Provider<T>>>,
) -> Vec<Contract<Provider<T>>> {
    providers
        .iter()
        .map(|provider| Contract::new(address.clone(), abi.clone(), provider.clone()))
        .collect::<Vec<_>>()
}

/// Parses rule filters.
///
/// # Arguments
///
/// * `rule_filters` - A vector of rule filter strings.
///
/// # Returns
///
/// A vector of tuples containing indices and values.
pub fn parse_rule_filter(
    rule_filters: &Vec<String>,
) -> Result<Vec<(Vec<usize>, String)>, WorkerError> {
    rule_filters
        .iter()
        .map(|rule_filter| {
            let parts: Vec<&str> = rule_filter.split(FILTER_VALUE_SPLIT_CHAR).collect();
            let indices = parts
                .get(FILTER_INDEX)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(FILTER_INDEX)))?
                .split(FILTER_INDEX_SPLIT_CHAR)
                .map(|s| {
                    s.parse()
                        .map_err(|_| WorkerError::InvalidTypeConvertError(s.to_string()))
                })
                .collect::<Result<Vec<usize>, WorkerError>>()?;
            let value = parts
                .get(FILTER_VALUE)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(FILTER_VALUE)))?
                .to_string();
            Ok((indices, value))
        })
        .collect()
}

/// Parses the expected value filter.
///
/// # Arguments
///
/// * `expected_value_filter` - A string representing the expected value filter.
///
/// # Returns
///
/// A tuple containing a vector of usize values and a string value.
pub fn parse_expected_value_filter(
    expected_value_filter: &String,
) -> Result<(Vec<usize>, String), WorkerError> {
    let parts: Vec<&str> = expected_value_filter
        .split(FILTER_VALUE_SPLIT_CHAR)
        .collect();
    let indices: Vec<usize> = parts
        .get(FILTER_INDEX)
        .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(FILTER_INDEX)))?
        .split(FILTER_INDEX_SPLIT_CHAR)
        .map(|s| {
            s.parse()
                .map_err(|_| WorkerError::InvalidTypeConvertError(s.to_string()))
        })
        .collect::<Result<Vec<usize>, WorkerError>>()?;
    let value = parts
        .get(FILTER_VALUE)
        .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(FILTER_VALUE)))?
        .to_string();
    Ok((indices, value))
}

/// Encodes parameters into a token.
///
/// # Arguments
///
/// * `params` - A vector of parameter strings.
/// * `param_type` - The parameter type.
///
/// # Returns
///
/// A `Token` instance.
pub fn encode_token(params: Vec<String>, param_type: &ParamType) -> Result<Token, WorkerError> {
    if params.is_empty() {
        return Ok(Token::Tuple(vec![])); // Default case for empty params
    }

    match param_type {
        ParamType::String => Ok(Token::String(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE)))?
                .clone(),
        )),
        ParamType::Address => Ok(Token::Address(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE)))?
                .parse::<Address>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Bool => Ok(Token::Bool(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE)))?
                .parse::<bool>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Uint(_) => Ok(Token::Uint(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE)))?
                .parse::<Uint>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Int(_) => Ok(Token::Int(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE)))?
                .parse::<Int>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Bytes => Ok(Token::Bytes(
            hex::decode(
                &params
                    .get(DEFAULT_PARAM_VALUE)
                    .ok_or_else(|| {
                        WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE))
                    })?
                    .to_string(),
            )
            .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::FixedBytes(_) => Ok(Token::FixedBytes(
            hex::decode(
                &params
                    .get(DEFAULT_PARAM_VALUE)
                    .ok_or_else(|| {
                        WorkerError::InvalidIndex(IndexType::USize(DEFAULT_PARAM_VALUE))
                    })?
                    .to_string(),
            )
            .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Array(inner_type) => {
            let tokens: Vec<Token> = params
                .into_iter()
                .map(|p| encode_token(vec![p], inner_type))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Token::Array(tokens))
        }
        ParamType::FixedArray(inner_type, size) => {
            let tokens: Vec<Token> = params
                .into_iter()
                .take(*size)
                .map(|p| encode_token(vec![p], inner_type))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Token::FixedArray(tokens))
        }
        ParamType::Tuple(inner_types) => {
            let tokens: Vec<Token> = inner_types
                .iter()
                .zip(params.into_iter())
                .map(|(t, p)| encode_token(vec![p], t))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Token::Tuple(tokens))
        }
    }
}

/// Checks if the type comparator is valid.
///
/// # Arguments
///
/// * `value` - The token to check.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// A boolean indicating if the type comparator is valid.
pub fn check_type_comparator(value: &Token, comparator: &str) -> bool {
    match value {
        Token::Uint(_) => {
            if UINT_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }
        Token::Int(_) => {
            if INT_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }
        Token::Address(_) => {
            if ADDRESS_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }
        Token::Bool(_) => {
            if BOOL_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }
        Token::String(_) => {
            if STRING_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }
        Token::Bytes(_) => {
            if BYTES_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }

        Token::FixedBytes(_) => {
            if FIXED_BYTES_COMPARATOR_TYPE.contains(&comparator) {
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Compares two values based on a comparator.
///
/// # Arguments
///
/// * `value` - The value to compare.
/// * `expected_value` - The expected value.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// An optional string containing the value if the comparison is true.
pub fn parse_compare<T: PartialOrd + ToString>(
    value: &T,
    expected_value: &T,
    comparator: &str,
) -> Option<String> {
    match comparator {
        "==" if value == expected_value => Some(value.to_string()),
        ">" if value > expected_value => Some(value.to_string()),
        ">=" if value >= expected_value => Some(value.to_string()),
        "<" if value < expected_value => Some(value.to_string()),
        "<=" if value <= expected_value => Some(value.to_string()),
        "!=" if value != expected_value => Some(value.to_string()),
        _ => None,
    }
}

/// Compares a token with an expected value based on a comparator.
///
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
    token: &Token,
    expected_value: String,
    comparator: String,
) -> Result<Option<String>, WorkerError> {
    match token {
        Token::Uint(value) => {
            if !check_type_comparator(token, &comparator) {
                return Ok(None);
            }

            let expected_value = parse_string_to_uint(expected_value)?;
            Ok(parse_compare(value, &expected_value, &comparator))
        }
        Token::Int(value) => {
            if !check_type_comparator(token, &comparator) {
                return Ok(None);
            }
            let expected_value = parse_string_to_int(expected_value)?;
            Ok(parse_compare(value, &expected_value, &comparator))
        }
        Token::Bool(value) => {
            if !check_type_comparator(token, &comparator) {
                return Ok(None);
            }
            let expected_value = parse_string_to_bool(expected_value)?;
            Ok(parse_compare(value, &expected_value, &comparator))
        }
        Token::String(value) => {
            if !check_type_comparator(token, &comparator) {
                return Ok(None);
            }
            Ok(parse_compare(value, &expected_value, &comparator))
        }
        Token::Address(value) => {
            if !check_type_comparator(token, &comparator) {
                return Ok(None);
            }
            let expected_value = parse_string_to_address(expected_value)?;

            Ok(parse_compare(value, &expected_value, &comparator))
        }
        Token::Bytes(value) | Token::FixedBytes(value) => {
            if !check_type_comparator(token, &comparator) {
                return Ok(None);
            }
            let parsing_value = hex::encode(&value);
            Ok(parse_compare(&parsing_value, &expected_value, &comparator))
        }
        _ => Ok(None),
    }
}

/// Decodes a token based on the expected value path.
///
/// # Arguments
///
/// * `token` - The token to decode.
/// * `param_type` - The parameter type.
/// * `expected_value_path` - The path to the expected value.
///
/// # Returns
///
/// An optional token containing the decoded value.
pub fn decode_token(
    token: &Token,
    param_type: &ParamType,
    expected_value_path: &[usize],
) -> Result<Option<Token>, WorkerError> {
    if expected_value_path.is_empty() {
        return match (param_type, token) {
            (ParamType::Uint(_), Token::Uint(value)) => Ok(Some(Token::Uint(value.clone()))),
            (ParamType::Address, Token::Address(value)) => Ok(Some(Token::Address(value.clone()))),
            (ParamType::Bool, Token::Bool(value)) => Ok(Some(Token::Bool(value.clone()))),
            (ParamType::String, Token::String(value)) => Ok(Some(Token::String(value.clone()))),
            (ParamType::Bytes, Token::Bytes(value)) => Ok(Some(Token::Bytes(value.clone()))),
            (ParamType::Int(_), Token::Int(value)) => Ok(Some(Token::Int(value.clone()))),
            (ParamType::FixedBytes(_), Token::FixedBytes(value)) => {
                Ok(Some(Token::FixedBytes(value.clone())))
            }
            _ => Ok(None),
        };
    }

    let (index, rest) = expected_value_path
        .split_first()
        .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Token::Tuple(tokens) = token {
                let inner_type = inner_types
                    .get(*index)
                    .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(*index)))?;
                let inner_token = tokens
                    .get(*index)
                    .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                return decode_token(inner_token, inner_type, rest);
            }
        }
        ParamType::Array(inner_type) => {
            if let Token::Array(array_tokens) = token {
                let inner_token = array_tokens
                    .get(*index)
                    .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                return decode_token(inner_token, inner_type, rest);
            }
        }
        ParamType::FixedArray(inner_type, _) => {
            if let Token::FixedArray(array_tokens) = token {
                let inner_token = array_tokens
                    .get(*index)
                    .ok_or_else(|| WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                return decode_token(inner_token, inner_type, rest);
            }
        }
        _ => {}
    }
    Ok(None)
}

/// Parses and decodes a token based on the rule filter and expected value.
///
/// # Arguments
///
/// * `token` - The token to decode.
/// * `param_type` - The parameter type.
/// * `rule_filter` - The rule filter.
/// * `expected_value_index` - The expected value index.
/// * `expected_value` - The expected value.
/// * `comparator` - The comparator.
///
/// # Returns
///
/// A result containing an optional string with the decoded value.
pub fn parse_decode_token<'a>(
    token: &Token,
    param_type: &ParamType,
    rule_filter: &Vec<String>,
    rule_filter_comparator: &Vec<String>,
    expected_value_filter: &String,
    expected_value_filter_comparator: &String,
) -> Result<Option<String>, WorkerError> {
    let parsed_rule_filter = parse_rule_filter(rule_filter)?;
    let (parsed_expected_value_index_key, expected_value) =
        parse_expected_value_filter(expected_value_filter)?;

    // Rule Filter Decoding
    for ((parsed_rule_key, parsed_rule_value), comparator) in
        zip(parsed_rule_filter, rule_filter_comparator)
    {
        if let Some(value) = decode_token(&token, &param_type, &parsed_rule_key)? {
            if let None = compare_token(&value, parsed_rule_value, comparator.to_string())? {
                return Ok(None);
            }
        } else {
            return Err(WorkerError::InvalidTokenValue(token.clone()).into());
        }
    }
    // Expected Value Decoding
    if let Some(value) = decode_token(&token, &param_type, &parsed_expected_value_index_key)? {
        return compare_token(
            &value,
            expected_value.to_string(),
            expected_value_filter_comparator.to_string(),
        );
    } else {
        return Err(WorkerError::InvalidTokenValue(token.clone()).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::init;

    #[test]
    fn test_compare_token() -> Result<(), WorkerError> {
        init();

        let result = compare_token(
            &Token::FixedBytes([0, 1, 74, 52].to_vec()),
            "000014a34".to_string(),
            "==".to_string(),
        );

        println!("{:?}", result);

        Ok(())
    }
}
