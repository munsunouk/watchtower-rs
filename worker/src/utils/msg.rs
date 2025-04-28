use ethers::abi::{ParamType, Token};

use ethers::types::{Log, U64};

use watch_tower_lib::utils::types::RuleID;
use watch_tower_lib::utils::DbRuleType;

#[derive(Clone, Debug)]
/// # Description
/// This struct represents the raw message for RPC call.
/// # Fields
/// * `token` - The token of the RPC call.
/// * `rule_id` - The rule ID.
pub struct RpcCallRawMessage {
    pub token: Token,
    pub param_type: ParamType,
    pub rule_id: RuleID,
}

impl RpcCallRawMessage {
    pub fn new(token: Token, param_type: ParamType, rule_id: RuleID) -> Self {
        Self {
            token,
            param_type,
            rule_id,
        }
    }
}

#[derive(Clone, Debug)]
/// # Description
/// This struct represents the raw message for contract call.
/// # Fields
/// * `block_tokens` - The call result from the target contracts with block number.
/// * `rule_id` - The rule ID.
pub struct ContractCallRawMessage {
    pub block_tokens: Vec<(Token, U64)>,
    pub rule_id: RuleID,
}

impl ContractCallRawMessage {
    pub fn new(block_tokens: Vec<(Token, U64)>, rule_id: RuleID) -> Self {
        Self {
            block_tokens,
            rule_id,
        }
    }
}

#[derive(Clone, Debug)]
/// # Description
/// This struct represents the raw message for contract event.
/// # Fields
/// * `event_logs` - The detected transaction logs from the target contracts.
/// * `block_number` - The block number.
pub struct ContractEventRawMessage {
    pub event_logs: Vec<Log>,
    pub block_number: U64,
}

impl ContractEventRawMessage {
    pub fn new(event_logs: Vec<Log>, block_number: U64) -> Self {
        Self {
            event_logs,
            block_number,
        }
    }
}

/// # Description
/// This struct represents the raw message for decoding.
/// # Fields
///
/// * `rule_id` - The rule ID.
/// * `rule_type` - The rule type.
/// * `tokens` - The tokens.
/// * `block_number` - The block number.
/// * `tx_hash` - The transaction hash.
#[derive(Clone, Debug)]
pub struct DecodeRawMessage {
    pub rule_id: RuleID,
    pub rule_type: DbRuleType,
    pub tokens: Vec<Option<Token>>,
    pub block_number: Option<U64>,
    pub tx_hash: Option<String>,
}

impl DecodeRawMessage {
    pub fn new(
        rule_id: RuleID,
        rule_type: DbRuleType,
        tokens: Vec<Option<Token>>,
        block_number: Option<U64>,
        tx_hash: Option<String>,
    ) -> Self {
        Self {
            rule_id,
            rule_type,
            tokens,
            block_number,
            tx_hash,
        }
    }
}
