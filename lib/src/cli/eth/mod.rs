pub mod metadata;

pub use metadata::ProviderMetadata;

use crate::utils::{
    constants::{ChainID, DEFAULT_CALL_RETRIES, DEFAULT_CALL_RETRY_INTERVAL_MS, LIMIT_RETRY_CALL},
    error::ClientError,
};
use ethers::{
    abi::Detokenize,
    prelude::ContractCall,
    providers::{JsonRpcClient, Middleware, Provider},
    types::{
        Block, BlockId, Filter, Log, SyncingStatus, Transaction, TransactionReceipt, TxpoolContent,
        H256, U256, U64,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, sync::Arc};
use tokio::time::{sleep, Duration};
use url::Url;

#[derive(Clone)]
pub struct EthClient<T> {
    /// The metadata of the provider.
    pub metadata: ProviderMetadata,
    /// The ethers.rs wrapper for the connected chain.
    provider: Arc<Provider<T>>,
}

impl<T: JsonRpcClient> EthClient<T> {
    /// Instantiates a new `EthClient` instance for the given chain.
    pub fn new(metadata: ProviderMetadata, provider: Arc<Provider<T>>) -> Self {
        Self { metadata, provider }
    }

    /// Returns name which chain this client interacts with.
    pub fn get_chain_name(&self) -> String {
        self.metadata.name.clone()
    }

    /// Returns id which chain this client interacts with.
    pub fn get_chain_id(&self) -> ChainID {
        self.metadata.id
    }

    /// Returns the provider URL.
    pub fn get_url(&self) -> Url {
        self.metadata.url.clone()
    }

    /// Returns `Arc<Provider>`.
    pub fn get_provider(&self) -> Arc<Provider<T>> {
        self.provider.clone()
    }

    /// Make a JSON RPC request to the chain provider via the internal connection, and return the
    /// result. This method wraps the original JSON RPC call and retries whenever the request fails
    /// until it exceeds the maximum retries.
    async fn rpc_call<P, R>(&self, method: &str, params: P) -> Result<R, ClientError>
    where
        P: Debug + Serialize + Send + Sync + Clone,
        R: Serialize + DeserializeOwned + Debug + Send,
    {
        let mut retries_remaining: u8 = DEFAULT_CALL_RETRIES;
        let mut error_msg = String::default();

        while retries_remaining > LIMIT_RETRY_CALL {
            match self.provider.request(method, params.clone()).await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    // retry on error
                    retries_remaining = retries_remaining.saturating_sub(1);
                    error_msg = error.to_string();
                }
            }
            sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
        }
        tracing::error!(
            "[{}] ❗️ [method: {}] [Error: {}]",
            &self.get_chain_name(),
            method,
            ClientError::InternalProviderError(error_msg.clone())
        );

        Err(ClientError::InternalProviderError(error_msg).into())
    }

    /// Make a contract call to the chain provider via the internal connection, and return the
    /// result. This method wraps the original contract call and retries whenever the request fails
    /// until it exceeds the maximum retries.
    pub async fn contract_call<M, D>(
        &self,
        raw_call: ContractCall<M, D>,
        method: &str,
    ) -> Result<D, ClientError>
    where
        M: Middleware,
        D: Serialize + DeserializeOwned + Debug + Send + Detokenize,
    {
        let mut retries_remaining: u8 = DEFAULT_CALL_RETRIES;
        let mut error_msg = String::default();

        while retries_remaining > LIMIT_RETRY_CALL {
            match raw_call.call().await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(error) => {
                    // retry on error
                    retries_remaining = retries_remaining.saturating_sub(1);
                    error_msg = error.to_string();
                }
            }
            sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
        }
        tracing::error!(
            "[{}] ❗️ [method: {}] [Error: {}]",
            &self.get_chain_name(),
            method,
            ClientError::InternalProviderError(error_msg.clone()),
        );

        Err(ClientError::InternalProviderError(error_msg).into())
    }

    /// Verifies whether the configured chain ID and the provider's actual chain ID matches.
    pub async fn verify_chain_id(&self) -> Result<(), ClientError> {
        let chain_id: U256 = self.rpc_call("eth_chainId", ()).await?;
        if self.get_chain_id() != chain_id.as_u32() {
            tracing::error!(
                "[{}] ❗️ [{}]",
                &self.get_chain_name(),
                ClientError::InvalidChainId
            );
            return Err(ClientError::InvalidChainId.into());
        }
        Ok(())
    }

    /// Retrieves the latest mined block number of the connected chain.
    pub async fn get_latest_block_number(&self) -> Result<U64, ClientError> {
        self.rpc_call("eth_blockNumber", ()).await
    }

    /// Retrieves the block information of the given block hash.
    pub async fn get_block_with_txs(
        &self,
        id: BlockId,
    ) -> Result<Option<Block<Transaction>>, ClientError> {
        self.rpc_call("eth_getBlockByNumber", (id, true)).await
    }

    /// Retrieves the block information of the given block hash.
    pub async fn get_block(&self, id: BlockId) -> Result<Option<Block<H256>>, ClientError> {
        self.rpc_call("eth_getBlockByNumber", (id, false)).await
    }

    /// Retrieves the transaction of the given transaction hash.
    pub async fn get_transaction(&self, hash: H256) -> Result<Option<Transaction>, ClientError> {
        self.rpc_call("eth_getTransactionByHash", vec![hash]).await
    }

    /// Retrieves the transaction receipt of the given transaction hash.
    pub async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, ClientError> {
        self.rpc_call("eth_getTransactionReceipt", vec![hash]).await
    }

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s).
    pub async fn get_txpool_content(&self) -> Result<TxpoolContent, ClientError> {
        self.rpc_call("txpool_content", ()).await
    }

    /// Returns an array of all logs matching the given filter.
    pub async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>, ClientError> {
        self.rpc_call("eth_getLogs", vec![filter]).await
    }

    /// Returns an object with data about the sync status or false.
    pub async fn is_syncing(&self) -> Result<SyncingStatus, ClientError> {
        self.rpc_call("eth_syncing", ()).await
    }
}
