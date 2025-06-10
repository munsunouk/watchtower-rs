use ethers::{abi::Token, types::U256, utils::hex};
use num_bigint::BigInt;
use serde::Deserialize;
use std::borrow::Cow;
use validator::Validate;

use crate::utils::{
    error::GeneralError,
    types::{ChainID, GeneralToken},
};

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ParamConfig {
    pub pool_config: Vec<PoolConfig>,
    pub service_config: Vec<ServiceConfig>,
    pub oid_config: Vec<OidConfig>,
    pub balance_config: Vec<BalanceConfig>,
    pub url_config: Vec<UrlConfig>,
    pub channel_config: Vec<ChannelConfig>,
    pub validator_config: Vec<ValidatorConfig>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PoolConfig {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ServiceConfig {
    pub name: String,
    pub target_index: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OidConfig {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct BalanceConfig {
    pub name: String,
    pub blockchain: String,
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UrlConfig {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ChannelConfig {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ValidatorConfig {
    pub name: String,
    pub address: String,
    pub controller_address: String,
}

/// # Description
/// This struct represents the configuration for the application.
/// # Arguments
/// * `evm_providers` - The EVM providers.
/// * `sentry_config` - The Sentry configuration.
/// * `postgres_config` - The Postgres configuration.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Configuration {
    #[validate]
    pub notification_config: Vec<NotificationConfig>,
    #[validate]
    pub rpc_config: Vec<RPCConfig>,
    #[validate]
    pub evm_providers: Vec<EVMProvider>,
    #[validate]
    pub sentry_config: SentryConfig,
    #[validate]
    pub postgres_config: PostgresConfig,
    #[validate]
    pub contract_config: Vec<ContractConfig>,
    #[validate]
    pub rpc_call_target: Vec<RPCTargetValue>,
    #[validate]
    pub contract_call_target: Vec<ContractCallTargetValue>,
    #[validate]
    pub blockchain_call_target: Vec<BlockchainTargetValue>,
    #[validate]
    pub contract_event_target: Vec<ContractEventTargetValue>,
    #[validate]
    pub notification_call_target: Vec<NotificationCallTargetValue>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Rule {
    pub name: String,
    pub time_interval: u64,
    pub script: String,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NotificationConfig {
    pub service: String,
    pub key: String,
}

/// # Description
/// This struct represents the configuration for a URL.
/// # Arguments
/// * `name` - The name of the URL.
/// * `url` - The URL.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RPCConfig {
    pub service: String,
}

/// This struct represents the configuration for an EVM provider.
/// # Arguments
/// * `name` - The network name.
/// * `id` - The chain ID.
/// * `provider` - The provider URL.
/// * `call_time_interval` - The call time interval.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct EVMProvider {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    pub id: ChainID,
    #[validate(length(min = 1))]
    pub provider: Vec<String>,
}

/// # Description
/// This struct represents the configuration for Sentry.
/// # Arguments
/// * `environment` - The environment identifier for Sentry.
/// * `dsn` - The DSN for Sentry.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct SentryConfig {
    pub environment: Option<Cow<'static, str>>,
    #[validate(url)]
    pub dsn: String,
}

/// # Description
/// This struct represents the configuration for Postgres.
/// # Arguments
/// * `url` - The database URL.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct PostgresConfig {
    #[validate(url)]
    pub url: String,
}

/// # Description
/// This struct represents the configuration for Address.
/// # Arguments
/// * `service` - The service name.
/// * `contract` - The contract name.
/// * `address` - The address of the Address.
/// * `params` - The params of the Address.
/// * `target_index` - The target index of the Address.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct ContractConfig {
    pub service: String,
    pub blockchain: String,
    pub contract: String,
    pub address: String,
    pub path: String,
}

/// # Description
/// This struct represents the configuration for ContractTargetValue.
/// # Arguments
/// * `name` - The name of the ContractTargetValue.
/// * `params` - The params of the ContractTargetValue.
/// * `target_index` - The target index of the ContractTargetValue.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct ContractCallTargetValue {
    pub name: String,
    #[serde(deserialize_with = "deserialize_params")]
    pub params: Vec<Option<GeneralToken>>,
    pub param_nessesary: Vec<String>,
    pub available_contract: Option<String>,
    pub target_index: String,
}

/// # Description
/// This struct represents the configuration for ContractEventTargetValue.
/// # Arguments
/// * `name` - The name of the ContractEventTargetValue.
/// * `params` - The params of the ContractEventTargetValue.
/// * `target_index` - The target index of the ContractEventTargetValue.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct ContractEventTargetValue {
    pub name: String,
    pub event_index: i32,
    pub target_index: String,
}

#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct NotificationCallTargetValue {
    pub name: String,
    pub params: Vec<Option<GeneralToken>>,
    pub param_nessesary: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ParamValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Array(Vec<ParamValue>),
    Tuple(Vec<ParamValue>),
    Address(String),
    Bytes(String),
    FixedBytes(String),
    Uint(String),
    Int(String),
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamValue::String(s) => write!(f, "{}", s),
            ParamValue::Number(n) => write!(f, "{}", n),
            ParamValue::Boolean(b) => write!(f, "{}", b),
            ParamValue::Array(arr) => write!(
                f,
                "[{}]",
                arr.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            ParamValue::Tuple(tuple) => write!(
                f,
                "({})",
                tuple
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            ParamValue::Address(addr) => write!(f, "{}", addr),
            ParamValue::Bytes(bytes) => write!(f, "{}", bytes),
            ParamValue::FixedBytes(bytes) => write!(f, "{}", bytes),
            ParamValue::Uint(uint) => write!(f, "{}", uint),
            ParamValue::Int(int) => write!(f, "{}", int),
        }
    }
}

