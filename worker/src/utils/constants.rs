/// Default Memory Value Order
pub const DEFAULT_MEMORY_VALUE_ORDER: u64 = 0;

/// Default Function Input Index - 0 is the first input parameter in Tuple
pub const DEFAULT_FN_INPUT_INDEX: usize = 0;

/// Add Memory Value Order
pub const ADD_MEMORY_VALUE_ORDER: u64 = 1;

/// Health Check Time Interval (1 Hour)
pub const HEALETH_CHECK_INTERVAL: i32 = 3600;

/// Local Time Format
pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// SQLX Query Warn - Try to avoid SQL Insert Log
pub const SQLX_QUERY_WARN: &str = "sqlx::query=warn";

// JSON-RPC Response Keys
pub const JSON_RPC_RESULT: &str = "result";

// Variable Names
pub const VAR_META_DATA: &str = "meta_data";
pub const VAR_NOTIFICATION: &str = "notification";
pub const VAR_PARAM_NECESSARY: &str = "param_nessesary";
pub const VAR_FUNCTION_PARAMS: &str = "function_params";
pub const VAR_CHAIN_ID: &str = "chain_id";
pub const VAR_BLOCKCHAIN: &str = "blockchain";
pub const VAR_NAME: &str = "name";
pub const VAR_CALL_TYPE: &str = "call_type";
pub const VAR_METHOD_TYPE: &str = "method_type";
pub const VAR_API_BODY: &str = "api_body";
pub const VAR_API_QUERY: &str = "api_query";
pub const VAR_TARGET_INDEX: &str = "target_index";
pub const VAR_ADDRESS: &str = "address";
pub const VAR_ABI: &str = "abi";
pub const VAR_EVENT_INDEX: &str = "event_index";
pub const VAR_IDENTIFIER: &str = "identifier";
pub const VAR_SERVICE: &str = "service";
pub const VAR_CONTRACT: &str = "contract";
pub const VAR_KEY: &str = "key";
pub const VAR_METHOD_PARAMS: &str = "method_params";
pub const VAR_AVAILABLE_CONTRACT: &str = "available_contract";

// Meta Data Types
pub const META_DATA_VAULT_ADDRESS: &str = "VaultAddress";
pub const META_DATA_APY: &str = "APY";
pub const META_DATA_FLOAT: &str = "Float";

// Block Target Types
pub const BLOCK_TARGET_TIMESTAMP: &str = "timestamp";
pub const BLOCK_TARGET_NUMBER: &str = "number";
pub const BLOCK_TARGET_HASH: &str = "hash";

// Notification Service
pub const NOTIFICATION_SLACK: &str = "Slack";

// Parameter Names
pub const PARAM_CHANNEL: &str = "Channel";
pub const PARAM_TIME_INTERVAL: &str = "TimeInterval";
pub const PARAM_MESSAGE: &str = "Message";
pub const PARAM_BLOCK_NUMBER: &str = "BlockNumber";
pub const PARAM_BALANCE: &str = "Balance";
pub const PARAM_URL: &str = "Url";
pub const PARAM_FEED: &str = "Feed";
pub const PARAM_VAULT_ADDRESS: &str = "VaultAddress";
pub const PARAM_POOL: &str = "Pool";
pub const PARAM_OID: &str = "OID";
pub const PARAM_VALIDATOR: &str = "Validator";
pub const PARAM_KEY: &str = "Key";

// API Query Keys
pub const API_QUERY_ACTIVE: &str = "active";

// Contract Types
pub const CONTRACT_STATE: &str = "State";
pub const CONTRACT_CANDIDATE: &str = "Candidate";

// Blockchain Call Names
pub const CALL_LATEST_BLOCK: &str = "LatestBlock";
pub const CALL_LATEST_TIMESTAMP: &str = "LatestTimestamp";
pub const CALL_BALANCE: &str = "Balance";

// Task Names
pub const TASK_SPAWN_BLOCKING: &str = "spawn_blocking";
