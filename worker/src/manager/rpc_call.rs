use std::collections::HashMap;
use std::sync::Arc;

use ethers::types::U64;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use watch_tower_lib::db::postgres::PostgresClient;

use crate::rule::parse_compare;
use crate::rule::rpc_call::RpcCall;
use crate::utils::constants::RuleID;
use crate::utils::error::{IndexType, WorkerError};
use crate::utils::msg::RpcCallRawMessage;

/// Manages RPC call operations.
#[derive(Clone)]
pub struct RpcCallManager {
    /// A map of RPC calls indexed by rule ID.
    pub rpc_calls: HashMap<RuleID, RpcCall>,
    /// The channel to receive RPC call messages.
    pub receiver: Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
    /// The Postgres client for database operations.
    pub db_client: PostgresClient,
}

impl RpcCallManager {
    /// Creates a new `RpcCallManager` instance.
    ///
    /// # Arguments
    ///
    /// * `rpc_calls` - A map of RPC calls indexed by rule ID.
    /// * `receiver` - The channel to receive RPC call messages.
    /// * `db_client` - The Postgres client for database operations.
    ///
    /// # Returns
    ///
    /// A new instance of `RpcCallManager`.
    pub fn new(
        rpc_calls: HashMap<RuleID, RpcCall>,
        receiver: Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
        db_client: PostgresClient,
    ) -> Self {
        Self {
            rpc_calls,
            receiver,
            db_client,
        }
    }

    /// Runs the RPC call manager, processing messages from the receiver.
    pub async fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            let msg = self
                .receiver
                .lock()
                .await
                .recv()
                .await
                .ok_or(WorkerError::InvalidMessage)?;

            let rpc_call = self
                .rpc_calls
                .get(&msg.rule_id)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(msg.rule_id)))?;

            let status = msg.status;

            let expected_value = U64::from_dec_str(&rpc_call.rule.expected_value)
                .unwrap_or_else(|_| panic!("{}", WorkerError::InvalidTypeConvert.to_string()));
            let result = parse_compare(&status, &expected_value, &rpc_call.rule.comparator);

            if result.is_some() {
                self.insert_rpc_call_log(
                    msg.rule_id.try_into().map_err(|_| {
                        WorkerError::InvalidTypeConvertError(msg.rule_id.to_string())
                    })?,
                    &status.to_string(),
                )
                .await;

                tracing::warn!(
                    "[Rule ID : {}] ⚠️ [Value : {}]",
                    msg.rule_id,
                    &status.to_string()
                );
            }
        }
    }

    /// Inserts an RPC call log into the database.
    ///
    /// # Arguments
    ///
    /// * `rule_id` - The ID of the rule associated with the RPC call.
    /// * `value` - The value to be logged.
    async fn insert_rpc_call_log(&self, rule_id: i32, value: &str) {
        self.db_client
            .insert_rpc_call_log(value, rule_id)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[Rule ID : {}] ❗️ [Error : {}]",
                    rule_id,
                    WorkerError::InvalidRpcCallLog(err.to_string())
                );
            });
    }
}
