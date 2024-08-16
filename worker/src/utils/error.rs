use ethers::abi::Token;
use thiserror::Error;

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
}
