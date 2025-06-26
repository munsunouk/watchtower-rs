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

use std::mem::replace;
use std::{collections::HashMap, sync::Arc};

use ethers::{
    abi::{Abi, Int, ParamType, Token},
    prelude::*,
    utils::hex,
};

use watch_tower_lib::{
    rule::TargetIndex,
    utils::{
        convert_hex_param, convert_hex_token, format_float_to_4_decimal, parse_string_to_float,
        types::GeneralToken,
    },
};

use crate::{option_or_err, parse::evaluation::ParseResultType, utils::error::WorkerError};

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
    providers: &[Arc<Provider<T>>],
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
    params: &[Option<Token>],
    param_type: &ParamType,
) -> Result<Token, WorkerError> {
    if params.is_empty() {
        return Ok(Token::Tuple(vec![])); // Default case for empty params
    }

    // Filter out None values and unwrap Some values
    let unwrapped_params: Vec<Token> = params
        .iter()
        .filter_map(|opt_token| opt_token.clone())
        .collect();

    if unwrapped_params.is_empty() {
        return Ok(Token::Tuple(vec![]));
    }

    match param_type {
        ParamType::String => Ok(unwrapped_params[0].to_owned()),
        ParamType::Address => Ok(unwrapped_params[0].to_owned()),
        ParamType::Bool => Ok(unwrapped_params[0].to_owned()),
        ParamType::Uint(_) => Ok(unwrapped_params[0].to_owned()),
        ParamType::Int(_) => Ok(unwrapped_params[0].to_owned()),
        ParamType::Bytes => Ok(unwrapped_params[0].to_owned()),
        ParamType::FixedBytes(_) => Ok(unwrapped_params[0].to_owned()),
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
        Token::String(value) => Ok(value.to_string()),
        _ => Err(WorkerError::InvalidTypeConvertError(format!(
            "parse_token_to_string Expected Token, got {token:?}",
        ))),
    }
}

fn convert_value_to_param_type(value: &Value) -> Result<ParamType, WorkerError> {
    match value {
        Value::Null => Ok(ParamType::Tuple(vec![])),
        Value::Bool(_) => Ok(ParamType::Bool),
        Value::String(s) => Ok(convert_hex_param(s)?),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let _ = Int::from(i);
                Ok(ParamType::Uint(256))
            } else if let Some(u) = n.as_u64() {
                let _ = U256::from(u);
                Ok(ParamType::Uint(256))
            } else {
                Ok(ParamType::String)
            }
        }
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
            let inner_types: Result<Vec<ParamType>, WorkerError> =
                obj.values().map(convert_value_to_param_type).collect();
            Ok(ParamType::Tuple(inner_types?))
        }
    }
}

fn convert_value_to_token(value: &Value) -> Result<Token, WorkerError> {
    match value {
        Value::Null => Ok(Token::Tuple(vec![])),
        Value::Bool(b) => Ok(Token::Bool(*b)),
        Value::String(s) => Ok(convert_hex_token(s)?),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let int_value = Int::from(i);
                Ok(Token::Int(int_value))
            } else if let Some(u) = n.as_u64() {
                let uint_value = U256::from(u);
                Ok(Token::Uint(uint_value))
            } else {
                Ok(Token::String(n.to_string()))
            }
        }
        Value::Array(arr) => {
            let tokens: Result<Vec<Token>, WorkerError> =
                arr.iter().map(convert_value_to_token).collect();
            Ok(Token::Tuple(tokens?))
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
            (ParamType::String, Token::String(value)) => Ok(Token::String(value.to_string())),
            (ParamType::Bytes, Token::Bytes(value)) => Ok(Token::Bytes(value.to_vec())),
            (ParamType::FixedBytes(_), Token::FixedBytes(value)) => {
                Ok(Token::FixedBytes(value.to_vec()))
            }
            (ParamType::Array(_), Token::Array(value))
            | (ParamType::Tuple(_), Token::Array(value)) => Ok(Token::Array(value.to_vec())),
            (ParamType::Tuple(_), Token::Tuple(value)) => Ok(Token::Tuple(value.to_vec())),
            _ => Err(WorkerError::InvalidTypeConvertError(format!(
                "decode_token expected_index Expected Token, got {token:?}",
            ))),
        };
    }

    let (index, rest) = option_or_err!(expected_index.split_first());

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Token::Tuple(tokens) | Token::Array(tokens) = token {
                let inner_type = option_or_err!(inner_types.get(*index));

                let inner_token = option_or_err!(tokens.get(*index));

                return decode_token(inner_token, inner_type, rest);
            } else {
                return Err(WorkerError::InvalidTypeConvertError(format!(
                    "param_type Expected Token, got {token:?}",
                )));
            }
        }
        ParamType::Array(inner_type) => {
            if let Token::Tuple(tokens) | Token::Array(tokens) = token {
                let inner_token = option_or_err!(tokens.get(*index));

                return decode_token(inner_token, inner_type, rest);
            }
        }
        ParamType::FixedArray(inner_type, _) => {
            if let Token::FixedArray(array_tokens) = token {
                let inner_token = option_or_err!(array_tokens.get(*index));

                return decode_token(inner_token, inner_type, rest);
            }
        }
        _ => {}
    }

    Err(WorkerError::InvalidTypeConvertError(format!(
        "decode_token Expected Token, got {token:?}",
    )))
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
fn check_target_index(param_type: &ParamType, target_index: &[usize]) -> Result<bool, WorkerError> {
    if target_index.is_empty() {
        // Base case: If the index is empty, it's valid at this level.
        return Ok(true);
    }

    // We checked is_empty, so split_first is safe.
    let (current_index, rest_index) = option_or_err!(target_index.split_first());

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Some(inner_type) = inner_types.get(*current_index) {
                // Recurse with the inner type and the rest of the index path.
                check_target_index(inner_type, rest_index)
            } else {
                Ok(false)
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
        _ if !rest_index.is_empty() => Ok(false),
        // If it's not a tuple or array, and we have reached this point, it means target_index
        // had exactly one element (`current_index`). This signifies an attempt to index
        // into a non-indexable (simple) type like Uint, Address, Bool, etc.
        _ => Ok(false),
    }
}

