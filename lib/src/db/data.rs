use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Clone, Debug)]
pub struct RpcCallRuleData {
    pub id: Option<i32>,
    pub name: String,
    pub url: String,
    pub expected_value: i32,
    pub comparator: String,
    pub call_time_interval: i32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContractCallRuleData {
    pub id: Option<i32>,
    pub name: String,
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub method_params: Vec<String>,
    pub rule_filter: Vec<String>,
    pub rule_filter_comparator: Vec<String>,
    pub expected_value_filter: String,
    pub expected_value_filter_comparator: String,
    pub check_block_interval: i32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContractEventRuleData {
    pub id: Option<i32>,
    pub name: String,
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub event_index: i32,
    pub rule_filter: Vec<String>,
    pub rule_filter_comparator: Vec<String>,
    pub expected_value_filter: String,
    pub expected_value_filter_comparator: String,
}
