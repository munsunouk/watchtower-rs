use std::{collections::HashMap, sync::Arc};

use ethers::providers::JsonRpcClient;
use tokio::sync::RwLock;
use watch_tower_lib::utils::{evaluation::EvaluationRule, types::RuleID};

use crate::rule::{ContractCall, RpcCall};

use super::types::SharedChainState;

/// # Description
/// This struct represents the shared states for different types of rules.
/// # Fields
///
/// * `rpc_call_states` - An arc read-write lock of a hash map of rule IDs and their corresponding RPC call rules.
/// * `contract_call_states` - An arc read-write lock of a hash map of rule IDs and their corresponding contract call rules.
/// * `contract_event_states` - An arc read-write lock of a hash map of chain IDs, rule IDs, and their corresponding contract event rules.
/// * `evaluator_states` - An arc read-write lock of a hash map of rule IDs and their corresponding evaluation rules.
#[derive(Clone)]
pub struct SharedStates<T> {
    pub rpc_call_states: Arc<RwLock<HashMap<RuleID, RpcCall>>>,
    pub contract_call_states: Arc<RwLock<HashMap<RuleID, ContractCall<T>>>>,
    pub contract_event_states: SharedChainState<T>,
    pub evaluator_states: Arc<RwLock<HashMap<RuleID, EvaluationRule>>>,
}

impl<T: JsonRpcClient> SharedStates<T> {
    pub fn new(
        rpc_call_states: Arc<RwLock<HashMap<RuleID, RpcCall>>>,
        contract_call_states: Arc<RwLock<HashMap<RuleID, ContractCall<T>>>>,
        contract_event_states: SharedChainState<T>,
        evaluator_states: Arc<RwLock<HashMap<RuleID, EvaluationRule>>>,
    ) -> Self {
        Self {
            rpc_call_states,
            contract_call_states,
            contract_event_states,
            evaluator_states,
        }
    }
}
