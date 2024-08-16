use serde::Deserialize;
use std::fs;
use watch_tower_lib::db::{
    data::{ContractCallRuleData, ContractEventRuleData, RpcCallRuleData},
    postgres::PostgresClient,
};

#[allow(dead_code)] // TODO: leave it till Api
#[derive(Deserialize, Debug)]
struct RuleData {
    rpc_call_rule_data: Option<Vec<RpcCallRuleData>>,
    contract_call_rule_data: Option<Vec<ContractCallRuleData>>,
    contract_event_rule_data: Option<Vec<ContractEventRuleData>>,
}

#[allow(dead_code)] // TODO: leave it till Api
impl RuleData {
    pub fn new(path: &str) -> Self {
        let data = fs::read_to_string(path).unwrap();
        let rule_data: RuleData = serde_json::from_str(&data).unwrap();
        rule_data
    }

    async fn insert_rule(&self, postgres_client: PostgresClient) {
        if let Some(rules) = self.rpc_call_rule_data.clone() {
            for rule in rules {
                let _ = postgres_client.insert_rpc_call_rule(rule).await;
            }
        }

        if let Some(rules) = self.contract_call_rule_data.clone() {
            for rule in rules {
                let _ = postgres_client.insert_contract_call_rule(rule).await;
            }
        }

        if let Some(rules) = self.contract_event_rule_data.clone() {
            for rule in rules {
                let _ = postgres_client.insert_contract_event_rule(rule).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_data() {
        const DATA_PATH: &str = "./src/utils/data/sample_data.json";
        const DATABASE_URL: &str = "postgres://root:secret@localhost:5432/postgres";

        let rule_data = RuleData::new(DATA_PATH);

        let postgres_client = PostgresClient::new(DATABASE_URL).await.unwrap();
        rule_data.insert_rule(postgres_client).await;
    }
}
