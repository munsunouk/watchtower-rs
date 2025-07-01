/// The Maximum retries of a single json rpc request.
pub const MAX_RETRY_CALL: u8 = 3;
/// Limit Retry Call
pub const LIMIT_RETRY_CALL: u8 = 0;
/// The default retry interval of a single json rpc request in milliseconds.
pub const DEFAULT_CALL_RETRY_INTERVAL_MS: u64 = 3000;
/// Default attempts for retry operations
pub const DEFAULT_ATTEMPTS: u8 = 0;
/// Default maximum attempts for retry operations
pub const DEFAULT_MAX_ATTEMPTS: u8 = 3;
/// Default backoff time in milliseconds
pub const DEFAULT_BACKOFF_MS: u64 = 1000;

/// Chain Split Index
pub const CHAIN_SPLIT_INDEX: usize = 1;

/// Address Split Index
pub const ADDRESS_SPLIT_INDEX: usize = 1;

/// URL Split Index
pub const URL_SPLIT_INDEX: usize = 1;

/// Interval Split Index
pub const INTERVAL_SPLIT_INDEX: usize = 2;

/// Block Number Split Index
pub const BLOCK_NUMBER_SPLIT_INDEX: usize = 3;

/// Event Index Split Index
pub const EVENT_INDEX_SPLIT_INDEX: usize = 3;

// DB Query PATH
pub const SCHEMA: &str = include_str!("../cli/db/sql/create_schema.sql");

// DB SCHEMA
pub const DB_SCHEMA_LOAD: &str = "SELECT * FROM %%TABLE_NAME%%";
pub const DB_TABLE_NAME: &str = "%%TABLE_NAME%%";
pub const DB_SCHEMA_MAX_ID: &str = "SELECT COALESCE(MAX(id), 0) AS max_id FROM %%TABLE_NAME%%";
pub const RULE_ID: &str = "%%RULE_ID%%";
pub const DB_SELECT_ID_BY_NAME: &str = "SELECT id FROM %%TABLE_NAME%% WHERE name = $1";
pub const DB_SCHEMA_EXISTS: &str = "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = 'rule'
            )";

// DB Table Name

pub const RULE: &str = "rule";

// DB Column Name
pub const DB_ID_COLUMN: &str = "id";
pub const DB_NAME_COLUMN: &str = "name";
pub const DB_VALUE_COLUMN: &str = "value";
pub const DB_URL_COLUMN: &str = "url";
pub const DB_EVENT_INDEX_COLUMN: &str = "event_index";
pub const DB_ADDRESS_COLUMN: &str = "address";
pub const DB_BLOCK_NUMBER_COLUMN: &str = "block_number";
pub const DB_ABI_COLUMN: &str = "abi";
pub const DB_METHOD_TYPE_COLUMN: &str = "method_type";
pub const DB_CALL_TYPE_COLUMN: &str = "call_type";
pub const DB_METHOD_PARAMS_COLUMN: &str = "method_params";
pub const DB_API_BODY_TYPE_COLUMN: &str = "api_body";
pub const DB_RULE_FILTER_COLUMN: &str = "rule_filter";
pub const DB_CHECK_BLOCK_INTERVAL_COLUMN: &str = "check_block_interval";
pub const DB_TARGET_BLOCK_NUMBER_COLUMN: &str = "target_block_number";
pub const DB_CALL_TIME_INTERVAL_COLUMN: &str = "call_time_interval";
pub const DB_CHAIN_ID_COLUMN: &str = "chain_id";
pub const DB_EXPECTED_VALUE_COLUMN: &str = "expected_value";
pub const DB_VALUES_COLUMN: &str = "values";
pub const DB_RULE_ID_COLUMN: &str = "rule_id";
pub const DB_EVALUATION_RULE_ID_COLUMN: &str = "evaluation_rule_id";
pub const DB_TX_HASH_COLUMN: &str = "tx_hash";
pub const DB_MAX_ID_COLUMN: &str = "max_id";

