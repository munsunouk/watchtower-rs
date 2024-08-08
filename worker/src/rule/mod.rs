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
use std::str::FromStr;
use std::sync::Arc;

use ethers::{
    abi::{Abi, Int, ParamType, Token, Uint},
    prelude::*,
    utils::hex,
};

use anyhow::Result;

use crate::utils::constants::COMPARATOR_EQUAL;
use crate::utils::constants::INVALID_TOKEN_VALUE;
use crate::utils::constants::INVALID_TYPE_ABI;

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
    from_str(&input.to_string()).expect(INVALID_TYPE_ABI)
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
    input.try_into().unwrap()
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
    input.parse::<Address>().unwrap()
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
fn set_schedule(check_interval: usize) -> Schedule {
    Schedule::from_str(&format!("*/{} * * * * *", check_interval)).unwrap()
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
pub fn create_contract<T: JsonRpcClient>(
    address: &Address,
    abi: &Abi,
    provider: Arc<Provider<T>>,
) -> Contract<Provider<T>> {
    Contract::new(address.clone(), abi.clone(), provider)
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
pub fn parse_rule_filter(rule_filters: &Vec<String>) -> Vec<(Vec<usize>, String)> {
    rule_filters
        .iter()
        .map(|rule_filter| {
            let parts: Vec<&str> = rule_filter.split('-').collect();
            let indices: Vec<usize> = parts[0].split('.').map(|s| s.parse().unwrap()).collect();
            let value = parts[1].to_string();
            (indices, value)
        })
        .collect()
}

/// Parses the expected value index.
///
/// # Arguments
///
/// * `expected_value_index` - A string representing the expected value index.
///
/// # Returns
///
/// A vector of usize values.
pub fn parse_expected_value_index(expected_value_index: &String) -> Vec<usize> {
    expected_value_index
        .split('.')
        .map(|s| s.parse().unwrap())
        .collect()
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
pub fn encode_token(params: Vec<String>, param_type: &ParamType) -> Token {
    if params.is_empty() {
        return Token::Tuple(vec![]); // Default case for empty params
    }

    match param_type {
        ParamType::String => Token::String(params[0].clone()),
        ParamType::Address => Token::Address(params[0].parse::<Address>().unwrap()),
        ParamType::Bool => Token::Bool(params[0].parse::<bool>().unwrap()),
        ParamType::Uint(_) => Token::Uint(params[0].parse::<Uint>().unwrap()),
        ParamType::Int(_) => Token::Int(params[0].parse::<Int>().unwrap()),
        ParamType::Bytes => Token::Bytes(hex::decode(&params[0]).unwrap()),
        ParamType::FixedBytes(size) => {
            let bytes = hex::decode(&params[0]).unwrap();
            assert_eq!(bytes.len(), *size);
            Token::FixedBytes(bytes)
        }
        ParamType::Array(inner_type) => {
            let tokens: Vec<Token> = params
                .into_iter()
                .map(|p| encode_token(vec![p], inner_type))
                .collect();
            Token::Array(tokens)
        }
        ParamType::FixedArray(inner_type, size) => {
            let tokens: Vec<Token> = params
                .into_iter()
                .take(*size)
                .map(|p| encode_token(vec![p], inner_type))
                .collect();
            Token::FixedArray(tokens)
        }
        ParamType::Tuple(inner_types) => {
            let tokens: Vec<Token> = inner_types
                .iter()
                .zip(params.into_iter())
                .map(|(t, p)| encode_token(vec![p], t))
                .collect();
            Token::Tuple(tokens)
        }
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
        ">" if value > expected_value => Some(value.to_string()),
        ">=" if value >= expected_value => Some(value.to_string()),
        "<" if value < expected_value => Some(value.to_string()),
        "<=" if value <= expected_value => Some(value.to_string()),
        "==" if value == expected_value => Some(value.to_string()),
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
pub fn compare_token(token: &Token, expected_value: String, comparator: String) -> Option<String> {
    match token {
        Token::Uint(value) => {
            let expected_value = expected_value.parse::<Uint>().unwrap();
            parse_compare(value, &expected_value, &comparator)
        }
        Token::Int(value) => {
            let expected_value = expected_value.parse::<Int>().unwrap();
            parse_compare(value, &expected_value, &comparator)
        }
        Token::Bool(value) => {
            let expected_value = expected_value.parse::<bool>().unwrap();
            parse_compare(value, &expected_value, &comparator)
        }
        Token::String(value) => parse_compare(value, &expected_value, &comparator),
        Token::Bytes(value) | Token::FixedBytes(value) => {
            let parsing_value = hex::encode(&value);
            parse_compare(&parsing_value, &expected_value, &comparator)
        }
        _ => None,
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
) -> Option<Token> {
    if expected_value_path.is_empty() {
        return match (param_type, token) {
            (ParamType::Uint(_), Token::Uint(value)) => Some(Token::Uint(value.clone())),
            (ParamType::Address, Token::Address(value)) => Some(Token::Address(value.clone())),
            (ParamType::Bool, Token::Bool(value)) => Some(Token::Bool(value.clone())),
            (ParamType::String, Token::String(value)) => Some(Token::String(value.clone())),
            (ParamType::Bytes, Token::Bytes(value)) => Some(Token::Bytes(value.clone())),
            (ParamType::Int(_), Token::Int(value)) => Some(Token::Int(value.clone())),
            (ParamType::FixedBytes(_), Token::FixedBytes(value)) => {
                Some(Token::FixedBytes(value.clone()))
            }
            _ => None,
        };
    }

    let (index, rest) = expected_value_path.split_first().unwrap();

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Token::Tuple(tokens) = token {
                if let Some(inner_type) = inner_types.get(*index) {
                    if let Some(inner_token) = tokens.get(*index) {
                        return decode_token(inner_token, inner_type, rest);
                    }
                }
            }
        }
        ParamType::Array(inner_type) => {
            if let Token::Array(array_tokens) = token {
                if let Some(inner_token) = array_tokens.get(*index) {
                    return decode_token(inner_token, inner_type, rest);
                }
            }
        }
        ParamType::FixedArray(inner_type, _) => {
            if let Token::FixedArray(array_tokens) = token {
                if let Some(inner_token) = array_tokens.get(*index) {
                    return decode_token(inner_token, inner_type, rest);
                }
            }
        }
        _ => {}
    }
    None
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
    expected_value_index: &String,
    expected_value: &String,
    comparator: &String,
) -> Result<Option<String>> {
    let parsed_rule_filter = parse_rule_filter(rule_filter);
    let parsed_expected_value_index_key = parse_expected_value_index(expected_value_index);

    for (parsed_rule_key, parsed_rule_value) in parsed_rule_filter {
        if let Some(value) = decode_token(&token, &param_type, &parsed_rule_key) {
            let comparator = COMPARATOR_EQUAL;

            if let None = compare_token(&value, parsed_rule_value, comparator.to_string()) {
                return Ok(None);
            }
        } else {
            return Err(anyhow::anyhow!(INVALID_TOKEN_VALUE));
        }
    }

    if let Some(value) = decode_token(&token, &param_type, &parsed_expected_value_index_key) {
        return Ok(compare_token(
            &value,
            expected_value.to_string(),
            comparator.to_string(),
        ));
    } else {
        return Err(anyhow::anyhow!(INVALID_TOKEN_VALUE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::init;

    #[test]
    fn test_compare_token() -> anyhow::Result<()> {
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
