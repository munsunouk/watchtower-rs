use cron::Schedule;
use ethers::{
    providers::JsonRpcClient,
    types::{Address, BlockNumber, Filter, Log, U64},
};
use futures::future::join_all;
use std::collections::HashMap; // Add this import

use tokio::sync::mpsc::UnboundedSender;
use watch_tower_lib::cli::eth::EthClient;

use crate::utils::{
    constants::{RuleID, DEFAULT_BLOCK_NUMBER, DEFAULT_INDEX, NEW_BLOCK_OFFSET},
    error::{IndexType, WorkerError},
    traits::Fetcher,
};
use crate::utils::{
    constants::{BLOCK_OFFSET, BOOTSTRAP_BLOCK_CHUNK_SIZE},
    msg::ContractEventRawMessage,
};
use crate::{
    rule::{contract_event::ContractEvent, set_schedule},
    utils::constants::NEXT_BLOCK,
};

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
    /// The call time interval for each fetcher.
    call_time_interval: u64,
    // The block from which to start fetching.
    from_block: U64,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> Fetcher for ContractEventFetcher<T> {
    /// Returns the schedule for the fetcher.
    fn schedule(&self) -> Result<Schedule, WorkerError> {
        set_schedule(
            self.call_time_interval
                .try_into()
                .expect(&WorkerError::InvalidTypeConvert.to_string()),
        )
    }

    /// Runs the fetcher, fetching contract events at scheduled intervals.
    async fn run(&mut self) -> Result<(), WorkerError> {
        self.initialize().await?;
        loop {
            self.wait_until_next_time().await?;
            self.process().await?;
        }
    }

    /// Processes the contract event, fetching the latest block number and sending the contract event log.
    async fn process(&mut self) -> Result<(), WorkerError> {
        let from = self.from_block;
        let to = self.get_latest_block_number().await?;

        if from > to {
            return Ok(());
        }

        let event_logs = self.get_event_logs(from, to).await;

        if let Ok(event_logs) = event_logs {
            self.sender
                .send(ContractEventRawMessage::new(event_logs, to))
                .map_err(|_| WorkerError::InvalidMessage)?;

            let chain_id = self.get_client().await.get_chain_id();

            tracing::info!(
                "[Chain ID : {}] ✨ [Block Number :({:?} … {:?})]",
                chain_id,
                from,
                to
            );
            self.replace_from_block(to);
        }

        Ok(())
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
        from_block_numbers: HashMap<RuleID, U64>,
        call_time_interval: u64,
    ) -> Self {
        Self {
            client,
            sender,
            contract_events,
            from_block_numbers,
            call_time_interval,
            from_block: U64::from(DEFAULT_BLOCK_NUMBER),
        }
    }

    /// Initializes the event fetcher.
    async fn initialize(&mut self) -> Result<(), WorkerError> {
        self.get_client()
            .await
            .verify_chain_id()
            .await
            .map_err(|_| WorkerError::InvalidClient)?;
        self.check_zero_blocks().await?;
        let oldest_block = self.get_oldest_block()?;
        self.from_block = *oldest_block;

        Ok(())
    }

    /// Fetches events from the given block range.
    async fn get_event_logs(&self, mut from: U64, to: U64) -> Result<Vec<Log>, WorkerError> {
        let mut logs = vec![];
        // Split from_block into smaller chunks
        while from <= to {
            let chunk_to_block =
                std::cmp::min(from + BOOTSTRAP_BLOCK_CHUNK_SIZE - BLOCK_OFFSET, to);

            let filter = Filter::new()
                .from_block(BlockNumber::from(from))
                .to_block(BlockNumber::from(chunk_to_block))
                .address(self.get_addresses());

            let target_logs_chunk = self
                .get_client()
                .await
                .get_logs(&filter)
                .await
                .map_err(|_| WorkerError::InvalidClient)?;
            logs.extend(target_logs_chunk);

            from = chunk_to_block + NEXT_BLOCK;
        }

        Ok(logs)
    }

    /// Gets the latest block number.
    async fn get_latest_block_number(&self) -> Result<U64, WorkerError> {
        self.get_client()
            .await
            .get_latest_block_number()
            .await
            .map_err(|_| WorkerError::InvalidClient)
    }

    /// Gets the client.
    async fn get_client(&self) -> &EthClient<T> {
        &self.client
    }

    /// Gets the oldest block among Rules.
    fn get_oldest_block(&self) -> Result<&U64, WorkerError> {
        self.from_block_numbers
            .values()
            .min()
            .ok_or(WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))
    }

    /// Checks if the from block is zero and replaces it with the latest block number.
    async fn check_zero_blocks(&mut self) -> Result<(), WorkerError> {
        let latest_block = self.get_latest_block_number().await?;

        let futures = self
            .from_block_numbers
            .iter_mut()
            .map(|(_, block)| async {
                if *block == U64::from(DEFAULT_BLOCK_NUMBER) {
                    *block = latest_block
                }
            })
            .collect::<Vec<_>>();

        join_all(futures).await;

        Ok(())
    }

    /// Gets the addresses of the contract events.
    fn get_addresses(&self) -> Vec<Address> {
        self.contract_events
            .values()
            .map(|event| event.rule.address)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Replaces the from block with the given block number.
    #[inline]
    fn replace_from_block(&mut self, to: U64) {
        self.from_block = to.saturating_add(U64::from(NEW_BLOCK_OFFSET));
    }
}
