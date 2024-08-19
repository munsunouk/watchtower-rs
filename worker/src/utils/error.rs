use ethers::abi::Token;
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
pub enum WorkerError {
    #[error("Worker shutdown {0}")]
    GeneralShutdown(String),
    #[error("Invalid config file path provided. Please check your file path.")]
    InvalidConfigFilePath,
    #[error("Invalid config file structure provided. Please check your file structure.")]
    InvalidConfigFileStructure,
    #[error("Invalid type ABI")]
    InvalidTypeABI,
    #[error("Invalid token value {0}")]
    InvalidTokenValue(Token),
    #[error("Invalid rpc call log: {0}")]
    InvalidRpcCallLog(String),
    #[error("Invalid contract call log: {0}")]
    InvalidContractCallLog(String),
    #[error("Invalid contract event log: {0}")]
    InvalidContractEventLog(String),
    #[error("Invalid index: {0}")]
    InvalidIndex(IndexType),
    #[error("Invalid type convert")]
    InvalidTypeConvert,
    #[error("Invalid type convert error: {0}")]
    InvalidTypeConvertError(String),
    #[error("Invalid database: {0}")]
    InvalidDatabase(String),
    #[error("Invalid message")]
    InvalidMessage,
    #[error("Invalid client")]
    InvalidClient,
    #[error("Invalid runtime")]
    InvalidRuntime,
}
