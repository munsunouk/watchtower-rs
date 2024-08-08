use crate::fetcher::rpc_call::RpcCallFetcher;
use crate::manager::contract_event::ContractEventManager;
use crate::manager::rpc_call::RpcCallManager;
use crate::rule::contract_event::ContractEventBlockLog;
use crate::rule::rpc_call::RpcCallRule;
use crate::rule::RpcCall;
use crate::utils::config::EVMProvider;
use crate::utils::msg::RpcCallRawMessage;
use crate::utils::traits::PeriodicWorker;
use anyhow::Result;
use ethers::providers::{Http, JsonRpcClient, Provider};
use ethers::types::U64;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, Mutex};
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

use watch_tower_lib::cli::eth::{EthClient, ProviderMetadata};
use watch_tower_lib::db::postgres::PostgresClient;
use watch_tower_lib::utils::constants::{ChainID, DEFAULT_GET_LOGS_BATCH_SIZE};
use watch_tower_lib::utils::error::{
    INVALID_CONFIG_FILE_PATH, INVALID_CONFIG_FILE_STRUCTURE, INVALID_PROVIDER_URL,
};

use crate::utils::constants::{
    RuleID, DB_CONTRACT_CALL_RULE, DB_CONTRACT_EVENT_BLOCK_LOG, DB_CONTRACT_EVENT_RULE,
    DB_RPC_CALL_RULE, SQLX_QUERY_OFF, TIME_FORMAT,
};
use crate::{
    fetcher::{contract_call::ContractCallFetcher, contract_event::ContractEventFetcher},
    manager::contract_call::ContractCallManager,
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, ContractCall,
        ContractEvent,
    },
    utils::{
        config::Configuration,
        msg::{ContractCallRawMessage, ContractEventRawMessage},
    },
};

/// A Watchtower CLI runtime that can be used to run
pub struct Runner {
    /// Fetchers for RPC calls.
    pub rpc_call_fetchers: Vec<RpcCallFetcher>,
    /// Manager for RPC calls.
    pub rpc_call_manager: RpcCallManager,
    /// Fetchers for contract calls.
    pub contract_call_fetchers: Vec<ContractCallFetcher<Http>>,
    /// Fetchers for contract events.
    pub contract_event_fetchers: Vec<ContractEventFetcher<Http>>,
    /// Manager for contract calls.
    pub contract_call_manager: ContractCallManager<Http>,
    /// Manager for contract events.
    pub contract_event_manager: ContractEventManager<Http>,
}

impl Runner {
    /// Creates a new `Runner` instance.
    ///
    /// # Arguments
    ///
    /// * `config_path` - A string slice that holds the path to the configuration file.
    ///
    /// # Returns
    ///
    /// A new instance of `Runner`.
    pub async fn new(config_path: &str) -> Self {
        Self::set_log();

        // Channel
        let (rpc_call_sender, rpc_call_receiver) = Self::set_rpc_call_channel();
        let (contract_call_sender, contract_call_receiver) = Self::set_contract_call_channel();
        let (contract_event_sender, contract_event_receiver) = Self::set_contract_event_channel();

        // Config
        let config: Configuration = Self::set_config(config_path).unwrap();

        //DB
        let db_client = Self::set_db(&config.postgres_config.url).await.unwrap();

        //DB Rules
        let rpc_call_rules = Self::load_rpc_call_rules(&db_client).await;
        let contract_call_rules = Self::load_contract_call_rules(&db_client).await;
        let contract_event_rules = Self::load_contract_event_rules(&db_client).await;
        let contract_event_blocks = Self::load_contract_event_block_logs(&db_client).await;

        //Client
        let rpc_client = Self::set_client();
        let clients = Self::set_eth_clients(config.evm_providers);

        //Rules
        let rpc_calls = Self::set_rpc_calls(rpc_call_rules, rpc_client);
        let contract_calls = Self::set_contract_calls(contract_call_rules, clients.clone());
        let contract_events = Self::set_contract_events(contract_event_rules, clients);

        //Fetcher
        let rpc_call_fetchers = Self::set_rpc_call_fetchers(rpc_calls.clone(), rpc_call_sender);
        let contract_call_fetchers =
            Self::set_contract_call_fetchers(contract_calls.clone(), contract_call_sender);
        let contract_event_fetchers = Self::set_contract_event_fetchers(
            contract_events.clone(),
            contract_event_blocks.clone(),
            contract_event_sender,
        );

        //Manager
        let rpc_call_manager: RpcCallManager = Self::set_rpc_call_manager(
            rpc_calls.clone(),
            rpc_call_receiver.clone(),
            db_client.clone(),
        );
        let contract_call_manager: ContractCallManager<Http> = Self::set_contract_call_manager(
            contract_calls.clone(),
            contract_call_receiver.clone(),
            db_client.clone(),
        );
        let contract_event_manager: ContractEventManager<Http> = Self::set_contract_event_manager(
            contract_events.clone(),
            contract_event_receiver.clone(),
            db_client,
        );

        Self {
            rpc_call_fetchers,
            contract_call_fetchers,
            contract_event_fetchers,
            rpc_call_manager,
            contract_call_manager,
            contract_event_manager,
        }
    }

