/// Module for contract call rules and related functionality.
pub mod contract_call;
/// Module for contract event rules and related functionality.
pub mod contract_event;

/// Module for RPC call rules and related functionality.
pub mod rpc_call;

use std::{collections::HashMap, str::FromStr};

use ethers::{abi::Uint, utils::hex};
use num_bigint::BigInt;

use crate::utils::{constants::FILTER_INDEX_SPLIT_CHAR, error::GeneralError, types::GeneralToken};

/// # Description
/// This function parses a token to a string.
/// # Arguments
///
/// * `token` - The token to parse.
///
/// # Returns
///
pub fn parse_token_to_string(token: &GeneralToken) -> Result<String, GeneralError> {
    match token {
        GeneralToken::Uint(value) => Ok(value.to_string()),
        GeneralToken::Int(value) => Ok(value.to_string()),
        GeneralToken::Address(value) => Ok(format!("{:#x}", value)),
        GeneralToken::Bool(value) => Ok(value.to_string()),
        GeneralToken::Bytes(value) => Ok(hex::encode(value)),
        GeneralToken::FixedBytes(value) => Ok(hex::encode(value)),
        GeneralToken::String(value) => Ok(value.as_str().to_string()),
        GeneralToken::Float(value) => Ok(value.to_string()),
        _ => Err(GeneralError::InvalidTypeConvertError(format!(
            "{:?}",
            token
        ))),
    }
}

/// # Description
/// This function parses values into a vector of indices.
/// # Arguments
///
/// * `values` - A vector of strings.
///
/// # Returns
///
/// Returns a vector of indices.
pub fn parse_string_to_index(value: String) -> Result<Vec<usize>, GeneralError> {
    value
        .split(FILTER_INDEX_SPLIT_CHAR)
        .map(|s| s.parse().map_err(Into::into))
        .collect::<Result<Vec<usize>, GeneralError>>()
}

pub fn parse_int_to_uint(int: &BigInt) -> Result<Uint, GeneralError> {
    int.to_string().parse::<Uint>().map_err(Into::into)
}

pub fn parse_string_to_target_index(value: String) -> Result<Vec<TargetIndex>, GeneralError> {
    value
        .split(FILTER_INDEX_SPLIT_CHAR)
        .map(|s| s.parse().map_err(Into::into))
        .collect::<Result<Vec<TargetIndex>, GeneralError>>()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetIndex {
    Index(usize),
    ForEach,
    Object(HashMap<String, String>),
}

impl FromStr for TargetIndex {
    type Err = GeneralError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "~" {
            Ok(TargetIndex::ForEach)
        } else if s.starts_with('{') && s.ends_with('}') {
            // Parse object format: {key: value} or {key: 'value'}
            let content = &s[1..s.len() - 1]; // Remove { and }
            let mut map = HashMap::new();

            for pair in content.split(',') {
                let parts: Vec<&str> = pair.split(':').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().trim_matches('"');
                    let value = parts[1].trim().trim_matches('"');
                    map.insert(key.to_string(), value.to_string());
                }
            }

            Ok(TargetIndex::Object(map))
        } else {
            s.parse::<usize>()
                .map(TargetIndex::Index)
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string_to_target_index() {
        // Test Index
        let input = "1";
        let result = parse_string_to_target_index(input.to_string()).unwrap();
        assert_eq!(result, vec![TargetIndex::Index(1)]);

        // Test ForEach
        let input = "~";
        let result = parse_string_to_target_index(input.to_string()).unwrap();
        assert_eq!(result, vec![TargetIndex::ForEach]);

        // Test Object with clean format (no backslashes)
        let input = "{proxyAddress: 0x2665701293fCbEB223D11A08D826563EDcCE423A}";
        let result = parse_string_to_target_index(input.to_string()).unwrap();
        let mut expected_map = HashMap::new();
        expected_map.insert(
            "proxyAddress".to_string(),
            "0x2665701293fCbEB223D11A08D826563EDcCE423A".to_string(),
        );
        assert_eq!(result, vec![TargetIndex::Object(expected_map)]);

        // Test mixed: Index + Object
        let input = "1.{proxyAddress: 0x2665701293fCbEB223D11A08D826563EDcCE423A}";
        let result = parse_string_to_target_index(input.to_string()).unwrap();
        let mut expected_map = HashMap::new();
        expected_map.insert(
            "proxyAddress".to_string(),
            "0x2665701293fCbEB223D11A08D826563EDcCE423A".to_string(),
        );
        assert_eq!(
            result,
            vec![TargetIndex::Index(1), TargetIndex::Object(expected_map)]
        );

        // Test multiple indices
        let input = "1.2.3";
        let result = parse_string_to_target_index(input.to_string()).unwrap();
        assert_eq!(
            result,
            vec![
                TargetIndex::Index(1),
                TargetIndex::Index(2),
                TargetIndex::Index(3)
            ]
        );

        // Test multiple object properties
        let input = "{proxyAddress: 0x2665701293fCbEB223D11A08D826563EDcCE423A, path: usdc-usd}";
        let result = parse_string_to_target_index(input.to_string()).unwrap();
        let mut expected_map = HashMap::new();
        expected_map.insert(
            "proxyAddress".to_string(),
            "0x2665701293fCbEB223D11A08D826563EDcCE423A".to_string(),
        );
        expected_map.insert("path".to_string(), "usdc-usd".to_string());
        assert_eq!(result, vec![TargetIndex::Object(expected_map)]);
    }
}
