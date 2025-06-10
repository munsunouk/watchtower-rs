use crate::{
    cli::rpc::RpcClient,
    utils::{
        constants::{
            DB_API_BODY_TYPE_COLUMN, DB_CALL_TIME_INTERVAL_COLUMN, DB_CALL_TYPE_COLUMN,
            DB_ID_COLUMN, DB_METHOD_TYPE_COLUMN, DB_NAME_COLUMN, DB_URL_COLUMN, DB_VALUES_COLUMN,
        },
        parse_i32_to_usize, parse_json_to_value, parse_string_to_method,
        parse_string_to_rpc_call_type,
        types::RuleID,
        RpcCallType,
    },
};
use ethers::{
    abi::{ParamType, Token},
    types::{U256, U64},
};
use reqwest::Method;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};
use std::str::FromStr;

use crate::utils::error::GeneralError;

use super::{parse_string_to_index, parse_string_to_target_index, TargetIndex};

/// # Description
/// This struct represents a rule for RPC calls.
/// # Arguments
/// * `id` - The rule ID.
/// * `url` - The URL to call.
/// * `call_time_interval` - The call time interval.
/// * `values` - The values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcCallRule {
    pub url: String,
    pub url_token: Option<String>,
    pub call_type: RpcCallType,
    pub method_type: Method,
    pub api_body: Option<Value>,
    pub api_query: Option<Value>,
    pub target_index: Vec<TargetIndex>,
}

impl TryFrom<&PgRow> for RpcCallRule {
    type Error = GeneralError;
    /// # Description
    /// This function creates an `RpcCallRule` from a database row.
    /// # Arguments
    /// * `row` - A reference to a `PgRow`.
    /// # Returns
    ///
    /// A new instance of `RpcCallRule`.
    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        let call_type =
            parse_string_to_rpc_call_type(row.get(DB_CALL_TYPE_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?;
        let method_type = parse_string_to_method(row.get(DB_METHOD_TYPE_COLUMN));
        let api_body = parse_json_to_value(row.get(DB_API_BODY_TYPE_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
        })?;

        let api_query = parse_json_to_value(row.get(DB_API_BODY_TYPE_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
        })?;

        let target_index =
            parse_string_to_target_index(row.get(DB_VALUES_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?;

        Ok(RpcCallRule {
            url: row.get(DB_URL_COLUMN),
            url_token: None,
            call_type,
            method_type,
            api_body: Some(api_body),
            api_query: Some(api_query),
            target_index,
        })
    }
}

impl RpcCallRule {
    pub fn new(
        url: String,
        url_token: Option<String>,
        call_type: String,
        method_type: String,
        api_body: Option<Value>,
        api_query: Option<Value>,
        target_index: String,
    ) -> Result<Self, GeneralError> {
        let call_type = parse_string_to_rpc_call_type(call_type).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
        })?;

        let method_type = parse_string_to_method(method_type);

        let target_index = parse_string_to_target_index(target_index).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
        })?;

        Ok(Self {
            url,
            url_token,
            call_type,
            method_type,
            api_body,
            api_query,
            target_index,
        })
    }
}
