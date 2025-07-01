pub mod metadata;

pub use metadata::ProviderMetadata;

use crate::utils::constants::{
    DEFAULT_CALL_RETRY_INTERVAL_MS, ETH_TIMEOUT_DURATION_SECS, RPC_ETH_BLOCK_NUMBER,
    RPC_ETH_CHAIN_ID, RPC_ETH_GET_BALANCE, RPC_ETH_GET_BLOCK_BY_NUMBER, RPC_ETH_GET_LOGS,
    RPC_ETH_GET_TRANSACTION_BY_HASH, RPC_ETH_GET_TRANSACTION_RECEIPT, RPC_ETH_SYNCING,
    RPC_PARAM_LATEST, RPC_TXPOOL_CONTENT,
};
use crate::utils::{error::ClientError, types::ChainID};
use ethers::{
    abi::Token,
    contract::Contract,
    providers::{JsonRpcClient, Provider},
    types::{
        Address, Block, BlockId, Filter, Log, SyncingStatus, Transaction, TransactionReceipt,
        TxpoolContent, H256, U256, U64,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, sync::Arc, time::Duration};
use tokio::time::{sleep, timeout};

#[derive(Clone)]
pub struct EthClient<T> {
    /// The metadata of the provider.
    pub metadata: ProviderMetadata,
    /// The ethers.rs wrapper for the connected chain.
    providers: Vec<Arc<Provider<T>>>,
}

impl<T: JsonRpcClient> EthClient<T> {
    /// Instantiates a new `EthClient` instance for the given chain.
    pub fn new(metadata: ProviderMetadata, providers: Vec<Arc<Provider<T>>>) -> Self {
        Self {
            metadata,
            providers,
        }
    }

    /// Returns name which chain this client interacts with.
    pub fn get_chain_name(&self) -> &String {
        &self.metadata.name
    }

    /// Returns id which chain this client interacts with.
    pub fn get_chain_id(&self) -> ChainID {
        self.metadata.id
    }

    /// Returns `&[Arc<Provider>]`.
    pub fn get_providers(&self) -> &[Arc<Provider<T>>] {
        &self.providers
    }

    /// Make a JSON RPC request to the chain provider via the internal connection, and return the
    /// result. This method wraps the original JSON RPC call and retries whenever the request fails
    /// until it exceeds the maximum retries.
    async fn rpc_call<P, R>(&self, method: &str, params: P) -> Result<R, ClientError>
    where
        P: Debug + Serialize + Send + Sync + Copy,
        R: Serialize + DeserializeOwned + Debug + Send,
    {
        const TIMEOUT_DURATION: Duration = Duration::from_secs(ETH_TIMEOUT_DURATION_SECS);

        for (index, provider) in self.providers.iter().enumerate() {
            match timeout(TIMEOUT_DURATION, provider.request(method, params)).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(error)) => {
                    // If this is not the last provider, sleep and try the next one
                    if index < self.providers.len() - 1 {
                        sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
                        continue;
                    }
                    // If this is the last provider, return the error
                    let error_msg = format!(
                        "[{}] ❗️ [method: {}] [Error: {}]",
                        self.get_chain_name(),
                        method,
                        error.to_string()
                    );
                    return Err(ClientError::InternalProviderError(error_msg));
                }
                Err(_) => {
                    // If this is not the last provider, sleep and try the next one
                    if index < self.providers.len() - 1 {
                        sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
                        continue;
                    }
                    // If this is the last provider, return the error
                    let error_msg = format!(
                        "[{}] ❗️ [method: {}] [Error: Request timed out]",
                        self.get_chain_name(),
                        method
                    );
                    return Err(ClientError::InternalProviderError(error_msg));
                }
            }
        }

        // This should never be reached, but just in case
        let error_msg = format!(
            "[{}] ❗️ [method: {}] [Error: No providers available]",
            self.get_chain_name(),
            method
        );
        Err(ClientError::InternalProviderError(error_msg))
    }

    /// Make a contract call to the chain provider via the internal connection, and return the
    /// result. This method wraps the original contract call and retries whenever the request fails
    /// until it last contract fails.
    pub async fn contracts_call(
        &self,
        contracts: &[Contract<Provider<T>>],
        method: &str,
        method_params: &Token,
        block_id: BlockId,
    ) -> Result<Token, ClientError> {
        for contract in contracts.iter() {
            let raw_call = contract
                .method::<_, Token>(method, method_params.to_owned())?
                .block(block_id);

            match raw_call.call().await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(error) => {
                    let error_msg = format!(
                        "[{}] ❗️ [method: {}] [Error: {}]",
                        self.get_chain_name(),
                        method,
                        error.to_string()
                    );
                    return Err(ClientError::InternalProviderError(error_msg));
                }
            }
        }

        // This should never be reached, but just in case
        let error_msg = format!(
            "[{}] ❗️ [method: {}] [Error: No contracts available]",
            self.get_chain_name(),
            method
        );
        Err(ClientError::InternalProviderError(error_msg))
    }

    /// Verifies whether the configured chain ID and the provider's actual chain ID matches.
    pub async fn verify_chain_id(&self) -> Result<(), ClientError> {
        let chain_id: U256 = self.rpc_call(RPC_ETH_CHAIN_ID, ()).await?;
        if self.get_chain_id() != chain_id.as_u32() {
            return Err(ClientError::InvalidChainId(
                self.get_chain_name().to_string(),
            ));
        }
        Ok(())
    }

    /// Retrieves the latest mined block number of the connected chain.
    pub async fn get_latest_block_number(&self) -> Result<U64, ClientError> {
        self.rpc_call(RPC_ETH_BLOCK_NUMBER, ()).await
    }

    pub async fn get_latest_block(&self) -> Result<Block<H256>, ClientError> {
        self.rpc_call(RPC_ETH_GET_BLOCK_BY_NUMBER, (RPC_PARAM_LATEST, false))
            .await
    }

    /// Retrieves the block information of the given block hash.
    pub async fn get_block_with_txs(
        &self,
        id: BlockId,
    ) -> Result<Option<Block<Transaction>>, ClientError> {
        self.rpc_call(RPC_ETH_GET_BLOCK_BY_NUMBER, (id, true)).await
    }

    /// Retrieves the block information of the given block hash.
    pub async fn get_block(&self, id: BlockId) -> Result<Option<Block<H256>>, ClientError> {
        self.rpc_call(RPC_ETH_GET_BLOCK_BY_NUMBER, (id, false))
            .await
    }

    /// Retrieves the balance of the given address at the given block.
    pub async fn get_balance(
        &self,
        address: Address,
        block_id: BlockId,
    ) -> Result<U256, ClientError> {
        self.rpc_call(RPC_ETH_GET_BALANCE, (address, block_id))
            .await
    }

    /// Retrieves the transaction of the given transaction hash.
    pub async fn get_transaction(&self, hash: H256) -> Result<Option<Transaction>, ClientError> {
        self.rpc_call(RPC_ETH_GET_TRANSACTION_BY_HASH, (hash,))
            .await
    }

    /// Retrieves the transaction receipt of the given transaction hash.
    pub async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, ClientError> {
        self.rpc_call(RPC_ETH_GET_TRANSACTION_RECEIPT, (hash,))
            .await
    }

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s).
    pub async fn get_txpool_content(&self) -> Result<TxpoolContent, ClientError> {
        self.rpc_call(RPC_TXPOOL_CONTENT, ()).await
    }

    /// Returns an array of all logs matching the given filter.
    pub async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>, ClientError> {
        self.rpc_call(RPC_ETH_GET_LOGS, (filter,)).await
    }

    /// Returns an object with data about the sync status or false.
    pub async fn is_syncing(&self) -> Result<SyncingStatus, ClientError> {
        self.rpc_call(RPC_ETH_SYNCING, ()).await
    }
}