    /// Runs the `Runner` instance.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub async fn run(&self) -> Result<()> {
        let rpc_call_fetchers = self.rpc_call_fetchers.clone();
        let contract_call_fetchers = self.contract_call_fetchers.clone();
        let contract_event_fetchers = self.contract_event_fetchers.clone();
        let mut rpc_call_manager = self.rpc_call_manager.clone();
        let mut contract_call_manager = self.contract_call_manager.clone();
        let mut contract_event_manager = self.contract_event_manager.clone();

        tokio::try_join!(
            async {
                for mut fetcher in rpc_call_fetchers {
                    tokio::spawn(async move {
                        fetcher.run().await;
                    });
                }
                Ok::<(), anyhow::Error>(())
            },
            async {
                for mut fetcher in contract_call_fetchers {
                    tokio::spawn(async move {
                        fetcher.run().await;
                    });
                }
                Ok::<(), anyhow::Error>(())
            },
            async {
                for mut fetcher in contract_event_fetchers {
                    tokio::spawn(async move {
                        fetcher.run().await;
                    });
                }
                Ok::<(), anyhow::Error>(())
            },
            async {
                tokio::spawn(async move {
                    rpc_call_manager.run().await;
                });
                Ok::<(), anyhow::Error>(())
            },
            async {
                tokio::spawn(async move {
                    contract_call_manager.run().await;
                });
                Ok::<(), anyhow::Error>(())
            },
            async {
                tokio::spawn(async move {
                    contract_event_manager.run().await;
                });
                Ok::<(), anyhow::Error>(())
            }
        )?;

