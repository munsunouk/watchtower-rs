use ethers::providers::{Http, JsonRpcClient, Provider};
use ethers::types::U64;
use futures::future::join_all;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

use watch_tower_lib::cli::eth::{EthClient, ProviderMetadata};
use watch_tower_lib::db::postgres::PostgresClient;
use watch_tower_lib::utils::constants::ChainID;
use watch_tower_lib::utils::error::{ClientError, DatabaseError};

use crate::rule::contract_call::ContractCallBlockLog;
use crate::utils::constants::{
    DB_CONTRACT_CALL_BLOCK_LOG, DEFAULT_BLOCK_NUMBER, DEFAULT_CALL_TIME_INTERVAL,
};
use crate::utils::error::WorkerError;
use crate::{
    fetcher::{
        contract_call::ContractCallFetcher, contract_event::ContractEventFetcher,
        rpc_call::RpcCallFetcher,
    },
    manager::{
        contract_call::ContractCallManager, contract_event::ContractEventManager,
        rpc_call::RpcCallManager,
    },
    rule::{
        contract_call::ContractCallRule,
        contract_event::{ContractEventBlockLog, ContractEventRule},
        rpc_call::RpcCallRule,
        ContractCall, ContractEvent, RpcCall,
    },
    utils::{
        config::{Configuration, EVMProvider},
        constants::{
            RuleID, DB_CONTRACT_CALL_RULE, DB_CONTRACT_EVENT_RULE, DB_RPC_CALL_RULE,
            SQLX_QUERY_WARN, TIME_FORMAT,
        },
        msg::{ContractCallRawMessage, ContractEventRawMessage, RpcCallRawMessage},
        traits::Fetcher,
    },
};

/// A Watchtower CLI runtime that can be used to run
pub struct Runner {
    /// Fetchers for RPC calls.
    pub rpc_call_fetchers: Vec<RpcCallFetcher>,
    /// Fetchers for contract calls.
    pub contract_call_fetchers: Vec<ContractCallFetcher<Http>>,
    /// Fetchers for contract events.
    pub contract_event_fetchers: Vec<ContractEventFetcher<Http>>,
    /// Manager for RPC calls.
    pub rpc_call_manager: RpcCallManager,
    /// Manager for contract events.
    pub contract_call_manager: ContractCallManager<Http>,
    /// Manager for contract calls.
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

        //Channel
        let (rpc_call_sender, rpc_call_receiver) = Self::set_rpc_call_channel();
        let (contract_call_sender, contract_call_receiver) = Self::set_contract_call_channel();
        let (contract_event_sender, contract_event_receiver) = Self::set_contract_event_channel();

        //Config
        let config: Configuration = Self::set_config(config_path).unwrap();
        let chain_intervals = Self::set_chain_intervals(config.clone().evm_providers);

        //DB
        let db_client = Self::set_db(&config.postgres_config.url).await.unwrap();

        //DB Rule
        let rpc_call_rules = Self::load_rpc_call_rules(&db_client).await;
        let contract_call_rules = Self::load_contract_call_rules(&db_client).await;
        let contract_call_blocks = Self::load_contract_call_block_logs(&db_client).await;
        let contract_event_rules = Self::load_contract_event_rules(&db_client).await;
        let contract_event_blocks = Self::load_contract_event_block_logs(&db_client).await;

        //Client
        let rpc_client = Self::set_client();
        let clients = Self::set_eth_clients(config.evm_providers);

        //Rule
        let rpc_calls = Self::set_rpc_calls(rpc_call_rules, rpc_client);
        let contract_calls = Self::set_contract_calls(contract_call_rules, clients.clone());
        let contract_events = Self::set_contract_events(contract_event_rules, clients.clone());
        let contract_chain_events = Self::set_contract_chain_events(contract_events.clone());

        //Fetcher
        let rpc_call_fetchers = Self::set_rpc_call_fetchers(rpc_calls.clone(), rpc_call_sender);
        let contract_call_fetchers = Self::set_contract_call_fetchers(
            contract_calls.clone(),
            contract_call_sender,
            contract_call_blocks.clone(),
            chain_intervals.clone(),
        );
        let contract_event_fetchers = Self::set_contract_event_fetchers(
            clients,
            contract_chain_events,
            contract_event_sender,
            contract_event_blocks.clone(),
            chain_intervals,
        );

