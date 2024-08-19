/// The Maximum retries of a single json rpc request.
pub const MAX_RETRY_CALL: u8 = 3;
/// Limit Retry Call
pub const LIMIT_RETRY_CALL: u8 = 0;
/// The default retry interval of a single json rpc request in milliseconds.
pub const DEFAULT_CALL_RETRY_INTERVAL_MS: u64 = 3000;

// DB Query PATH
pub const SCHEMA: &str = include_str!("../db/sql/create_schema.sql");
pub const INSERT_RPC_LOG: &str = include_str!("../db/sql/insert_rpc_log.sql");
pub const INSERT_CONTRACT_CALL_LOG: &str = include_str!("../db/sql/insert_contract_call_log.sql");
pub const INSERT_CONTRACT_EVENT_LOG: &str =
    include_str!("../db/sql/insert_contract_event_call_log.sql");
pub const INSERT_CONTRACT_EVENT_BLOCK_LOGS: &str =
    include_str!("../db/sql/insert_contract_event_block_logs.sql");
pub const INSERT_CONTRACT_CALL_BLOCK_LOG: &str =
    include_str!("../db/sql/insert_contract_call_block_log.sql");
pub const SELECT_JOIN_EVENT_RULE_CHAIN_ID: &str =
    include_str!("../db/sql/select_join_event_rule_chain_id.sql");
pub const INSERT_RPC_CALL_RULE: &str = include_str!("../db/sql/insert_rpc_call_rule.sql");
pub const INSERT_CONTRACT_CALL_RULE: &str = include_str!("../db/sql/insert_contract_call_rule.sql");
pub const INSERT_CONTRACT_EVENT_RULE: &str =
    include_str!("../db/sql/insert_contract_event_rule.sql");

// DB SCHEMA
pub const DB_SCHEMA_LOAD: &str = "SELECT * FROM %%TABLE_NAME%%";
pub const DB_TABLE_NAME: &str = "%%TABLE_NAME%%";

/// The type of EVM chain ID's.
pub type ChainID = u32;
