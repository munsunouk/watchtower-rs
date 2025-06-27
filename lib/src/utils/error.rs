use uint::{FromDecStrErr, FromHexError, FromStrRadixErr};

use crate::utils::constants::{
    HTTP_ERROR_BODY, HTTP_ERROR_CONNECTION, HTTP_ERROR_DECODE, HTTP_ERROR_REDIRECT,
    HTTP_ERROR_REQUEST, HTTP_ERROR_RESPONSE, HTTP_ERROR_TIMEOUT,
};
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
    #[error("[Error Type : Client], [Issue : HTTP request error {0}]")]
    HttpRequestError(String) = 1009,
    #[error("[Error Type : Client], [Issue : HTTP response error {0}]")]
    HttpResponseError(String) = 1010,
    #[error("[Error Type : Client], [Issue : HTTP timeout error {0}]")]
    HttpTimeoutError(String) = 1011,
    #[error("[Error Type : Client], [Issue : HTTP redirect error {0}]")]
    HttpRedirectError(String) = 1012,
    #[error("[Error Type : Client], [Issue : HTTP connection error {0}]")]
    HttpConnectionError(String) = 1013,
    #[error("[Error Type : Client], [Issue : HTTP body error {0}]")]
    HttpBodyError(String) = 1014,
    #[error("[Error Type : Client], [Issue : HTTP decode error {0}]")]
    HttpDecodeError(String) = 1015,
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
    #[error("[Error Type : General], [Issue : Invalid option: {0}]")]
    InvalidOption(String) = 1023,
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
        GeneralError::InvalidTypeConvertError(format!("sqlx error: {}", err))
    }
}

impl From<std::num::TryFromIntError> for GeneralError {
    fn from(err: std::num::TryFromIntError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("try from int error: {}", err))
    }
}

impl From<std::num::ParseIntError> for GeneralError {
    fn from(err: std::num::ParseIntError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("parse int error: {}", err))
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
            sqlx::Error::PoolClosed => {
                DatabaseError::GenericAquire(format!("pool closed error: {}", err))
            }
            sqlx::Error::PoolTimedOut => {
                DatabaseError::GenericAquire(format!("pool timed out error: {}", err))
            }
            _ => DatabaseError::GenericCreateError(format!("create error: {}", err)),
        }
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(err: std::io::Error) -> Self {
        DatabaseError::GenericInitError(format!("init error: {}", err))
    }
}

impl From<ethers::contract::AbiError> for ClientError {
    fn from(err: ethers::contract::AbiError) -> Self {
        ClientError::InternalProviderError(format!("abi error: {}", err))
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for GeneralError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        GeneralError::InvalidTypeConvertError(format!("box error: {}", err))
    }
}

impl From<FromHexError> for GeneralError {
    fn from(err: FromHexError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("from hex error: {}", err))
    }
}

impl From<FromDecStrErr> for GeneralError {
    fn from(err: FromDecStrErr) -> Self {
        GeneralError::InvalidTypeConvertError(format!("from dec str error: {}", err))
    }
}

impl From<SlackClientError> for ClientError {
    fn from(err: SlackClientError) -> Self {
        ClientError::InvalidSlackConnection(format!("slack client error: {}", err))
    }
}

impl From<GeneralError> for ClientError {
    fn from(err: GeneralError) -> Self {
        ClientError::InvalidTypeConvertError(format!("general error: {}", err))
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::InvalidSlackConnection(format!("io error: {}", err))
    }
}

impl From<chrono::ParseError> for ClientError {
    fn from(err: chrono::ParseError) -> Self {
        ClientError::InvalidTypeConvertError(format!("chrono parse error: {}", err))
    }
}

impl From<std::num::ParseFloatError> for GeneralError {
    fn from(err: std::num::ParseFloatError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("parse float error: {}", err))
    }
}

impl From<num_bigint::ParseBigIntError> for GeneralError {
    fn from(err: num_bigint::ParseBigIntError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("parse big int error: {}", err))
    }
}

impl From<rustc_hex::FromHexError> for GeneralError {
    fn from(err: rustc_hex::FromHexError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("from hex error: {}", err))
    }
}

impl From<std::str::ParseBoolError> for GeneralError {
    fn from(err: std::str::ParseBoolError) -> Self {
        GeneralError::InvalidTypeConvertError(format!("parse bool error: {}", err))
    }
}

impl From<FromStrRadixErr> for GeneralError {
    fn from(err: FromStrRadixErr) -> Self {
        GeneralError::InvalidTypeConvertError(format!("from str radix error: {}", err))
    }
}

impl From<GeneralError> for DatabaseError {
    fn from(err: GeneralError) -> Self {
        DatabaseError::GenericInitError(format!("general error: {}", err))
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        let error_msg = err.to_string();

        if err.is_timeout() {
            ClientError::HttpTimeoutError(format!("timeout error: {}", error_msg))
        } else if err.is_redirect() {
            ClientError::HttpRedirectError(format!("redirect error: {}", error_msg))
        } else if err.is_connect() {
            ClientError::HttpConnectionError(format!("connection error: {}", error_msg))
        } else if err.is_request() {
            ClientError::HttpRequestError(format!("request error: {}", error_msg))
        } else if err.is_body() {
            ClientError::HttpBodyError(format!("body error: {}", error_msg))
        } else if err.is_decode() {
            ClientError::HttpDecodeError(format!("decode error: {}", error_msg))
        } else if err.is_status() {
            ClientError::HttpResponseError(format!("status error: {}", error_msg))
        } else {
            ClientError::InternalProviderError(format!("internal provider error: {}", error_msg))
        }
    }
}

impl From<reqwest::Error> for GeneralError {
    fn from(err: reqwest::Error) -> Self {
        GeneralError::InvalidTypeConvertError(format!("reqwest error: {}", err))
    }
}

/// Macro for using ? operator on Option values with automatic InvalidOption conversion
#[macro_export]
macro_rules! option_or_err {
    ($option:expr) => {{
        $option.ok_or($crate::utils::error::GeneralError::InvalidOption(format!(
            "{} was None",
            stringify!($option)
        )))?
    }};
}

impl ClientError {
    /// Create an HTTP error with additional context
    pub fn http_error(kind: &str, url: Option<&str>, details: &str) -> Self {
        let context = if let Some(url) = url {
            format!("{} for URL {}: {}", kind, url, details)
        } else {
            format!("{}: {}", kind, details)
        };

        match kind {
            HTTP_ERROR_TIMEOUT => ClientError::HttpTimeoutError(context),
            HTTP_ERROR_REDIRECT => ClientError::HttpRedirectError(context),
            HTTP_ERROR_CONNECTION => ClientError::HttpConnectionError(context),
            HTTP_ERROR_REQUEST => ClientError::HttpRequestError(context),
            HTTP_ERROR_BODY => ClientError::HttpBodyError(context),
            HTTP_ERROR_DECODE => ClientError::HttpDecodeError(context),
            HTTP_ERROR_RESPONSE => ClientError::HttpResponseError(context),
            _ => ClientError::InternalProviderError(context),
        }
    }
}
