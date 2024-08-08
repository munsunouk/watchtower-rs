use std::collections::HashMap;
use std::sync::Arc;

use ethers::abi::{decode, Token};
use ethers::providers::JsonRpcClient;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use watch_tower_lib::db::postgres::PostgresClient;

use crate::rule::{contract_event::ContractEvent, parse_decode_token};
use crate::utils::constants::{RuleID, INVALID_CONTRACT_EVENT_LOG};
use crate::utils::msg::ContractEventRawMessage;

/// Manages contract event operations.
#[derive(Clone)]
pub struct ContractEventManager<T> {
    /// A map of contract events indexed by rule ID.
    pub contract_events: HashMap<RuleID, ContractEvent<T>>,
    /// The channel to receive contract event messages.
    pub receiver: Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
    /// The Postgres client for database operations.
    pub db_client: PostgresClient,
}

impl<T: JsonRpcClient> ContractEventManager<T> {
    /// Creates a new `ContractEventManager` instance.
    ///
    /// # Arguments
    ///
    /// * `contract_events` - A map of contract events indexed by rule ID.
    /// * `receiver` - The channel to receive contract event messages.
    /// * `db_client` - The Postgres client for database operations.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEventManager`.
    pub fn new(
        contract_events: HashMap<RuleID, ContractEvent<T>>,
        receiver: Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
        db_client: PostgresClient,
    ) -> Self {
        Self {
            contract_events,
            receiver,
            db_client,
        }
    }

    /// Runs the contract event manager, processing messages from the receiver.
    pub async fn run(&mut self) {
        loop {
            let msg = self.receiver.lock().await.recv().await.unwrap();

            let contract_event = self.contract_events.get(&msg.rule_id).unwrap();
            let input_param_type = contract_event.get_raw_input_param_type().unwrap();
            let parsing_input_param_type = contract_event.get_input_param_type().unwrap();

            if msg.event_logs.is_empty() {
                self.send_contract_event_block_log(
                    &contract_event.client.get_chain_name(),
                    msg.rule_id.try_into().unwrap(),
                    msg.block_number.try_into().unwrap(),
                )
                .await;

                continue;
            }

            let mut stream = tokio_stream::iter(msg.event_logs);

            while let Some(log) = stream.next().await {
                if !contract_event.is_target_event(&log.topics[contract_event.rule.event_index]) {
                    continue;
                }

                let token = Token::Tuple(decode(&[input_param_type.clone()], &log.data).unwrap());

                let decoded_token = parse_decode_token(
                    &token,
                    &parsing_input_param_type,
                    &contract_event.rule.rule_filter,
                    &contract_event.rule.expected_value_index,
                    &contract_event.rule.expected_value,
                    &contract_event.rule.comparator,
                )
                .unwrap();

                if decoded_token.is_some() {
                    let tx_log = format!("{:?}", log.transaction_hash.unwrap());

                    self.send_contract_event_log(
                        &contract_event.client.get_chain_name(),
                        msg.rule_id.try_into().unwrap(),
                        &decoded_token.unwrap(),
                        &tx_log,
                    )
                    .await;
                }
            }

            self.send_contract_event_block_log(
                &contract_event.client.get_chain_name(),
                msg.rule_id.try_into().unwrap(),
                msg.block_number.try_into().unwrap(),
            )
            .await;
        }
    }

    async fn send_contract_event_block_log(
        &self,
        chain_name: &str,
        rule_id: i32,
        block_number: i32,
    ) {
        self.db_client
            .insert_contract_event_block_log(rule_id, block_number)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[{}] ❗️ [{}] [Error: {}]",
                    chain_name,
                    INVALID_CONTRACT_EVENT_LOG,
                    err
                );
            });
    }

    async fn send_contract_event_log(
        &self,
        chain_name: &str,
        rule_id: i32,
        value: &str,
        tx_log: &str,
    ) {
        self.db_client
            .insert_contract_event_log(value, tx_log, rule_id)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[{}] ❗️ [{}] [Error: {}]",
                    chain_name,
                    INVALID_CONTRACT_EVENT_LOG,
                    err
                );
            });
    }
}
