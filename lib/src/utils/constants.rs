/// The Maximum retries of a single json rpc request.
pub const MAX_RETRY_CALL: u8 = 3;
/// Limit Retry Call
pub const LIMIT_RETRY_CALL: u8 = 0;
/// The default retry interval of a single json rpc request in milliseconds.
pub const DEFAULT_CALL_RETRY_INTERVAL_MS: u64 = 3000;

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
pub const INSERT_RPC_LOG: &str = include_str!("../cli/db/sql/insert_rpc_log.sql");
pub const INSERT_CONTRACT_CALL_LOG: &str =
    include_str!("../cli/db/sql/insert_contract_call_log.sql");
pub const INSERT_CONTRACT_EVENT_LOG: &str =
    include_str!("../cli/db/sql/insert_contract_event_call_log.sql");
pub const INSERT_CONTRACT_EVENT_BLOCK_LOGS: &str =
    include_str!("../cli/db/sql/insert_contract_event_block_logs.sql");
pub const INSERT_CONTRACT_CALL_BLOCK_LOG: &str =
    include_str!("../cli/db/sql/insert_contract_call_block_log.sql");
pub const INSERT_ASSIGN_DATA: &str = include_str!("../cli/db/sql/insert_assign_data.sql");
pub const SELECT_ASSIGN_DATA: &str = include_str!("../cli/db/sql/select_assign_data.sql");
pub const SELECT_JOIN_EVENT_RULE_CHAIN_ID: &str =
    include_str!("../cli/db/sql/select_join_event_rule_chain_id.sql");
pub const SELECT_LOG_BY_RULE_ID: &str = include_str!("../cli/db/sql/select_log_by_rule_id.sql");
pub const SELECT_BY_START_DATE: &str = include_str!("../cli/db/sql/select_by_start_date.sql");
pub const SELECT_LOG_BY_RULE_ID_START_DATE: &str =
    include_str!("../cli/db/sql/select_log_by_rule_id_start_date.sql");
pub const SELECT_RULE_BY_NAME_START_DATE: &str =
    include_str!("../cli/db/sql/select_rule_by_name_start_date.sql");
pub const SELECT_EVALUATION_RULE_BY_NAME_START_DATE: &str =
    include_str!("../cli/db/sql/select_evaluation_rule_by_name_start_date.sql");
pub const SELECT_EVALUATION_RULE_BY_NAME: &str =
    include_str!("../cli/db/sql/select_evaluation_rule_by_name.sql");
pub const SELECT_RULE_BY_NAME: &str = include_str!("../cli/db/sql/select_rule_by_name.sql");
pub const UPDATE_RPC_CALL_RULE: &str = include_str!("../cli/db/sql/update_rpc_call_rule.sql");
pub const UPDATE_CONTRACT_CALL_RULE: &str =
    include_str!("../cli/db/sql/update_contract_call_rule.sql");
pub const UPDATE_CONTRACT_EVENT_RULE: &str =
    include_str!("../cli/db/sql/update_contract_event_rule.sql");
pub const ADD_RPC_CALL_RULE: &str = include_str!("../cli/db/sql/add_rpc_call_rule.sql");
pub const ADD_CONTRACT_CALL_RULE: &str = include_str!("../cli/db/sql/add_contract_call_rule.sql");
pub const ADD_CONTRACT_EVENT_RULE: &str = include_str!("../cli/db/sql/add_contract_event_rule.sql");
pub const UPDATE_EVALUATION_RULE: &str = include_str!("../cli/db/sql/update_evaluation_rule.sql");
pub const ADD_EVALUATION_RULE: &str = include_str!("../cli/db/sql/add_evaluation_rule.sql");

pub const DELETE_BY_ID: &str = include_str!("../cli/db/sql/delete_by_id.sql");
pub const DELETE_BY_RULE_ID: &str = include_str!("../cli/db/sql/delete_by_rule_id.sql");

pub const DELETE_EVALUATION_RULE_NAME: &str =
    include_str!("../cli/db/sql/delete_evaluation_name.sql");
pub const SELECT_TABLE_BY_NAME: &str = include_str!("../cli/db/sql/select_table_by_name.sql");
pub const SELECT_TABLE_BY_ID: &str = include_str!("../cli/db/sql/select_table_by_id.sql");
pub const SELECT_TABLE_BY_EVALUATION_RULE_ID_WITH_LIMIT: &str =
    include_str!("../cli/db/sql/select_log_by_evaluation_rule_id_with_limit.sql");

// DB SCHEMA
pub const DB_SCHEMA_LOAD: &str = "SELECT * FROM %%TABLE_NAME%%";
pub const DB_TABLE_NAME: &str = "%%TABLE_NAME%%";
pub const DB_SCHEMA_MAX_ID: &str = "SELECT COALESCE(MAX(id), 0) AS max_id FROM %%TABLE_NAME%%";
pub const RULE_ID: &str = "%%RULE_ID%%";
pub const DB_SELECT_ID_BY_NAME: &str = "SELECT id FROM %%TABLE_NAME%% WHERE name = $1";
pub const DB_SCHEMA_EXISTS: &str = "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = 'rpc_call_rule'
            )";

// DB Table Name
pub const RPC_CALL_LOG: &str = "rpc_call_log";
pub const CONTRACT_CALL_LOG: &str = "contract_call_log";
pub const CONTRACT_EVENT_LOG: &str = "contract_event_log";
pub const RPC_CALL_RULE: &str = "rpc_call_rule";
pub const CONTRACT_CALL_RULE: &str = "contract_call_rule";
pub const CONTRACT_EVENT_RULE: &str = "contract_event_rule";
pub const EVALUATION_RULE: &str = "evaluation_rule";
pub const CONTRACT_CALL_BLOCK_LOG: &str = "contract_call_block_log";
pub const CONTRACT_EVENT_BLOCK_LOG: &str = "contract_event_block_log";
pub const RPC_CALL: &str = "rpc_call";
pub const CONTRACT_CALL: &str = "contract_call";
pub const CONTRACT_EVENT: &str = "contract_event";
pub const EVALUATION: &str = "evaluation";

// Rule Type
pub const RPC_CALL_RULE_TYPE: &str = "rpccall";
pub const CONTRACT_CALL_RULE_TYPE: &str = "contractcall";
pub const CONTRACT_EVENT_RULE_TYPE: &str = "contractevent";
pub const EVALUATION_RULE_TYPE: &str = "evaluation";
pub const CONTRACT_CALL_LOG_TYPE: &str = "contractcalllog";
pub const CONTRACT_EVENT_LOG_TYPE: &str = "contracteventlog";
pub const RPC_CALL_LOG_TYPE: &str = "rpccalllog";
pub const CONTRACT_CALL_BLOCK_LOG_TYPE: &str = "contractcallblocklog";
pub const CONTRACT_EVENT_BLOCK_LOG_TYPE: &str = "contracteventblocklog";

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
pub const UINT_ARITHMETIC_TYPE: [&str; 4] =
    [OPERATOR_ADD, OPERATOR_SUB, OPERATOR_MUL, OPERATOR_DIV];
pub const INT_ARITHMETIC_TYPE: [&str; 4] = [OPERATOR_ADD, OPERATOR_SUB, OPERATOR_MUL, OPERATOR_DIV];
pub const FLOAT_ARITHMETIC_TYPE: [&str; 4] =
    [OPERATOR_ADD, OPERATOR_SUB, OPERATOR_MUL, OPERATOR_DIV];
pub const STRING_ARITHMETIC_TYPE: [&str; 1] = [OPERATOR_ADD];