        Ok(())
    }

    /// Loads RPC call rules from the database.
    ///
    /// # Arguments
    ///
    /// * `db_client` - A reference to the Postgres client.
    ///
    /// # Returns
    ///
    /// A vector of `RpcCallRule`.
    pub async fn load_rpc_call_rules(db_client: &PostgresClient) -> Vec<RpcCallRule> {
        let result = db_client.load(DB_RPC_CALL_RULE).await.unwrap();

        let rpc_calls: Vec<RpcCallRule> = result.iter().map(|row| RpcCallRule::from(row)).collect();
        rpc_calls
    }

    /// Loads contract call rules from the database.
    ///
    /// # Arguments
    ///
    /// * `db_client` - A reference to the Postgres client.
    ///
    /// # Returns
    ///
    /// A vector of `ContractCallRule`.
    pub async fn load_contract_call_rules(db_client: &PostgresClient) -> Vec<ContractCallRule> {
        let result = db_client.load(DB_CONTRACT_CALL_RULE).await.unwrap();

        let contract_calls: Vec<ContractCallRule> = result
            .iter()
            .map(|row| ContractCallRule::from(row))
            .collect();
        contract_calls
    }

    /// Loads contract event rules from the database.
    ///
    /// # Arguments
    ///
    /// * `db_client` - A reference to the Postgres client.
    ///
    /// # Returns
    ///
    /// A vector of `ContractEventRule`.
    pub async fn load_contract_event_rules(db_client: &PostgresClient) -> Vec<ContractEventRule> {
        let result = db_client.load(DB_CONTRACT_EVENT_RULE).await.unwrap();

        let contract_events: Vec<ContractEventRule> = result
            .iter()
            .map(|row| ContractEventRule::from(row))
            .collect();
        contract_events
    }

    /// Loads contract event block logs from the database.
    ///
    /// # Arguments
    ///
    /// * `db_client` - A reference to the Postgres client.
    ///
    /// # Returns
    ///
    /// A hashmap of `RuleID` to `U64`.
    pub async fn load_contract_event_block_logs(
        db_client: &PostgresClient,
    ) -> HashMap<RuleID, U64> {
        let result = db_client.load(DB_CONTRACT_EVENT_BLOCK_LOG).await.unwrap();

        let contract_events: HashMap<RuleID, U64> = result
            .iter()
            .map(|row| {
                let block_log = ContractEventBlockLog::from(row);
                (block_log.id, block_log.block_number)
            })
            .collect();
        contract_events
    }

    /// Sets the metadata for a provider.
    ///
    /// # Arguments
    ///
    /// * `chain_name` - The name of the chain.
    /// * `chain_url` - The URL of the chain.
    /// * `chain_id` - The ID of the chain.
    /// * `block_confirmations` - The number of block confirmations.
    /// * `get_logs_batch_size` - The batch size for getting logs.
    ///
    /// # Returns
    ///
    /// A `ProviderMetadata` instance.
    fn set_metadata(
        chain_name: String,
        chain_url: String,
        chain_id: ChainID,
        block_confirmations: u64,
        get_logs_batch_size: Option<u64>,
    ) -> ProviderMetadata {
        ProviderMetadata::new(
            chain_name,
            chain_url,
            chain_id,
            block_confirmations,
            get_logs_batch_size.unwrap_or(DEFAULT_GET_LOGS_BATCH_SIZE),
        )
    }

    /// Sets the provider.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the provider.
    ///
    /// # Returns
    ///
    /// An `Arc` of `Provider<Http>`.
    fn set_provider(url: &str) -> Arc<Provider<Http>> {
        let provider = Provider::<Http>::try_from(url).expect(INVALID_PROVIDER_URL);
        Arc::new(provider)
    }

    /// Sets the Ethereum client.
    ///
    /// # Arguments
    ///
    /// * `metadata` - The metadata of the provider.
    /// * `provider` - An `Arc` of the provider.
    ///
    /// # Returns
    ///
    /// An `EthClient` instance.
    fn set_eth_client<T: JsonRpcClient>(
        metadata: ProviderMetadata,
        provider: Arc<Provider<T>>,
    ) -> EthClient<T> {
        EthClient::new(metadata, provider)
    }

    /// Sets multiple Ethereum clients.
    ///
    /// # Arguments
    ///
    /// * `providers` - A vector of `EVMProvider` instances.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `ChainID` and the value is an `EthClient<Http>` instance.
    fn set_eth_clients(providers: Vec<EVMProvider>) -> HashMap<ChainID, EthClient<Http>> {
        let mut clients = HashMap::new();

        for provider in providers {
            let metadata = Self::set_metadata(
                provider.name.clone(),
                provider.provider.clone(),
                provider.id,
                provider.block_confirmations,
                provider.get_logs_batch_size,
            );

            let arc_provider = Self::set_provider(&provider.provider);

            let eth_client = Self::set_eth_client(metadata, arc_provider);
            clients.insert(provider.id, eth_client);
        }

        clients
    }

    /// Sets the HTTP client.
    ///
    /// # Returns
    ///
    /// A `Client` instance.
    fn set_client() -> Client {
        Client::new()
    }

    /// Sets an RPC call.
    ///
    /// # Arguments
    ///
    /// * `client` - An HTTP client.
    /// * `rule` - An RPC call rule.
    ///
    /// # Returns
    ///
    /// An `RpcCall` instance.
    fn set_rpc_call(client: Client, rule: RpcCallRule) -> RpcCall {
        RpcCall::new(client, rule)
    }

    /// Sets multiple RPC calls.
    ///
    /// # Arguments
    ///
    /// * `rpc_call_rules` - A vector of `RpcCallRule` instances.
    /// * `rpc_client` - An HTTP client instance.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `RuleID` and the value is an `RpcCall` instance.
    fn set_rpc_calls(
        rpc_call_rules: Vec<RpcCallRule>,
        rpc_client: Client,
    ) -> HashMap<RuleID, RpcCall> {
        rpc_call_rules
            .into_iter()
            .map(|rule| {
                let rpc_call = Self::set_rpc_call(rpc_client.clone(), rule.clone());
                (rule.id, rpc_call)
            })
            .collect()
    }

    /// Sets a contract call.
    ///
    /// # Arguments
    ///
    /// * `client` - An Ethereum client.
    /// * `rule` - A contract call rule.
    ///
    /// # Returns
    ///
    /// A `ContractCall` instance.
    fn set_contract_call<T: JsonRpcClient>(
        client: EthClient<T>,
        rule: ContractCallRule,
    ) -> ContractCall<T> {
        ContractCall::new(client, rule)
    }

    /// Sets multiple RPC calls.
    ///
    /// # Arguments
    ///
    /// * `rpc_call_rules` - A vector of `RpcCallRule` instances.
    /// * `rpc_client` - An HTTP client instance.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `RuleID` and the value is an `RpcCall` instance.
    fn set_contract_calls(
        contract_call_rules: Vec<ContractCallRule>,
        clients: HashMap<ChainID, EthClient<Http>>,
    ) -> HashMap<RuleID, ContractCall<Http>> {
        contract_call_rules
            .into_iter()
            .map(|rule| {
                let contract_call = Self::set_contract_call(
                    clients.get(&rule.chain_id).unwrap().clone(),
                    rule.clone(),
                );
                (rule.id, contract_call)
            })
            .collect()
    }

    /// Sets a contract event.
    ///
    /// # Arguments
    ///
    /// * `client` - An Ethereum client.
    /// * `rule` - A contract event rule.
    ///
    /// # Returns
    ///
    /// A `ContractEvent` instance.
    fn set_contract_event<T: JsonRpcClient>(
        client: EthClient<T>,
        rule: ContractEventRule,
    ) -> ContractEvent<T> {
        ContractEvent::new(client, rule)
    }

    /// Sets multiple contract events.
    ///
    /// # Arguments
    ///
    /// * `contract_event_rules` - A vector of `ContractEventRule` instances.
    /// * `clients` - A hashmap of Ethereum clients.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `RuleID` and the value is a `ContractEvent` instance.
    fn set_contract_events(
        contract_event_rules: Vec<ContractEventRule>,
        clients: HashMap<ChainID, EthClient<Http>>,
    ) -> HashMap<RuleID, ContractEvent<Http>> {
        contract_event_rules
            .into_iter()
            .map(|rule| {
                let contract_event = Self::set_contract_event(
                    clients.get(&rule.chain_id).unwrap().clone(),
                    rule.clone(),
                );
                (rule.id, contract_event)
            })
            .collect()
    }

    /// Sets the RPC call channel.
    ///
    /// # Returns
    ///
    /// A tuple containing the sender and receiver for the RPC call channel.
    fn set_rpc_call_channel() -> (
        UnboundedSender<RpcCallRawMessage>,
        Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel::<RpcCallRawMessage>();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    /// Sets the contract call channel.
    ///
    /// # Returns
    ///
    /// A tuple containing the sender and receiver for the contract call channel.
    fn set_contract_call_channel() -> (
        UnboundedSender<ContractCallRawMessage>,
        Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel::<ContractCallRawMessage>();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    /// Sets the contract event channel.
    ///
    /// # Returns
    ///
    /// A tuple containing the sender and receiver for the contract event channel.
    fn set_contract_event_channel() -> (
        UnboundedSender<ContractEventRawMessage>,
        Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel::<ContractEventRawMessage>();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    /// Sets the RPC call fetcher.
    ///
    /// # Arguments
    ///
    /// * `rpc_call` - An RPC call instance.
    /// * `rpc_call_sender` - The sender for the RPC call channel.
    ///
    /// # Returns
    ///
    /// An `RpcCallFetcher` instance.
    fn set_rpc_call_fetcher(
        rpc_call: RpcCall,
        rpc_call_sender: UnboundedSender<RpcCallRawMessage>,
    ) -> RpcCallFetcher {
        let rpc_call_fetcher = RpcCallFetcher::new(rpc_call, rpc_call_sender);
        rpc_call_fetcher
    }

    /// Sets multiple RPC call fetchers.
    ///
    /// # Arguments
    ///
    /// * `rpc_calls` - A hashmap of RPC call instances.
    /// * `rpc_call_sender` - The sender for the RPC call channel.
    ///
    /// # Returns
    ///
    /// A vector of `RpcCallFetcher` instances.
    fn set_rpc_call_fetchers(
        rpc_calls: HashMap<RuleID, RpcCall>,
        rpc_call_sender: UnboundedSender<RpcCallRawMessage>,
    ) -> Vec<RpcCallFetcher> {
        rpc_calls
            .into_iter()
            .map(|(_, rpc_call)| Self::set_rpc_call_fetcher(rpc_call, rpc_call_sender.clone()))
            .collect()
    }

    /// Sets the contract call fetcher.
    ///
    /// # Arguments
    ///
    /// * `contract_call` - A contract call instance.
    /// * `contract_call_sender` - The sender for the contract call channel.
    ///
    /// # Returns
    ///
    /// A `ContractCallFetcher` instance.
    fn set_contract_call_fetcher<T: JsonRpcClient>(
        contract_call: ContractCall<T>,
        contract_call_sender: UnboundedSender<ContractCallRawMessage>,
    ) -> ContractCallFetcher<T> {
        let contract_call_fetcher = ContractCallFetcher::new(contract_call, contract_call_sender);
        contract_call_fetcher
    }

    /// Sets multiple contract call fetchers.
    ///
    /// # Arguments
    ///
    /// * `contract_calls` - A hashmap of contract call instances.
    /// * `contract_call_sender` - The sender for the contract call channel.
    ///
    /// # Returns
    ///
    /// A vector of `ContractCallFetcher` instances.
    fn set_contract_call_fetchers(
        contract_calls: HashMap<RuleID, ContractCall<Http>>,
        contract_call_sender: UnboundedSender<ContractCallRawMessage>,
    ) -> Vec<ContractCallFetcher<Http>> {
        contract_calls
            .into_iter()
            .map(|(_, contract_call)| {
                Self::set_contract_call_fetcher(contract_call, contract_call_sender.clone())
            })
            .collect()
    }

    /// Sets the contract event fetcher.
    ///
    /// # Arguments
    ///
    /// * `contract_event` - A contract event instance.
    /// * `contract_event_sender` - The sender for the contract event channel.
    /// * `waiting_block_number` - The waiting block number.
    ///
    /// # Returns
    ///
    /// A `ContractEventFetcher` instance.
    fn set_contract_event_fetcher<T: JsonRpcClient>(
        contract_event: ContractEvent<T>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
        waiting_block_number: U64,
    ) -> ContractEventFetcher<T> {
        let event_fetcher =
            ContractEventFetcher::new(contract_event, contract_event_sender, waiting_block_number);
        event_fetcher
    }

    /// Sets multiple contract event fetchers.
    ///
    /// # Arguments
    ///
    /// * `contract_events` - A hashmap of contract event instances.
    /// * `contract_event_blocks` - A hashmap of contract event blocks.
    /// * `contract_event_sender` - The sender for the contract event channel.
    ///
    /// # Returns
    ///
    /// A vector of `ContractEventFetcher` instances.
    fn set_contract_event_fetchers(
        contract_events: HashMap<RuleID, ContractEvent<Http>>,
        contract_event_blocks: HashMap<RuleID, U64>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
    ) -> Vec<ContractEventFetcher<Http>> {
        contract_events
            .clone()
            .into_iter()
            .map(|(_, contract_event)| {
                let waiting_block_number =
                    contract_event_blocks.get(&contract_event.rule.id).unwrap();

                Self::set_contract_event_fetcher(
                    contract_event,
                    contract_event_sender.clone(),
                    *waiting_block_number,
                )
            })
            .collect()
    }

    /// Sets the RPC call manager.
    ///
    /// # Arguments
    ///
    /// * `rpc_calls` - A hashmap of RPC call instances.
    /// * `rpc_call_receiver` - The receiver for the RPC call channel.
    /// * `db_client` - A Postgres client instance.
    ///
    /// # Returns
    ///
    /// An `RpcCallManager` instance.
    fn set_rpc_call_manager(
        rpc_calls: HashMap<RuleID, RpcCall>,
        rpc_call_receiver: Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
        db_client: PostgresClient,
    ) -> RpcCallManager {
        let rpc_call_manager = RpcCallManager::new(rpc_calls, rpc_call_receiver, db_client);
        rpc_call_manager
    }

    /// Sets the contract call manager.
    ///
    /// # Arguments
    ///
    /// * `contract_calls` - A hashmap of contract call instances.
    /// * `contract_call_receiver` - The receiver for the contract call channel.
    /// * `db_client` - A Postgres client instance.
    ///
    /// # Returns
    ///
    /// A `ContractCallManager` instance.
    fn set_contract_call_manager<T: JsonRpcClient>(
        contract_calls: HashMap<RuleID, ContractCall<T>>,
        contract_call_receiver: Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
        db_client: PostgresClient,
    ) -> ContractCallManager<T> {
        let contract_call_manager =
            ContractCallManager::new(contract_calls, contract_call_receiver, db_client);
        contract_call_manager
    }

    /// Sets the contract event manager.
    ///
    /// # Arguments
    ///
    /// * `contract_events` - A hashmap of contract event instances.
    /// * `contract_event_receiver` - The receiver for the contract event channel.
    /// * `db_client` - A Postgres client instance.
    ///
    /// # Returns
    ///
    /// A `ContractEventManager` instance.
    fn set_contract_event_manager<T: JsonRpcClient>(
        contract_events: HashMap<RuleID, ContractEvent<T>>,
        contract_event_receiver: Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
        db_client: PostgresClient,
    ) -> ContractEventManager<T> {
        let contract_event_manager =
            ContractEventManager::new(contract_events, contract_event_receiver, db_client);
        contract_event_manager
    }

    /// Sets the log configuration.
    fn set_log() {
        let format = fmt::format()
            .with_timer(fmt::time::ChronoLocal::new(TIME_FORMAT.to_string()))
            .with_level(true)
            .with_target(true)
            .with_ansi(true)
            .compact();

        tracing_subscriber::fmt()
            .event_format(format)
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(Level::INFO.into())
                    .add_directive(SQLX_QUERY_OFF.parse().unwrap()), // Exclude sqlx::query logs
            )
            .init();
    }

    /// Sets up the database client.
    ///
    /// # Arguments
    ///
    /// * `db_url` - The URL of the database.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `PostgresClient` instance.
    async fn set_db(db_url: &str) -> anyhow::Result<PostgresClient> {
        let client = PostgresClient::new(db_url).await?;
        Ok(client)
    }

    /// Sets the configuration.
    ///
    /// # Arguments
    ///
    /// * `spec` - The path to the configuration file.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Configuration` instance.
    pub fn set_config(spec: &str) -> anyhow::Result<Configuration> {
        let user_config_file = std::fs::File::open(spec).expect(INVALID_CONFIG_FILE_PATH);
        let user_config: Configuration =
            serde_yaml::from_reader(user_config_file).expect(INVALID_CONFIG_FILE_STRUCTURE);

        Ok(user_config)
    }
}
