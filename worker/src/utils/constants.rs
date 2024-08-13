use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;

// DB Tables
pub const DB_RPC_CALL_RULE: &str = "rpc_call_rule";
pub const DB_CONTRACT_CALL_RULE: &str = "contract_call_rule";
pub const DB_CONTRACT_EVENT_RULE: &str = "contract_event_rule";
pub const DB_ID_COLUMN: &str = "id";
pub const DB_URL_COLUMN: &str = "url";
pub const DB_EVENT_INDEX_COLUMN: &str = "event_index";
pub const DB_ADDRESS_COLUMN: &str = "address";
pub const DB_BLOCK_NUMBER_COLUMN: &str = "block_number";
pub const DB_ABI_COLUMN: &str = "abi";
pub const DB_METHOD_PARAMS_COLUMN: &str = "method_params";
pub const DB_RULE_FILTER_COLUMN: &str = "rule_filter";
pub const DB_RULE_FILTER_COMPARATOR_COLUMN: &str = "rule_filter_comparator";
pub const DB_EXPECTED_VALUE_FILTER_COLUMN: &str = "expected_value_filter";
pub const DB_EXPECTED_VALUE_FILTER_COMPARATOR_COLUMN: &str = "expected_value_filter_comparator";
pub const DB_CHECK_INTERVAL_COLUMN: &str = "check_interval";
pub const DB_CHAIN_ID_COLUMN: &str = "chain_id";
pub const DB_COMPARATOR_COLUMN: &str = "comparator";
pub const DB_EXPECTED_VALUE_COLUMN: &str = "expected_value";

// Log targets
pub const INVALID_TYPE_ABI: &str = "invalid type ABI";
pub const INVALID_TOKEN_VALUE: &str = "invalid token value";
pub const INVALID_RPC_CALL_LOG: &str = "invalid rpc call log";
pub const INVALID_CONTRACT_CALL_LOG: &str = "invalid contract call log";
pub const INVALID_CONTRACT_EVENT_LOG: &str = "invalid contract event log";

// SQLX Query Warn - Try to avoid SQL Insert Log
pub const SQLX_QUERY_WARN: &str = "sqlx::query=warn";

pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// The block range chunk size for getLogs requests.
pub const BOOTSTRAP_BLOCK_CHUNK_SIZE: u64 = 2000;

// Default Block Number - 0 is recent block number
pub const DEFAULT_BLOCK_NUMBER: u64 = 0;

// Default Function Input Index - 0 is the first input parameter in Tuple
pub const DEFAULT_FN_INPUT_INDEX: usize = 0;

// Default Check Interval
pub const DEFAULT_CHECK_INTERVAL: u64 = 15;

// Next Block - 1 is the next block number
pub const NEXT_BLOCK: u64 = 1;

// Comparator Type Allow by each type
pub const UINT_COMPARATOR_TYPE: [&str; 6] = ["==", ">", ">=", "<", "<=", "!="];
pub const INT_COMPARATOR_TYPE: [&str; 6] = ["==", ">", ">=", "<", "<=", "!="];
pub const ADDRESS_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const BOOL_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const STRING_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const BYTES_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const FIXED_BYTES_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];

// Config Path
pub const CONFIG_PATH: &str = "./src/utils/configs/config.testnet.yaml";

pub type RuleID = usize;

pub static TOKIO_THREADS_TOTAL: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static TOKIO_THREADS_ALIVE: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
