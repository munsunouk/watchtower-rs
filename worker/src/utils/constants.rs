/// Default Memory Value Order
pub const DEFAULT_MEMORY_VALUE_ORDER: u64 = 0;

/// Default Function Input Index - 0 is the first input parameter in Tuple
pub const DEFAULT_FN_INPUT_INDEX: usize = 0;

/// Add Memory Value Order
pub const ADD_MEMORY_VALUE_ORDER: u64 = 1;

/// Config Path
pub const CONFIG_PATH: &str = "./worker/config.yaml";
pub const PARAM_CONFIG_PATH: &str = "./worker/param.yaml";

/// Health Check Time Interval (1 Minute)
pub const HEALETH_CHECK_INTERVAL: u64 = 60;

/// Local Time Format
pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// SQLX Query Warn - Try to avoid SQL Insert Log
pub const SQLX_QUERY_WARN: &str = "sqlx::query=warn";
