use std::collections::HashMap;

use cron::Schedule;
use ethers::{
    providers::JsonRpcClient,
    types::{Address, BlockNumber, Filter, Log, U64},
};

use tokio::sync::mpsc::UnboundedSender;
use watch_tower_lib::cli::eth::EthClient;

use crate::utils::{constants::RuleID, traits::Fetcher};
use crate::utils::{constants::BOOTSTRAP_BLOCK_CHUNK_SIZE, msg::ContractEventRawMessage};
use crate::{
    rule::{contract_event::ContractEvent, set_schedule},
    utils::constants::NEXT_BLOCK,
};

use anyhow::Result;

/// The essential task that listens and fetches new events.
#[derive(Clone)]
pub struct ContractEventFetcher<T> {
    /// The client of the chain.
    client: EthClient<T>,
    /// The contract event.
    contract_events: HashMap<RuleID, ContractEvent<T>>,
    /// The channel sending event messages.
    sender: UnboundedSender<ContractEventRawMessage>,
    /// the block numbers of RuleID by ChainId
    from_block_numbers: HashMap<RuleID, U64>,
    /// The chain interval.
    chain_interval: u64,
    // The block from which to start fetching.
    from_block: U64,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> Fetcher for ContractEventFetcher<T> {
    /// Returns the schedule for the fetcher.
    fn schedule(&self) -> Schedule {
        set_schedule(self.chain_interval.try_into().unwrap())
    }

    /// Runs the fetcher, fetching contract events at scheduled intervals.
    async fn run(&mut self) {
        self.initialize().await;
        loop {
            self.wait_until_next_time().await;
            self.process().await;
        }
    }

    /// Processes the contract event, fetching the latest block number and sending the contract event log.
    async fn process(&mut self) {
        let from = self.from_block;
        let to = self.get_latest_block_number().await;

        if from > to {
            return;
        }

        let target_logs = self.get_event_logs(from, to).await;

        if let Ok(target_logs) = target_logs {
            self.sender
                .send(ContractEventRawMessage::new(target_logs, to))
                .unwrap();

            let chain_id = self.get_client().await.get_chain_id();

            tracing::info!(
                "[Chain ID :{}] ✨ [Block Number :({:?} … {:?})]",
                chain_id,
                from,
                to
            );
            self.replace_from_block(to);
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
        client: EthClient<T>,
        contract_events: HashMap<RuleID, ContractEvent<T>>,
        sender: UnboundedSender<ContractEventRawMessage>,
        from_blocks: HashMap<RuleID, U64>,
        chain_interval: u64,
    ) -> Self {
        Self {
            client,
            sender,
            contract_events,
            from_block_numbers: from_blocks,
            chain_interval,
            from_block: U64::from(0),
        }
    }

    /// Initializes the event handler.
    async fn initialize(&mut self) {
        // Prevent chain id mismatch in DB
        let _ = self.get_client().await.verify_chain_id().await;
        let oldest_block = self.get_oldest_block();
        self.from_block = *oldest_block;
    }

    /// Fetches events from the given block range.
    async fn get_event_logs(&self, mut from: U64, to: U64) -> Result<Vec<Log>> {
        let mut logs = vec![];
        // Split from_block into smaller chunks
        while from <= to {
            let chunk_to_block = std::cmp::min(from + BOOTSTRAP_BLOCK_CHUNK_SIZE - 1, to);

            let filter = Filter::new()
                .from_block(BlockNumber::from(from))
                .to_block(BlockNumber::from(chunk_to_block))
                .address(self.get_addresses());

            let target_logs_chunk = self.get_client().await.get_logs(&filter).await.unwrap();
            logs.extend(target_logs_chunk);

            from = chunk_to_block + NEXT_BLOCK;
        }

        Ok(logs)
    }

    async fn get_latest_block_number(&self) -> U64 {
        self.get_client()
            .await
            .get_latest_block_number()
            .await
            .unwrap()
    }

    async fn get_client(&self) -> &EthClient<T> {
        &self.client
    }

    fn get_oldest_block(&self) -> &U64 {
        self.from_block_numbers.values().min().unwrap()
    }

    fn get_addresses(&self) -> Vec<Address> {
        self.contract_events
            .values()
            .map(|event| event.rule.address)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Increment the waiting block.
    #[inline]
    fn replace_from_block(&mut self, to: U64) {
        self.from_block = to.saturating_add(U64::from(1));
    }
}
