use std::collections::HashMap;
use std::sync::Arc;

use ethers::providers::JsonRpcClient;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use watch_tower_lib::db::postgres::PostgresClient;

use crate::rule::{contract_call::ContractCall, parse_decode_token};
use crate::utils::constants::{RuleID, INVALID_CONTRACT_CALL_LOG};
use crate::utils::msg::ContractCallRawMessage;

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

    /// Runs the contract call manager, processing messages from the receiver.
    pub async fn run(&mut self) {
        loop {
            let msg = self.receiver.lock().await.recv().await.unwrap();

            let contract_call = self.contract_calls.get(&msg.rule_id).unwrap();
            let output_param_type = contract_call.get_output_param_type().unwrap();

            let decoded_token = parse_decode_token(
                &msg.call_token,
                &output_param_type,
                &contract_call.rule.rule_filter,
                &contract_call.rule.expected_value_index,
                &contract_call.rule.expected_value,
                &contract_call.rule.comparator,
            )
            .unwrap();

            if decoded_token.is_some() {
                self.send_contract_call_log(
                    &contract_call.client.get_chain_name(),
                    msg.rule_id.try_into().unwrap(),
                    &decoded_token.unwrap(),
                    msg.block_number.try_into().unwrap(),
                )
                .await
            }
        }
    }

    async fn send_contract_call_log(
        &self,
        chain_name: &str,
        rule_id: i32,
        decoded_token: &str,
        block_number: i32,
    ) {
        self.db_client
            .insert_contract_call_log(decoded_token, block_number, rule_id)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[{}] ❗️ [{}] [Error: {}]",
                    chain_name,
                    INVALID_CONTRACT_CALL_LOG,
                    err
                );
            });
    }
}
