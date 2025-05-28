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

use serde_json::Value;

use std::{collections::HashMap, sync::Arc};

use ethers::{
    abi::{Abi, Int, ParamType, Token},
    prelude::*,
    utils::hex,
};

use watch_tower_lib::{
    rule::TargetIndex,
    utils::{
        constants::DEFAULT_INDEX,
        convert_hex_param, convert_hex_token,
        error::{GeneralError, IndexType},
    },
};

use crate::{
    parse::evaluation::ParseResultType,
    utils::{constants::FILTER_INDEX_SPLIT_CHAR, error::WorkerError},
};

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
/// * `params` - A vector of tokens.
/// * `param_type` - The parameter type.
///
/// # Returns
///
/// A `Token` instance.
pub fn encode_token(
    params: Vec<Option<Token>>,
    param_type: &ParamType,
) -> Result<Token, WorkerError> {
    if params.is_empty() {
        return Ok(Token::Tuple(vec![])); // Default case for empty params
    }

    // Filter out None values and unwrap Some values
    let unwrapped_params: Vec<Token> = params
        .into_iter()
        .filter_map(|opt_token| opt_token)
        .collect();

    if unwrapped_params.is_empty() {
        return Ok(Token::Tuple(vec![]));
    }

    match param_type {
        ParamType::String => Ok(unwrapped_params[0].clone()),
        ParamType::Address => Ok(unwrapped_params[0].clone()),
        ParamType::Bool => Ok(unwrapped_params[0].clone()),
        ParamType::Uint(_) => Ok(unwrapped_params[0].clone()),
        ParamType::Int(_) => Ok(unwrapped_params[0].clone()),
        ParamType::Bytes => Ok(unwrapped_params[0].clone()),
        ParamType::FixedBytes(_) => Ok(unwrapped_params[0].clone()),
        ParamType::Array(_) => Ok(Token::Array(unwrapped_params)),
        ParamType::FixedArray(_, size) => Ok(Token::FixedArray(
            unwrapped_params.into_iter().take(*size).collect(),
        )),
        ParamType::Tuple(_) => Ok(Token::Tuple(unwrapped_params)),
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
pub fn _parse_token_to_string(token: &Token) -> Result<String, WorkerError> {
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
            // Handle JSON-RPC response by extracting the result field
            if obj.contains_key("result") {
                if let Some(result) = obj.get("result") {
                    return convert_value_to_param_type(result);
                }
            }
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
            // Handle JSON-RPC response by extracting the result field
            if obj.contains_key("result") {
                if let Some(result) = obj.get("result") {
                    return convert_value_to_token(result);
                }
            }
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
            (ParamType::Array(_), Token::Array(value)) => Ok(Token::Array(value.clone())),
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
            } else {
                println!("inner_types : {:?}", inner_types);
                println!("token : {:?}", token);
                return Err(WorkerError::InvalidTypeConvert);
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
/// `true` if the index is valid, `false` otherwise.
fn check_target_index(param_type: &ParamType, target_index: &[usize]) -> bool {
    if target_index.is_empty() {
        // Base case: If the index is empty, it's valid at this level.
        return true;
    }

    // We checked is_empty, so split_first is safe.
    let (current_index, rest_index) = target_index.split_first().unwrap();

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Some(inner_type) = inner_types.get(*current_index) {
                // Recurse with the inner type and the rest of the index path.
                check_target_index(inner_type, rest_index)
            } else {
                false
            }
        }
        ParamType::Array(inner_type) | ParamType::FixedArray(inner_type, _) => {
            // For arrays/fixed arrays, we need to check the inner type against the rest of the index path.
            // The validity of the current_index itself depends on the runtime array content,
            // which decode_token handles. Here, we only validate the depth and structure.
            check_target_index(inner_type, rest_index)
        }
        // If it's not a tuple or array, but the index path is not empty (`rest_index` is not empty),
        // it means the index goes deeper than the type structure allows.
        _ if !rest_index.is_empty() => false,
        // If it's not a tuple or array, and we have reached this point, it means target_index
        // had exactly one element (`current_index`). This signifies an attempt to index
        // into a non-indexable (simple) type like Uint, Address, Bool, etc.
        _ => false,
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
    target_index: &Vec<TargetIndex>,
) -> Result<Token, WorkerError> {
    // Ensure the token and param_type are wrapped correctly first.
    let (wrapped_token, wrapped_param_type) =
        ensure_token_wrapper(token.clone(), param_type.clone());

    // Convert TargetIndex to usize indices, handling ForEach
    let mut indices = Vec::new();
    let mut foreach_positions = Vec::new();

    // First pass: collect indices and foreach positions
    for (i, idx) in target_index.iter().enumerate() {
        match idx {
            TargetIndex::Index(n) => indices.push(*n),
            TargetIndex::ForEach => {
                indices.push(0); // Start with 0
                foreach_positions.push(i);
            }
        }
    }

    // If no foreach, just try once
    if foreach_positions.is_empty() {
        if check_target_index(&wrapped_param_type, &indices) {
            return decode_token(&wrapped_token, &wrapped_param_type, &indices);
        }
        return Err(WorkerError::InvalidIndexDepth);
    }

    let mut decoded_tokens = Vec::new();
    let max_tries = 1000; // Limit to prevent infinite loops

    // Function to try all combinations recursively
    fn try_combinations(
        indices: &mut Vec<usize>,
        foreach_positions: &[usize],
        current_pos: usize,
        max_tries: usize,
        wrapped_token: &Token,
        wrapped_param_type: &ParamType,
        decoded_tokens: &mut Vec<Token>,
    ) -> Result<(), WorkerError> {
        if current_pos >= foreach_positions.len() {
            if check_target_index(wrapped_param_type, indices) {
                decoded_tokens.push(decode_token(wrapped_token, wrapped_param_type, indices)?);
            }
            return Ok(());
        }

        let pos = foreach_positions[current_pos];
        for try_num in 0..max_tries {
            indices[pos] = try_num;
            try_combinations(
                indices,
                foreach_positions,
                current_pos + 1,
                max_tries,
                wrapped_token,
                wrapped_param_type,
                decoded_tokens,
            )?;
        }
        Ok(())
    }

    // Start recursive combination trying
    try_combinations(
        &mut indices,
        &foreach_positions,
        0,
        max_tries,
        &wrapped_token,
        &wrapped_param_type,
        &mut decoded_tokens,
    )?;

    if decoded_tokens.is_empty() {
        return Err(WorkerError::InvalidIndexDepth);
    }

    Ok(Token::Array(decoded_tokens))
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

fn ensure_token_wrapper(token: Token, param_type: ParamType) -> (Token, ParamType) {
    let (token, param_type) = initial_ensure_token_wrapper(token, param_type);
    let (token, param_type) = nested_ensure_token_wrapper(token, param_type);
    (token, param_type)
}

fn initial_ensure_token_wrapper(token: Token, param_type: ParamType) -> (Token, ParamType) {
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
/// # Description
/// This function ensures a token wrapper.
/// # Arguments
///
/// * `token` - The token to ensure.
///
/// # Returns
///
/// A `Token` instance.
fn nested_ensure_token_wrapper(token: Token, param_type: ParamType) -> (Token, ParamType) {
    match (token.clone(), param_type.clone()) {
        // If both are tuples, recursively check their inner types
        (Token::Tuple(tokens), ParamType::Tuple(param_types)) => {
            // If param_type has more nesting, but token is not, wrap the entire token
            if param_types
                .iter()
                .any(|pt| matches!(pt, ParamType::Tuple(_)))
                && !tokens.iter().any(|t| matches!(t, Token::Tuple(_)))
            {
                (Token::Tuple(vec![token]), param_type)
            } else {
                let mut wrapped_tokens = Vec::new();
                for (token, param_type) in tokens.into_iter().zip(param_types.into_iter()) {
                    let (wrapped_token, _) = nested_ensure_token_wrapper(token, param_type);
                    wrapped_tokens.push(wrapped_token);
                }
                (Token::Tuple(wrapped_tokens), param_type)
            }
        }

        // If param_type is tuple but token is not, wrap token
        (token, ParamType::Tuple(param_types)) => {
            (Token::Tuple(vec![token]), ParamType::Tuple(param_types))
        }
        _ => (token, param_type),
    }
}

pub fn decode_meta_data(
    token: &Token,
    variables: &mut HashMap<String, ParseResultType>,
) -> Result<Token, GeneralError> {
    let meta_data = variables.get("meta_data").unwrap();
    match meta_data {
        ParseResultType::String(meta_data) if meta_data == "VaultAddress" => {
            if let Token::Array(arr) = token {
                let sum = arr.iter().fold(U256::zero(), |acc, token| {
                    if let Token::Uint(value) = token {
                        acc + value
                    } else {
                        acc
                    }
                });
                Ok(Token::Uint(sum))
            } else {
                Err(GeneralError::InvalidTypeConvert)
            }
        }
        ParseResultType::String(meta_data) if meta_data == "any" => {
            if let Token::Array(arr) = token {
                let any = arr.iter().any(|token| {
                    if let Token::Uint(value) = token {
                        !value.is_zero()
                    } else {
                        false
                    }
                });
                Ok(Token::Bool(any))
            } else {
                Err(GeneralError::InvalidTypeConvert)
            }
        }
        _ => Ok(token.clone()),
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
