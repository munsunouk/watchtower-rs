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
pub const DB_CHECK_BLOCK_INTERVAL_COLUMN: &str = "check_block_interval";
pub const DB_CALL_TIME_INTERVAL_COLUMN: &str = "call_time_interval";
pub const DB_CHAIN_ID_COLUMN: &str = "chain_id";
pub const DB_COMPARATOR_COLUMN: &str = "comparator";
pub const DB_EXPECTED_VALUE_COLUMN: &str = "expected_value";
pub const DB_CONTRACT_CALL_BLOCK_LOG: &str = "contract_call_block_log";

/// SQLX Query Warn - Try to avoid SQL Insert Log
pub const SQLX_QUERY_WARN: &str = "sqlx::query=warn";

pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// The block range chunk size for getLogs requests.
pub const BOOTSTRAP_BLOCK_CHUNK_SIZE: u64 = 2000;

/// Default Block Number - 0 is recent block number
pub const DEFAULT_BLOCK_NUMBER: u64 = 0;

/// Default Function Input Index - 0 is the first input parameter in Tuple
pub const DEFAULT_FN_INPUT_INDEX: usize = 0;

/// Block Offset - ensuring that the block range is inclusive of both the from and to blocks.
pub const BLOCK_OFFSET: u64 = 1;

/// New Block Offset - ensuring that the from block is the next block number
pub const NEW_BLOCK_OFFSET: u64 = 1;

/// Default Check Interval
pub const DEFAULT_CALL_TIME_INTERVAL: u64 = 15;

/// Next Block - 1 is the next block number
pub const NEXT_BLOCK: u64 = 1;

/// Max Block Length Limit for contract call bootstrap
pub const MAX_BLOCK_LENGTH_LIMIT: u64 = 10;

/// Add Memory Value Order
pub const ADD_MEMORY_VALUE_ORDER: u64 = 1;

/// Default Memory Value Order
pub const DEFAULT_MEMORY_VALUE_ORDER: u64 = 0;

// Parsing VALUE Filter
pub const FILTER_VALUE_SPLIT_CHAR: &str = "-";
pub const FILTER_INDEX_SPLIT_CHAR: &str = ".";
pub const FILTER_INDEX: usize = 0;
pub const FILTER_VALUE: usize = 1;

/// Default Param Value - Need to change dynamically
pub const DEFAULT_PARAM_VALUE: usize = 0;

/// Default Index for any index type
pub const DEFAULT_INDEX: usize = 0;

// Comparator Type Allow by each type
pub const UINT_COMPARATOR_TYPE: [&str; 6] = ["==", ">", ">=", "<", "<=", "!="];
pub const INT_COMPARATOR_TYPE: [&str; 6] = ["==", ">", ">=", "<", "<=", "!="];
pub const ADDRESS_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const BOOL_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const STRING_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const BYTES_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];
pub const FIXED_BYTES_COMPARATOR_TYPE: [&str; 2] = ["==", "!="];

/// Config Path
pub const CONFIG_PATH: &str = "./src/utils/config/config.testnet.yaml";

pub type RuleID = usize;

pub static TOKIO_THREADS_TOTAL: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));
pub static TOKIO_THREADS_ALIVE: Lazy<AtomicU64> =
    Lazy::new(|| AtomicU64::new(DEFAULT_MEMORY_VALUE_ORDER));
