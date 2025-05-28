use crate::utils::constants::{
    ADD_CONTRACT_CALL_RULE, ADD_CONTRACT_EVENT_RULE, ADD_EVALUATION_RULE, ADD_RPC_CALL_RULE,
    CONTRACT_CALL_BLOCK_LOG, CONTRACT_CALL_LOG, CONTRACT_CALL_RULE, CONTRACT_EVENT_BLOCK_LOG,
    CONTRACT_EVENT_LOG, CONTRACT_EVENT_RULE, DB_SCHEMA_EXISTS, DB_SCHEMA_LOAD, DB_SCHEMA_MAX_ID,
    DB_SELECT_ID_BY_NAME, DB_TABLE_NAME, DELETE_BY_ID, DELETE_BY_RULE_ID,
    DELETE_EVALUATION_RULE_NAME, EVALUATION_RULE, INSERT_ASSIGN_DATA,
    INSERT_CONTRACT_CALL_BLOCK_LOG, INSERT_CONTRACT_CALL_LOG, INSERT_CONTRACT_EVENT_BLOCK_LOGS,
    INSERT_CONTRACT_EVENT_LOG, INSERT_RPC_LOG, RPC_CALL_LOG, RPC_CALL_RULE, SCHEMA,
    SELECT_ASSIGN_DATA, SELECT_BY_START_DATE, SELECT_EVALUATION_RULE_BY_NAME,
    SELECT_JOIN_EVENT_RULE_CHAIN_ID, SELECT_LOG_BY_RULE_ID, SELECT_LOG_BY_RULE_ID_START_DATE,
    SELECT_RULE_BY_NAME, SELECT_TABLE_BY_EVALUATION_RULE_ID_WITH_LIMIT, SELECT_TABLE_BY_ID,
    SELECT_TABLE_BY_NAME, UPDATE_CONTRACT_CALL_RULE, UPDATE_CONTRACT_EVENT_RULE,
    UPDATE_EVALUATION_RULE, UPDATE_RPC_CALL_RULE,
};
use crate::utils::error::DatabaseError;
use crate::utils::DbRuleType;
use ethers::abi::Token;
use ethers::types::{H160, U256};
use futures::executor::block_on;
use sqlx::{
    pool::Pool,
    postgres::{PgListener, PgRow},
    Executor, PgPool, Postgres, Row,
};

use crate::cli::db::data::{ContractCallRuleData, ContractEventRuleData, RpcCallRuleData};

use super::data::EvaluationRuleData;

use chrono::{DateTime, Utc};
use std::future::Future;
use std::str::FromStr;
use tokio::time;

// use postgres::types::Json;
use postgres::{Client, NoTls};
use serde_json::Value;
// use postgres_types::Json;

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

pub fn select_assign_data_sync(name: &str) -> Result<U256, DatabaseError> {
    // let mut client =
    //     Client::connect("postgres://root:secret@localhost:5434/postgres", NoTls).unwrap();

    // let row = client
    //     .query_one("SELECT * FROM assign_data WHERE name = $1", &[&name])
    //     .unwrap();

    Ok(U256::from(23769979))
}

pub fn select_fetched_raw_data_with_filter(
    rule_type: &str,
    rule_id: i32,
) -> Result<Token, DatabaseError> {
    // let mut client =
    //     Client::connect("postgres://root:secret@localhost:5434/postgres", NoTls).unwrap();

    // let row = client
    //     .query_one(
    //         "SELECT * FROM fetched_raw_data WHERE rule_type = $2 AND rule_id = $3",
    //         &[&rule_type, &rule_id],
    //     )
    //     .unwrap();

    Ok(Token::Int(U256::from(9999998)))

    // Ok(row)
}

/// Postgres's Pool type for the DatabasePool
#[derive(Debug, Clone)]
pub struct PostgresClient {
    pool: Pool<Postgres>,
}