        //Manager
        let rpc_call_manager = Self::set_rpc_call_manager(
            rpc_calls.clone(),
            rpc_call_receiver.clone(),
            db_client.clone(),
        );
        let contract_call_manager = Self::set_contract_call_manager(
            contract_calls.clone(),
            contract_call_receiver.clone(),
            db_client.clone(),
        );
        let contract_event_manager = Self::set_contract_event_manager(
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
    pub async fn run(&self) -> Result<(), WorkerError> {
        let tasks = self.set_tasks();

        // Await all futures
        join_all(tasks).await;

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
        let result = db_client.select_table(DB_RPC_CALL_RULE).await.unwrap();

        let rpc_calls: Vec<RpcCallRule> = result.iter().map(|row| row.into()).collect();
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
        let result = db_client.select_table(DB_CONTRACT_CALL_RULE).await.unwrap();

        let contract_calls: Vec<ContractCallRule> = result.iter().map(|row| row.into()).collect();
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
        let result = db_client
            .select_table(DB_CONTRACT_EVENT_RULE)
            .await
            .unwrap();

        let contract_events: Vec<ContractEventRule> = result.iter().map(|row| row.into()).collect();
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
    /// A hashmap of `ChainID` to a hashmap of `RuleID` to `U64`.
    pub async fn load_contract_event_block_logs(
        db_client: &PostgresClient,
    ) -> HashMap<ChainID, HashMap<RuleID, U64>> {
        let result = db_client.select_join_event_rule_chain_id().await.unwrap();

        let mut contract_events: HashMap<ChainID, HashMap<RuleID, U64>> = HashMap::new();

        for row in result {
            let block_log = ContractEventBlockLog::from(&row);
            contract_events
                .entry(block_log.chain_id)
                .or_insert_with(HashMap::new)
                .insert(block_log.id, block_log.block_number);
        }

        contract_events
    }

    pub async fn load_contract_call_block_logs(db_client: &PostgresClient) -> HashMap<RuleID, U64> {
        let result = db_client
            .select_table(DB_CONTRACT_CALL_BLOCK_LOG)
            .await
            .unwrap();

        let mut contract_calls: HashMap<RuleID, U64> = HashMap::new();

        for row in result {
            let block_log = ContractCallBlockLog::from(&row);
            contract_calls.insert(block_log.id, block_log.block_number);
        }

        contract_calls
    }

    /// Sets the metadata for a provider.
    ///
    /// # Arguments
    ///
    /// * `chain_name` - The name of the chain.
    /// * `chain_url` - The URL of the chain.
    /// * `chain_id` - The ID of the chain.
    ///
    /// # Returns
    ///
    /// A `ProviderMetadata` instance.
    fn set_metadata(
        chain_name: String,
        chain_urls: Vec<String>,
        chain_id: ChainID,
    ) -> ProviderMetadata {
        ProviderMetadata::new(chain_name, chain_urls, chain_id)
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
        let provider =
            Provider::<Http>::try_from(url).expect(&ClientError::InvalidProviderURL.to_string());
        Arc::new(provider)
    }

    fn set_providers(urls: &Vec<String>) -> Vec<Arc<Provider<Http>>> {
        urls.into_iter()
            .map(|url| Self::set_provider(url))
            .collect()
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
        providers: Vec<Arc<Provider<T>>>,
    ) -> EthClient<T> {
        EthClient::new(metadata, providers)
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
            );

            let arc_providers = Self::set_providers(&provider.provider);

            let eth_client = Self::set_eth_client(metadata, arc_providers);
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

    fn set_chain_intervals(providers: Vec<EVMProvider>) -> HashMap<ChainID, u64> {
        let mut chain_intervals = HashMap::new();

        for provider in providers {
            chain_intervals.insert(provider.id, provider.call_time_interval);
        }

        chain_intervals
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

    fn set_contract_chain_events(
        contract_events: HashMap<RuleID, ContractEvent<Http>>,
    ) -> HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>> {
        let mut result: HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>> = HashMap::new();

        for (rule_id, contract_event) in contract_events {
            let chain_id = contract_event.rule.chain_id;
            result
                .entry(chain_id)
                .or_insert_with(HashMap::new)
                .insert(rule_id, contract_event);
        }

        result
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
        from_block_number: U64,
        call_time_interval: u64,
    ) -> ContractCallFetcher<T> {
        let contract_call_fetcher = ContractCallFetcher::new(
            contract_call,
            contract_call_sender,
            from_block_number,
            call_time_interval,
        );
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
        from_block_numbers: HashMap<RuleID, U64>,
        call_time_intervals: HashMap<ChainID, u64>,
    ) -> Vec<ContractCallFetcher<Http>> {
        contract_calls
            .into_iter()
            .map(|(rule_id, contract_call)| {
                let default_block_number = U64::from(DEFAULT_BLOCK_NUMBER);
                let from_block_number = from_block_numbers
                    .get(&rule_id)
                    .unwrap_or(&default_block_number);
                let call_time_interval = call_time_intervals
                    .get(&contract_call.rule.chain_id)
                    .unwrap_or(&DEFAULT_CALL_TIME_INTERVAL);

                Self::set_contract_call_fetcher(
                    contract_call,
                    contract_call_sender.clone(),
                    *from_block_number,
                    *call_time_interval,
                )
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
        client: EthClient<T>,
        contract_events: HashMap<RuleID, ContractEvent<T>>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
        from_block_numbers: HashMap<RuleID, U64>,
        call_time_interval: u64,
    ) -> ContractEventFetcher<T> {
        let event_fetcher = ContractEventFetcher::new(
            client,
            contract_events,
            contract_event_sender,
            from_block_numbers,
            call_time_interval,
        );
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
        chain_clients: HashMap<ChainID, EthClient<Http>>,
        contract_chain_events: HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
        contract_event_blocks: HashMap<ChainID, HashMap<RuleID, U64>>,
        call_time_intervals: HashMap<ChainID, u64>,
    ) -> Vec<ContractEventFetcher<Http>> {
        contract_chain_events
            .clone()
            .into_iter()
            .map(|(chain_id, contract_events)| {
                let mut default_block_numbers = HashMap::new();
                default_block_numbers.insert(
                    chain_id.try_into().unwrap(),
                    U64::from(DEFAULT_BLOCK_NUMBER),
                );
                let from_block_numbers = contract_event_blocks
                    .get(&chain_id)
                    .unwrap_or(&default_block_numbers);
                let call_time_interval = call_time_intervals
                    .get(&chain_id)
                    .unwrap_or(&DEFAULT_CALL_TIME_INTERVAL);

                let client = chain_clients.get(&chain_id).unwrap();

                Self::set_contract_event_fetcher(
                    client.clone(),
                    contract_events,
                    contract_event_sender.clone(),
                    from_block_numbers.clone(),
                    *call_time_interval,
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
                    .add_directive(SQLX_QUERY_WARN.parse().unwrap()), // Exclude sqlx::query logs
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
    async fn set_db(db_url: &str) -> Result<PostgresClient, DatabaseError> {
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
    pub fn set_config(spec: &str) -> Result<Configuration, WorkerError> {
        let user_config_file =
            std::fs::File::open(spec).expect(&WorkerError::InvalidConfigFilePath.to_string());
        let user_config: Configuration = serde_yaml::from_reader(user_config_file)
            .expect(&WorkerError::InvalidConfigFileStructure.to_string());

        Ok(user_config)
    }

    pub fn set_tasks(&self) -> Vec<JoinHandle<()>> {
        let mut tasks = Vec::new();

        // Add RPC call fetcher tasks by Each Rule
        for mut fetcher in self.rpc_call_fetchers.clone() {
            tasks.push(tokio::spawn(async move { fetcher.run().await }));
        }

        // Add contract call fetcher tasks by Each Rule
        for mut fetcher in self.contract_call_fetchers.clone() {
            tasks.push(tokio::spawn(async move { fetcher.run().await }));
        }

        // Add contract event fetcher tasks by Each Chain
        for mut fetcher in self.contract_event_fetchers.clone() {
            tasks.push(tokio::spawn(async move { fetcher.run().await }));
        }

        // Add manager tasks
        let mut rpc_call_manager = self.rpc_call_manager.clone();
        tasks.push(tokio::spawn(async move { rpc_call_manager.run().await }));

        // Add contract call manager task
        let mut contract_call_manager = self.contract_call_manager.clone();
        tasks.push(tokio::spawn(
            async move { contract_call_manager.run().await },
        ));

        // Add contract event manager task
        let mut contract_event_manager = self.contract_event_manager.clone();
        tasks.push(tokio::spawn(
            async move { contract_event_manager.run().await },
        ));

        tasks
    }
}
