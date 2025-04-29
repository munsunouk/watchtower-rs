/// Module for contract call rules and related functionality.
pub mod contract_call;
/// Module for contract event rules and related functionality.
pub mod contract_event;
// pub mod db;
pub mod get;
/// Module for RPC call rules and related functionality.
pub mod rpc_call;
pub mod store;

pub use contract_call::ContractCall;
pub use contract_event::ContractEvent;
pub use rpc_call::RpcCall;

use cron::Schedule;
use serde_json::Value;

use std::{str::FromStr, sync::Arc};

use ethers::{
    abi::{Abi, Int, ParamType, Token, Uint},
    prelude::*,
    utils::hex,
};

use watch_tower_lib::{
    cli::db::postgres::PostgresClient,
    utils::{
        constants::DEFAULT_INDEX,
        convert_hex_param,
        convert_hex_token,
        error::IndexType,
        DbRuleType,
        // evaluation::EvaluationRule,
    },
};

use crate::utils::{
    constants::{DEFAULT_PARAM_VALUE, FILTER_INDEX_SPLIT_CHAR},
    error::WorkerError,
};

/// # Description
/// This function sets a cron schedule based on the check interval.
/// # Arguments
///
/// * `check_interval` - The interval in seconds.
///
/// # Returns
///
/// A `Schedule` instance.
pub fn set_schedule(check_interval: usize) -> Result<Schedule, WorkerError> {
    let format_schedule = format!("*/{} * * * * *", check_interval);

    Schedule::from_str(&format_schedule)
        .map_err(|_| WorkerError::InvalidTypeConvertError(format_schedule))
}

/// # Description
/// This function creates a new Ethereum contract instance.
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
        .map(|provider| Contract::new(*address, abi.clone(), provider.clone()))
        .collect::<Vec<_>>()
}

/// # Description
/// This function encodes parameters into a token.
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
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                    DEFAULT_PARAM_VALUE,
                )))?
                .clone(),
        )),
        ParamType::Address => Ok(Token::Address(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                    DEFAULT_PARAM_VALUE,
                )))?
                .parse::<Address>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Bool => Ok(Token::Bool(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                    DEFAULT_PARAM_VALUE,
                )))?
                .parse::<bool>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Uint(_) => Ok(Token::Uint(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                    DEFAULT_PARAM_VALUE,
                )))?
                .parse::<Uint>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Int(_) => Ok(Token::Int(
            params
                .get(DEFAULT_PARAM_VALUE)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                    DEFAULT_PARAM_VALUE,
                )))?
                .parse::<Int>()
                .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::Bytes => Ok(Token::Bytes(
            hex::decode(
                params
                    .get(DEFAULT_PARAM_VALUE)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                        DEFAULT_PARAM_VALUE,
                    )))?,
            )
            .map_err(|_| WorkerError::InvalidTypeConvert)?,
        )),
        ParamType::FixedBytes(_) => Ok(Token::FixedBytes(
            hex::decode(
                params
                    .get(DEFAULT_PARAM_VALUE)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                        DEFAULT_PARAM_VALUE,
                    )))?,
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

/// # Description
/// This function parses a token to a string.
/// # Arguments
///
/// * `token` - The token to parse.
///
/// # Returns
///
pub fn parse_token_to_string(token: &Token) -> Result<String, WorkerError> {
    match token {
        Token::Uint(value) => Ok(value.to_string()),
        Token::Int(value) => Ok(value.to_string()),
        Token::Address(value) => Ok(value.to_string()),
        Token::Bool(value) => Ok(value.to_string()),
        Token::Bytes(value) => Ok(hex::encode(value)),
        Token::FixedBytes(value) => Ok(hex::encode(value)),
        Token::String(value) => Ok(value.clone()),
        _ => Err(WorkerError::InvalidTypeConvert),
    }
}

fn convert_value_to_param_type(value: &Value) -> Result<ParamType, WorkerError> {
    match value {
        Value::String(s) => Ok(convert_hex_param(s).map_err(|_| WorkerError::InvalidTypeConvert)?),
        Value::Number(n) => {
            if n.is_i64() {
                if let Some(i) = n.as_i64() {
                    if i >= 0 {
                        U256::try_from(i).map_err(|_| WorkerError::InvalidTypeConvert)?;
                        return Ok(ParamType::Uint(256));
                    }
                    // Default to Int256 if unsigned conversion is not safe
                    return Ok(ParamType::Int(256));
                }
            }
            Err(WorkerError::InvalidTypeConvertError(n.to_string()))
        }
        Value::Bool(_) => Ok(ParamType::Bool),
        Value::Array(arr) => {
            let inner_types: Result<Vec<ParamType>, WorkerError> =
                arr.iter().map(convert_value_to_param_type).collect();
            Ok(ParamType::Tuple(inner_types?))
        }
        Value::Object(obj) => {
            let inner_types: Result<Vec<ParamType>, WorkerError> = obj
                .iter()
                .map(|(_, v)| convert_value_to_param_type(v))
                .collect();
            Ok(ParamType::Tuple(inner_types?))
        }
        _ => Err(WorkerError::InvalidTypeConvertError(value.to_string())),
    }
}

