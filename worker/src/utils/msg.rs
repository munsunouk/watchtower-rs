use ethers::abi::Token;

use ethers::types::{Log, U64};

use crate::utils::constants::RuleID;

#[derive(Clone, Debug)]
/// The message format passed through the block channel.
pub struct RpcCallRawMessage {
    pub status: U64,
    pub rule_id: RuleID,
}

impl RpcCallRawMessage {
    pub fn new(status: U64, rule_id: RuleID) -> Self {
        Self { status, rule_id }
    }
}

#[derive(Clone, Debug)]
/// The message format passed through the block channel.
pub struct ContractCallRawMessage {
    /// The processed block number.
    pub block_number: U64,
    /// The detected transaction logs from the target contracts.
    pub call_token: Token,
    pub rule_id: RuleID,
}

impl ContractCallRawMessage {
    pub fn new(block_number: U64, call_token: Token, rule_id: RuleID) -> Self {
        Self {
            block_number,
            call_token,
            rule_id,
        }
    }
}

#[derive(Clone, Debug)]
/// The message format passed through the block channel.
pub struct ContractEventRawMessage {
    /// The detected transaction logs from the target contracts.
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
