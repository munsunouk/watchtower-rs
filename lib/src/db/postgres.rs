use crate::utils::error::DatabaseError;
use sqlx::postgres::PgRow;
use sqlx::{pool::Pool, Executor, PgPool, Postgres};

use crate::utils::constants::{
    DB_SCHEMA_LOAD, DB_TABLE_NAME, INSERT_CONTRACT_CALL_BLOCK_LOG, INSERT_CONTRACT_CALL_LOG,
    INSERT_CONTRACT_CALL_RULE, INSERT_CONTRACT_EVENT_BLOCK_LOGS, INSERT_CONTRACT_EVENT_LOG,
    INSERT_CONTRACT_EVENT_RULE, INSERT_RPC_CALL_RULE, INSERT_RPC_LOG, SCHEMA,
    SELECT_JOIN_EVENT_RULE_CHAIN_ID,
};

use crate::db::data::{ContractCallRuleData, ContractEventRuleData, RpcCallRuleData};

///Postgres's Pool type for the DatabasePool
#[derive(Debug, Clone)]
pub struct PostgresClient {
    pool: Pool<Postgres>,
}

impl PostgresClient {
    pub async fn new(database_url: &str) -> Result<Self, DatabaseError> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;

        Ok(Self { pool })
    }
}

impl PostgresClient {
    pub async fn initiate(&self) -> Result<(), DatabaseError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|err| DatabaseError::GenericAquire(err.to_string()))?;

        let _ = conn
            .execute(SCHEMA)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    pub async fn insert_rpc_call_log(
        &self,
        value: &str,
        rule_id: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_RPC_LOG)
            .bind(value)
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn insert_contract_call_log(
        &self,
        value: &str,
        block_number: i32,
        rule_id: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_CALL_LOG)
            .bind(value)
            .bind(block_number)
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn insert_contract_event_log(
        &self,
        value: &str,
        tx_hash: &str,
        rule_id: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_EVENT_LOG)
            .bind(value)
            .bind(tx_hash)
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn insert_contract_event_block_logs(
        &self,
        block_number: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_EVENT_BLOCK_LOGS)
            .bind(block_number)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn insert_contract_call_block_logs(
        &self,
        id: i32,
        block_number: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_CALL_BLOCK_LOG)
            .bind(id)
            .bind(block_number)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn select_table(&self, table_name: &str) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&DB_SCHEMA_LOAD.replace(DB_TABLE_NAME, table_name))
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    pub async fn select_join_event_rule_chain_id(&self) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(SELECT_JOIN_EVENT_RULE_CHAIN_ID)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    pub async fn insert_rpc_call_rule(
        &self,
        rule_data: RpcCallRuleData,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_RPC_CALL_RULE)
            .bind(rule_data.id)
            .bind(rule_data.name)
            .bind(rule_data.url)
            .bind(rule_data.expected_value)
            .bind(rule_data.comparator)
            .bind(rule_data.call_time_interval)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn insert_contract_call_rule(
        &self,
        rule_data: ContractCallRuleData,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_CALL_RULE)
            .bind(rule_data.id)
            .bind(rule_data.name)
            .bind(rule_data.chain_id)
            .bind(rule_data.address)
            .bind(rule_data.abi)
            .bind(rule_data.method_params)
            .bind(rule_data.rule_filter)
            .bind(rule_data.rule_filter_comparator)
            .bind(rule_data.expected_value_filter)
            .bind(rule_data.expected_value_filter_comparator)
            .bind(rule_data.check_block_interval)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn insert_contract_event_rule(
        &self,
        rule_data: ContractEventRuleData,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_EVENT_RULE)
            .bind(rule_data.id)
            .bind(rule_data.name)
            .bind(rule_data.chain_id)
            .bind(rule_data.address)
            .bind(rule_data.abi)
            .bind(rule_data.event_index)
            .bind(rule_data.rule_filter)
            .bind(rule_data.rule_filter_comparator)
            .bind(rule_data.expected_value_filter)
            .bind(rule_data.expected_value_filter_comparator)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tracing_subscriber;

    const DB_URL: &str = "postgres://root:secret@localhost:5432/postgres";

    #[tokio::test]
    async fn test_postgres_client() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        client.initiate().await?;

        Ok(())
    }
}