fn convert_value_to_token(value: &Value) -> Result<Token, WorkerError> {
    match value {
        Value::Null => Err(WorkerError::InvalidTypeConvert),
        Value::Bool(b) => Ok(Token::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                // Check if conversion is safe
                let int_value = Int::try_from(i).map_err(|_| WorkerError::InvalidTypeConvert)?;
                Ok(Token::Int(int_value))
            } else if let Some(u) = n.as_u64() {
                let uint_value = U256::try_from(u).map_err(|_| WorkerError::InvalidTypeConvert)?;
                Ok(Token::Uint(uint_value))
            } else {
                Err(WorkerError::InvalidTypeConvertError(n.to_string()))
            }
        }
        Value::String(s) => convert_hex_token(s).map_err(|_| WorkerError::InvalidTypeConvert),
        Value::Array(arr) => {
            let tokens: Result<Vec<Token>, WorkerError> =
                arr.iter().map(convert_value_to_token).collect();
            Ok(Token::Array(tokens?))
        }
        Value::Object(obj) => {
            // Handle objects as needed, for example, as a tuple or a struct
            let tokens: Result<Vec<Token>, WorkerError> =
                obj.values().map(convert_value_to_token).collect();
            Ok(Token::Tuple(tokens?))
        }
    }
}

/// # Description
/// This function decodes a token based on the expected value path.
/// # Arguments
///
/// * `token` - The token to decode.
/// * `param_type` - The parameter type.
/// * `expected_index` - The path to the expected value.
///
/// # Returns
///
/// An optional token containing the decoded value.
pub fn decode_token(
    token: &Token,
    param_type: &ParamType,
    expected_index: &[usize],
) -> Result<Token, WorkerError> {
    if expected_index.is_empty() {
        return match (param_type, token) {
            (ParamType::Uint(_), Token::Uint(value)) | (ParamType::Uint(_), Token::Int(value)) => {
                Ok(Token::Uint(*value))
            }
            (ParamType::Int(_), Token::Int(value)) | (ParamType::Int(_), Token::Uint(value)) => {
                Ok(Token::Int(*value))
            }
            (ParamType::Address, Token::Address(value)) => Ok(Token::Address(*value)),
            (ParamType::Bool, Token::Bool(value)) => Ok(Token::Bool(*value)),
            (ParamType::String, Token::String(value)) => Ok(Token::String(value.clone())),
            (ParamType::Bytes, Token::Bytes(value)) => Ok(Token::Bytes(value.clone())),
            (ParamType::FixedBytes(_), Token::FixedBytes(value)) => {
                Ok(Token::FixedBytes(value.clone()))
            }
            _ => Err(WorkerError::InvalidTypeConvert),
        };
    }

    let (index, rest) = expected_index
        .split_first()
        .ok_or(WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Token::Tuple(tokens) | Token::Array(tokens) = token {
                let inner_type = inner_types
                    .get(*index)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                let inner_token = tokens
                    .get(*index)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                return decode_token(inner_token, inner_type, rest);
            }
        }
        ParamType::Array(inner_type) => {
            if let Token::Array(array_tokens) = token {
                let inner_token = array_tokens
                    .get(*index)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                return decode_token(inner_token, inner_type, rest);
            }
        }
        ParamType::FixedArray(inner_type, _) => {
            if let Token::FixedArray(array_tokens) = token {
                let inner_token = array_tokens
                    .get(*index)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(*index)))?;

                return decode_token(inner_token, inner_type, rest);
            }
        }
        _ => {}
    }

    Err(WorkerError::InvalidTypeConvert)
}

/// # Description
/// Checks if the target index path is valid for the given parameter type structure.
/// # Arguments
///
/// * `param_type` - The parameter type structure to validate against.
/// * `target_index` - The index path to validate.
///
/// # Returns
///
/// `Ok(())` if the index is valid, `Err(WorkerError)` otherwise.
fn check_target_index(param_type: &ParamType, target_index: &[usize]) -> Result<(), WorkerError> {
    if target_index.is_empty() {
        // Base case: If the index is empty, it's valid at this level.
        return Ok(());
    }

    // We checked is_empty, so split_first is safe.
    let (current_index, rest_index) = target_index.split_first().unwrap();

    match param_type {
        ParamType::Tuple(inner_types) => {
            let inner_type = inner_types
                .get(*current_index)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(*current_index)))?;
            // Recurse with the inner type and the rest of the index path.
            check_target_index(inner_type, rest_index)
        }
        ParamType::Array(inner_type) | ParamType::FixedArray(inner_type, _) => {
            // For arrays/fixed arrays, we need to check the inner type against the rest of the index path.
            // The validity of the current_index itself depends on the runtime array content,
            // which decode_token handles. Here, we only validate the depth and structure.
            check_target_index(inner_type, rest_index)
        }
        // If it's not a tuple or array, but the index path is not empty (`rest_index` is not empty),
        // it means the index goes deeper than the type structure allows.
        _ if !rest_index.is_empty() => Err(WorkerError::InvalidIndexDepth),
        // If it's not a tuple or array, and we have reached this point, it means target_index
        // had exactly one element (`current_index`). This signifies an attempt to index
        // into a non-indexable (simple) type like Uint, Address, Bool, etc.
        _ => Err(WorkerError::InvalidIndexAccessOnNonCompositeType),
    }
}

