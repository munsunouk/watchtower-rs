pub mod metadata;

pub use metadata::ProviderMetadata;

use crate::utils::{constants::DEFAULT_CALL_RETRY_INTERVAL_MS, error::ClientError, types::ChainID};
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

    /// Returns `Arc<Provider>`.
    pub fn get_providers(&self) -> Vec<Arc<Provider<T>>> {
        self.providers.clone()
    }

    /// Make a JSON RPC request to the chain provider via the internal connection, and return the
    /// result. This method wraps the original JSON RPC call and retries whenever the request fails
    /// until it exceeds the maximum retries.
    async fn rpc_call<P, R>(&self, method: &str, params: P) -> Result<R, ClientError>
    where
        P: Debug + Serialize + Send + Sync + Copy,
        R: Serialize + DeserializeOwned + Debug + Send,
    {
        const MAX_RETRIES: u32 = 3;
        const TIMEOUT_DURATION: Duration = Duration::from_secs(300);
        let mut retry_count = 0;
        let mut last_error = None;

        for provider in self.providers.iter() {
            while retry_count < MAX_RETRIES {
                match timeout(TIMEOUT_DURATION, provider.request(method, params)).await {
                    Ok(Ok(result)) => return Ok(result),
                    Ok(Err(error)) => {
                        last_error = Some(error);
                        retry_count += 1;
                        if retry_count < MAX_RETRIES {
                            // Exponential backoff
                            let backoff = Duration::from_millis(
                                DEFAULT_CALL_RETRY_INTERVAL_MS * (1 << (retry_count - 1)),
                            );
                            sleep(backoff).await;
                        }
                    }
                    Err(_) => {
                        last_error = Some(ethers::providers::ProviderError::CustomError(
                            "Request timed out".to_string(),
                        ));
                        retry_count += 1;
                        if retry_count < MAX_RETRIES {
                            sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
                        }
                    }
                }
            }
            retry_count = 0; // Reset retry count for next provider
        }

        let error_msg = match last_error {
            Some(error) => format!(
                "[{}] ❗️ [method: {}] [Error: {}] [Retries: {}]",
                self.get_chain_name(),
                method,
                error.to_string(),
                MAX_RETRIES
            ),
            None => format!(
                "[{}] ❗️ [method: {}] [Error: No providers available]",
                self.get_chain_name(),
                method
            ),
        };

        Err(ClientError::InternalProviderError(error_msg))
    }

    /// Make a contract call to the chain provider via the internal connection, and return the
    /// result. This method wraps the original contract call and retries whenever the request fails
    /// until it last contract fails.
    pub async fn contracts_call(
        &self,
        contracts: Vec<Contract<Provider<T>>>,
        method: &str,
        method_params: Token,
        block_id: BlockId,
    ) -> Result<Token, ClientError> {
        let mut error_msg = String::default();

        for contract in contracts.iter() {
            let raw_call = contract
                .method::<_, Token>(method, method_params.clone())
                .map_err(|err| ClientError::InternalProviderError(err.to_string()))?
                .block(block_id);

            match raw_call.call().await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(error) => {
                    error_msg = format!(
                        "[{}] ❗️ [method: {}] [Error: {}]",
                        self.get_chain_name(),
                        method,
                        error.to_string()
                    );
                }
            }
            sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
        }

        let client_error = ClientError::InternalProviderError(error_msg);

        Err(client_error)
    }

    /// Verifies whether the configured chain ID and the provider's actual chain ID matches.
    pub async fn verify_chain_id(&self) -> Result<(), ClientError> {
        let chain_id: U256 = self.rpc_call("eth_chainId", ()).await?;
        if self.get_chain_id() != chain_id.as_u32() {
            return Err(ClientError::InvalidChainId(
                self.get_chain_name().to_string(),
            ));
        }
        Ok(())
    }

    /// Retrieves the latest mined block number of the connected chain.
    pub async fn get_latest_block_number(&self) -> Result<U64, ClientError> {
        self.rpc_call("eth_blockNumber", ()).await
    }

    pub async fn get_latest_block(&self) -> Result<Block<H256>, ClientError> {
        self.rpc_call("eth_getBlockByNumber", ("latest", false))
            .await
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

    /// Retrieves the balance of the given address at the given block.
    pub async fn get_balance(
        &self,
        address: Address,
        block_id: BlockId,
    ) -> Result<U256, ClientError> {
        self.rpc_call("eth_getBalance", (address, block_id)).await
    }

    /// Retrieves the transaction of the given transaction hash.
    pub async fn get_transaction(&self, hash: H256) -> Result<Option<Transaction>, ClientError> {
        self.rpc_call("eth_getTransactionByHash", (hash,)).await
    }

    /// Retrieves the transaction receipt of the given transaction hash.
    pub async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, ClientError> {
        self.rpc_call("eth_getTransactionReceipt", (hash,)).await
    }

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s).
    pub async fn get_txpool_content(&self) -> Result<TxpoolContent, ClientError> {
        self.rpc_call("txpool_content", ()).await
    }

    /// Returns an array of all logs matching the given filter.
    pub async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>, ClientError> {
        self.rpc_call("eth_getLogs", (filter,)).await
    }

    /// Returns an object with data about the sync status or false.
    pub async fn is_syncing(&self) -> Result<SyncingStatus, ClientError> {
        self.rpc_call("eth_syncing", ()).await
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use ethers::{
        providers::{Http, Provider},
        types::U256,
    };

    use crate::utils::types::ChainID;

    use super::{EthClient, ProviderMetadata};

    #[tokio::test]
    async fn test() {
        let client = EthClient::<Http>::new(
            ProviderMetadata::new(
                "<YOUR CHAIN NAME>".to_string(),
                vec!["<YOUR RPC URL>".to_string()],
                3068 as ChainID,
            ),
            vec![Arc::new(Provider::try_from("<YOUR RPC URL>").unwrap())],
        );

        let balance: U256 = client
            .rpc_call(
                "eth_getBalance",
                ["0x1e1d0be9865afe4a13435bba659e539690841c32", "latest"],
            )
            .await
            .unwrap();
        println!("balance: {:?}", balance);
    }
}
