use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("An internal error thrown when making a call to the provider. Please check your provider's status {0}")]
    InternalProviderError(String),
    #[error("Invalid chain ID provided. Please check your provider's or contract's chain ID.")]
    InvalidChainId,
    #[error("Invalid provider URL provided. Please check your provider's URL.")]
    InvalidProviderURL,
    #[error("Invalid contract call. Please check your contract's method name and parameters.")]
    InvalidContractCall(String),
}

#[derive(Error, Debug)]
pub enum SentryError {
    #[error("Invalid params to build Sentry client")]
    InvalidParams,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database acquire error {0}")]
    GenericAquire(String),
    #[error("Database insert error {0}")]
    GenericInsertError(String),
    #[error("Database select error {0}")]
    GenericSelectError(String),
    #[error("Database create error {0}")]
    GenericCreateError(String),
    #[error("Database init error {0}")]
    GenericInitError(String),
}