/// # Description
/// This function decodes a token based on the expected value path.
/// # Arguments
///
/// * `token` - The token to decode.
/// * `param_type` - The parameter type.
/// * `values` - The expected value path.
///
/// # Returns
pub fn decodes_token(
    token: &Token,
    param_type: &ParamType,
    target_index: &Vec<usize>,
) -> Result<Token, WorkerError> {
    // Ensure the token and param_type are wrapped correctly first.
    let (wrapped_token, wrapped_param_type) =
        ensure_token_wrapper(token.clone(), param_type.clone());

    // Validate the target index against the (potentially wrapped) parameter type structure.
    check_target_index(&wrapped_param_type, target_index)?;

    // Proceed with decoding using the wrapped types and validated index.
    decode_token(&wrapped_token, &wrapped_param_type, target_index)
}

/// # Description
/// This function parses values into a vector of indices.
/// # Arguments
///
/// * `values` - A vector of strings.
///
/// # Returns
///
/// Returns a vector of vectors of indices.
pub fn parse_string_to_values(values: Vec<String>) -> Result<Vec<Vec<usize>>, WorkerError> {
    values
        .iter()
        .map(|rule_filter| {
            let indices = rule_filter
                .split(FILTER_INDEX_SPLIT_CHAR)
                .map(|s| {
                    s.parse()
                        .map_err(|_| WorkerError::InvalidTypeConvertError(s.to_string()))
                })
                .collect::<Result<Vec<usize>, WorkerError>>()?;
            Ok(indices)
        })
        .collect()
}

/// # Description
/// This function ensures a token wrapper.
/// # Arguments
///
/// * `token` - The token to ensure.
///
/// # Returns
///
/// A `Token` instance.
fn ensure_token_wrapper(token: Token, param_type: ParamType) -> (Token, ParamType) {
    if let Token::Tuple(_) = token {
        (token, param_type)
    } else if let ParamType::Tuple(_) = param_type {
        (Token::Tuple(vec![token]), param_type)
    } else {
        (
            Token::Tuple(vec![token]),
            ParamType::Tuple(vec![param_type]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::init;
    use watch_tower_lib::utils::compare_token;

    #[test]
    fn test_compare_token() -> Result<(), WorkerError> {
        init();

        let result = compare_token(
            &Token::FixedBytes([0, 1, 74, 52].to_vec()),
            &Token::FixedBytes([0, 1, 74, 52].to_vec()),
            "==",
        );

        println!("{:?}", result);

        Ok(())
    }

    // #[test]
    // fn test_decodes_token() -> Result<(), WorkerError> {
    //     let test_token = Token::Tuple(vec![
    //         Token::Uint(U256::from(200)),
    //         Token::Tuple(vec![
    //             Token::Tuple(vec![
    //                 Token::Int(U256::from(10900000)),
    //                 Token::Int(U256::from(2)),
    //                 Token::Int(U256::from(10900000)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Int(U256::from(0)),
    //                 Token::Int(U256::from(0)),
    //                 Token::Int(U256::from(0)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Int(U256::from(0)),
    //                 Token::Int(U256::from(2)),
    //                 Token::Int(U256::from(20000)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Uint(U256::from(500000000)),
    //                 Token::Uint(U256::from(3)),
    //                 Token::Uint(U256::from(500000000)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Uint(U256::from(10000000)),
    //                 Token::Uint(U256::from(1)),
    //                 Token::Uint(U256::from(10000000)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Int(U256::from(0)),
    //                 Token::Int(U256::from(0)),
    //                 Token::Int(U256::from(0)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Int(U256::from(21000)),
    //                 Token::Int(U256::from(2)),
    //                 Token::Int(U256::from(21000)),
    //             ]),
    //             Token::Tuple(vec![
    //                 Token::Tuple(vec![
    //                     Token::Int(U256::from(0)),
    //                     Token::Int(U256::from(0)),
    //                     Token::Int(U256::from(0)),
    //                 ]),
    //                 Token::Tuple(vec![
    //                     Token::Int(U256::from(0)),
    //                     Token::Int(U256::from(0)),
    //                     Token::Int(U256::from(0)),
    //                 ]),
    //                 Token::Tuple(vec![
    //                     Token::Uint(U256::from(709000000)),
    //                     Token::Uint(U256::from(3)),
    //                     Token::Uint(U256::from(709000000)),
    //                 ]),
    //                 Token::Tuple(vec![
    //                     Token::Int(U256::from(0)),
    //                     Token::Int(U256::from(0)),
    //                     Token::Int(U256::from(0)),
    //                 ]),
    //             ]),
    //         ]),
    //     ]);

    //     let decoded_tokens = decodes_token(&test_token, &ParamType::Tuple(vec![]), &vec![vec![0]])?;
    //     println!("{:?}", decoded_tokens);
    //     Ok(())
    // }
}