fn deserialize_params<'de, D>(deserializer: D) -> Result<Vec<Option<GeneralToken>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values: Vec<ParamValue> = Vec::deserialize(deserializer)?;

    let tokens: Vec<Option<GeneralToken>> = values
        .into_iter()
        .map(|value| {
            match value {
                ParamValue::String(s) => {
                    if s.starts_with("0x") {
                        // Try to parse as address
                        if let Ok(addr) = s.parse::<ethers::types::Address>() {
                            Some(GeneralToken::Address(addr))
                        } else {
                            Some(GeneralToken::String(s))
                        }
                    } else {
                        Some(GeneralToken::String(s))
                    }
                }
                ParamValue::Number(n) => Some(GeneralToken::Uint(ethers::types::U256::from(n))),
                ParamValue::Boolean(b) => Some(GeneralToken::Bool(b)),
                ParamValue::Address(addr) => {
                    if let Ok(addr) = addr.parse::<ethers::types::Address>() {
                        Some(GeneralToken::Address(addr))
                    } else {
                        None
                    }
                }
                ParamValue::Bytes(bytes) => {
                    if let Ok(bytes) = hex::decode(bytes) {
                        Some(GeneralToken::Bytes(bytes))
                    } else {
                        None
                    }
                }
                ParamValue::FixedBytes(bytes) => {
                    if let Ok(bytes) = hex::decode(bytes) {
                        Some(GeneralToken::FixedBytes(bytes))
                    } else {
                        None
                    }
                }
                ParamValue::Uint(uint) => {
                    if let Ok(n) = uint.parse::<u64>() {
                        Some(GeneralToken::Uint(ethers::types::U256::from(n)))
                    } else {
                        None
                    }
                }
                ParamValue::Int(int) => {
                    if let Ok(n) = int.parse::<i64>() {
                        Some(GeneralToken::Int(BigInt::from(n)))
                    } else {
                        None
                    }
                }
                ParamValue::Array(arr) => {
                    let tokens: Vec<Option<GeneralToken>> = arr
                        .into_iter()
                        .map(|v| match v {
                            ParamValue::String(s) => {
                                if s.starts_with("0x") {
                                    if let Ok(addr) = s.parse::<ethers::types::Address>() {
                                        Some(GeneralToken::Address(addr))
                                    } else {
                                        Some(GeneralToken::String(s))
                                    }
                                } else {
                                    Some(GeneralToken::String(s))
                                }
                            }
                            ParamValue::Number(n) => {
                                Some(GeneralToken::Uint(ethers::types::U256::from(n)))
                            }
                            ParamValue::Boolean(b) => Some(GeneralToken::Bool(b)),
                            _ => None,
                        })
                        .collect();
                    Some(GeneralToken::Array(
                        tokens.into_iter().filter_map(|t| t).collect(),
                    ))
                }
                ParamValue::Tuple(tuple) => {
                    let tokens: Vec<Option<GeneralToken>> = tuple
                        .into_iter()
                        .map(|v| match v {
                            ParamValue::String(s) => {
                                if s.starts_with("0x") {
                                    if let Ok(addr) = s.parse::<ethers::types::Address>() {
                                        Some(GeneralToken::Address(addr))
                                    } else {
                                        Some(GeneralToken::String(s))
                                    }
                                } else {
                                    Some(GeneralToken::String(s))
                                }
                            }
                            ParamValue::Number(n) => {
                                Some(GeneralToken::Uint(ethers::types::U256::from(n)))
                            }
                            ParamValue::Boolean(b) => Some(GeneralToken::Bool(b)),
                            _ => None,
                        })
                        .collect();
                    Some(GeneralToken::Tuple(
                        tokens.into_iter().filter_map(|t| t).collect(),
                    ))
                }
            }
        })
        .collect();

    Ok(tokens)
}

