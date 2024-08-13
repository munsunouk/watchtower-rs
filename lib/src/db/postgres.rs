use crate::utils::error::DatabaseError;
use sqlx::postgres::PgRow;
use sqlx::{pool::Pool, Executor, PgPool, Postgres};

use crate::utils::constants::{
    DB_SCHEMA_LOAD, DB_TABLE_NAME, INSERT_CONTRACT_CALL_LOG, INSERT_CONTRACT_EVENT_BLOCK_LOGS,
    INSERT_CONTRACT_EVENT_LOG, INSERT_RPC_LOG, SCHEMA, SELECT_JOIN_EVENT_RULE_CHAIN_ID,
};

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
}

#[cfg(test)]
mod tests {
    use crate::utils::constants::{INSERT_RPC_RULE_DATA, SAMPLE_DATA};

    use super::*;
    use sqlx::Row;
    use tracing_subscriber;

    const DB_URL: &str = "postgres://root:secret@localhost:5432/postgres";

    #[tokio::test]
    async fn test_postgres_client() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        client.initiate().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_rpc_rule_data() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        let mut conn = client
            .pool
            .acquire()
            .await
            .map_err(|err| DatabaseError::GenericAquire(err.to_string()))?;

        let _ = conn
            .execute(INSERT_RPC_RULE_DATA)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_sample_data() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        let mut conn = client
            .pool
            .acquire()
            .await
            .map_err(|err| DatabaseError::GenericAquire(err.to_string()))?;

        let _ = conn
            .execute(SAMPLE_DATA)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_contract_call_log() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        let _ = client
            .insert_contract_call_log("100000", 123, 1)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_contract_event_log() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        let _ = client
            .insert_contract_event_log("100000", "0xa189…a0a3", 1)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_contract_event_block_logs() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        let _ = client
            .insert_contract_event_block_logs(19157523)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_select_join_event_rule_chain_id() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new(DB_URL).await?;

        let result = client.select_join_event_rule_chain_id().await?;

        println!("{:?}", result.get(0).unwrap().columns());

        Ok(())
    }
}
