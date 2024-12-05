use std::{collections::HashMap, sync::Arc};

use ethers::{providers::JsonRpcClient, types::U64};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use tokio_stream::StreamExt;
use watch_tower_lib::{db::postgres::PostgresClient, utils::types::RuleID};

use crate::{
    rule::{contract_call::ContractCall, parse_decode_token},
    utils::{
        error::{IndexType, WorkerError},
        msg::ContractCallRawMessage,
        traits::Manager,
    },
};

/// Manages contract call operations.
#[derive(Clone)]
pub struct ContractCallManager<T> {
    /// A map of contract calls indexed by rule ID.
    pub contract_calls: HashMap<RuleID, ContractCall<T>>,
    /// The channel to receive contract call messages.
    pub receiver: Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
    /// The Postgres client for database operations.
    pub db_client: PostgresClient,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> Manager for ContractCallManager<T> {
    /// Runs the contract call manager, processing messages from the receiver.
    async fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            let msg = self
                .receiver
                .lock()
                .await
                .recv()
                .await
                .ok_or(WorkerError::InvalidMessage)?;

            let contract_call = self
                .contract_calls
                .get(&msg.rule_id)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(msg.rule_id)))?;
            let output_param_type = contract_call.get_output_param_type()?;

            let mut stream = tokio_stream::iter(msg.block_tokens);
            let mut last_block_number = U64::default();

            while let Some((token, block_number)) = stream.next().await {
                let decoded_token = parse_decode_token(
                    &token,
                    &output_param_type,
                    &contract_call.rule.rule_filter,
                    &contract_call.rule.rule_filter_comparator,
                    &contract_call.rule.expected_value_filter,
                    &contract_call.rule.expected_value_filter_comparator,
                )?;

                if let Some(decoded_token) = decoded_token {
                    self.update_contract_call_log(
                        msg.rule_id.try_into().map_err(|_| {
                            WorkerError::InvalidTypeConvertError(msg.rule_id.to_string())
                        })?,
                        &decoded_token.clone(),
                        block_number.try_into().map_err(|_| {
                            WorkerError::InvalidTypeConvertError(block_number.to_string())
                        })?,
                    )
                    .await;

                    tracing::warn!("[Rule ID : {}] ⚠️ [Value : {}]", msg.rule_id, &decoded_token);
                }

                last_block_number = block_number;
            }

            self.update_contract_call_block_logs(
                msg.rule_id
                    .try_into()
                    .map_err(|_| WorkerError::InvalidTypeConvertError(msg.rule_id.to_string()))?,
                last_block_number.try_into().map_err(|_| {
                    WorkerError::InvalidTypeConvertError(last_block_number.to_string())
                })?,
            )
            .await;
        }
    }
}

impl<T: JsonRpcClient> ContractCallManager<T> {
    /// Creates a new `ContractCallManager` instance.
    ///
    /// # Arguments
    ///
    /// * `contract_calls` - A map of contract calls indexed by rule ID.
    /// * `receiver` - The channel to receive contract call messages.
    /// * `db_client` - The Postgres client for database operations.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractCallManager`.
    pub fn new(
        contract_calls: HashMap<RuleID, ContractCall<T>>,
        receiver: Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
        db_client: PostgresClient,
    ) -> Self {
        Self {
            contract_calls,
            receiver,
            db_client,
        }
    }

    /// Updates a contract call log into the database.
    ///
    /// # Arguments
    ///
    /// * `rule_id` - The ID of the rule associated with the contract call.
    /// * `decoded_token` - The decoded token value to be logged.
    /// * `block_number` - The block number associated with the contract call.
    async fn update_contract_call_log(&self, rule_id: i32, decoded_token: &str, block_number: i32) {
        self.db_client
            .update_contract_call_log(decoded_token, block_number, rule_id)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[Rule ID : {}] ❗️ [Error : {}]",
                    rule_id,
                    WorkerError::InvalidContractCallLog(err.to_string()),
                );
            });
    }

    /// Updates a contract call block log into the database.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number associated with the contract call.
    async fn update_contract_call_block_logs(&self, rule_id: i32, block_number: i32) {
        self.db_client
            .update_contract_call_block_logs(rule_id, block_number)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[Rule ID : {}] ❗️ [Error : {}]",
                    rule_id,
                    WorkerError::InvalidContractCallLog(err.to_string()),
                );
            });
    }
}
