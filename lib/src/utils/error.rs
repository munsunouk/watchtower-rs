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
    #[error("[Error Type : General], [Issue : Invalid type conversion]")]
    InvalidTypeConvert = 1013,
    #[error("[Error Type : General], [Issue : Invalid type ABI]")]
    InvalidTypeABI = 1014,
    #[error("[Error Type : General], [Issue : Invalid type convert error {0}]")]
    InvalidTypeConvertError(String) = 1015,
    #[error("[Error Type : General], [Issue : Invalid index: {0}]")]
    InvalidIndex(IndexType) = 1016,
    #[error("[Error Type : General], [Issue : Invalid empty token]")]
    InvalidEmptyToken = 1017,
    #[error("[Error Type : General], [Issue : Invalid rule name]")]
    InvalidRuleName = 1018,
    #[error("[Error Type : General], [Issue : Invalid rule decode: {0}]")]
    InvalidRuleDecode(String) = 1019,
    #[error("[Error Type : General], [Issue : Invalid evaluate opration : {0}]")]
    InvalidOperator(String) = 1020,
    #[error("[Error Type : General], [Issue : Invalid config file path provided. Please check your file path.]")]
    InvalidConfigFilePath = 1021,
    #[error("[Error Type : General], [Issue : Invalid config file structure provided. Please check your file structure.]")]
    InvalidConfigFileStructure = 1022,
    #[error("[Error Type : General], [Issue : Invalid database: {0}]")]
    InvalidDatabase(String) = 1023,
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
