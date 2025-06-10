/// Module for contract call rules and related functionality.
pub mod contract_call;
/// Module for contract event rules and related functionality.
pub mod contract_event;

/// Module for RPC call rules and related functionality.
pub mod rpc_call;

use contract_call::ContractCallRule;
use contract_event::ContractEventRule;
use rpc_call::RpcCallRule;
use serde_json::Value;

use std::{str::FromStr, sync::Arc};

use ethers::{
    abi::{Abi, Int, ParamType, Token, Uint},
    prelude::*,
    utils::hex,
};

use crate::{
    cli::db::postgres::PostgresClient,
    utils::{
        constants::{DEFAULT_INDEX, FILTER_INDEX_SPLIT_CHAR},
        convert_hex_param, convert_hex_token,
        error::{GeneralError, IndexType},
        types::GeneralToken,
        DbRuleType,
    },
};

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
        GeneralToken::Address(value) => Ok(value.to_string()),
        GeneralToken::Bool(value) => Ok(value.to_string()),
        GeneralToken::Bytes(value) => Ok(hex::encode(value)),
        GeneralToken::FixedBytes(value) => Ok(hex::encode(value)),
        GeneralToken::String(value) => Ok(value.clone()),
        _ => Err(GeneralError::InvalidTypeConvert),
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
        .map(|s| {
            s.parse()
                .map_err(|_| GeneralError::InvalidTypeConvertError(s.to_string()))
        })
        .collect::<Result<Vec<usize>, GeneralError>>()
}

pub fn parse_string_to_target_index(value: String) -> Result<Vec<TargetIndex>, GeneralError> {
    value
        .split(FILTER_INDEX_SPLIT_CHAR)
        .map(|s| {
            s.parse()
                .map_err(|_| GeneralError::InvalidTypeConvertError(s.to_string()))
        })
        .collect::<Result<Vec<TargetIndex>, GeneralError>>()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetIndex {
    Index(usize),
    ForEach,
}

impl FromStr for TargetIndex {
    type Err = GeneralError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "~" {
            Ok(TargetIndex::ForEach)
        } else {
            s.parse::<usize>()
                .map(TargetIndex::Index)
                .map_err(|_| GeneralError::InvalidTypeConvertError(s.to_string()))
        }
    }
}
