use thiserror::Error;

use crate::parse::evaluation::Rule;
use std::fmt::Debug;
use watch_tower_lib::utils::error::{ClientError, DatabaseError, GeneralError, SentryError};
use watch_tower_lib::utils::{error::IndexType, DbTable};

#[derive(Error, Debug)]
#[repr(u16)]
pub enum WorkerError {
    #[error("[Error Type : Worker], [Issue : Worker shutdown], [Error : {0}]")]
    GeneralShutdown(String) = 2001,
    #[error("[Error Type : Worker], [Issue : Invalid index], [Error : {0}]")]
    InvalidIndex(IndexType) = 2003,
    #[error("[Error Type : Worker], [Issue : Invalid type convert error {0}]")]
    InvalidTypeConvertError(String) = 2005,
    #[error("[Error Type : Worker], [Issue : Invalid database], [Error : {0}]")]
    InvalidDatabase(String) = 2006,
    #[error("[Error Type : Worker], [Issue : Invalid message]")]
    InvalidMessage = 2007,
    #[error("[Error Type : Worker], [Issue : Internal provider error], [Error : {0}]")]
    InternalProviderError(String) = 2012,
    #[error("[Error Type : Worker], [Rule Type : {0:?}], [Error : {1}]")]
    InvalidParamType(DbTable, String) = 2017,
    #[error("[Error Type : Worker], [Rule Name : {0}], [Issue : Failed to spawn], [Error : {1}]")]
    FailedSpawn(String, String) = 2018,
    #[error("[Error Type : Worker], [Issue : Invalid index depth]")]
    InvalidIndexDepth = 2021,
    #[error("[Error Type : Worker], [Issue : Invalid contract], [Error : {0}]")]
    InvalidContract(String) = 2022,

    #[error(
        "[Error Type : General], [Issue : Invalid file open error. Please check your file. {0}]"
    )]
    InvalidFileOpenError(String) = 2023,
    #[error("[Error Type : General], [Issue : Invalid config file structure. Please check your config file. {0}]")]
    InvalidConfigFileStructure(String) = 2024,
    #[error(
        "[Error Type : Worker], [Rule Name : {0}], [Issue : Failed to process task], [Error : {1}]"
    )]
    FailedTask(String, String) = 2025,
    #[error("[Error Type : Worker], [Issue : Invalid rule decode], [Error : {0}]")]
    InvalidRuleDecode(String) = 2026,
    #[error("[Error Type : Worker], [Issue : Invalid operator], [Error : {0}]")]
    InvalidOperator(String) = 2027,
    #[error("[Error Type : Worker], [Issue : Invalid ABI]")]
    InvalidTypeABI = 2029,
    #[error("[Error Type : Worker], [Issue : Invalid option], [Error : {0}]")]
    InvalidOption(String) = 2031,
}

impl From<reqwest::Error> for WorkerError {
    fn from(error: reqwest::Error) -> Self {
        WorkerError::InternalProviderError(error.to_string())
    }
}

impl From<GeneralError> for WorkerError {
    fn from(err: GeneralError) -> Self {
        match err {
            GeneralError::InvalidTypeConvertError(msg) => WorkerError::InvalidTypeConvertError(msg),
            GeneralError::InvalidIndex(index_type) => WorkerError::InvalidIndex(index_type),
            GeneralError::InvalidTypeABI => {
                WorkerError::InvalidTypeConvertError("Invalid ABI type".to_string())
            }
            GeneralError::InvalidOperator(op) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid operator: {op}"))
            }
            _ => WorkerError::InvalidTypeConvertError(err.to_string()),
        }
    }
}

impl From<ClientError> for WorkerError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::InternalProviderError(msg) => WorkerError::InternalProviderError(msg),
            ClientError::InvalidChainId(msg) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid chain ID: {msg}"))
            }
            ClientError::InvalidProviderURL(msg) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid provider URL: {msg}"))
            }
            ClientError::InvalidContractCall(msg) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid contract call: {msg}"))
            }
            ClientError::InvalidResponse(msg) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid response: {msg}"))
            }
            ClientError::InvalidSlackMessage(msg) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid Slack message: {msg}"))
            }
            ClientError::InvalidSlackConnection(msg) => {
                WorkerError::InvalidTypeConvertError(format!("Invalid Slack connection: {msg}"))
            }
            ClientError::InvalidTypeConvertError(msg) => WorkerError::InvalidTypeConvertError(msg),
        }
    }
}

impl From<serde_yaml::Error> for WorkerError {
    fn from(err: serde_yaml::Error) -> Self {
        WorkerError::InvalidConfigFileStructure(err.to_string())
    }
}

impl From<std::io::Error> for WorkerError {
    fn from(err: std::io::Error) -> Self {
        WorkerError::InvalidFileOpenError(err.to_string())
    }
}

impl From<DatabaseError> for WorkerError {
    fn from(err: DatabaseError) -> Self {
        WorkerError::InvalidDatabase(err.to_string())
    }
}

impl From<pest::error::Error<Rule>> for WorkerError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        WorkerError::InvalidRuleDecode(err.to_string())
    }
}
impl From<tokio::task::JoinError> for WorkerError {
    fn from(err: tokio::task::JoinError) -> Self {
        WorkerError::FailedTask("spawn_blocking".to_string(), err.to_string())
    }
}

impl From<cron::error::Error> for WorkerError {
    fn from(err: cron::error::Error) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<std::num::ParseIntError> for WorkerError {
    fn from(err: std::num::ParseIntError) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<hex::FromHexError> for WorkerError {
    fn from(err: hex::FromHexError) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<serde_json::Error> for WorkerError {
    fn from(err: serde_json::Error) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<rustc_hex::FromHexError> for WorkerError {
    fn from(err: rustc_hex::FromHexError) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<ethers::abi::AbiError> for WorkerError {
    fn from(_err: ethers::abi::AbiError) -> Self {
        WorkerError::InvalidTypeABI
    }
}

impl From<std::convert::Infallible> for WorkerError {
    fn from(err: std::convert::Infallible) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<SentryError> for WorkerError {
    fn from(err: SentryError) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<tracing_subscriber::filter::ParseError> for WorkerError {
    fn from(err: tracing_subscriber::filter::ParseError) -> Self {
        WorkerError::InvalidTypeConvertError(err.to_string())
    }
}

// Convert Option<T> to WorkerError when None
impl<T: Debug> From<Option<T>> for WorkerError {
    fn from(option: Option<T>) -> Self {
        WorkerError::InvalidOption(format!("{option:?}"))
    }
}

impl WorkerError {
    pub fn log(&self) {
        let msg = format!("[Error Code : {}] ❗️ {}", self.discriminant(), self);
        tracing::error!("{}", msg);
        sentry::capture_message(&msg, sentry::Level::Error);
    }

    pub fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}

/// Macro for using ? operator on Option values with automatic InvalidOption conversion
#[macro_export]
macro_rules! option_or_err {
    ($option:expr) => {{
        $option.ok_or($crate::utils::error::WorkerError::InvalidOption(format!(
            "{} was None",
            stringify!($option)
        )))?
    }};
}
