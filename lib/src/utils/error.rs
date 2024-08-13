use thiserror::Error;

pub const PROVIDER_INTERNAL_ERROR: &str =
	"An internal error thrown when making a call to the provider. Please check your provider's status";

pub const INVALID_CHAIN_ID: &str =
    "Invalid chain ID provided. Please check your provider's or contract's chain ID.";

pub const INVALID_PROVIDER_URL: &str =
    "Invalid provider URL provided. Please check your provider's URL.";

pub const INVALID_CONFIG_FILE_PATH: &str =
    "Invalid config.yaml file path provided. Please check your file path.";

pub const INVALID_CONFIG_FILE_STRUCTURE: &str =
    "Invalid config.yaml file structure provided. Please check your file structure.";

pub const INVALID_SENTRY_CLIENT_PARAMS: &str = "Invalid params to build Sentry client";

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database insert error {0}")]
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
