use serde::Deserialize;
use std::borrow::Cow;
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
    pub evm_providers: Vec<EVMProvider>,
    #[validate]
    pub sentry_config: SentryConfig,
    #[validate]
    pub postgres_config: PostgresConfig,
    #[validate]
    pub abi_config: Vec<AbiConfig>,
}

/// # Description
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
/// This struct represents the configuration for ABI.
/// # Arguments
/// * `name` - The name of the ABI.
/// * `path` - The path to the ABI file.
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct AbiConfig {
    pub name: String,
    pub path: String,
}
#[derive(Default, Debug, Clone, Deserialize, Validate)]
pub struct SlackConfig {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 1))]
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
    let user_config_file = std::fs::File::open(spec)
        .unwrap_or_else(|_| panic!("{}", GeneralError::InvalidConfigFilePath.to_string()));
    let user_config: Configuration = serde_yaml::from_reader(user_config_file)
        .unwrap_or_else(|_| panic!("{}", GeneralError::InvalidConfigFileStructure.to_string()));

    user_config
}
