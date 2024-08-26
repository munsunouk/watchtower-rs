use serde::Deserialize;
use std::fs;
use watch_tower_lib::db::{
    data::{ContractCallRuleData, ContractEventRuleData, RpcCallRuleData},
    postgres::PostgresClient,
};

use sqlx::Row;

use super::constants::{DB_CONTRACT_CALL_RULE, DB_CONTRACT_EVENT_RULE, DB_RPC_CALL_RULE};

//TODO: Remove data.rs after API is ready
pub const MAX_ID: &str = "max_id";
pub const INCREASE_ID: i32 = 1;
pub const DEFAULT_ID: usize = 0;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct RuleData {
    rpc_call_rule_data: Option<Vec<RpcCallRuleData>>,
    contract_call_rule_data: Option<Vec<ContractCallRuleData>>,
    contract_event_rule_data: Option<Vec<ContractEventRuleData>>,
}

#[allow(dead_code)]
impl RuleData {
    pub fn new(path: &str) -> Self {
        let data = fs::read_to_string(path).unwrap();
        let rule_data: RuleData = serde_json::from_str(&data).unwrap();
        rule_data
    }

    async fn check_rule_id(&mut self, postgres_client: PostgresClient) {
        let rpc_call_result = postgres_client
            .select_table_by_max_id(DB_RPC_CALL_RULE)
            .await
            .unwrap();
        let rpc_call_row = rpc_call_result.get(DEFAULT_ID).unwrap();

        let contract_call_result = postgres_client
            .select_table_by_max_id(DB_CONTRACT_CALL_RULE)
            .await
            .unwrap();
        let contract_call_row = contract_call_result.get(DEFAULT_ID).unwrap();

        let contract_event_result = postgres_client
            .select_table_by_max_id(DB_CONTRACT_EVENT_RULE)
            .await
            .unwrap();
        let contract_event_row = contract_event_result.get(DEFAULT_ID).unwrap();

        let mut rpc_call_row_max_id = rpc_call_row.get::<i32, _>(MAX_ID);
        let mut contract_call_row_max_id = contract_call_row.get::<i32, _>(MAX_ID);
        let mut contract_event_row_max_id = contract_event_row.get::<i32, _>(MAX_ID);

        let rpc_call_rules = if let Some(mut rules) = self.rpc_call_rule_data.clone() {
            for rule in &mut rules {
                if rule.id.is_none() {
                    rpc_call_row_max_id += INCREASE_ID;
                    rule.id = Some(rpc_call_row_max_id);
                }
            }
            Some(rules)
        } else {
            None
        };

        let contract_call_rules = if let Some(mut rules) = self.contract_call_rule_data.clone() {
            for rule in &mut rules {
                if rule.id.is_none() {
                    contract_call_row_max_id += INCREASE_ID;
                    rule.id = Some(contract_call_row_max_id);
                }
            }
            Some(rules)
        } else {
            None
        };

        let contract_event_rules = if let Some(mut rules) = self.contract_event_rule_data.clone() {
            for rule in &mut rules {
                if rule.id.is_none() {
                    contract_event_row_max_id += INCREASE_ID;
                    rule.id = Some(contract_event_row_max_id);
                }
            }
            Some(rules)
        } else {
            None
        };

        self.rpc_call_rule_data = rpc_call_rules;
        self.contract_call_rule_data = contract_call_rules;
        self.contract_event_rule_data = contract_event_rules;
    }

    /// Updates the rules in the database.
    ///
    /// # Arguments
    ///
    /// * `postgres_client` - The PostgreSQL client.
    async fn update_rule(&mut self, postgres_client: PostgresClient) {
        self.check_rule_id(postgres_client.clone()).await;

        if let Some(rules) = self.rpc_call_rule_data.clone() {
            for rule in rules {
                let _ = postgres_client.update_rpc_call_rule(rule).await;
            }
        }

        if let Some(rules) = self.contract_call_rule_data.clone() {
            for rule in rules {
                let _ = postgres_client.update_contract_call_rule(rule).await;
            }
        }

        if let Some(rules) = self.contract_event_rule_data.clone() {
            for rule in rules {
                let _ = postgres_client.update_contract_event_rule(rule).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_data() {
        const DATA_PATH: &str = "./worker/src/utils/data/sample_data.json";
        const DATABASE_URL: &str = "postgres://root:secret@localhost:5432/postgres";

        let mut rule_data = RuleData::new(DATA_PATH);

        let postgres_client = PostgresClient::new(DATABASE_URL).await.unwrap();
        rule_data.update_rule(postgres_client).await;
    }
}