impl PostgresClient {
    pub async fn new(database_url: &str) -> Result<Self, DatabaseError> {
        Self::with_retry(|| async {
            let pool = PgPool::connect(database_url)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
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
}

impl PostgresClient {
    /// Initiate
    ///
    /// # Description
    /// This function initiates the database by creating the schema and tables.
    ///
    /// # Returns
    /// A Result<(), DatabaseError> indicating the success or failure of the operation.
    pub async fn initiate(&self) -> Result<(), DatabaseError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|err| DatabaseError::GenericAquire(err.to_string()))?;

        conn.execute(SCHEMA)
            .await
            .map_err(|err| DatabaseError::GenericInitError(err.to_string()))?;

        Ok(())
    }

    /// Update Rpc Call Log
    ///
    /// # Description
    /// This function updates the RPC call log.
    ///
    /// # Arguments
    /// * `value` - The value to be logged.
    /// * `rule_id` - The ID of the rule.
    /// * `evaluation_rule_id` - The ID of the evaluation rule.
    pub async fn update_rpc_call_log(
        &self,
        value: &str,
        rule_id: i32,
        evaluation_rule_id: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_RPC_LOG)
            .bind(value)
            .bind(rule_id)
            .bind(evaluation_rule_id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        // Notify about the update
        let notification = format!("rpc_call_log_update:{}:{}", rule_id, evaluation_rule_id);
        sqlx::query(&format!(r#"NOTIFY "rpc_updates", '{}'"#, notification))
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericCreateError(err.to_string()))?;

        Ok(())
    }

    /// Update Contract Call Log
    ///
    /// # Description
    /// This function updates the contract call log.
    ///
    /// # Arguments
    /// * `value` - The value to be logged.
    /// * `block_number` - The block number.
    /// * `rule_id` - The ID of the rule.
    /// * `evaluation_rule_id` - The ID of the evaluation rule.
    pub async fn update_contract_call_log(
        &self,
        value: &str,
        block_number: i32,
        rule_id: i32,
        evaluation_rule_id: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_CALL_LOG)
            .bind(value)
            .bind(block_number)
            .bind(rule_id)
            .bind(evaluation_rule_id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    /// Update Contract Event Log
    ///
    /// # Description
    /// This function updates the contract event log.
    ///
    /// # Arguments
    /// * `value` - The value to be logged.
    /// * `tx_hash` - The transaction hash.
    /// * `rule_id` - The ID of the rule.
    /// * `evaluation_rule_id` - The ID of the evaluation rule.
    pub async fn update_contract_event_log(
        &self,
        value: &str,
        tx_hash: &str,
        rule_id: i32,
        evaluation_rule_id: i32,
    ) -> Result<(), DatabaseError> {
        sqlx::query(INSERT_CONTRACT_EVENT_LOG)
            .bind(value)
            .bind(tx_hash)
            .bind(rule_id)
            .bind(evaluation_rule_id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    /// Update Contract Event Block Logs
    ///
    /// # Description
    /// This function updates the contract event block logs.
    ///
    /// # Arguments
    /// * `block_number` - The block number.
    pub async fn update_contract_event_block_logs(
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

    /// Update Contract Call Block Logs
    ///
    /// # Description
    /// This function updates the contract call block logs.
    ///
    /// # Arguments
    /// * `id` - The ID of the rule.
    /// * `block_number` - The block number.
    pub async fn update_contract_call_block_logs(
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

    pub async fn update_assign_data(
        &self,
        name: &str,
        value: i32,
        rule_id: i32,
        rule_type: DbRuleType,
    ) -> Result<(), DatabaseError> {
        let rule_type_str = rule_type.to_str();
        println!("rule_type_str: {:?}", rule_type_str);
        println!("rule_id: {:?}", rule_id);
        println!("name: {:?}", name);
        println!("value: {:?}", value);

        sqlx::query(INSERT_ASSIGN_DATA)
            .bind(name)
            .bind(value)
            .bind(rule_id)
            .bind(rule_type_str)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        println!("inserted assign data");
        Ok(())
    }

    // pub fn select_assign_data_sync(&self, name: &str) -> Result<PgRow, DatabaseError> {
    //     let mut conn = sqlx::postgres::PgConnection::connect_sync(&self.pool.options().url)
    //         .map_err(|err| DatabaseError::GenericAquire(err.to_string()))?;

    //     sqlx::query(SELECT_ASSIGN_DATA)
    //         .bind(name)
    //         .fetch_one(&mut conn)
    //         .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))
    // }

    /// Select Table
    ///
    /// # Description
    /// This function selects the table.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    pub async fn select_table(&self, table_name: DbRuleType) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&DB_SCHEMA_LOAD.replace(DB_TABLE_NAME, table_name.to_str()))
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Select Table By Max Id
    ///
    /// # Description
    /// This function selects the table by max id.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    pub async fn select_table_by_max_id(
        &self,
        table_name: DbRuleType,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&DB_SCHEMA_MAX_ID.replace(DB_TABLE_NAME, table_name.to_str()))
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Select Join Event Rule Chain Id
    ///
    /// # Description
    /// This function selects the join event rule chain id.
    ///
    /// # Returns
    /// A Result<Vec<PgRow>, DatabaseError> indicating the success or failure of the operation.
    pub async fn select_join_event_rule_chain_id(&self) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(SELECT_JOIN_EVENT_RULE_CHAIN_ID)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Select Log By Rule Id
    ///
    /// # Description
    /// This function selects the log by rule id.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    /// * `rule_id` - The ID of the rule.
    pub async fn select_log_by_rule_id(
        &self,
        table_name: DbRuleType,
        rule_id: i32,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result =
            sqlx::query(&SELECT_LOG_BY_RULE_ID.replace(DB_TABLE_NAME, table_name.to_str()))
                .bind(rule_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    pub async fn select_table_by_name(
        &self,
        table_name: DbRuleType,
        name: &str,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&SELECT_TABLE_BY_NAME.replace(DB_TABLE_NAME, table_name.to_str()))
            .bind(name)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;
        Ok(result)
    }

    pub async fn select_table_by_evaluation_rule_id_with_limit(
        &self,
        table_name: DbRuleType,
        evaluation_rule_id: i32,
        limit: i32,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(
            &SELECT_TABLE_BY_EVALUATION_RULE_ID_WITH_LIMIT
                .replace(DB_TABLE_NAME, table_name.to_str()),
        )
        .bind(evaluation_rule_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;
        Ok(result)
    }

    /// Select By Start Date
    ///
    /// # Description
    /// This function selects the log by start date.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    /// * `start_date` - The start date.
    pub async fn select_by_start_date(
        &self,
        table_name: DbRuleType,
        start_date: i64,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&SELECT_BY_START_DATE.replace(DB_TABLE_NAME, table_name.to_str()))
            .bind(start_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Select Log Rule By Id Start Date
    ///
    /// # Description
    /// This function selects the log rule by id start date.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    /// * `rule_id` - The ID of the rule.
    /// * `start_date` - The start date.
    pub async fn select_log_rule_by_id_start_date(
        &self,
        table_name: DbRuleType,
        rule_id: i32,
        start_date: i64,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(
            &SELECT_LOG_BY_RULE_ID_START_DATE.replace(DB_TABLE_NAME, table_name.to_str()),
        )
        .bind(rule_id)
        .bind(start_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Select Rule By Rule Id
    ///
    /// # Description
    /// This function selects the rule by name.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    /// * `name` - The name of the rule.
    pub async fn select_rule_by_name(
        &self,
        table_name: DbRuleType,
        name: String,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let wrapped_name = format!("%{}%", name);

        let mut result =
            sqlx::query(&SELECT_RULE_BY_NAME.replace(DB_TABLE_NAME, table_name.to_str()))
                .bind(name)
                .fetch_all(&self.pool)
                .await
                .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        let evaluation_result = sqlx::query(SELECT_EVALUATION_RULE_BY_NAME)
            .bind(wrapped_name)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        result.extend(evaluation_result);

        Ok(result)
    }

    /// Select Table Rule By Id
    ///
    /// # Description
    /// This function selects the table rule by id.
    pub async fn select_table_rule_by_id(
        &self,
        table_name: DbRuleType,
        id: i32,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let result = sqlx::query(&SELECT_TABLE_BY_ID.replace(DB_TABLE_NAME, table_name.to_str()))
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Update Rpc Call Rule
    ///
    /// # Description
    /// This function updates the RPC call rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn update_rpc_call_rule(
        &self,
        rule_data: &RpcCallRuleData,
    ) -> Result<(), DatabaseError> {
        let RpcCallRuleData {
            url,
            call_type,
            method_type,
            api_body,
            values,
            call_time_interval,
        } = rule_data;

        sqlx::query(UPDATE_RPC_CALL_RULE)
            .bind(url)
            .bind(call_type)
            .bind(method_type)
            .bind(api_body)
            .bind(values)
            .bind(call_time_interval)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    /// Update Contract Call Rule
    ///
    /// # Description
    /// This function updates the contract call rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn update_contract_call_rule(
        &self,
        rule_data: &ContractCallRuleData,
    ) -> Result<(), DatabaseError> {
        let ContractCallRuleData {
            chain_id,
            address,
            abi,
            method_params,
            values,
            check_block_interval,
            target_block_number,
        } = rule_data;
        sqlx::query(UPDATE_CONTRACT_CALL_RULE)
            .bind(chain_id)
            .bind(address)
            .bind(abi)
            .bind(method_params)
            .bind(values)
            .bind(check_block_interval)
            .bind(target_block_number)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        Ok(())
    }

    /// Update Contract Event Rule
    ///
    /// # Description
    /// This function updates the contract event rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn update_contract_event_rule(
        &self,
        rule_data: &ContractEventRuleData,
    ) -> Result<(), DatabaseError> {
        let ContractEventRuleData {
            chain_id,
            address,
            abi,
            event_index,
            values,
        } = rule_data;
        sqlx::query(UPDATE_CONTRACT_EVENT_RULE)
            .bind(chain_id)
            .bind(address)
            .bind(abi)
            .bind(event_index)
            .bind(values)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    /// Update Evaluation Rule
    ///
    /// # Description
    /// This function updates the evaluation rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn update_evaluation_rule(
        &self,
        rule_data: &EvaluationRuleData,
    ) -> Result<(), DatabaseError> {
        let EvaluationRuleData {
            rule_filter,
            expected_value,
        } = rule_data;

        sqlx::query(UPDATE_EVALUATION_RULE)
            .bind(rule_filter)
            .bind(expected_value)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        Ok(())
    }

    /// Add Rpc Call Rule
    ///
    /// # Description
    /// This function adds the RPC call rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn add_rpc_call_rule(
        &self,
        rule_data: &RpcCallRuleData,
    ) -> Result<(), DatabaseError> {
        let RpcCallRuleData {
            url,
            call_type,
            method_type,
            api_body,
            values,
            call_time_interval,
        } = rule_data;

        sqlx::query(ADD_RPC_CALL_RULE)
            .bind(url)
            .bind(call_type)
            .bind(method_type)
            .bind(api_body)
            .bind(values)
            .bind(call_time_interval)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        Ok(())
    }

    /// Add Contract Call Rule
    ///
    /// # Description
    /// This function adds the contract call rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn add_contract_call_rule(
        &self,
        rule_data: &ContractCallRuleData,
    ) -> Result<(), DatabaseError> {
        let ContractCallRuleData {
            chain_id,
            address,
            abi,
            method_params,
            values,
            check_block_interval,
            target_block_number,
        } = rule_data;
        sqlx::query(ADD_CONTRACT_CALL_RULE)
            .bind(chain_id)
            .bind(address)
            .bind(abi)
            .bind(method_params)
            .bind(values)
            .bind(check_block_interval)
            .bind(target_block_number)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    /// Add Contract Event Rule
    ///
    /// # Description
    /// This function adds the contract event rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn add_contract_event_rule(
        &self,
        rule_data: &ContractEventRuleData,
    ) -> Result<(), DatabaseError> {
        let ContractEventRuleData {
            chain_id,
            address,
            abi,
            event_index,
            values,
        } = rule_data;
        sqlx::query(ADD_CONTRACT_EVENT_RULE)
            .bind(chain_id)
            .bind(address)
            .bind(abi)
            .bind(event_index)
            .bind(values)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    /// Add Evaluation Rule
    ///
    /// # Description
    /// This function adds the evaluation rule.
    ///
    /// # Arguments
    /// * `rule_data` - The data of the rule.
    pub async fn add_evaluation_rule(
        &self,
        rule_data: &EvaluationRuleData,
    ) -> Result<(), DatabaseError> {
        let EvaluationRuleData {
            rule_filter,
            expected_value,
        } = rule_data;
        sqlx::query(ADD_EVALUATION_RULE)
            .bind(rule_filter)
            .bind(expected_value)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    pub async fn delete_evaluation_rules_id(&self, id: i32) -> Result<(), DatabaseError> {
        sqlx::query(&DELETE_BY_ID.replace(DB_TABLE_NAME, EVALUATION_RULE))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

        Ok(())
    }

    /// Delete By Rule Id
    ///
    /// # Description
    /// This function deletes the rule by rule id.
    ///
    /// # Arguments
    /// * `table` - The name of the table.
    /// * `rule_id` - The id of the rule.
    pub async fn delete_by_rule_id(
        &self,
        table: DbRuleType,
        rule_id: i32,
    ) -> Result<(), DatabaseError> {
        match table {
            DbRuleType::RpcCall => {
                let wrapped_name = format!("rpccall_{}", rule_id);

                sqlx::query(&DELETE_BY_RULE_ID.replace(DB_TABLE_NAME, RPC_CALL_LOG))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(&DELETE_BY_ID.replace(DB_TABLE_NAME, RPC_CALL_RULE))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(DELETE_EVALUATION_RULE_NAME)
                    .bind(wrapped_name)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;
            }
            DbRuleType::ContractCall => {
                let wrapped_name = format!("contractcall_{}", rule_id);
                sqlx::query(&DELETE_BY_RULE_ID.replace(DB_TABLE_NAME, CONTRACT_CALL_LOG))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(&DELETE_BY_ID.replace(DB_TABLE_NAME, CONTRACT_CALL_BLOCK_LOG))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(&DELETE_BY_ID.replace(DB_TABLE_NAME, CONTRACT_CALL_RULE))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(DELETE_EVALUATION_RULE_NAME)
                    .bind(wrapped_name)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;
            }
            DbRuleType::ContractEvent => {
                let wrapped_name = format!("contractevent_{}", rule_id);
                sqlx::query(&DELETE_BY_RULE_ID.replace(DB_TABLE_NAME, CONTRACT_EVENT_LOG))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(&DELETE_BY_ID.replace(DB_TABLE_NAME, CONTRACT_EVENT_BLOCK_LOG))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(&DELETE_BY_ID.replace(DB_TABLE_NAME, CONTRACT_EVENT_RULE))
                    .bind(rule_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

                sqlx::query(DELETE_EVALUATION_RULE_NAME)
                    .bind(wrapped_name)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Select Latest Logs
    ///
    /// # Description
    /// This function selects the latest logs.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    /// * `limit` - The limit.
    pub async fn select_latest_logs(
        &self,
        table_name: DbRuleType,
        limit: i32,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let query = format!(
            "SELECT * FROM {} ORDER BY created_at DESC LIMIT $1",
            table_name.to_str()
        );

        let result = sqlx::query(&query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    pub async fn get_rule_name_by_id(
        &self,
        rule_id: i32,
        table_name: DbRuleType,
    ) -> Result<Option<PgRow>, DatabaseError> {
        let query = format!("SELECT name FROM {} WHERE id = $1", table_name.to_str());

        let result = sqlx::query(&query)
            .bind(rule_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    /// Select Latest Logs Per Rule
    ///
    /// # Description
    /// This function selects the latest logs per rule.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table.
    pub async fn select_latest_logs_per_rule(
        &self,
        table_name: DbRuleType,
    ) -> Result<Vec<PgRow>, DatabaseError> {
        let query = format!(
            "SELECT DISTINCT ON (rule_id) * FROM {} ORDER BY rule_id, created_at DESC",
            table_name.to_str()
        );

        let result = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result)
    }

    pub async fn schema_exists(&self) -> Result<bool, DatabaseError> {
        let result = sqlx::query(DB_SCHEMA_EXISTS)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::GenericAquire(e.to_string()))?;

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
        table_name: DbRuleType,
    ) -> Result<bool, DatabaseError> {
        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1)",
            table_name.to_str()
        );

        let result = sqlx::query(&query)
            .bind(rule_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        Ok(result.get(0))
    }

    /// Clean Log Table
    ///
    /// # Description
    /// This function cleans (deletes) logs from the specified table that are older than the given timestamp.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to clean
    /// * `older_than_timestamp` - Delete logs older than this Unix timestamp (in seconds)
    pub async fn clean_log_table(
        &self,
        table_name: DbRuleType,
        older_than_timestamp: i64,
    ) -> Result<u64, DatabaseError> {
        let query = format!(
            "DELETE FROM {} WHERE created_at < to_timestamp($1)",
            table_name.to_str()
        );

        let result = sqlx::query(&query)
            .bind(older_than_timestamp)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Check if an index exists in the database
    ///
    /// # Arguments
    /// * `index_name` - The name of the index to check
    pub async fn index_exists(&self, index_name: &str) -> Result<bool, DatabaseError> {
        let result = sqlx::query("SELECT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = $1)")
            .bind(index_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::GenericAquire(e.to_string()))?;

        Ok(result.get(0))
    }

    /// Create indexes if they don't exist
    pub async fn create_indexes(&self) -> Result<(), DatabaseError> {
        // Create indexes for rpc_call_log if they don't exist
        if !self.index_exists("idx_rpc_call_log_rule_id").await? {
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_rpc_call_log_rule_id ON rpc_call_log(rule_id)",
            )
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self
            .index_exists("idx_rpc_call_log_evaluation_rule_id")
            .await?
        {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_rpc_call_log_evaluation_rule_id ON rpc_call_log(evaluation_rule_id)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self.index_exists("idx_rpc_call_log_created_at").await? {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_rpc_call_log_created_at ON rpc_call_log(created_at)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        // Create indexes for contract_call_log if they don't exist
        if !self.index_exists("idx_contract_call_log_rule_id").await? {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_call_log_rule_id ON contract_call_log(rule_id)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self
            .index_exists("idx_contract_call_log_evaluation_rule_id")
            .await?
        {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_call_log_evaluation_rule_id ON contract_call_log(evaluation_rule_id)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self
            .index_exists("idx_contract_call_log_created_at")
            .await?
        {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_call_log_created_at ON contract_call_log(created_at)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self
            .index_exists("idx_contract_call_log_block_number")
            .await?
        {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_call_log_block_number ON contract_call_log(block_number)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        // Create indexes for contract_event_log if they don't exist
        if !self.index_exists("idx_contract_event_log_rule_id").await? {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_event_log_rule_id ON contract_event_log(rule_id)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self
            .index_exists("idx_contract_event_log_evaluation_rule_id")
            .await?
        {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_event_log_evaluation_rule_id ON contract_event_log(evaluation_rule_id)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self
            .index_exists("idx_contract_event_log_created_at")
            .await?
        {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_event_log_created_at ON contract_event_log(created_at)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        if !self.index_exists("idx_contract_event_log_tx_hash").await? {
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_contract_event_log_tx_hash ON contract_event_log(tx_hash)")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::GenericCreateError(e.to_string()))?;
        }

        Ok(())
    }

    /// Insert Fetched Raw Data
    ///
    /// # Description
    /// This function inserts fetched raw data into the database.
    ///
    /// # Arguments
    /// * `rule_type` - The type of the rule.
    /// * `rule_id` - The ID of the rule.
    /// * `values` - The list of Ethereum tokens to be logged.
    /// * `tx_hash` - The transaction hash (optional).
    /// * `block_number` - The block number (optional).
    pub async fn insert_fetched_raw_data(
        &self,
        rule_type: DbRuleType,
        rule_id: i32,
        values: &[Option<Token>],
        tx_hash: Option<&str>,
        block_number: Option<i32>,
    ) -> Result<(), DatabaseError> {
        let query = "INSERT INTO fetched_raw_data (rule_type, rule_id, values, tx_hash, block_number) VALUES ($1, $2, $3, $4, $5)";

        // Convert tokens to JSON
        let values_json = serde_json::to_value(values)
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        sqlx::query(query)
            .bind(rule_type.to_wrapped_str().unwrap())
            .bind(rule_id)
            .bind(values_json)
            .bind(tx_hash)
            .bind(block_number)
            .execute(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        Ok(())
    }

    pub async fn get_fetched_raw_data_with_filter(
        &self,
        select_data: &str,
        query: Query,
    ) -> Result<Vec<SelectData>, DatabaseError> {
        let mut sql = String::from("SELECT ");
        let mut params: Vec<QueryParam> = Vec::new();

        sql.push_str(select_data);
        sql.push_str(" FROM fetched_raw_data WHERE 1=1");

        if let Some(rule_type) = query.rule_type {
            sql.push_str(" AND rule_type = $1");
            params.push(QueryParam::String(rule_type.to_wrapped_str().unwrap()));
        }

        if let Some(rule_id) = query.rule_id {
            sql.push_str(" AND rule_id = $2");
            params.push(QueryParam::I32(rule_id));
        }

        // Handle block number filters
        if let Some(start_block) = query.start_block_number {
            sql.push_str(" AND block_number >= $3");
            params.push(QueryParam::I32(start_block as i32));
        } else {
            // If start_block is None, get the latest block
            sql.push_str(" AND block_number = (SELECT MAX(block_number) FROM fetched_raw_data)");
        }

        if let Some(end_block) = query.end_block_number {
            sql.push_str(" AND block_number <= $4");
            params.push(QueryParam::I32(end_block as i32));
        } else {
            // If start_block is None, get the latest block
            sql.push_str(" AND block_number = (SELECT MAX(block_number) FROM fetched_raw_data)");
        }

        // Handle timestamp filters
        if let Some(start_time) = query.start_timestamp {
            sql.push_str(" AND timestamp >= $5");
            params.push(QueryParam::DateTime(start_time));
        } else {
            // If start_timestamp is None, get the latest timestamp
            sql.push_str(" AND timestamp = (SELECT MAX(timestamp) FROM fetched_raw_data)");
        }

        if let Some(end_time) = query.end_timestamp {
            sql.push_str(" AND timestamp <= $6");
            params.push(QueryParam::DateTime(end_time));
        } else {
            // If start_timestamp is None, get the latest timestamp
            sql.push_str(" AND timestamp = (SELECT MAX(timestamp) FROM fetched_raw_data)");
        }

        let mut query_builder = sqlx::query(&sql);

        for param in params {
            match param {
                QueryParam::I32(val) => query_builder = query_builder.bind(val),
                QueryParam::String(val) => query_builder = query_builder.bind(val),
                QueryParam::DateTime(val) => query_builder = query_builder.bind(val),
            }
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            match select_data {
                "values" => {
                    if let Ok(json_value) = row.try_get::<serde_json::Value, _>(0) {
                        let tokens = parse_jsonb_to_tokens(json_value)?;
                        results.push(SelectData::Values(tokens));
                    }
                }
                "block_number" => {
                    if let Ok(block_number) = row.try_get::<i32, _>(0) {
                        results.push(SelectData::BlockNumber(block_number as usize));
                    }
                }
                "timestamp" => {
                    if let Ok(timestamp) = row.try_get::<DateTime<Utc>, _>(0) {
                        results.push(SelectData::Timestamp(timestamp));
                    }
                }
                _ => {}
            }
        }

        Ok(results)
    }
}

struct PgListenClient {
    inner: PgListener,
}

impl PgListenClient {
    pub async fn new(url: &str) -> Result<Self, DatabaseError> {
        let inner = PgListener::connect(url)
            .await
            .map_err(|e| DatabaseError::GenericAquire(e.to_string()))?;
        Ok(Self { inner })
    }

    pub async fn listen_for_rpc_updates(&mut self) -> Result<(), DatabaseError> {
        self.inner
            .listen("rpc_updates")
            .await
            .map_err(|e| DatabaseError::GenericAquire(e.to_string()))?;
        Ok(())
    }

    pub async fn get_next_notification(&mut self) -> Result<String, DatabaseError> {
        let notification = self
            .inner
            .recv()
            .await
            .map_err(|e| DatabaseError::GenericAquire(e.to_string()))?;
        Ok(notification.payload().to_string())
    }

    // pub async fn listen_and_notify_slack(
    //     &mut self,
    //     slack_client: &crate::cli::slack::SlackClient,
    // ) -> Result<(), DatabaseError> {
    //     // Start listening for RPC updates
    //     self.listen_for_rpc_updates().await?;

    //     println!("Started listening for database events...");

    //     // Continuously listen for notifications
    //     while let Ok(notification) = self.get_next_notification().await {
    //         let title = "Database Event Notification";
    //         let message = format!("Received update: {}", notification);

    //         if let Err(e) = slack_client.send_alert(title, &message).await {
    //             eprintln!("Failed to send Slack notification: {}", e);
    //         }
    //     }

    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {

    use crate::config::set_config;

    use super::*;
    use serde_json::json;
    use std::sync::Once;
    use tracing_subscriber;

    static INIT: Once = Once::new();

    fn setup() -> String {
        let config = set_config("/Users/munseon-ug/rust/watchtower/config.yaml");
        config.postgres_config.url
    }

    #[tokio::test]
    async fn test_postgres_client_schema_exists() -> Result<(), DatabaseError> {
        let client = PostgresClient::new(&setup()).await?;

        let result = client.schema_exists().await?;

        println!("result: {}", result);

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_client() -> Result<(), DatabaseError> {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });

        let client = PostgresClient::new(&setup()).await?;

        client.initiate().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_client_select_name() -> Result<(), DatabaseError> {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });

        let client = PostgresClient::new(&setup()).await?;

        let table_name = DbRuleType::ContractCall;

        let name = "Bifrost-Chainlink-Oracle-usdc".to_string();

        println!("name: {}", name);

        let result = client.select_rule_by_name(table_name, name).await?;

        println!("{:?}", result);

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_client_delete_by_rule_name() -> Result<(), DatabaseError> {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });

        let client = PostgresClient::new(&setup()).await?;
        let table = DbRuleType::ContractCall;
        let rule_id = 1;

        client.delete_by_rule_id(table, rule_id).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_client_add_rpc_call_rule() -> Result<(), DatabaseError> {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });

        let client = PostgresClient::new(&setup()).await?;

        let body = json!({
            "active": "bc1p2cmsnvtvxxvvyxm055vc45827zdyvawsyps6ctqta7lapuh2hepqsp5qas|bc1q6ylrskh4p6u983kx8f0mp0ztwer850u0xzeszj"
        });

        let values = vec!["1.0.0".to_string(), "1.1.0".to_string()];

        let rule_data = RpcCallRuleData {
            url: "https://blockchain.info/balance".to_string(),
            call_type: "query".to_string(),
            method_type: "POST".to_string(),
            api_body: body,
            values,
            call_time_interval: 10,
        };

        client.add_rpc_call_rule(&rule_data).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_client_listen() -> Result<(), DatabaseError> {
        let mut client = PgListenClient::new(&setup()).await?;

        let result = client.inner.listen_all(["postgres"]).await.unwrap();

        println!("Listening!: {:?}", result);

        // let result = client.inner.recv().await.unwrap();
        // println!("Listening!: {:?}", result);

        Ok(())
    }

    #[tokio::test]
    async fn test_rpc_log_notification() -> Result<(), DatabaseError> {
        let mut listener = PgListenClient::new(&setup()).await?;
        let client = PostgresClient::new(&setup()).await?;

        // Start listening for RPC updates
        listener.listen_for_rpc_updates().await?;

        // Update RPC log (this will trigger a notification)
        client.update_rpc_call_log("test_value", 1, 1).await?;

        // Receive the notification
        let notification = listener.get_next_notification().await?;
        println!("Notification: {:?}", notification);
        assert!(notification.contains("rpc_call_log_update:1:1"));

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_client_select_table_by_evaluation_rule_id_with_limit(
    ) -> Result<(), DatabaseError> {
        let client = PostgresClient::new(&setup()).await?;

        let result = client
            .select_table_by_evaluation_rule_id_with_limit(DbRuleType::ContractCallLog, 1, 1)
            .await?;

        println!("{:?}", result);

        Ok(())
    }

    #[test]
    fn test_update_assign_data() -> Result<(), DatabaseError> {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });

        // let client = PostgresClient::new(&setup()).await?;

        // Test data
        let name = "usdc";
        // let value = 42;
        // let rule_id = 1;
        // let rule_type = DbRuleType::ContractCall;

        // // Update assign data
        // client
        //     .update_assign_data(name, value, rule_id, rule_type)
        //     .await?;

        // let result = select_assign_data_sync(name)?;
        // println!("result: {:?}", result);
        // let value = result.get::<usize, i32>(4);
        // println!("result: {:?}", value);

        let rule_type = "contractcall";
        let rule_id = 2;

        let data = select_fetched_raw_data_with_filter(rule_type, rule_id as i32).unwrap();

        // if let Ok(json_value) = data.get::<usize, serde_json::Value>(3) {
        //     let tokens = parse_jsonb_to_tokens(json_value)?;
        //     println!("tokens: {:?}", tokens);
        // }
        // let value = data.try_get::<serde_json::Value, _>(3)?;
        // println!("value: {:?}", value);

        // // Verify the data was inserted by selecting it
        // let result = client.select_assign_data_sync(name)?;
        // println!("result: {:?}", result);
        // let selected_value: i32 = result.try_get("value")?;
        // let selected_rule_id: i32 = result.try_get("rule_id")?;
        // let selected_rule_type: String = result.try_get("rule_type")?;

        // assert_eq!(selected_value, value);
        // assert_eq!(selected_rule_id, rule_id);
        // assert_eq!(selected_rule_type, rule_type.to_str());

        Ok(())
    }

    #[tokio::test]
    async fn test_clean_log_table() -> Result<(), DatabaseError> {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });

        let client = PostgresClient::new(&setup()).await?;

        // Get current timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Clean logs older than current timestamp
        let rows_affected = client
            .clean_log_table(DbRuleType::ContractCallLog, now)
            .await?;

        println!("Cleaned {} rows from RPC call logs", rows_affected);

        Ok(())
    }

    #[tokio::test]
    async fn test_indexes_creation() -> Result<(), DatabaseError> {
        let client = PostgresClient::new(&setup()).await?;

        // Initialize schema to create tables and indexes
        client.initiate().await?;

        // Test log table indexes
        assert!(client.index_exists("idx_rpc_call_log_rule_id").await?);
        assert!(
            client
                .index_exists("idx_rpc_call_log_evaluation_rule_id")
                .await?
        );
        assert!(client.index_exists("idx_rpc_call_log_created_at").await?);

        assert!(client.index_exists("idx_contract_call_log_rule_id").await?);
        assert!(
            client
                .index_exists("idx_contract_call_log_evaluation_rule_id")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_call_log_created_at")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_call_log_block_number")
                .await?
        );

        assert!(
            client
                .index_exists("idx_contract_event_log_rule_id")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_event_log_evaluation_rule_id")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_event_log_created_at")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_event_log_tx_hash")
                .await?
        );

        // Test block log indexes
        assert!(
            client
                .index_exists("idx_contract_event_block_log_block_number")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_call_block_log_block_number")
                .await?
        );

        // Test rule table indexes
        assert!(client.index_exists("idx_rpc_call_rule_name").await?);
        assert!(client.index_exists("idx_contract_call_rule_name").await?);
        assert!(client.index_exists("idx_contract_event_rule_name").await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_indexes_without_data_loss() -> Result<(), DatabaseError> {
        let client = PostgresClient::new(&setup()).await?;

        // Create indexes without affecting existing data
        client.create_indexes().await?;

        // Verify indexes were created
        assert!(client.index_exists("idx_rpc_call_log_rule_id").await?);
        assert!(
            client
                .index_exists("idx_rpc_call_log_evaluation_rule_id")
                .await?
        );
        assert!(client.index_exists("idx_rpc_call_log_created_at").await?);

        assert!(client.index_exists("idx_contract_call_log_rule_id").await?);
        assert!(
            client
                .index_exists("idx_contract_call_log_evaluation_rule_id")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_call_log_created_at")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_call_log_block_number")
                .await?
        );

        assert!(
            client
                .index_exists("idx_contract_event_log_rule_id")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_event_log_evaluation_rule_id")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_event_log_created_at")
                .await?
        );
        assert!(
            client
                .index_exists("idx_contract_event_log_tx_hash")
                .await?
        );

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

#[derive(Debug)]
pub struct Query {
    pub value_id: Option<usize>,
    pub start_block_number: Option<usize>,
    pub end_block_number: Option<usize>,
    pub start_timestamp: Option<DateTime<Utc>>,
    pub end_timestamp: Option<DateTime<Utc>>,
    pub rule_type: Option<DbRuleType>,
    pub rule_id: Option<i32>,
}

fn jsonb_to_vec(value: &str) -> Result<Vec<Value>, serde_json::Error> {
    serde_json::from_str::<Vec<Value>>(value)
}
