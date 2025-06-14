use thiserror::Error;

use watch_tower_lib::utils::{error::IndexType, DbRuleType};

#[derive(Error, Debug)]
#[repr(u16)]
pub enum WorkerError {
    #[error("[Error Type : Worker], [Issue : Worker shutdown], [Error : {0}]")]
    GeneralShutdown(String) = 2001,
    #[error("[Error Type : Worker], [Issue : Invalid index], [Error : {0}]")]
    InvalidIndex(IndexType) = 2003,
    #[error("[Error Type : Worker], [Issue : Invalid type convert]")]
    InvalidTypeConvertError(String) = 2005,
    #[error("[Error Type : Worker], [Issue : Invalid database], [Error : {0}]")]
    InvalidDatabase(String) = 2006,
    #[error("[Error Type : Worker], [Issue : Invalid message]")]
    InvalidMessage = 2007,
    #[error("[Error Type : Worker], [Issue : Invalid client]")]
    InvalidClient = 2008,
    #[error("[Error Type : Worker], [Issue : Invalid runtime]")]
    InvalidRuntime = 2009,
    #[error("[Error Type : Worker], [Issue : Internal provider error], [Error : {0}]")]
    InternalProviderError(String) = 2012,
    #[error("[Error Type : Worker], [Issue : Invalid sentry], [Error : {0}]")]
    InvalidSentry(String) = 2013,
    #[error("[Error Type : Worker], [Rule Type : {0:?}], [Error : {1}]")]
    InvalidParamType(DbRuleType, String) = 2017,
    #[error("[Error Type : Worker], [Task Type : {0:?}] [Issue : Failed to spawn], [Error : {2}]")]
    FailedSpawn(String, usize, String) = 2018,
    #[error(
        "[Error Type : Worker], [Task Type : {0:?}], [Rule Type : {1:?}], [Rule ID : {2}], [Issue : Failed to process task], [Error : {3}]"
    )]
    FailedTask(String, DbRuleType, usize, String) = 2019,
    #[error("[Error Type : Worker], [Issue : Invalid index depth]")]
    InvalidIndexDepth = 2021,
}

impl From<reqwest::Error> for WorkerError {
    fn from(error: reqwest::Error) -> Self {
        WorkerError::InternalProviderError(error.to_string())
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
