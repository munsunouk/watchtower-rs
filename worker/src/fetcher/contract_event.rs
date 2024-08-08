use cron::Schedule;
use ethers::{
    providers::JsonRpcClient,
    types::{BlockNumber, Filter, Log, U64},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::rule::contract_event::ContractEvent;
use crate::utils::traits::PeriodicWorker;
use crate::utils::{constants::BOOTSTRAP_BLOCK_CHUNK_SIZE, msg::ContractEventRawMessage};

use anyhow::Result;

/// The essential task that listens and fetches new events.
#[derive(Clone)]
pub struct ContractEventFetcher<T> {
    /// The contract event.
    pub contract_event: ContractEvent<T>,
    /// The channel sending event messages.
    pub sender: UnboundedSender<ContractEventRawMessage>,
    /// The block waiting for enough confirmations.
    from_block: U64,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> PeriodicWorker for ContractEventFetcher<T> {
    /// Returns the schedule for the periodic worker.
    fn schedule(&self) -> Schedule {
        self.contract_event.rule.check_interval.clone()
    }

    /// Starts the event handler. Reads every new mined block of the connected chain and starts to
    /// publish to the event channel.
    async fn run(&mut self) {
        self.initialize().await;
        loop {
            self.wait_until_next_time().await;
            self.process_confirmed_block().await;
        }
    }
}

impl<T: JsonRpcClient> ContractEventFetcher<T> {
    /// Instantiates a new `ContractEventFetcher` instance.
    ///
    /// # Arguments
    ///
    /// * `contract_event` - The contract event.
    /// * `sender` - The channel sending event messages.
    /// * `waiting_block` - The block waiting for enough confirmations.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEventFetcher`.
    pub fn new(
        contract_event: ContractEvent<T>,
        sender: UnboundedSender<ContractEventRawMessage>,
        waiting_block: U64,
    ) -> Self {
        Self {
            sender,
            contract_event,
            from_block: waiting_block,
        }
    }

    /// Initializes the event handler.
    async fn initialize(&mut self) {
        let _ = self.contract_event.client.verify_chain_id().await;

        let latest_block = self
            .contract_event
            .client
            .get_latest_block_number()
            .await
            .unwrap();

        // Initialize waiting block to the latest block
        if self.from_block == U64::default() || self.from_block >= latest_block {
            self.from_block = latest_block;

            tracing::info!(
                "[{}] 💤 Idle, best: #{:?}",
                &self.contract_event.client.get_chain_name(),
                self.from_block
            );
        } else {
            self.bootstrap().await;
        }
    }

    /// Processes confirmed blocks and sends the result through the channel.
    async fn process_confirmed_block(&mut self) {
        let from = self.from_block;
        let to = from.saturating_add(U64::from(1u64));

        let filter = Filter::new()
            .from_block(BlockNumber::from(from))
            .to_block(BlockNumber::from(to))
            .address(self.contract_event.rule.address);

        let target_logs = self.contract_event.fetch_event(filter).await;

        if let Ok(target_logs) = target_logs {
            self.sender
                .send(ContractEventRawMessage::new(
                    target_logs,
                    to,
                    self.contract_event.rule.id,
                ))
                .unwrap();

            self.replace_from_block(to);

            if from < to {
                tracing::info!(
                    "[{}] ✨ Imported #({:?} … {:?})",
                    &self.contract_event.client.get_chain_name(),
                    from,
                    to
                );
            } else {
                tracing::info!(
                    "[{}] ✨ Imported #{:?}",
                    &self.contract_event.client.get_chain_name(),
                    self.from_block
                );
            }
        }
    }

    /// Bootstrap the event handler by fetching all the events from the waiting block to the latest
    /// block.
    async fn bootstrap(&mut self) {
        let from = self.from_block;

        let to = self
            .contract_event
            .client
            .get_latest_block_number()
            .await
            .unwrap();

        let target_logs = self.get_bootstrap_events(from, to).await;

        if let Ok(target_logs) = target_logs {
            if !target_logs.is_empty() {
                self.sender
                    .send(ContractEventRawMessage::new(
                        target_logs,
                        to,
                        self.contract_event.rule.id,
                    ))
                    .unwrap();

                tracing::info!(
                    "[{}] ✨ Imported #({:?} … {:?})",
                    &self.contract_event.client.get_chain_name(),
                    from,
                    to
                );
            }

            self.replace_from_block(to);
        }
    }

    async fn get_bootstrap_events(&self, mut from: U64, to: U64) -> Result<Vec<Log>> {
        let mut logs = vec![];
        // Split from_block into smaller chunks
        while from <= to {
            let chunk_to_block = std::cmp::min(from + BOOTSTRAP_BLOCK_CHUNK_SIZE - 1, to);

            let filter = Filter::new()
                .from_block(BlockNumber::from(from))
                .to_block(BlockNumber::from(to))
                .address(self.contract_event.rule.address);

            let target_logs_chunk = self.contract_event.fetch_event(filter).await.unwrap();
            logs.extend(target_logs_chunk);

            from = chunk_to_block + 1;
        }

        Ok(logs)
    }

    /// Increment the waiting block.
    #[inline]
    fn replace_from_block(&mut self, to: U64) {
        self.from_block = to;
    }
}
