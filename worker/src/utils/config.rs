use serde::Deserialize;
use std::borrow::Cow;
use watch_tower_lib::utils::constants::ChainID;

/// Configuration for the application.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Configuration {
    /// EVM provider configurations.
    pub evm_providers: Vec<EVMProvider>,
    /// Sentry configuration.
    pub sentry_config: SentryConfig,
    /// Postgres configuration.
    pub postgres_config: PostgresConfig,
}

/// Configuration for an EVM provider.
#[derive(Debug, Clone, Deserialize)]
pub struct EVMProvider {
    /// Network name.
    pub name: String,
    /// Chain ID.
    pub id: ChainID,
    /// Endpoint provider URL.
    pub provider: Vec<String>,
    /// Check interval.
    pub call_time_interval: u64,
}

/// Configuration for Sentry.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, Deserialize)]
pub struct SentryConfig {
    /// Environment identifier for Sentry.
    pub environment: Option<Cow<'static, str>>,
    /// DSN for Sentry.
    pub dsn: String,
}

/// Configuration for Postgres.
#[derive(Default, Debug, Clone, Deserialize)]
pub struct PostgresConfig {
    /// Database URL.
    pub url: String,
}
