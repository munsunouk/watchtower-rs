use ethers::{
    providers::{Http, JsonRpcClient, Provider},
    types::U64,
};
use futures::{future::join_all, FutureExt};
use reqwest::Client;

use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};

use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

use watch_tower_lib::{
    cli::eth::{EthClient, ProviderMetadata},
    db::postgres::PostgresClient,
    utils::{
        error::ClientError,
        types::{ChainID, RuleID},
    },
};

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
        contract_call::{ContractCallBlockLog, ContractCallRule},
        contract_event::{ContractEventBlockLog, ContractEventRule},
        rpc_call::RpcCallRule,
        ContractCall, ContractEvent, RpcCall,
    },
    utils::{
        config::{Configuration, EVMProvider},
        constants::{
            DB_CONTRACT_CALL_BLOCK_LOG, DB_CONTRACT_CALL_RULE, DB_CONTRACT_EVENT_RULE,
            DB_RPC_CALL_RULE, DEFAULT_BLOCK_NUMBER, DEFAULT_CALL_TIME_INTERVAL, SQLX_QUERY_WARN,
            TIME_FORMAT,
        },
        error::{IndexType, WorkerError},
        msg::{ContractCallRawMessage, ContractEventRawMessage, RpcCallRawMessage},
        traits::{Fetcher, Manager},
    },
};

/// A Watchtower CLI runtime that can be used to run
pub struct Runner {
    /// The fetchers.
    pub fetchers: Vec<Arc<Mutex<dyn Fetcher>>>,
    /// The managers.
    pub managers: Vec<Arc<Mutex<dyn Manager>>>,
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
    pub async fn new(config_path: &str) -> Result<Self, WorkerError> {
        Self::set_log()?;

        //Channel
        let (rpc_call_sender, rpc_call_receiver) = Self::build_rpc_call_channel();
        let (contract_call_sender, contract_call_receiver) = Self::build_contract_call_channel();
        let (contract_event_sender, contract_event_receiver) = Self::build_contract_event_channel();

        //Config
        let config = Self::set_config(config_path);
        let chain_intervals = Self::set_chain_intervals(config.clone().evm_providers);

        //DB
        let db_client = Self::build_db(&config.postgres_config.url).await?;

        //DB Rule
        let rpc_call_rules = Self::load_rpc_call_rules(&db_client).await?;
        let contract_call_rules = Self::load_contract_call_rules(&db_client).await?;
        let contract_call_blocks = Self::load_contract_call_block_logs(&db_client).await?;
        let contract_event_rules = Self::load_contract_event_rules(&db_client).await?;
        let contract_event_blocks = Self::load_contract_event_block_logs(&db_client).await?;

        //Client
        let rpc_client = Self::build_rpc_client();
        let clients = Self::build_eth_clients(config.evm_providers);

        //Rule
        let rpc_calls = Self::build_rpc_calls(rpc_call_rules, rpc_client);
        let contract_calls = Self::build_contract_calls(contract_call_rules, clients.clone())?;
        let contract_events = Self::build_contract_events(contract_event_rules, clients.clone())?;
        let contract_chain_events = Self::build_contract_chain_events(contract_events.clone());

        //Fetcher
        let fetchers = Self::build_fetchers(
            rpc_calls.clone(),
            rpc_call_sender,
            contract_calls.clone(),
            contract_call_sender,
            contract_call_blocks,
            contract_chain_events,
            contract_event_sender,
            contract_event_blocks,
            chain_intervals,
            clients,
        )?;

        //Manager
        let managers = Self::build_managers(
            rpc_calls.clone(),
            rpc_call_receiver.clone(),
            contract_calls.clone(),
            contract_call_receiver.clone(),
            contract_events.clone(),
            contract_event_receiver.clone(),
            db_client.clone(),
        )?;

        Ok(Self { fetchers, managers })
    }

