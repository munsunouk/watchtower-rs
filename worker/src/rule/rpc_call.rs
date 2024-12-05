use std::sync::Arc;

use super::parse_i32_to_usize;
use ethers::types::U64;
use reqwest::{Client, StatusCode};
use sqlx::{postgres::PgRow, Row};
use watch_tower_lib::{utils::error::ClientError, utils::types::RuleID};

use crate::utils::constants::{
    DB_CALL_TIME_INTERVAL_COLUMN, DB_COMPARATOR_COLUMN, DB_EXPECTED_VALUE_COLUMN, DB_ID_COLUMN,
    DB_URL_COLUMN,
};
use serde::{Deserialize, Serialize};

/// Represents a JSON-RPC request.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<String>,
}

/// Represents a rule for RPC calls.
#[derive(Debug, Clone)]
pub struct RpcCallRule {
    pub id: RuleID,
    pub url: String,
    pub expected_value: String,
    pub comparator: String,
    pub call_time_interval: usize,
}

impl From<&PgRow> for RpcCallRule {
    /// Creates an `RpcCallRule` from a database row.
    ///
    /// # Arguments
    ///
    /// * `row` - A reference to a `PgRow`.
    ///
    /// # Returns
    ///
    /// A new instance of `RpcCallRule`.
    fn from(row: &PgRow) -> Self {
        RpcCallRule {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN)),
            url: row.get(DB_URL_COLUMN),
            expected_value: row.get(DB_EXPECTED_VALUE_COLUMN),
            comparator: row.get(DB_COMPARATOR_COLUMN),
            call_time_interval: parse_i32_to_usize(row.get(DB_CALL_TIME_INTERVAL_COLUMN)),
        }
    }
}

/// Represents an RPC call.
#[derive(Clone)]
pub struct RpcCall {
    pub rule: RpcCallRule,
    pub client: Arc<Client>,
    request: JsonRpcRequest,
}

impl RpcCall {
    /// Creates a new `RpcCall` instance.
    ///
    /// # Arguments
    ///
    /// * `client` - The HTTP client.
    /// * `rule` - The RPC call rule.
    ///
    /// # Returns
    ///
    /// A new instance of `RpcCall`.
    pub fn new(client: Arc<Client>, rule: RpcCallRule) -> Self {
        let request = Self::get_rpc_request();

        Self {
            rule,
            client,
            request,
        }
    }

    /// Gets the JSON-RPC request.
    ///
    /// # Returns
    ///
    /// A `JsonRpcRequest` instance.
    fn get_rpc_request() -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            id: "curltest".to_string(),
            method: "eth_syncing".to_string(),
            params: Vec::<String>::new(),
        }
    }

    /// Fetches the RPC call status.
    ///
    /// # Returns
    ///
    /// A result containing the status as `U64`.
    pub async fn fetch_rpc_call_status(&self) -> Result<U64, ClientError> {
        let response = self
            .client
            .post(&self.rule.url)
            .json(&self.request)
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().as_u16().into()),
            Err(_) => Ok(StatusCode::NOT_FOUND.as_u16().into()),
        }
    }
}
