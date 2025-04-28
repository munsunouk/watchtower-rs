use thiserror::Error;

use crate::utils::msg::RpcCallRawMessage;
use tokio::sync::mpsc::error::SendError;
use watch_tower_lib::utils::{error::IndexType, DbRuleType};

#[derive(Error, Debug)]
#[repr(u16)]
pub enum WorkerError {
    #[error("[Error Type : Worker], [Issue : Worker shutdown], [Error : {0}]")]
    GeneralShutdown(String) = 2001,
    #[error("[Error Type : Worker], [Rule Type : {0:?}], [Rule ID : {1}], [Issue : Invalid log], [Error : {2}]")]
    InvalidLog(DbRuleType, usize, String) = 2002,
    #[error("[Error Type : Worker], [Issue : Invalid index], [Error : {0}]")]
    InvalidIndex(IndexType) = 2003,
    #[error("[Error Type : Worker], [Issue : Invalid type convert]")]
    InvalidTypeConvert = 2004,
    #[error("[Error Type : Worker], [Issue : Invalid type convert error], [Error : {0}]")]
    InvalidTypeConvertError(String) = 2005,
    #[error("[Error Type : Worker], [Issue : Invalid database], [Error : {0}]")]
    InvalidDatabase(String) = 2006,
    #[error("[Error Type : Worker], [Issue : Invalid message]")]
    InvalidMessage = 2007,
    #[error("[Error Type : Worker], [Issue : Invalid client]")]
    InvalidClient = 2008,
    #[error("[Error Type : Worker], [Issue : Invalid runtime]")]
    InvalidRuntime = 2009,
    #[error("[Error Type : Worker], [Issue : Invalid token]")]
    InvalidToken = 2010,
    #[error("[Error Type : Worker], [Issue : Invalid empty token]")]
    InvalidEmptyToken = 2011,
    #[error("[Error Type : Worker], [Issue : Internal provider error], [Error : {0}]")]
    InternalProviderError(String) = 2012,
    #[error("[Error Type : Worker], [Issue : Invalid sentry], [Error : {0}]")]
    InvalidSentry(String) = 2013,
    #[error("[Error Type : Worker], [Rule Type : {0:?}], [Issue : Channel closed, but {1} continues running]")]
    InvalidChannel(DbRuleType, String) = 2014,
    #[error(
        "[Error Type : Worker], [Rule Type : {0:?}], [Issue : Failed to sync {1} shared state]"
    )]
    FailedToSyncSharedState(DbRuleType, String) = 2015,
    #[error(
        "[Error Type : Worker], [Rule Type : {0:?}], [Rule ID : {1}], [Issue : Invalid {2} index]"
    )]
    InvalidRuleIndex(DbRuleType, usize, String) = 2016,
    #[error("[Error Type : Worker], [Rule Type : {0:?}], [Error : {1}]")]
    InvalidParamType(DbRuleType, String) = 2017,
    #[error("[Error Type : Worker], [Task Type : {0:?}] [Issue : Failed to spawn], [Error : {2}]")]
    FailedSpawn(String, usize, String) = 2018,
    #[error(
        "[Error Type : Worker], [Task Type : {0:?}], [Rule Type : {1:?}], [Rule ID : {2}], [Issue : Failed to process task], [Error : {3}]"
    )]
    FailedTask(String, DbRuleType, usize, String) = 2019,
    #[error("[Error Type : Worker], [Issue : Invalid tx hash]")]
    InvalidTxHash(String) = 2020,
    #[error("[Error Type : Worker], [Issue : Invalid index depth]")]
    InvalidIndexDepth = 2021,
    #[error("[Error Type : Worker], [Issue : Invalid index access on non composite type]")]
    InvalidIndexAccessOnNonCompositeType = 2022,
    #[error("Token is not an integer type")]
    NotInteger,
    #[error("Integer value out of range for i64: {0}")]
    OutOfRange(String),
}

impl From<reqwest::Error> for WorkerError {
    fn from(error: reqwest::Error) -> Self {
        WorkerError::InternalProviderError(error.to_string())
    }
}

impl From<SendError<RpcCallRawMessage>> for WorkerError {
    fn from(_error: SendError<RpcCallRawMessage>) -> Self {
        WorkerError::InvalidMessage
    }
}

impl WorkerError {
    pub fn log(&self) {
        let msg = format!(
            "[Error Code : {}] ❗️ {}",
            self.discriminant(),
            self.to_string()
        );
        tracing::error!("{}", msg);
        sentry::capture_message(&msg, sentry::Level::Error);
    }

    fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}
