use std::collections::HashMap;
use std::sync::Arc;

use ethers::abi::{decode, Token};
use ethers::providers::JsonRpcClient;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use watch_tower_lib::db::postgres::PostgresClient;

use crate::rule::{contract_event::ContractEvent, parse_decode_token};
use crate::utils::constants::{RuleID, DEFAULT_INDEX};
use crate::utils::error::{IndexType, WorkerError};
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
    pub async fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            let msg = self
                .receiver
                .lock()
                .await
                .recv()
                .await
                .ok_or(WorkerError::InvalidMessage)?;

            if msg.event_logs.is_empty() {
                self.insert_contract_event_block_logs(msg.block_number.try_into().map_err(
                    |_| WorkerError::InvalidTypeConvertError(msg.block_number.to_string()),
                )?)
                .await;

                continue;
            }

            let mut stream = tokio_stream::iter(msg.event_logs);

            while let Some(log) = stream.next().await {
                for (_, event) in self.contract_events.iter() {
                    if event.is_target_event(log.topics.get(event.rule.event_index).ok_or(
                        WorkerError::InvalidIndex(IndexType::USize(event.rule.event_index)),
                    )?)? {
                        let contract_event = self
                            .contract_events
                            .get(&event.rule.id)
                            .ok_or(WorkerError::InvalidIndex(IndexType::USize(event.rule.id)))?;
                        let input_param_type = contract_event.get_raw_input_param_type()?;
                        let parsing_input_param_type = contract_event.get_input_param_type()?;

                        let token =
                            Token::Tuple(decode(&[input_param_type.clone()], &log.data).map_err(
                                |_| WorkerError::InvalidTypeConvertError(log.data.to_string()),
                            )?);

                        let decoded_token = parse_decode_token(
                            &token,
                            &parsing_input_param_type,
                            &contract_event.rule.rule_filter,
                            &contract_event.rule.rule_filter_comparator,
                            &contract_event.rule.expected_value_filter,
                            &contract_event.rule.expected_value_filter_comparator,
                        )?;

                        if let Some(decoded_value) = decoded_token {
                            if let Some(tx_log) = log.transaction_hash {
                                let tx_log = format!("{:?}", tx_log);

                                self.insert_contract_event_log(
                                    event.rule.id.try_into().map_err(|_| {
                                        WorkerError::InvalidTypeConvertError(
                                            event.rule.id.to_string(),
                                        )
                                    })?,
                                    &decoded_value,
                                    &tx_log,
                                )
                                .await;

                                tracing::warn!(
                                    "[Rule ID : {}] ⚠️ [Value : {}]",
                                    event.rule.id,
                                    decoded_value
                                );
                            }
                        }
                    }
                }

                self.insert_contract_event_block_logs(msg.block_number.try_into().map_err(
                    |_| WorkerError::InvalidTypeConvertError(msg.block_number.to_string()),
                )?)
                .await;
            }
        }
    }

    /// Inserts a contract event block log into the database.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number associated with the contract event.
    async fn insert_contract_event_block_logs(&self, block_number: i32) {
        self.db_client
            .insert_contract_event_block_logs(block_number)
            .await
            .unwrap_or_else(|err| {
                let chain_id = self
                    .contract_events
                    .values()
                    .next()
                    .unwrap_or_else(|| {
                        panic!(
                            "{}",
                            WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)).to_string()
                        )
                    })
                    .rule
                    .chain_id;

                tracing::error!(
                    "[Chain ID : {}] ❗️ [Error: {}]",
                    chain_id,
                    WorkerError::InvalidContractEventLog(err.to_string()),
                );
            });
    }

    /// Inserts a contract event log into the database.
    ///
    /// # Arguments
    ///
    /// * `rule_id` - The ID of the rule associated with the contract event.
    /// * `value` - The value to be logged.
    /// * `tx_log` - The transaction log associated with the contract event.
    async fn insert_contract_event_log(&self, rule_id: i32, value: &str, tx_log: &str) {
        self.db_client
            .insert_contract_event_log(value, tx_log, rule_id)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(
                    "[Rule ID : {}] ❗️ [Error : {}]",
                    rule_id,
                    WorkerError::InvalidContractEventLog(err.to_string()),
                );
            });
    }
}
