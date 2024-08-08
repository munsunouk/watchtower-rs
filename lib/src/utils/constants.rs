/// The default retries of a single json rpc request.
pub const DEFAULT_CALL_RETRIES: u8 = 3;
/// The default retry interval of a single json rpc request in milliseconds.
pub const DEFAULT_CALL_RETRY_INTERVAL_MS: u64 = 3000;

pub const DEFAULT_GET_LOGS_BATCH_SIZE: u64 = 1;

// DB Query PATH
pub const SCHEMA: &str = include_str!("../db/sql/create_schema.sql");
pub const INSERT_RPC_LOG: &str = include_str!("../db/sql/insert_rpc_log.sql");
pub const INSERT_CONTRACT_CALL_LOG: &str = include_str!("../db/sql/insert_contract_call_log.sql");
pub const INSERT_CONTRACT_EVENT_LOG: &str =
    include_str!("../db/sql/insert_contract_event_call_log.sql");
pub const INSERT_CONTRACT_EVENT_BLOCK_LOG: &str =
    include_str!("../db/sql/insert_contract_event_block_log.sql");
pub const SAMPLE_DATA: &str = include_str!("../db/sql/sample_data.sql");

// DB SCHEMA
pub const DB_SCHEMA_LOAD: &str = "SELECT * FROM %%TABLE_NAME%%";
pub const DB_TABLE_NAME: &str = "%%TABLE_NAME%%";

/// The type of EVM chain ID's.
pub type ChainID = u32;