// Arithmetic operator type & comparator type & logic operator type & boolean literal type
pub const OPERATOR_ADD: &str = "+";
pub const OPERATOR_SUB: &str = "-";
pub const OPERATOR_MUL: &str = "*";
pub const OPERATOR_DIV: &str = "/";
pub const OPERATOR_POW: &str = "**";
pub const COMPARATOR_EQUAL: &str = "==";
pub const COMPARATOR_NOT_EQUAL: &str = "!=";
pub const COMPARATOR_GREATER: &str = ">";
pub const COMPARATOR_GREATER_EQUAL: &str = ">=";
pub const COMPARATOR_LESS: &str = "<";
pub const COMPARATOR_LESS_EQUAL: &str = "<=";
pub const LOGIC_OPERATOR_AND: &str = "&&";
pub const LOGIC_OPERATOR_OR: &str = "||";
pub const BOOLEAN_LITERAL_TRUE: &str = "true";
pub const BOOLEAN_LITERAL_FALSE: &str = "false";

/// Default Index for any index type
pub const DEFAULT_INDEX: usize = 0;

/// Rule Filter Split Character
pub const RULE_FILTER_SPLIT_CHAR: &str = "_";

/// Rule ID Split Index
pub const RULE_ID_SPLIT_INDEX: usize = 1;

/// Value ID Split Index
pub const VALUE_ID_SPLIT_INDEX: usize = 2;

/// Parsing VALUE Filter
pub const FILTER_INDEX_SPLIT_CHAR: &str = ".";

// Comparator Type Allow by each type
pub const UINT_COMPARATOR_TYPE: [&str; 6] = [
    COMPARATOR_EQUAL,
    COMPARATOR_GREATER,
    COMPARATOR_GREATER_EQUAL,
    COMPARATOR_LESS,
    COMPARATOR_LESS_EQUAL,
    COMPARATOR_NOT_EQUAL,
];
pub const INT_COMPARATOR_TYPE: [&str; 6] = [
    COMPARATOR_EQUAL,
    COMPARATOR_GREATER,
    COMPARATOR_GREATER_EQUAL,
    COMPARATOR_LESS,
    COMPARATOR_LESS_EQUAL,
    COMPARATOR_NOT_EQUAL,
];

pub const FLOAT_COMPARATOR_TYPE: [&str; 6] = [
    COMPARATOR_EQUAL,
    COMPARATOR_GREATER,
    COMPARATOR_GREATER_EQUAL,
    COMPARATOR_LESS,
    COMPARATOR_LESS_EQUAL,
    COMPARATOR_NOT_EQUAL,
];
pub const ADDRESS_COMPARATOR_TYPE: [&str; 2] = [COMPARATOR_EQUAL, COMPARATOR_NOT_EQUAL];
pub const BOOL_COMPARATOR_TYPE: [&str; 2] = [COMPARATOR_EQUAL, COMPARATOR_NOT_EQUAL];
pub const STRING_COMPARATOR_TYPE: [&str; 2] = [COMPARATOR_EQUAL, COMPARATOR_NOT_EQUAL];
pub const BYTES_COMPARATOR_TYPE: [&str; 2] = [COMPARATOR_EQUAL, COMPARATOR_NOT_EQUAL];
pub const FIXED_BYTES_COMPARATOR_TYPE: [&str; 2] = [COMPARATOR_EQUAL, COMPARATOR_NOT_EQUAL];

// Operator Type Allow by each type
pub const UINT_ARITHMETIC_TYPE: [&str; 5] = [
    OPERATOR_ADD,
    OPERATOR_SUB,
    OPERATOR_MUL,
    OPERATOR_DIV,
    OPERATOR_POW,
];
pub const INT_ARITHMETIC_TYPE: [&str; 5] = [
    OPERATOR_ADD,
    OPERATOR_SUB,
    OPERATOR_MUL,
    OPERATOR_DIV,
    OPERATOR_POW,
];
pub const FLOAT_ARITHMETIC_TYPE: [&str; 5] = [
    OPERATOR_ADD,
    OPERATOR_SUB,
    OPERATOR_MUL,
    OPERATOR_DIV,
    OPERATOR_POW,
];
pub const STRING_ARITHMETIC_TYPE: [&str; 1] = [OPERATOR_ADD];

pub const CATEGORY_NAME: &str = "category";