    /// Runs the `Runner` instance.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub async fn run(&self) -> Result<(), WorkerError> {
        let tasks = self.spawn_tasks();

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
    pub async fn load_rpc_call_rules(
        db_client: &PostgresClient,
    ) -> Result<Vec<RpcCallRule>, WorkerError> {
        let result = db_client
            .select_table(DB_RPC_CALL_RULE)
            .await
            .map_err(|e| WorkerError::InvalidDatabase(e.to_string()))?;

        let rpc_calls: Vec<RpcCallRule> = result.iter().map(|row| row.into()).collect();
        Ok(rpc_calls)
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
    pub async fn load_contract_call_rules(
        db_client: &PostgresClient,
    ) -> Result<Vec<ContractCallRule>, WorkerError> {
        let result = db_client
            .select_table(DB_CONTRACT_CALL_RULE)
            .await
            .map_err(|e| WorkerError::InvalidDatabase(e.to_string()))?;

        let contract_calls: Vec<ContractCallRule> = result.iter().map(|row| row.into()).collect();
        Ok(contract_calls)
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
    pub async fn load_contract_event_rules(
        db_client: &PostgresClient,
    ) -> Result<Vec<ContractEventRule>, WorkerError> {
        let result = db_client
            .select_table(DB_CONTRACT_EVENT_RULE)
            .await
            .map_err(|e| WorkerError::InvalidDatabase(e.to_string()))?;

        let contract_events: Vec<ContractEventRule> = result.iter().map(|row| row.into()).collect();
        Ok(contract_events)
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
    ) -> Result<HashMap<ChainID, HashMap<RuleID, U64>>, WorkerError> {
        let result = db_client
            .select_join_event_rule_chain_id()
            .await
            .map_err(|e| WorkerError::InvalidDatabase(e.to_string()))?;

        let mut contract_events: HashMap<ChainID, HashMap<RuleID, U64>> = HashMap::new();

        for row in result {
            let block_log = ContractEventBlockLog::from(&row);
            contract_events
                .entry(block_log.chain_id)
                .or_default()
                .insert(block_log.id, block_log.block_number);
        }

        Ok(contract_events)
    }

    pub async fn load_contract_call_block_logs(
        db_client: &PostgresClient,
    ) -> Result<HashMap<RuleID, U64>, WorkerError> {
        let result = db_client
            .select_table(DB_CONTRACT_CALL_BLOCK_LOG)
            .await
            .map_err(|e| WorkerError::InvalidDatabase(e.to_string()))?;

        let mut contract_calls: HashMap<RuleID, U64> = HashMap::new();

        for row in result {
            let block_log = ContractCallBlockLog::from(&row);
            contract_calls.insert(block_log.id, block_log.block_number);
        }

        Ok(contract_calls)
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
    fn build_provider(url: &str) -> Arc<Provider<Http>> {
        let provider = Provider::<Http>::try_from(url)
            .unwrap_or_else(|_| panic!("{}", ClientError::InvalidProviderURL.to_string()));
        Arc::new(provider)
    }

    fn set_providers(urls: &[String]) -> Vec<Arc<Provider<Http>>> {
        urls.iter().map(|url| Self::build_provider(url)).collect()
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
    fn build_eth_client<T: JsonRpcClient>(
        metadata: ProviderMetadata,
        providers: Vec<Arc<Provider<T>>>,
    ) -> EthClient<T> {
        EthClient::new(metadata, providers)
    }

    /// Builds multiple Ethereum clients.
    ///
    /// # Arguments
    ///
    /// * `providers` - A vector of `EVMProvider` instances.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `ChainID` and the value is an `EthClient<Http>` instance.
    fn build_eth_clients(providers: Vec<EVMProvider>) -> HashMap<ChainID, EthClient<Http>> {
        let mut clients = HashMap::new();

        for provider in providers {
            let metadata = Self::set_metadata(
                provider.name.clone(),
                provider.provider.clone(),
                provider.id,
            );

            let arc_providers = Self::set_providers(&provider.provider);

            let eth_client = Self::build_eth_client(metadata, arc_providers);
            clients.insert(provider.id, eth_client);
        }

        clients
    }

    /// Sets the HTTP client.
    ///
    /// # Returns
    ///
    /// A `Client` instance.
    fn build_rpc_client() -> Arc<Client> {
        Arc::new(Client::new())
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
    fn build_rpc_call(client: Arc<Client>, rule: RpcCallRule) -> RpcCall {
        RpcCall::new(client, rule)
    }

    /// Builds multiple RPC calls.
    ///
    /// # Arguments
    ///
    /// * `rpc_call_rules` - A vector of `RpcCallRule` instances.
    /// * `rpc_client` - An HTTP client instance.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `RuleID` and the value is an `RpcCall` instance.
    fn build_rpc_calls(
        rpc_call_rules: Vec<RpcCallRule>,
        rpc_client: Arc<Client>,
    ) -> HashMap<RuleID, RpcCall> {
        rpc_call_rules
            .into_iter()
            .map(|rule| {
                let rpc_call = Self::build_rpc_call(rpc_client.clone(), rule.clone());
                (rule.id, rpc_call)
            })
            .collect()
    }

    /// Builds a contract call.
    ///
    /// # Arguments
    ///
    /// * `client` - An Ethereum client.
    /// * `rule` - A contract call rule.
    ///
    /// # Returns
    ///
    /// A `ContractCall` instance.
    fn build_contract_call<T: JsonRpcClient>(
        client: EthClient<T>,
        rule: ContractCallRule,
    ) -> ContractCall<T> {
        ContractCall::new(client, rule)
    }

    /// Builds multiple contract calls.
    ///
    /// # Arguments
    ///
    /// * `rpc_call_rules` - A vector of `RpcCallRule` instances.
    /// * `rpc_client` - An HTTP client instance.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `RuleID` and the value is an `RpcCall` instance.
    fn build_contract_calls(
        contract_call_rules: Vec<ContractCallRule>,
        clients: HashMap<ChainID, EthClient<Http>>,
    ) -> Result<HashMap<RuleID, ContractCall<Http>>, WorkerError> {
        contract_call_rules
            .into_iter()
            .map(|rule| {
                let client = clients
                    .get(&rule.chain_id)
                    .ok_or(WorkerError::InvalidIndex(IndexType::U32(rule.chain_id)))?;
                let contract_call = Self::build_contract_call(client.clone(), rule.clone());
                Ok((rule.id, contract_call))
            })
            .collect()
    }

    /// Builds a contract event.
    ///
    /// # Arguments
    ///
    /// * `client` - An Ethereum client.
    /// * `rule` - A contract event rule.
    ///
    /// # Returns
    ///
    /// A `ContractEvent` instance.
    fn build_contract_event<T: JsonRpcClient>(
        client: EthClient<T>,
        rule: ContractEventRule,
    ) -> ContractEvent<T> {
        ContractEvent::new(client, rule)
    }

    /// Builds multiple contract events.
    ///
    /// # Arguments
    ///
    /// * `contract_event_rules` - A vector of `ContractEventRule` instances.
    /// * `clients` - A hashmap of Ethereum clients.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the key is a `RuleID` and the value is a `ContractEvent` instance.
    fn build_contract_events(
        contract_event_rules: Vec<ContractEventRule>,
        clients: HashMap<ChainID, EthClient<Http>>,
    ) -> Result<HashMap<RuleID, ContractEvent<Http>>, WorkerError> {
        contract_event_rules
            .into_iter()
            .map(|rule| {
                let client = clients
                    .get(&rule.chain_id)
                    .ok_or(WorkerError::InvalidIndex(IndexType::U32(rule.chain_id)))?;

                let contract_event = Self::build_contract_event(client.clone(), rule.clone());
                Ok((rule.id, contract_event))
            })
            .collect()
    }

    /// Builds a contract chain event.
    ///
    /// # Arguments
    ///
    /// * `contract_events` - A hashmap of contract events.
    ///
    /// # Returns
    ///
    /// A hashmap of `ChainID` to a hashmap of `RuleID` to `ContractEvent`.
    fn build_contract_chain_events(
        contract_events: HashMap<RuleID, ContractEvent<Http>>,
    ) -> HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>> {
        let mut result: HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>> = HashMap::new();

        for (rule_id, contract_event) in contract_events {
            let chain_id = contract_event.rule.chain_id;
            result
                .entry(chain_id)
                .or_default()
                .insert(rule_id, contract_event);
        }

        result
    }

    /// Builds the RPC call channel.
    ///
    /// # Returns
    ///
    /// A tuple containing the sender and receiver for the RPC call channel.
    fn build_rpc_call_channel() -> (
        UnboundedSender<RpcCallRawMessage>,
        Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel::<RpcCallRawMessage>();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    /// Builds the contract call channel.
    ///
    /// # Returns
    ///
    /// A tuple containing the sender and receiver for the contract call channel.
    fn build_contract_call_channel() -> (
        UnboundedSender<ContractCallRawMessage>,
        Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel::<ContractCallRawMessage>();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    /// Builds the contract event channel.
    ///
    /// # Returns
    ///
    /// A tuple containing the sender and receiver for the contract event channel.
    fn build_contract_event_channel() -> (
        UnboundedSender<ContractEventRawMessage>,
        Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel::<ContractEventRawMessage>();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    /// Builds the fetchers.
    ///
    /// # Arguments
    ///
    /// * `rpc_calls` - A hashmap of RPC call instances.
    /// * `rpc_call_sender` - The sender for the RPC call channel.
    /// * `contract_calls` - A hashmap of contract call instances.
    /// * `contract_call_sender` - The sender for the contract call channel.
    /// * `contract_call_blocks` - A hashmap of contract call blocks.
    /// * `contract_chain_events` - A hashmap of contract chain events.
    /// * `contract_event_sender` - The sender for the contract event channel.
    /// * `contract_event_blocks` - A hashmap of contract event blocks.
    /// * `chain_intervals` - A hashmap of chain intervals.
    /// * `clients` - A hashmap of Ethereum clients.
    ///
    /// # Returns
    ///
    /// A vector of `Arc<Mutex<dyn Fetcher>>` instances.
    fn build_fetchers(
        rpc_calls: HashMap<RuleID, RpcCall>,
        rpc_call_sender: UnboundedSender<RpcCallRawMessage>,
        contract_calls: HashMap<RuleID, ContractCall<Http>>,
        contract_call_sender: UnboundedSender<ContractCallRawMessage>,
        contract_call_blocks: HashMap<RuleID, U64>,
        contract_chain_events: HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
        contract_event_blocks: HashMap<ChainID, HashMap<RuleID, U64>>,
        chain_intervals: HashMap<ChainID, u64>,
        clients: HashMap<ChainID, EthClient<Http>>,
    ) -> Result<Vec<Arc<Mutex<dyn Fetcher>>>, WorkerError> {
        let mut fetchers: Vec<Arc<Mutex<dyn Fetcher>>> = Vec::new();
        fetchers.extend(
            Self::build_rpc_call_fetchers(rpc_calls.clone(), rpc_call_sender)
                .into_iter()
                .map(|f| Arc::new(Mutex::new(f)) as Arc<Mutex<dyn Fetcher>>),
        );
        fetchers.extend(
            Self::build_contract_call_fetchers(
                contract_calls.clone(),
                contract_call_sender,
                contract_call_blocks.clone(),
                chain_intervals.clone(),
            )
            .into_iter()
            .map(|f| Arc::new(Mutex::new(f)) as Arc<Mutex<dyn Fetcher>>),
        );
        fetchers.extend(
            Self::build_contract_event_fetchers(
                clients,
                contract_chain_events,
                contract_event_sender,
                contract_event_blocks.clone(),
                chain_intervals,
            )?
            .into_iter()
            .map(|f| Arc::new(Mutex::new(f)) as Arc<Mutex<dyn Fetcher>>),
        );

        Ok(fetchers)
    }

    /// Builds the managers.
    ///
    /// # Arguments
    ///
    /// * `rpc_calls` - A hashmap of RPC call instances.
    /// * `rpc_call_receiver` - The receiver for the RPC call channel.
    /// * `contract_calls` - A hashmap of contract call instances.
    /// * `contract_call_receiver` - The receiver for the contract call channel.
    /// * `contract_events` - A hashmap of contract event instances.
    /// * `contract_event_receiver` - The receiver for the contract event channel.
    /// * `db_client` - A database client.
    ///
    /// # Returns
    ///
    /// A vector of `Arc<Mutex<dyn Manager>>` instances.
    fn build_managers(
        rpc_calls: HashMap<RuleID, RpcCall>,
        rpc_call_receiver: Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
        contract_calls: HashMap<RuleID, ContractCall<Http>>,
        contract_call_receiver: Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
        contract_events: HashMap<RuleID, ContractEvent<Http>>,
        contract_event_receiver: Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
        db_client: PostgresClient,
    ) -> Result<Vec<Arc<Mutex<dyn Manager>>>, WorkerError> {
        let rpc_manager = Arc::new(Mutex::new(Self::build_rpc_call_manager(
            rpc_calls,
            rpc_call_receiver,
            db_client.clone(),
        )));
        let contract_manager = Arc::new(Mutex::new(Self::build_contract_call_manager(
            contract_calls,
            contract_call_receiver,
            db_client.clone(),
        )));
        let contract_event_manager = Arc::new(Mutex::new(Self::build_contract_event_manager(
            contract_events,
            contract_event_receiver,
            db_client,
        )));

        let managers: Vec<Arc<Mutex<dyn Manager>>> =
            vec![rpc_manager, contract_manager, contract_event_manager];

        Ok(managers)
    }

    /// Builds the RPC call fetcher.
    ///
    /// # Arguments
    ///
    /// * `rpc_call` - An RPC call instance.
    /// * `rpc_call_sender` - The sender for the RPC call channel.
    ///
    /// # Returns
    ///
    /// An `RpcCallFetcher` instance.
    fn build_rpc_call_fetcher(
        rpc_call: RpcCall,
        rpc_call_sender: UnboundedSender<RpcCallRawMessage>,
    ) -> RpcCallFetcher {
        RpcCallFetcher::new(rpc_call, rpc_call_sender)
    }

    /// Builds multiple RPC call fetchers.
    ///
    /// # Arguments
    ///
    /// * `rpc_calls` - A hashmap of RPC call instances.
    /// * `rpc_call_sender` - The sender for the RPC call channel.
    ///
    /// # Returns
    ///
    /// A vector of `RpcCallFetcher` instances.
    fn build_rpc_call_fetchers(
        rpc_calls: HashMap<RuleID, RpcCall>,
        rpc_call_sender: UnboundedSender<RpcCallRawMessage>,
    ) -> Vec<RpcCallFetcher> {
        rpc_calls
            .into_values()
            .map(|rpc_call| Self::build_rpc_call_fetcher(rpc_call, rpc_call_sender.clone()))
            .collect()
    }

    /// Builds the contract call fetcher.
    ///
    /// # Arguments
    ///
    /// * `contract_call` - A contract call instance.
    /// * `contract_call_sender` - The sender for the contract call channel.
    ///
    /// # Returns
    ///
    /// A `ContractCallFetcher` instance.
    fn build_contract_call_fetcher<T: JsonRpcClient>(
        contract_call: ContractCall<T>,
        contract_call_sender: UnboundedSender<ContractCallRawMessage>,
        from_block: U64,
        call_time_interval: u64,
    ) -> ContractCallFetcher<T> {
        ContractCallFetcher::new(
            contract_call,
            contract_call_sender,
            from_block,
            call_time_interval,
        )
    }

    /// Builds multiple contract call fetchers.
    ///
    /// # Arguments
    ///
    /// * `contract_calls` - A hashmap of contract call instances.
    /// * `contract_call_sender` - The sender for the contract call channel.
    ///
    /// # Returns
    ///
    /// A vector of `ContractCallFetcher` instances.
    fn build_contract_call_fetchers(
        contract_calls: HashMap<RuleID, ContractCall<Http>>,
        contract_call_sender: UnboundedSender<ContractCallRawMessage>,
        from_blocks: HashMap<RuleID, U64>,
        call_time_intervals: HashMap<ChainID, u64>,
    ) -> Vec<ContractCallFetcher<Http>> {
        contract_calls
            .into_iter()
            .map(|(rule_id, contract_call)| {
                let default_block_number = U64::from(DEFAULT_BLOCK_NUMBER);
                let from_block = from_blocks.get(&rule_id).unwrap_or(&default_block_number);
                let call_time_interval = call_time_intervals
                    .get(&contract_call.rule.chain_id)
                    .unwrap_or(&DEFAULT_CALL_TIME_INTERVAL);

                Self::build_contract_call_fetcher(
                    contract_call,
                    contract_call_sender.clone(),
                    *from_block,
                    *call_time_interval,
                )
            })
            .collect()
    }

    /// Builds the contract event fetcher.
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
    fn build_contract_event_fetcher<T: JsonRpcClient>(
        client: EthClient<T>,
        contract_events: HashMap<RuleID, ContractEvent<T>>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
        from_blocks: HashMap<RuleID, U64>,
        call_time_interval: u64,
    ) -> ContractEventFetcher<T> {
        ContractEventFetcher::new(
            client,
            contract_events,
            contract_event_sender,
            from_blocks,
            call_time_interval,
        )
    }

    /// Builds multiple contract event fetchers.
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
    fn build_contract_event_fetchers(
        chain_clients: HashMap<ChainID, EthClient<Http>>,
        contract_chain_events: HashMap<ChainID, HashMap<RuleID, ContractEvent<Http>>>,
        contract_event_sender: UnboundedSender<ContractEventRawMessage>,
        contract_event_blocks: HashMap<ChainID, HashMap<RuleID, U64>>,
        call_time_intervals: HashMap<ChainID, u64>,
    ) -> Result<Vec<ContractEventFetcher<Http>>, WorkerError> {
        contract_chain_events
            .clone()
            .into_iter()
            .map(|(chain_id, contract_events)| {
                let mut default_block_numbers = HashMap::new();
                default_block_numbers.insert(
                    chain_id.try_into().unwrap_or_else(|_| {
                        panic!("{}", WorkerError::InvalidTypeConvert.to_string())
                    }),
                    U64::from(DEFAULT_BLOCK_NUMBER),
                );
                let from_blocks = contract_event_blocks
                    .get(&chain_id)
                    .unwrap_or(&default_block_numbers);
                let call_time_interval = call_time_intervals
                    .get(&chain_id)
                    .unwrap_or(&DEFAULT_CALL_TIME_INTERVAL);

                let client = chain_clients
                    .get(&chain_id)
                    .ok_or(WorkerError::InvalidIndex(IndexType::U32(chain_id)))?;

                let result = Self::build_contract_event_fetcher(
                    client.clone(),
                    contract_events,
                    contract_event_sender.clone(),
                    from_blocks.clone(),
                    *call_time_interval,
                );
                Ok(result)
            })
            .collect()
    }

    /// Builds the RPC call manager.
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
    fn build_rpc_call_manager(
        rpc_calls: HashMap<RuleID, RpcCall>,
        rpc_call_receiver: Arc<Mutex<UnboundedReceiver<RpcCallRawMessage>>>,
        db_client: PostgresClient,
    ) -> RpcCallManager {
        RpcCallManager::new(rpc_calls, rpc_call_receiver, db_client)
    }

    /// Builds the contract call manager.
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
    fn build_contract_call_manager<T: JsonRpcClient>(
        contract_calls: HashMap<RuleID, ContractCall<T>>,
        contract_call_receiver: Arc<Mutex<UnboundedReceiver<ContractCallRawMessage>>>,
        db_client: PostgresClient,
    ) -> ContractCallManager<T> {
        ContractCallManager::new(contract_calls, contract_call_receiver, db_client)
    }

    /// Builds the contract event manager.
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
    fn build_contract_event_manager<T: JsonRpcClient>(
        contract_events: HashMap<RuleID, ContractEvent<T>>,
        contract_event_receiver: Arc<Mutex<UnboundedReceiver<ContractEventRawMessage>>>,
        db_client: PostgresClient,
    ) -> ContractEventManager<T> {
        ContractEventManager::new(contract_events, contract_event_receiver, db_client)
    }

    /// Sets the log configuration.
    fn set_log() -> Result<(), WorkerError> {
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
                    .add_directive(SQLX_QUERY_WARN.parse().unwrap_or_else(|_| {
                        panic!("{}", WorkerError::InvalidTypeConvert.to_string())
                    })), // Exclude sqlx::query logs
            )
            .init();

        Ok(())
    }

    /// Builds the database client.
    ///
    /// # Arguments
    ///
    /// * `db_url` - The URL of the database.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `PostgresClient` instance.
    async fn build_db(db_url: &str) -> Result<PostgresClient, WorkerError> {
        let client = PostgresClient::new(db_url)
            .await
            .map_err(|e| WorkerError::InvalidDatabase(e.to_string()))?;
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
    pub fn set_config(spec: &str) -> Configuration {
        let user_config_file = std::fs::File::open(spec)
            .unwrap_or_else(|_| panic!("{}", WorkerError::InvalidConfigFilePath.to_string()));
        let user_config: Configuration = serde_yaml::from_reader(user_config_file)
            .unwrap_or_else(|_| panic!("{}", WorkerError::InvalidConfigFileStructure.to_string()));

        user_config
    }

    /// Sets the tasks.
    ///
    /// # Returns
    ///
    /// A vector of `JoinHandle<Result<(), WorkerError>>` instances.
    pub fn spawn_tasks(&self) -> Vec<JoinHandle<Result<(), WorkerError>>> {
        let mut tasks = Vec::new();

        // Add fetcher tasks
        for fetcher in &self.fetchers {
            let fetcher = Arc::clone(fetcher);
            tasks.push(tokio::spawn(async move {
                let result = std::panic::AssertUnwindSafe(async {
                    let mut fetcher = fetcher.lock().await;
                    fetcher.run().await
                })
                .catch_unwind()
                .await;

                match result {
                    Ok(res) => res,
                    Err(err) => Err(WorkerError::GeneralShutdown(format!(
                        "Fetcher task panicked: {:?}",
                        err
                    ))),
                }
            }));
        }

        // Add manager tasks
        for manager in &self.managers {
            let manager = Arc::clone(manager);
            tasks.push(tokio::spawn(async move {
                let result = std::panic::AssertUnwindSafe(async {
                    let mut manager = manager.lock().await;
                    manager.run().await
                })
                .catch_unwind()
                .await;

                match result {
                    Ok(res) => res,
                    Err(err) => Err(WorkerError::GeneralShutdown(format!(
                        "Manager task panicked: {:?}",
                        err
                    ))),
                }
            }));
        }

        tasks
    }
}
