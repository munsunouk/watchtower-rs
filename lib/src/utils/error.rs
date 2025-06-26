use uint::{FromDecStrErr, FromHexError, FromStrRadixErr};

use slack_morphism::errors::SlackClientError;
use std::fmt;
use thiserror::Error;

#[derive(Debug)]
pub enum IndexType {
    U32(u32),
    USize(usize),
}

impl fmt::Display for IndexType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexType::U32(val) => write!(f, "{}", val),
            IndexType::USize(val) => write!(f, "{}", val),
        }
    }
}

#[derive(Error, Debug)]
#[repr(u16)]
pub enum ClientError {
    #[error("[Error Type : Client], [Issue : An internal error thrown when making a call to the provider. Please check your provider's status {0}")]
    InternalProviderError(String) = 1001,
    #[error("[Error Type : Client], [Issue : Invalid chain ID provided. Please check your provider's or contract's chain ID {0}")]
    InvalidChainId(String) = 1002,
    #[error("[Error Type : Client], [Issue : Invalid provider URL provided {0}. Please check your provider's URL.")]
    InvalidProviderURL(String) = 1003,
    #[error("[Error Type : Client], [Issue : Invalid contract call. Please check your contract's method name and parameters.]")]
    InvalidContractCall(String) = 1004,
    #[error("[Error Type : Client], [Issue : Invalid response from provider. Please check your provider's response.]")]
    InvalidResponse(String) = 1005,
    #[error(
        "[Error Type : Client], [Issue : Invalid slack message. Please check your slack message.]"
    )]
    InvalidSlackMessage(String) = 1006,
    #[error("[Error Type : Client], [Issue : Invalid slack connection. Please check your slack connection.]")]
    InvalidSlackConnection(String) = 1007,
    #[error("[Error Type : Client], [Issue : Invalid type convert error {0}]")]
    InvalidTypeConvertError(String) = 1008,
}

#[derive(Error, Debug)]
#[repr(u16)]
pub enum SentryError {
    #[error("[Error Type : Sentry], [Issue : Invalid params to build Sentry client]")]
    InvalidParams = 1006,
}

#[derive(Error, Debug)]
#[repr(u16)]
pub enum DatabaseError {
    #[error("[Error Type : Database], [Issue : Database acquire error {0}")]
    GenericAquire(String) = 1007,
    #[error("[Error Type : Database], [Issue : Database insert error {0}")]
    GenericInsertError(String) = 1008,
    #[error("[Error Type : Database], [Issue : Database delete error {0}")]
    GenericDeleteError(String) = 1009,
    #[error("[Error Type : Database], [Issue : Database select error {0}")]
    GenericSelectError(String) = 1010,
    #[error("[Error Type : Database], [Issue : Database create error {0}")]
    GenericCreateError(String) = 1011,
    #[error("[Error Type : Database], [Issue : Database init error {0}")]
    GenericInitError(String) = 1012,
}

#[derive(Error, Debug)]
#[repr(u16)]
pub enum GeneralError {
    #[error("[Error Type : General], [Issue : Invalid type convert error {0}]")]
    InvalidTypeConvertError(String) = 1013,
    #[error("[Error Type : General], [Issue : Invalid type ABI]")]
    InvalidTypeABI = 1014,
    #[error("[Error Type : General], [Issue : Invalid index: {0}]")]
    InvalidIndex(IndexType) = 1015,
    #[error("[Error Type : General], [Issue : Invalid empty token]")]
    InvalidEmptyToken = 1016,
    #[error("[Error Type : General], [Issue : Invalid rule name]")]
    InvalidRuleName = 1017,
    #[error("[Error Type : General], [Issue : Invalid rule decode: {0}]")]
    InvalidRuleDecode(String) = 1018,
    #[error("[Error Type : General], [Issue : Invalid evaluate opration : {0}]")]
    InvalidOperator(String) = 1019,
    #[error("[Error Type : General], [Issue : Invalid config file path provided. Please check your file path.]")]
    InvalidConfigFilePath = 1020,
    #[error("[Error Type : General], [Issue : Invalid config file structure provided. Please check your file structure. {0}]")]
    InvalidConfigFileStructure(String) = 1021,
    #[error("[Error Type : General], [Issue : Invalid database: {0}]")]
    InvalidDatabase(String) = 1022,
}

impl GeneralError {
    pub fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }

    pub fn log(&self) {
        let msg = format!(
            "[Error Code : {}] ❗️ {}",
            self.discriminant(),
            self.to_string()
        );
        tracing::error!("{}", msg);
    }
}

impl From<sqlx::Error> for GeneralError {
    fn from(err: sqlx::Error) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<std::num::TryFromIntError> for GeneralError {
    fn from(err: std::num::TryFromIntError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<std::num::ParseIntError> for GeneralError {
    fn from(err: std::num::ParseIntError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<serde_json::Error> for GeneralError {
    fn from(_err: serde_json::Error) -> Self {
        GeneralError::InvalidRuleDecode("Invalid JSON array string".to_string())
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::PoolClosed => DatabaseError::GenericAquire(err.to_string()),
            sqlx::Error::PoolTimedOut => DatabaseError::GenericAquire(err.to_string()),
            _ => DatabaseError::GenericCreateError(err.to_string()),
        }
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(err: std::io::Error) -> Self {
        DatabaseError::GenericInitError(err.to_string())
    }
}

impl From<ethers::contract::AbiError> for ClientError {
    fn from(err: ethers::contract::AbiError) -> Self {
        ClientError::InternalProviderError(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for GeneralError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<FromHexError> for GeneralError {
    fn from(err: FromHexError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<FromDecStrErr> for GeneralError {
    fn from(err: FromDecStrErr) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<SlackClientError> for ClientError {
    fn from(err: SlackClientError) -> Self {
        ClientError::InvalidSlackConnection(err.to_string())
    }
}

impl From<GeneralError> for ClientError {
    fn from(err: GeneralError) -> Self {
        ClientError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::InvalidSlackConnection(err.to_string())
    }
}

impl From<chrono::ParseError> for ClientError {
    fn from(err: chrono::ParseError) -> Self {
        ClientError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<std::num::ParseFloatError> for GeneralError {
    fn from(err: std::num::ParseFloatError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<num_bigint::ParseBigIntError> for GeneralError {
    fn from(err: num_bigint::ParseBigIntError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<rustc_hex::FromHexError> for GeneralError {
    fn from(err: rustc_hex::FromHexError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<std::str::ParseBoolError> for GeneralError {
    fn from(err: std::str::ParseBoolError) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

impl From<FromStrRadixErr> for GeneralError {
    fn from(err: FromStrRadixErr) -> Self {
        GeneralError::InvalidTypeConvertError(err.to_string())
    }
}

/// Extension trait to add `?` operator support for Option
pub trait OptionExt<T> {
    fn or_invalid_option(self) -> Result<T, GeneralError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_invalid_option(self) -> Result<T, GeneralError> {
        self.ok_or(GeneralError::InvalidTypeConvertError(format!(
            "Option<{}> was None",
            std::any::type_name::<T>()
        )))
    }
}

/// Macro for using ? operator on Option values with automatic InvalidTypeConvertError conversion
#[macro_export]
macro_rules! option_or_err {
    ($option:expr) => {{
        $option.ok_or($crate::utils::error::GeneralError::InvalidTypeConvertError(
            format!("{} was None", stringify!($option)),
        ))?
    }};
}
