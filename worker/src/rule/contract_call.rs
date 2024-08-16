use ethers::{
    abi::{Abi, Function, ParamType, Token},
    prelude::*,
};

use super::{create_contracts, encode_token, parse_i32_to_usize, parse_to_abi, parse_to_address};
use sqlx::postgres::PgRow;
use sqlx::Row;
use watch_tower_lib::{
    cli::eth::EthClient,
    utils::{constants::ChainID, error::ClientError},
};

use crate::utils::{
    constants::{
        RuleID, DB_ABI_COLUMN, DB_ADDRESS_COLUMN, DB_BLOCK_NUMBER_COLUMN, DB_CHAIN_ID_COLUMN,
        DB_CHECK_BLOCK_INTERVAL_COLUMN, DB_EXPECTED_VALUE_FILTER_COLUMN,
        DB_EXPECTED_VALUE_FILTER_COMPARATOR_COLUMN, DB_ID_COLUMN, DB_METHOD_PARAMS_COLUMN,
        DB_RULE_FILTER_COLUMN, DB_RULE_FILTER_COMPARATOR_COLUMN, DEFAULT_FN_INPUT_INDEX,
    },
    error::WorkerError,
};

/// Represents a log of contract calls.
#[derive(Clone, Debug)]
pub struct ContractCallBlockLog {
    pub id: RuleID,
    pub block_number: U64,
}

impl From<&PgRow> for ContractCallBlockLog {
    /// Creates a `ContractCallBlockLog` from a database row.
    ///
    /// # Arguments
    ///
    /// * `row` - A reference to a `PgRow`.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractCallBlockLog`.
    fn from(row: &PgRow) -> Self {
        ContractCallBlockLog {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN)),
            block_number: U64::from(parse_i32_to_usize(row.get(DB_BLOCK_NUMBER_COLUMN))),
        }
    }
}

/// Represents a rule for contract calls.
#[derive(Debug, Clone)]
pub struct ContractCallRule {
    pub id: RuleID,
    pub chain_id: ChainID,
    pub address: Address,
    pub abi: Abi,
    pub method_params: Vec<String>,
    pub rule_filter: Vec<String>,
    pub rule_filter_comparator: Vec<String>,
    pub expected_value_filter: String,
    pub expected_value_filter_comparator: String,
    pub check_block_interval: U64,
}

impl From<&PgRow> for ContractCallRule {
    /// Creates a `ContractCallRule` from a database row.
    ///
    /// # Arguments
    ///
    /// * `row` - A reference to a `PgRow`.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractCallRule`.
    fn from(row: &PgRow) -> Self {
        ContractCallRule {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN)),
            chain_id: parse_i32_to_usize(row.get(DB_CHAIN_ID_COLUMN)) as ChainID,
            address: parse_to_address(row.get(DB_ADDRESS_COLUMN)),
            abi: parse_to_abi(row.get(DB_ABI_COLUMN)),
            method_params: row.get(DB_METHOD_PARAMS_COLUMN),
            rule_filter: row.get(DB_RULE_FILTER_COLUMN),
            rule_filter_comparator: row.get(DB_RULE_FILTER_COMPARATOR_COLUMN),
            expected_value_filter: row.get(DB_EXPECTED_VALUE_FILTER_COLUMN),
            expected_value_filter_comparator: row.get(DB_EXPECTED_VALUE_FILTER_COMPARATOR_COLUMN),
            check_block_interval: U64::from(parse_i32_to_usize(
                row.get(DB_CHECK_BLOCK_INTERVAL_COLUMN),
            )),
        }
    }
}

/// Represents a contract call.
#[derive(Clone)]
pub struct ContractCall<T> {
    pub rule: ContractCallRule,
    pub client: EthClient<T>,
    contracts: Vec<Contract<Provider<T>>>,
}

impl<T: JsonRpcClient> ContractCall<T> {
    /// Creates a new `ContractCall` instance.
    ///
    /// # Arguments
    ///
    /// * `client` - The Ethereum client.
    /// * `rule` - The contract call rule.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractCall`.
    pub fn new(client: EthClient<T>, rule: ContractCallRule) -> Self {
        let contracts: Vec<Contract<Provider<T>>> =
            create_contracts(&rule.address, &rule.abi, client.get_providers());

        Self {
            rule,
            client,
            contracts,
        }
    }

    /// Gets the function from the contract ABI.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the function.
    pub fn get_function(&self) -> Result<&Function, WorkerError> {
        let abi = self.contracts.first().unwrap().abi();

        let function = abi.functions().next().unwrap();

        Ok(function)
    }

    /// Gets the function name.
    ///
    /// # Returns
    ///
    /// A result containing the function name as a string.
    pub fn get_function_name(&self) -> Result<String, WorkerError> {
        let function = self.get_function()?;

        let function_name = function.name.clone();

        Ok(function_name)
    }

    /// Gets the input parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the input parameter type.
    pub fn get_input_param_type(&self) -> Result<ParamType, WorkerError> {
        let function = self.get_function()?;

        let function_input = function.inputs.clone();

        let input_param_type = if !function_input.is_empty() {
            let input_param = function_input
                .get(DEFAULT_FN_INPUT_INDEX)
                .expect(&WorkerError::InvalidTypeABI.to_string());

            input_param.kind.clone()
        } else {
            ParamType::Tuple(vec![])
        };

        Ok(input_param_type)
    }

    /// Gets the output parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the output parameter type.
    pub fn get_output_param_type(&self) -> Result<ParamType, WorkerError> {
        let function = self.get_function()?;

        let function_output = function.outputs.clone();

        let output_param_types: Vec<ParamType> = function_output
            .iter()
            .map(|param| param.kind.clone())
            .collect();

        let output_param_type = ParamType::Tuple(output_param_types);

        Ok(output_param_type)
    }

    /// Gets the method parameter token.
    ///
    /// # Returns
    ///
    /// A result containing the method parameter token.
    pub fn get_method_param_token(&self) -> Result<Token, WorkerError> {
        let method_params = self.rule.method_params.clone();
        let input_param_type = self.get_input_param_type()?;

        Ok(encode_token(method_params, &input_param_type))
    }

    /// Gets the method call.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The block ID.
    ///
    /// # Returns
    ///
    /// A result containing the method call.
    pub async fn get_method_call(&self, block_id: BlockId) -> Result<Token, ClientError> {
        let function_name = self
            .get_function_name()
            .map_err(|err| ClientError::InvalidContractCall(err.to_string()))?;
        let method_params = self
            .get_method_param_token()
            .map_err(|err| ClientError::InvalidContractCall(err.to_string()))?;

        let request = self
            .client
            .contracts_call(
                self.contracts.clone(),
                &function_name,
                method_params,
                block_id,
            )
            .await;
        request
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
        let db_result = client.select_table("contract_call_rule").await.unwrap();

        let raw_rule = ContractCallRule::from(&db_result[0]);

        println!("{:?}", raw_rule);

        Ok(())
    }
}
