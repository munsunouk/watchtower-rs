use ethers::types::U64;
use ethers::{
    abi::{Abi, Event, ParamType},
    prelude::*,
};

use sqlx::postgres::PgRow;
use sqlx::Row;

use watch_tower_lib::{cli::eth::EthClient, utils::constants::ChainID};

use super::{create_contracts, parse_i32_to_usize, parse_to_abi, parse_to_address};

use crate::utils::constants::{
    RuleID, DB_ABI_COLUMN, DB_ADDRESS_COLUMN, DB_BLOCK_NUMBER_COLUMN, DB_CHAIN_ID_COLUMN,
    DB_EVENT_INDEX_COLUMN, DB_EXPECTED_VALUE_FILTER_COLUMN,
    DB_EXPECTED_VALUE_FILTER_COMPARATOR_COLUMN, DB_ID_COLUMN, DB_RULE_FILTER_COLUMN,
    DB_RULE_FILTER_COMPARATOR_COLUMN,
};
use crate::utils::error::WorkerError;

/// Represents a log of contract events.
#[derive(Clone, Debug)]
pub struct ContractEventBlockLog {
    pub id: RuleID,
    pub block_number: U64,
    pub chain_id: ChainID,
}

impl From<&PgRow> for ContractEventBlockLog {
    /// Creates a `ContractEventBlockLog` from a database row.
    ///
    /// # Arguments
    ///
    /// * `row` - A reference to a `PgRow`.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEventBlockLog`.
    fn from(row: &PgRow) -> Self {
        ContractEventBlockLog {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN)),
            block_number: U64::from(parse_i32_to_usize(row.get(DB_BLOCK_NUMBER_COLUMN))),
            chain_id: parse_i32_to_usize(row.get(DB_CHAIN_ID_COLUMN)) as ChainID,
        }
    }
}

/// Represents a rule for contract events.
#[derive(Clone, Debug)]
pub struct ContractEventRule {
    pub id: RuleID,
    pub chain_id: ChainID,
    pub address: Address,
    pub abi: Abi,
    pub event_index: usize,
    pub rule_filter: Vec<String>,
    pub rule_filter_comparator: Vec<String>,
    pub expected_value_filter: String,
    pub expected_value_filter_comparator: String,
}

impl From<&PgRow> for ContractEventRule {
    /// Creates a `ContractEventRule` from a database row.
    ///
    /// # Arguments
    ///
    /// * `row` - A reference to a `PgRow`.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEventRule`.
    fn from(row: &PgRow) -> Self {
        ContractEventRule {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN)),
            chain_id: parse_i32_to_usize(row.get(DB_CHAIN_ID_COLUMN)) as ChainID,
            address: parse_to_address(row.get(DB_ADDRESS_COLUMN)),
            abi: parse_to_abi(row.get(DB_ABI_COLUMN)),
            event_index: parse_i32_to_usize(row.get(DB_EVENT_INDEX_COLUMN)),
            rule_filter: row.get(DB_RULE_FILTER_COLUMN),
            rule_filter_comparator: row.get(DB_RULE_FILTER_COMPARATOR_COLUMN),
            expected_value_filter: row.get(DB_EXPECTED_VALUE_FILTER_COLUMN),
            expected_value_filter_comparator: row.get(DB_EXPECTED_VALUE_FILTER_COMPARATOR_COLUMN),
        }
    }
}

/// Represents a contract event.
#[derive(Clone)]
pub struct ContractEvent<T> {
    pub rule: ContractEventRule,
    contracts: Vec<Contract<Provider<T>>>,
}

impl<T: JsonRpcClient> ContractEvent<T> {
    /// Creates a new `ContractEvent` instance.
    ///
    /// # Arguments
    ///
    /// * `client` - The Ethereum client.
    /// * `rule` - The contract event rule.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEvent`.
    pub fn new(client: EthClient<T>, rule: ContractEventRule) -> Self {
        let contracts: Vec<Contract<Provider<T>>> =
            create_contracts(&rule.address, &rule.abi, client.get_providers());
        Self { rule, contracts }
    }

    /// Gets the event from the contract ABI.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the event.
    pub fn get_event(&self) -> Result<&Event, WorkerError> {
        let abi = self.contracts.first().unwrap().abi();

        let event = abi.events().next().unwrap();

        Ok(event)
    }

    /// Gets the event signature.
    ///
    /// # Returns
    ///
    /// A result containing the event signature as `H256`.
    pub fn get_event_signature(&self) -> Result<H256, WorkerError> {
        let event = self.get_event()?;

        let signature = event.signature();

        Ok(signature)
    }

    /// Gets the raw input parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the raw input parameter type.
    pub fn get_raw_input_param_type(&self) -> Result<ParamType, WorkerError> {
        let event: &Event = self.get_event()?;

        let event_input = event.inputs.clone();

        let input_param_types: Vec<ParamType> =
            event_input.iter().map(|param| param.kind.clone()).collect();

        let input_param_types_cloned = input_param_types.clone();

        let input_param_type = input_param_types_cloned.get(self.rule.event_index).unwrap();

        Ok(input_param_type.clone())
    }

    /// Gets the input parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the input parameter type.
    pub fn get_input_param_type(&self) -> Result<ParamType, WorkerError> {
        let event = self.get_event()?;

        let event_input = event.inputs.clone();

        let input_param_types: Vec<ParamType> =
            event_input.iter().map(|param| param.kind.clone()).collect();

        let parsing_input_param_type = ParamType::Tuple(input_param_types);

        Ok(parsing_input_param_type)
    }

    /// Checks if the provided signature matches the target event signature.
    ///
    /// # Arguments
    ///
    /// * `signature` - The event signature to check.
    ///
    /// # Returns
    ///
    /// `true` if the signature matches the target event signature, otherwise `false`.
    pub fn is_target_event(&self, signature: &H256) -> bool {
        let target_signature = self.get_event_signature().unwrap();

        *signature == target_signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber;

    use watch_tower_lib::{db::postgres::PostgresClient, utils::error::DatabaseError};

    #[tokio::test]
    async fn test_postgres_client() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new("postgres://root:secret@localhost:5432/postgres").await?;

        // client.initiate().await?;
        let db_result = client.select_table("contract_event_rule").await.unwrap();

        let raw_rule = ContractEventRule::from(&db_result[0]);

        println!("{:?}", raw_rule);

        Ok(())
    }
}