/// # Description
/// This struct represents the configuration for RPCTargetValue.
/// # Arguments
/// * `name` - The name of the RPCTargetValue.
/// * `target_index` - The target index of the RPCTargetValue.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct RPCTargetValue {
    pub name: String,
    pub meta_data: String,
    pub call_type: String,
    pub method_type: String,
    pub target_index: String,
    pub param_nessesary: Vec<String>,
    pub api_body: Option<String>,
    pub api_query: Option<String>,
}

/// # Description
/// This struct represents the configuration for Metadata.
/// # Arguments
/// * `address` - The address of the Metadata.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct Metadata {
    pub address: String,
}

/// # Description
/// This struct represents the configuration for BlockchainTargetValue.
/// # Arguments
/// * `name` - The name of the BlockchainTargetValue.
/// * `function_name` - The function name of the BlockchainTargetValue.
/// * `metadata` - The metadata of the BlockchainTargetValue.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct BlockchainTargetValue {
    pub name: String,
    pub params: Vec<String>,
    // pub metadata: Option<Metadata>,
    pub param_nessesary: Vec<String>,
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
pub fn set_config(spec: &str) -> Configuration {
    // Get the project root directory (where Cargo.toml is located)
    let project_root = std::env::current_dir().unwrap();

    // Resolve the config path relative to the project root
    let config_path = project_root.join(spec);

    let user_config_file = std::fs::File::open(config_path).unwrap_or_else(|_| {
        panic!(
            "{}, {}",
            GeneralError::InvalidConfigFilePath.to_string(),
            spec
        )
    });

    let user_config: Configuration = serde_yaml::from_reader(user_config_file).unwrap();
    user_config
}

pub fn set_rule(spec: &str) -> Rule {
    let user_config_file = std::fs::File::open(spec).unwrap_or_else(|_| {
        panic!(
            "{}, {}",
            GeneralError::InvalidConfigFilePath.to_string(),
            spec
        )
    });
    let user_config: Rule = serde_yaml::from_reader(user_config_file).unwrap();
    user_config
}

pub fn set_test_config(spec: &str) -> Configuration {
    println!("spec");

    let user_config_file = std::fs::File::open(spec).unwrap_or_else(|_| {
        panic!(
            "{}, {}",
            GeneralError::InvalidConfigFilePath.to_string(),
            spec
        )
    });
    // let user_config: Configuration = serde_yaml::from_reader(user_config_file)
    //     .unwrap_or_else(|_| panic!("{}", GeneralError::InvalidConfigFileStructure.to_string()));

    let user_config: Configuration = serde_yaml::from_reader(user_config_file).unwrap();
    user_config
}

pub fn set_param_config(spec: &str) -> ParamConfig {
    // Get the project root directory (where Cargo.toml is located)
    let project_root = std::env::current_dir().unwrap();

    // Resolve the param config path relative to the project root
    let param_config_path = project_root.join(spec);

    let user_config_file = std::fs::File::open(param_config_path).unwrap_or_else(|_| {
        panic!(
            "{}, {}",
            GeneralError::InvalidConfigFilePath.to_string(),
            spec
        )
    });
    let user_config: ParamConfig = serde_yaml::from_reader(user_config_file).unwrap();
    user_config
}
