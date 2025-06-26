use crate::utils::constants::{DB_SCHEMA_EXISTS, DB_SCHEMA_LOAD, DB_TABLE_NAME, SCHEMA};
use crate::utils::error::DatabaseError;
use crate::utils::{read_service_files, DbTable};
use ethers::abi::Token;
use ethers::types::{H160, U256};
use sqlx::{pool::Pool, postgres::PgRow, Executor, PgPool, Postgres, Row};

use crate::cli::db::data::RuleData;

use chrono::{DateTime, Utc};
use std::future::Future;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::time;

/// Parse JSONB value to [Option<Token>]
///
/// # Arguments
/// * `json_value` - The JSONB value to parse
///
/// # Returns
/// A vector of Option<Token> representing the parsed tokens
pub fn parse_jsonb_to_tokens(
    json_value: serde_json::Value,
) -> Result<Vec<Option<Token>>, DatabaseError> {
    let mut tokens = Vec::new();

    if let Some(array) = json_value.as_array() {
        for value in array {
            if let Some(obj) = value.as_object() {
                let token = if let Some(hex_str) = obj.get("Uint").and_then(|v| v.as_str()) {
                    // Try parsing as hex first, then as decimal
                    if let Ok(num) = U256::from_str_radix(hex_str, 16) {
                        Some(Token::Uint(num))
                    } else if let Ok(num) = U256::from_dec_str(hex_str) {
                        Some(Token::Uint(num))
                    } else {
                        None
                    }
                } else if let Some(addr_str) = obj.get("Address").and_then(|v| v.as_str()) {
                    if let Ok(addr) = H160::from_str(addr_str) {
                        Some(Token::Address(addr))
                    } else {
                        None
                    }
                } else if let Some(bool_val) = obj.get("Bool").and_then(|v| v.as_bool()) {
                    Some(Token::Bool(bool_val))
                } else if let Some(str_val) = obj.get("String").and_then(|v| v.as_str()) {
                    Some(Token::String(str_val.to_string()))
                } else {
                    None
                };
                tokens.push(token);
            } else {
                tokens.push(None);
            }
        }
    }

    Ok(tokens)
}
/// Postgres's Pool type for the DatabasePool
#[derive(Debug, Clone)]
pub struct PostgresClient {
    pool: Pool<Postgres>,
}

impl PostgresClient {
    pub async fn new(database_url: &str) -> Result<Self, DatabaseError> {
        Self::with_retry(|| async {
            let pool = PgPool::connect(database_url).await?;
            Ok(Self { pool })
        })
        .await
    }

    async fn with_retry<F, Fut, T>(f: F) -> Result<T, DatabaseError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, DatabaseError>>,
    {
        let mut attempts = 0;
        let max_attempts = 3;
        let backoff_ms = 1000;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                    time::sleep(time::Duration::from_millis(backoff_ms * attempts as u64)).await;
                }
            }
        }
    }

    /// Initiate
    ///
    /// # Description
    /// This function initiates the database by creating the schema and tables,
    /// and integrates service data from YAML files.
    ///
    /// # Returns
    /// A Result<(), DatabaseError> indicating the success or failure of the operation.
    pub async fn initiate(&self, project_root: &PathBuf) -> Result<(), DatabaseError> {
        let mut conn = self.pool.acquire().await?;
        conn.execute(SCHEMA).await?;

        // Read and integrate service data
        let rules = read_service_files(project_root).await?;

        for rule in rules {
            self.add_rule(&rule).await?;
        }

        Ok(())
    }

    /// Select Table
    ///
    /// # Description
    /// This function selects the table.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    pub async fn select_table(&self, table_name: DbTable) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&DB_SCHEMA_LOAD.replace(DB_TABLE_NAME, table_name.to_str()))
            .fetch_all(&self.pool)
            .await?;

        Ok(result)
    }

    pub async fn schema_exists(&self) -> Result<bool, DatabaseError> {
        let result = sqlx::query(DB_SCHEMA_EXISTS).fetch_one(&self.pool).await?;

        Ok(result.get(0))
    }

    /// Check if RPC Call Rule Exists
    ///
    /// # Description
    /// This function checks if an RPC call rule exists in the database.
    ///
    /// # Arguments
    /// * `rule_id` - The ID of the rule to check.
    ///
    /// # Returns
    /// * `Result<bool, DatabaseError>` - True if the rule exists, false otherwise.
    pub async fn check_rule_exists(
        &self,
        rule_id: i32,
        table_name: DbTable,
    ) -> Result<bool, DatabaseError> {
        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1)",
            table_name.to_str()
        );

        let result = sqlx::query(&query)
            .bind(rule_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(result.get(0))
    }

    /// Add Rule
    ///
    /// # Description
    /// This function adds a new rule to the database.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn add_rule(&self, rule_data: &RuleData) -> Result<(), DatabaseError> {
        sqlx::query(include_str!("sql/add_rule.sql"))
            .bind(&rule_data.category)
            .bind(&rule_data.name)
            .bind(rule_data.time_interval)
            .bind(&rule_data.script)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update Rule
    ///
    /// # Description
    /// This function updates an existing rule in the database.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn update_rule(&self, rule_data: &RuleData) -> Result<(), DatabaseError> {
        sqlx::query(include_str!("sql/update_rule.sql"))
            .bind(&rule_data.category)
            .bind(rule_data.time_interval)
            .bind(&rule_data.script)
            .bind(&rule_data.name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum SelectData {
    Values(Vec<Option<Token>>),
    BlockNumber(usize),
    Timestamp(DateTime<Utc>),
}

#[derive(Debug)]
pub enum QueryParam {
    I32(i32),
    String(String),
    DateTime(DateTime<Utc>),
}