/// # Description
/// Converts TargetIndex vector to usize indices and foreach positions.
/// # Arguments
///
/// * `target_index` - The vector of TargetIndex to convert.
///
/// # Returns
///
/// A tuple containing (indices, foreach_positions) or an error.
fn convert_target_index_to_indices(
    target_index: &[TargetIndex],
    body: Option<&Value>,
    mut key_store: Option<&mut Vec<HashMap<String, Option<String>>>>,
) -> Result<(Vec<usize>, Vec<usize>), WorkerError> {
    let mut indices = Vec::new();
    let mut foreach_positions = Vec::new();

    // First pass: collect indices and foreach positions
    for (i, idx) in target_index.iter().enumerate() {
        match idx {
            TargetIndex::Index(n) => indices.push(*n),
            TargetIndex::ForEach => {
                indices.push(0);
                foreach_positions.push(i);
            }
            TargetIndex::Object(obj) => {
                let obj_clone = obj.clone();

                // Store obj into key_store vector
                if let Some(ref mut store) = key_store {
                    let converted_obj: HashMap<String, Option<String>> = obj_clone
                        .iter()
                        .map(|(k, v)| (k.clone(), Some(v.clone())))
                        .collect();
                    store.push(converted_obj);
                }

                // Now call find_target_index_by_object without key_store to avoid move
                let found_indices = find_target_index_by_object(option_or_err!(body), &obj_clone)?;
                let (found_indices, _) =
                    convert_target_index_to_indices(&found_indices, None, None)?;
                indices.extend(found_indices);
            }
        }
    }

    Ok((indices, foreach_positions))
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
    token: &mut Token,
    param_type: &mut ParamType,
    indices: &mut Vec<usize>,
    foreach_positions: &mut [usize],
) -> Result<Token, WorkerError> {
    // Ensure the token and param_type are wrapped correctly first.
    ensure_token_wrapper(token, param_type);

    // If no foreach, just try once
    if foreach_positions.is_empty() {
        if check_target_index(param_type, indices)? {
            return decode_token(token, param_type, indices);
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
            if check_target_index(wrapped_param_type, indices)? {
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
        indices,
        foreach_positions,
        0,
        max_tries,
        token,
        param_type,
        &mut decoded_tokens,
    )?;

    if decoded_tokens.is_empty() {
        return Err(WorkerError::InvalidIndexDepth);
    }

    Ok(Token::Tuple(decoded_tokens))
}

fn ensure_token_wrapper(token: &mut Token, param_type: &mut ParamType) {
    initial_ensure_token_wrapper(token, param_type);
    nested_ensure_token_wrapper(token, param_type);
}

fn initial_ensure_token_wrapper(token: &mut Token, param_type: &mut ParamType) {
    if let Token::Tuple(_) = token {
        // Early return for tuple tokens
    } else if let ParamType::Tuple(_) = param_type {
        let original_token = replace(token, Token::Bool(false));
        *token = Token::Tuple(vec![original_token]);
    } else {
        let original_token = replace(token, Token::Bool(false));
        let original_param_type = replace(param_type, ParamType::Bool);
        *token = Token::Tuple(vec![original_token]);
        *param_type = ParamType::Tuple(vec![original_param_type]);
    }
}

fn nested_ensure_token_wrapper(token: &mut Token, param_type: &mut ParamType) {
    let original_token = replace(token, Token::Bool(false));
    let original_param_type = replace(param_type, ParamType::Bool);

    let (processed_token, processed_param_type) = match (original_token, original_param_type) {
        (Token::Tuple(tokens), ParamType::Tuple(param_types))
        | (Token::Array(tokens), ParamType::Tuple(param_types)) => {
            if param_types
                .iter()
                .any(|pt| matches!(pt, ParamType::Tuple(_)))
                && !tokens.iter().any(|t| matches!(t, Token::Tuple(_)))
            {
                (
                    Token::Tuple(vec![Token::Tuple(tokens)]),
                    ParamType::Tuple(param_types),
                )
            } else if param_types
                .iter()
                .any(|pt| matches!(pt, ParamType::Array(_)))
                && !tokens.iter().any(|t| matches!(t, Token::Array(_)))
            {
                (
                    Token::Array(vec![Token::Array(tokens)]),
                    ParamType::Tuple(param_types),
                )
            } else {
                let mut wrapped_tokens = Vec::new();
                let mut wrapped_param_types = Vec::new();

                for (mut token, mut param_type) in tokens.into_iter().zip(param_types.into_iter()) {
                    nested_ensure_token_wrapper(&mut token, &mut param_type);
                    wrapped_tokens.push(token);
                    wrapped_param_types.push(param_type);
                }

                (
                    Token::Tuple(wrapped_tokens),
                    ParamType::Tuple(wrapped_param_types),
                )
            }
        }

        (token, ParamType::Tuple(param_types)) => {
            (Token::Tuple(vec![token]), ParamType::Tuple(param_types))
        }
        (token, param_type) => (token, param_type),
    };

    // Assign processed values back to mutable references
    *token = processed_token;
    *param_type = processed_param_type;
}

pub fn decode_meta_data(
    token: &GeneralToken,
    variables: &mut HashMap<String, ParseResultType>,
) -> Result<GeneralToken, WorkerError> {
    let meta_data = option_or_err!(variables.get("meta_data"));
    match meta_data {
        ParseResultType::String(meta_data) if meta_data == "VaultAddress" => {
            if let GeneralToken::Tuple(arr) = token {
                let sum = arr.iter().fold(U256::zero(), |acc, token| {
                    if let GeneralToken::Uint(value) = token {
                        acc + value
                    } else {
                        acc
                    }
                });
                Ok(GeneralToken::Uint(sum))
            } else {
                Err(WorkerError::InvalidTypeConvertError(format!(
                    "Expected Array, got {token:?}",
                )))
            }
        }

        ParseResultType::String(meta_data) if meta_data == "APY" => {
            if let GeneralToken::String(apy) = token {
                let mut apy_float = parse_string_to_float(apy)? * 100.0;
                apy_float = format_float_to_4_decimal(apy_float);

                Ok(GeneralToken::Float(apy_float))
            } else {
                Err(WorkerError::InvalidTypeConvertError(format!(
                    "Expected String, got {token:?}",
                )))
            }
        }

        ParseResultType::String(meta_data) if meta_data == "Float" => {
            if let GeneralToken::String(apy) = token {
                let mut apy_float = parse_string_to_float(apy)?;
                apy_float = format_float_to_4_decimal(apy_float);

                Ok(GeneralToken::Float(apy_float))
            } else {
                Err(WorkerError::InvalidTypeConvertError(format!(
                    "Expected String, got {token:?}",
                )))
            }
        }
        _ => Ok(token.to_owned()),
    }
}

/// # Description
/// This function finds the index of the object matching the TargetIndex::Object criteria.
/// # Arguments
///
/// * `body` - The JSON response body.
/// * `target_index` - The TargetIndex containing object criteria.
///
/// # Returns
///
/// A vector of TargetIndex with found indices.
pub fn find_target_index_by_object(
    body: &Value,
    target_object: &HashMap<String, String>,
) -> Result<Vec<TargetIndex>, WorkerError> {
    if let Value::Array(arr) = body {
        for (index, item) in arr.iter().enumerate() {
            if let Value::Object(item_obj) = item {
                for (key, value) in target_object {
                    if let Some(Value::String(s)) = item_obj.get(key) {
                        if s == value {
                            let first_key = option_or_err!(target_object.keys().next());

                            let key_field_index =
                                option_or_err!(item_obj.keys().position(|k| k == first_key));

                            return Ok(vec![
                                TargetIndex::Index(index),
                                TargetIndex::Index(key_field_index),
                            ]);
                        }
                    }
                }
            }
        }
    }
    Err(WorkerError::InvalidIndexDepth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::init;
    use watch_tower_lib::utils::{compare_token, types::GeneralToken};

    #[test]
    fn test_compare_token() -> Result<(), WorkerError> {
        init();

        let result = compare_token(
            &GeneralToken::FixedBytes([0, 1, 74, 52].to_vec()),
            &GeneralToken::FixedBytes([0, 1, 74, 52].to_vec()),
            "==",
        );

        println!("{result:?}");

        Ok(())
    }
}
