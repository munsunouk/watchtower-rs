/// SQLX Query Warn - Try to avoid SQL Insert Log
pub const SQLX_QUERY_WARN: &str = "sqlx::query=warn";

/// Local Time Format
pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Default Memory Value Order
pub const DEFAULT_MEMORY_VALUE_ORDER: u64 = 0;

/// Default Block Number - 0 is recent block number
pub const DEFAULT_BLOCK_NUMBER: u64 = 0;

/// Default Function Input Index - 0 is the first input parameter in Tuple
pub const DEFAULT_FN_INPUT_INDEX: usize = 0;

/// Default Chain ID - 0 is the default chain ID
pub const DEFAULT_CHAIN_ID: u32 = 3068;

/// Default Rule ID - 0 is the default rule ID
pub const DEFAULT_ROUND_ID: usize = 0;

/// Default Param Value - Need to change dynamically
pub const DEFAULT_PARAM_VALUE: usize = 0;

/// Block Offset - ensuring that the block range is inclusive of both the from and to blocks.
pub const BLOCK_OFFSET: u64 = 1;

/// New Block Offset - ensuring that the from block is the next block number
pub const NEW_BLOCK_OFFSET: u64 = 1;

/// Next Block - 1 is the next block number add to the current block number
pub const NEXT_BLOCK: u64 = 1;

/// Add Memory Value Order
pub const ADD_MEMORY_VALUE_ORDER: u64 = 1;

/// Default Check Interval - 15 seconds
pub const DEFAULT_CALL_TIME_INTERVAL: u64 = 15;

/// Max Block Length Limit for contract call bootstrap
pub const MAX_BLOCK_LENGTH_LIMIT: u64 = 5;

/// The block range chunk size for getLogs requests.
pub const BOOTSTRAP_BLOCK_CHUNK_SIZE: u64 = 2000;

/// Sync Time Interval (1 hour) (3600 seconds)
pub const SYNC_TIME: u64 = 15;

/// Health Check Time Interval (1 Minute)
pub const HEALETH_CHECK_INTERVAL: u64 = 60;

/// Parsing VALUE Filter
pub const FILTER_INDEX_SPLIT_CHAR: &str = ".";

/// Fetcher Name
pub const FETCHER_NAME: &str = "Fetcher";

/// Controller Name
pub const CONTROLLER_NAME: &str = "Controller";

/// Evaluator Name
pub const EVALUATOR_NAME: &str = "Evaluator";

/// Config Path
// pub const CONFIG_PATH: &str = "./worker/config.yaml";
pub const CONFIG_PATH: &str = "/Users/munseon-ug/rust/watchtower/worker/config.yaml";
