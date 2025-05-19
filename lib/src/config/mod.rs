use ethers::{
    abi::{Param, ParamType, Token},
    types::U256,
    utils::hex,
};
use serde::Deserialize;
use std::{borrow::Cow, clone, collections::HashMap};
use validator::Validate;

use crate::utils::{error::GeneralError, types::ChainID};

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
}

/// # Description
/// This struct represents the configuration for a URL.
/// # Arguments
/// * `name` - The name of the URL.
/// * `url` - The URL.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RPCConfig {
    pub name: String,
    pub url: String,
    pub call_type: String,
    pub method_type: String,
    pub api_body: Option<String>,
    pub api_query: Option<String>,
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
    pub params: Vec<Option<Token>>,
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

fn get_param_type(value: &ParamValue) -> ParamType {
    match value {
        ParamValue::String(_) => ParamType::String,
        ParamValue::Number(_) => ParamType::Uint(256),
        ParamValue::Boolean(_) => ParamType::Bool,
        ParamValue::Address(_) => ParamType::Address,
        ParamValue::Bytes(_) => ParamType::Bytes,
        ParamValue::FixedBytes(_) => ParamType::FixedBytes(32),
        ParamValue::Uint(_) => ParamType::Uint(256),
        ParamValue::Int(_) => ParamType::Int(256),
        ParamValue::Array(arr) => {
            if arr.is_empty() {
                ParamType::Array(Box::new(ParamType::String))
            } else {
                ParamType::Array(Box::new(get_param_type(&arr[0])))
            }
        }
        ParamValue::Tuple(tuple) => ParamType::Tuple(tuple.iter().map(get_param_type).collect()),
    }
}

fn deserialize_params<'de, D>(deserializer: D) -> Result<Vec<Option<Token>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values: Vec<ParamValue> = Vec::deserialize(deserializer)?;

    let tokens: Vec<Option<Token>> = values
        .into_iter()
        .map(|value| {
            match value {
                ParamValue::String(s) => {
                    if s.starts_with("0x") {
                        // Try to parse as address
                        if let Ok(addr) = s.parse::<ethers::types::Address>() {
                            Some(Token::Address(addr))
                        } else {
                            Some(Token::String(s))
                        }
                    } else {
                        Some(Token::String(s))
                    }
                }
                ParamValue::Number(n) => Some(Token::Uint(ethers::types::U256::from(n))),
                ParamValue::Boolean(b) => Some(Token::Bool(b)),
                ParamValue::Address(addr) => {
                    if let Ok(addr) = addr.parse::<ethers::types::Address>() {
                        Some(Token::Address(addr))
                    } else {
                        None
                    }
                }
                ParamValue::Bytes(bytes) => {
                    if let Ok(bytes) = hex::decode(bytes) {
                        Some(Token::Bytes(bytes))
                    } else {
                        None
                    }
                }
                ParamValue::FixedBytes(bytes) => {
                    if let Ok(bytes) = hex::decode(bytes) {
                        Some(Token::FixedBytes(bytes))
                    } else {
                        None
                    }
                }
                ParamValue::Uint(uint) => {
                    if let Ok(n) = uint.parse::<u64>() {
                        Some(Token::Uint(ethers::types::U256::from(n)))
                    } else {
                        None
                    }
                }
                ParamValue::Int(int) => {
                    if let Ok(n) = int.parse::<i64>() {
                        Some(Token::Int(U256::from(n)))
                    } else {
                        None
                    }
                }
                ParamValue::Array(arr) => {
                    let tokens: Vec<Option<Token>> = arr
                        .into_iter()
                        .map(|v| match v {
                            ParamValue::String(s) => {
                                if s.starts_with("0x") {
                                    if let Ok(addr) = s.parse::<ethers::types::Address>() {
                                        Some(Token::Address(addr))
                                    } else {
                                        Some(Token::String(s))
                                    }
                                } else {
                                    Some(Token::String(s))
                                }
                            }
                            ParamValue::Number(n) => {
                                Some(Token::Uint(ethers::types::U256::from(n)))
                            }
                            ParamValue::Boolean(b) => Some(Token::Bool(b)),
                            _ => None,
                        })
                        .collect();
                    Some(Token::Array(tokens.into_iter().filter_map(|t| t).collect()))
                }
                ParamValue::Tuple(tuple) => {
                    let tokens: Vec<Option<Token>> = tuple
                        .into_iter()
                        .map(|v| match v {
                            ParamValue::String(s) => {
                                if s.starts_with("0x") {
                                    if let Ok(addr) = s.parse::<ethers::types::Address>() {
                                        Some(Token::Address(addr))
                                    } else {
                                        Some(Token::String(s))
                                    }
                                } else {
                                    Some(Token::String(s))
                                }
                            }
                            ParamValue::Number(n) => {
                                Some(Token::Uint(ethers::types::U256::from(n)))
                            }
                            ParamValue::Boolean(b) => Some(Token::Bool(b)),
                            _ => None,
                        })
                        .collect();
                    Some(Token::Tuple(tokens.into_iter().filter_map(|t| t).collect()))
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
    pub target_index: String,
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
    pub metadata: Option<Metadata>,
}

/// # Description
/// This struct represents the configuration for Slack.
/// # Arguments
/// * `token` - The Slack token.
/// * `channel` - The Slack channel.
pub struct SlackConfig {
    pub token: String,

    pub channel: String,
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