// Database column names for RuleData
pub const DB_CATEGORY_COLUMN: &str = "category";
pub const DB_TIME_INTERVAL_COLUMN: &str = "time_interval";
pub const DB_SCRIPT_COLUMN: &str = "script";

// JSON token types
pub const JSON_TOKEN_UINT: &str = "Uint";
pub const JSON_TOKEN_ADDRESS: &str = "Address";
pub const JSON_TOKEN_BOOL: &str = "Bool";
pub const JSON_TOKEN_STRING: &str = "String";

// HTTP error types
pub const HTTP_ERROR_TIMEOUT: &str = "timeout";
pub const HTTP_ERROR_REDIRECT: &str = "redirect";
pub const HTTP_ERROR_CONNECTION: &str = "connection";
pub const HTTP_ERROR_REQUEST: &str = "request";
pub const HTTP_ERROR_BODY: &str = "body";
pub const HTTP_ERROR_DECODE: &str = "decode";
pub const HTTP_ERROR_RESPONSE: &str = "response";

// RPC method names
pub const RPC_ETH_CHAIN_ID: &str = "eth_chainId";
pub const RPC_ETH_BLOCK_NUMBER: &str = "eth_blockNumber";
pub const RPC_ETH_GET_BLOCK_BY_NUMBER: &str = "eth_getBlockByNumber";
pub const RPC_ETH_GET_BALANCE: &str = "eth_getBalance";
pub const RPC_ETH_GET_TRANSACTION_BY_HASH: &str = "eth_getTransactionByHash";
pub const RPC_ETH_GET_TRANSACTION_RECEIPT: &str = "eth_getTransactionReceipt";
pub const RPC_TXPOOL_CONTENT: &str = "txpool_content";
pub const RPC_ETH_GET_LOGS: &str = "eth_getLogs";
pub const RPC_ETH_SYNCING: &str = "eth_syncing";

// RPC parameters
pub const RPC_PARAM_LATEST: &str = "latest";
pub const RPC_PARAM_FALSE: &str = "false";
pub const RPC_PARAM_TRUE: &str = "true";

// File extensions and paths
pub const SERVICE_DIR: &str = "service";
pub const YAML_EXTENSION: &str = "yaml";

// RPC call types
pub const RPC_CALL_TYPE_BODY: &str = "body";
pub const RPC_CALL_TYPE_QUERY: &str = "query";

// Timeout and duration constants
pub const ETH_TIMEOUT_DURATION_SECS: u64 = 300;
pub const HTTP_TIMEOUT_SECS: u64 = 30;
pub const HTTP_CONNECT_TIMEOUT_SECS: u64 = 10;
pub const HTTP_POOL_MAX_IDLE_PER_HOST: usize = 0;

// Database constants
pub const DB_EXISTS_QUERY_INDEX: usize = 0;
pub const DB_CHECK_EXISTS_QUERY: &str = "SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1)";

// String parsing constants
pub const HEX_PREFIX_LENGTH: usize = 2;
pub const ETH_ADDRESS_LENGTH: usize = 40;
pub const HEX_RADIX: u32 = 16;
pub const DECIMAL_RADIX: u32 = 10;

// Float formatting constants
pub const FLOAT_PRECISION_MULTIPLIER: f64 = 10000.0;

// Array size constants
pub const UINT_COMPARATOR_COUNT: usize = 6;
pub const INT_COMPARATOR_COUNT: usize = 6;
pub const FLOAT_COMPARATOR_COUNT: usize = 6;
pub const ADDRESS_COMPARATOR_COUNT: usize = 2;
pub const BOOL_COMPARATOR_COUNT: usize = 2;
pub const STRING_COMPARATOR_COUNT: usize = 2;
pub const BYTES_COMPARATOR_COUNT: usize = 2;
pub const FIXED_BYTES_COMPARATOR_COUNT: usize = 2;
pub const UINT_ARITHMETIC_COUNT: usize = 5;
pub const INT_ARITHMETIC_COUNT: usize = 5;
pub const FLOAT_ARITHMETIC_COUNT: usize = 5;
pub const STRING_ARITHMETIC_COUNT: usize = 1;
