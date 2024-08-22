use crate::{
    rule::{contract_call::ContractCall, set_schedule},
    utils::{
        constants::{DEFAULT_BLOCK_NUMBER, MAX_BLOCK_LENGTH_LIMIT, NEW_BLOCK_OFFSET, NEXT_BLOCK},
        error::WorkerError,
        msg::ContractCallRawMessage,
        traits::Fetcher,
    },
};
use cron::Schedule;
use ethers::{abi::Token, providers::JsonRpcClient, types::U64};
use tokio::sync::mpsc::UnboundedSender;
use watch_tower_lib::utils::error::ClientError;

/// Struct representing a contract call fetcher.
#[derive(Clone)]
pub struct ContractCallFetcher<T> {
    /// The contract call to be fetched.
    pub contract_call: ContractCall<T>,
    /// The channel sending event messages.
    pub sender: UnboundedSender<ContractCallRawMessage>,
    /// The call time interval for each fetcher.
    call_time_interval: u64,
    // The block from which to start fetching.
    from_block: U64,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> Fetcher for ContractCallFetcher<T> {
    /// Returns the schedule for the fetcher.
    fn schedule(&self) -> Result<Schedule, WorkerError> {
        set_schedule(
            self.call_time_interval
                .try_into()
                .unwrap_or_else(|_| panic!("{}", WorkerError::InvalidTypeConvert.to_string())),
        )
    }

    /// Runs the fetcher, fetching contract calls at scheduled intervals.
    async fn run(&mut self) -> Result<(), WorkerError> {
        self.initialize().await?;
        loop {
            self.wait_until_next_time().await?;
            self.process().await?;
        }
    }

    /// Processes the contract call, fetching the latest block number and sending the contract call log.
    async fn process(&mut self) -> Result<(), WorkerError> {
        let latest_block = self.get_latest_block_number().await?;
        if !self.check_block_interval(latest_block).await {
            return Ok(());
        }

        let block_tokens = self.get_block_tokens(self.from_block, latest_block).await;

        let from = self.from_block;
        let to = latest_block;

        if let Ok(block_tokens) = block_tokens {
            self.sender
                .send(ContractCallRawMessage::new(
                    block_tokens,
                    self.contract_call.rule.id,
                ))
                .map_err(|_| WorkerError::InvalidMessage)?;

            tracing::info!(
                "[Rule ID : {}] ✨ [Block Number :({:?} … {:?})]",
                &self.contract_call.rule.id,
                from,
                to
            );
            self.replace_from_block(to);
        }

        Ok(())
    }
}

impl<T: JsonRpcClient> ContractCallFetcher<T> {
    /// Creates a new `ContractCallFetcher` instance.
    ///
    /// # Arguments
    ///
    /// * `contract_call` - The contract call to be fetched.
    /// * `sender` - The channel sending event messages.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractCallFetcher`.
    pub fn new(
        contract_call: ContractCall<T>,
        sender: UnboundedSender<ContractCallRawMessage>,
        from_block_number: U64,
        call_time_interval: u64,
    ) -> Self {
        Self {
            contract_call,
            sender,
            call_time_interval,
            from_block: from_block_number,
        }
    }

    /// Initializes the call fetcher
    async fn initialize(&mut self) -> Result<(), WorkerError> {
        // Prevent chain id mismatch in DB
        self.contract_call
            .client
            .verify_chain_id()
            .await
            .map_err(|_| WorkerError::InvalidClient)?;
        self.check_zero_block().await?;

        Ok(())
    }

    /// Gets the latest block number.
    async fn get_latest_block_number(&self) -> Result<U64, WorkerError> {
        self.contract_call
            .client
            .get_latest_block_number()
            .await
            .map_err(|_| WorkerError::InvalidClient)
    }

    /// Checks if the block number is greater than the call block interval.
    async fn check_block_interval(&self, latest_block: U64) -> bool {
        latest_block.saturating_sub(self.contract_call.rule.check_block_interval) >= self.from_block
    }

    /// Checks if the from block is zero and replaces it with the latest block number.
    async fn check_zero_block(&mut self) -> Result<(), WorkerError> {
        let latest_block = self.get_latest_block_number().await?;

        if self.from_block == U64::from(DEFAULT_BLOCK_NUMBER) {
            self.from_block = latest_block;
        }

        Ok(())
    }

    /// Gets the tokens for the given block range.
    async fn get_block_tokens(
        &mut self,
        from: U64,
        to: U64,
    ) -> Result<Vec<(Token, U64)>, ClientError> {
        let mut tokens = vec![];

        let bootstrap_block_length = to.saturating_sub(from);

        let mut from = if bootstrap_block_length > U64::from(MAX_BLOCK_LENGTH_LIMIT) {
            tracing::warn!(
                "[Rule ID : {}] ⚠️ [Block Length : {}]",
                &self.contract_call.rule.id,
                bootstrap_block_length
            );
            to.saturating_sub(U64::from(MAX_BLOCK_LENGTH_LIMIT))
        } else {
            from
        };

        self.replace_from_block(from);

        while from <= to {
            let token = self.contract_call.get_method_call(from.into()).await?;
            tokens.push((token, from));
            from = from.saturating_add(U64::from(NEXT_BLOCK));
        }

        Ok(tokens)
    }

    /// Replaces the from block with the given block number.
    #[inline]
    fn replace_from_block(&mut self, to: U64) {
        self.from_block = to.saturating_add(U64::from(NEW_BLOCK_OFFSET));
    }
}
