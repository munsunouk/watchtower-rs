use cron::Schedule;
use ethers::providers::JsonRpcClient;
use tokio::sync::mpsc::UnboundedSender;

use crate::rule::contract_call::ContractCall;
use crate::utils::constants::INVALID_CONTRACT_CALL_LOG;
use crate::utils::msg::ContractCallRawMessage;
use crate::utils::traits::PeriodicWorker;

/// Struct representing a contract call fetcher.
#[derive(Clone)]
pub struct ContractCallFetcher<T> {
    /// The contract call to be fetched.
    pub contract_call: ContractCall<T>,
    /// The channel sending event messages.
    pub sender: UnboundedSender<ContractCallRawMessage>,
}

#[async_trait::async_trait]
impl<T: JsonRpcClient> PeriodicWorker for ContractCallFetcher<T> {
    /// Returns the schedule for the periodic worker.
    fn schedule(&self) -> Schedule {
        self.contract_call.rule.check_interval.clone()
    }

    /// Runs the periodic worker, fetching contract calls at scheduled intervals.
    async fn run(&mut self) {
        loop {
            self.wait_until_next_time().await;
            self.process_contract_call().await;
        }
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

    /// Processes a contract call and sends the result through the channel.
    pub async fn process_contract_call(&self) {
        tracing::info!("Processing contract call");

        let target_token = match self.contract_call.fetch_method_call().await {
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

        let block_number = self
            .contract_call
            .client
            .get_latest_block_number()
            .await
            .unwrap();

        self.sender
            .send(ContractCallRawMessage::new(
                block_number,
                target_token,
                self.contract_call.rule.id,
            ))
            .unwrap();

        tracing::info!(
            "[{}] ✨ Imported #{:?}",
            &self.contract_call.client.get_chain_name(),
            block_number
        );
    }
}
