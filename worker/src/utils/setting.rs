use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

use ethers::providers::{Http, JsonRpcClient, Provider};
use reqwest::Client;
use sentry::ClientInitGuard;
use watch_tower_lib::{
    cli::{
        eth::{EthClient, ProviderMetadata},
        rpc::RpcClient,
        sentry::build_sentry_client,
    },
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, rpc_call::RpcCallRule,
    },
    utils::{error::ClientError, types::ChainID},
};

use crate::{
    rule::{ContractCall, ContractEvent, RpcCall},
    utils::config::ParamConfig,
};

use crate::utils::config::{Configuration, EVMProvider};

use super::error::WorkerError;

/// # Description
/// This function builds multiple Ethereum clients.
/// # Arguments
///
/// * `providers` - A vector of `EVMProvider` instances.
///
/// # Returns
///
/// A `HashMap` where the key is a `ChainID` and the value is an `EthClient<Http>` instance.
pub fn build_eth_clients(providers: &[EVMProvider]) -> HashMap<ChainID, EthClient<Http>> {
    let mut clients = HashMap::new();

    for provider in providers {
        let EVMProvider { name, provider, id } = provider;

        let metadata = set_metadata(name, provider, id);

        let arc_providers = set_providers(provider);

        let eth_client = build_eth_client(metadata, arc_providers);
        clients.insert(*id, eth_client);
    }

    clients
}

/// # Description
/// This function sets the Ethereum client.
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

/// # Description
/// This function sets the metadata for a provider.
/// # Arguments
/// * `chain_name` - The name of the chain.
/// * `chain_urls` - The URL of the chain.
/// * `chain_id` - The ID of the chain.
///
/// # Returns
///
/// A `ProviderMetadata` instance.
fn set_metadata(chain_name: &str, chain_urls: &[String], chain_id: &ChainID) -> ProviderMetadata {
    ProviderMetadata::new(chain_name, chain_urls, chain_id)
}

/// # Description
/// This function sets the providers.
/// # Arguments
///
/// * `urls` - A slice of strings representing the URLs of the providers.
///
/// # Returns
///
/// A vector of `Arc<Provider<Http>>` instances.
fn set_providers(urls: &[String]) -> Vec<Arc<Provider<Http>>> {
    urls.iter().map(|url| build_provider(url)).collect()
}

/// # Description
/// This function sets the provider.
/// # Arguments
///
/// * `url` - The URL of the provider.
///
/// # Returns
///
/// An `Arc` of `Provider<Http>`.
fn build_provider(url: &str) -> Arc<Provider<Http>> {
    let provider = Provider::<Http>::try_from(url).unwrap_or_else(|_| {
        panic!(
            "{}",
            ClientError::InvalidProviderURL(url.to_string()).to_string()
        )
    });
    Arc::new(provider)
}

pub fn build_sentry(
    dsn: &str,
    environment: &Option<Cow<'static, str>>,
) -> Result<ClientInitGuard, WorkerError> {
    Ok(build_sentry_client(dsn, environment)?)
}

/// # Description
/// This function builds a contract call.
/// # Arguments
///
/// * `client` - An Ethereum client.
/// * `rule` - A contract call rule.
///
/// # Returns
///
/// A `ContractCall` instance.
pub fn build_contract_call<T: JsonRpcClient + Clone>(
    client: &EthClient<T>,
    rule: &ContractCallRule,
) -> ContractCall<T> {
    ContractCall::new(client, rule)
}

/// # Description
/// This function builds a contract event.
/// # Arguments
///
/// * `client` - An Ethereum client.
/// * `rule` - A contract event rule.
///
/// # Returns
///
/// A `ContractEvent` instance.
pub fn build_contract_event<T: JsonRpcClient + Clone>(
    client: &EthClient<T>,
    rule: &ContractEventRule,
) -> ContractEvent<T> {
    ContractEvent::new(client, rule)
}

/// # Description
/// This function sets an RPC call.
/// # Arguments
///
/// * `client` - An HTTP client.
/// * `rule` - An RPC call rule.
///
/// # Returns
///
/// An `RpcCall` instance.
pub fn build_rpc_call(client: &RpcClient, rule: &RpcCallRule) -> RpcCall {
    RpcCall::new(client, rule)
}

/// # Description
/// This function sets the HTTP client.
/// # Returns
///
/// A `Client` instance.
pub fn build_rpc_client() -> Result<RpcClient, WorkerError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .pool_max_idle_per_host(0)
        .pool_idle_timeout(None)
        .user_agent("watch-tower-worker")
        .danger_accept_invalid_certs(true)
        .build()?;

    Ok(RpcClient::new(vec![Arc::new(client)]))
}

/// # Description
/// This function sets the configuration.
/// # Arguments
///
/// * `spec` - The path to the configuration file.
///
/// # Returns
///
/// A `Result` containing the `Configuration` instance.
pub fn set_config(pathbuf: &PathBuf) -> Result<Configuration, WorkerError> {
    let user_config_file = std::fs::File::open(pathbuf)?;

    let user_config: Configuration = serde_yaml::from_reader(user_config_file)?;

    Ok(user_config)
}

pub fn set_param_config(pathbuf: &PathBuf) -> Result<ParamConfig, WorkerError> {
    let user_config_file = std::fs::File::open(pathbuf)?;
    let user_config: ParamConfig = serde_yaml::from_reader(user_config_file)?;
    Ok(user_config)
}
