use cron::Schedule;
use ethers::providers::JsonRpcClient;
use ethers::types::U64;
use tokio::sync::mpsc::UnboundedSender;

use crate::rule::contract_call::ContractCall;
use crate::utils::constants::INVALID_CONTRACT_CALL_LOG;
use crate::utils::msg::ContractCallRawMessage;
use crate::utils::traits::Fetcher;

/// Struct representing a contract call fetcher.
#[derive(Clone)]
pub struct ContractCallFetcher<T> {
    /// The contract call to be fetched.
    pub contract_call: ContractCall<T>,
    /// The channel sending event messages.
    pub sender: UnboundedSender<ContractCallRawMessage>,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> Fetcher for ContractCallFetcher<T> {
    /// Returns the schedule for the fetcher.
    fn schedule(&self) -> Schedule {
        self.contract_call.rule.check_interval.clone()
    }

    /// Runs the fetcher, fetching contract calls at scheduled intervals.
    async fn run(&mut self) {
        loop {
            self.wait_until_next_time().await;
            self.process().await;
        }
    }

    /// Processes the contract call, fetching the latest block number and sending the contract call log.
    async fn process(&mut self) {
        let target_token = match self.contract_call.get_method_call().await {
            Ok(token) => token,
            Err(err) => {
                tracing::error!(
                    "[{}] ❗️ [{}] [Error: {}]",
                    &self.contract_call.client.get_chain_name(),
                    INVALID_CONTRACT_CALL_LOG,
                    err
                );
                return;
            }
        };

        let block_number = self.get_latest_block_number().await;

        self.sender
            .send(ContractCallRawMessage::new(
                block_number,
                target_token,
                self.contract_call.rule.id,
            ))
            .unwrap();

        tracing::info!(
            "[Rule ID : {}] ✨ [Block Number : {}]",
            &self.contract_call.rule.id,
            block_number
        );
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
    ) -> Self {
        Self {
            contract_call,
            sender,
        }
    }

    async fn get_latest_block_number(&self) -> U64 {
        self.contract_call
            .client
            .get_latest_block_number()
            .await
            .unwrap()
    }
}
