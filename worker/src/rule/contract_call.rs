use ethers::{
    abi::{Function, ParamType, Token},
    prelude::*,
};

use super::{create_contracts, encode_token};
use watch_tower_lib::{
    cli::eth::EthClient,
    rule::contract_call::ContractCallRule,
    utils::{
        constants::DEFAULT_INDEX,
        error::{ClientError, IndexType},
    },
};

use crate::utils::{constants::DEFAULT_FN_INPUT_INDEX, error::WorkerError};

/// Represents a contract call.
#[derive(Clone)]
pub struct ContractCall<T> {
    pub rule: ContractCallRule,
    pub client: EthClient<T>,
    contracts: Vec<Contract<Provider<T>>>,
}

impl<T: JsonRpcClient> ContractCall<T> {
    /// # Description
    /// This function creates a new `ContractCall` instance.
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

    /// # Description
    /// This function gets the function from the contract ABI.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the function.
    pub fn get_function(&self) -> Result<&Function, WorkerError> {
        let abi = self
            .contracts
            .first()
            .ok_or(WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?
            .abi();

        let function = abi
            .functions()
            .next()
            .ok_or(WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

        Ok(function)
    }

    /// # Description
    /// This function gets the function name.
    ///
    /// # Returns
    ///
    /// A result containing the function name as a string.
    pub fn get_function_name(&self) -> Result<&str, WorkerError> {
        let function = self.get_function()?;

        Ok(&function.name)
    }

    /// # Description
    /// This function gets the input parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the input parameter type.
    pub fn get_input_param_type(&self) -> Result<ParamType, WorkerError> {
        let function = self.get_function()?;

        let function_input = &function.inputs;

        let input_param_type = if function_input.is_empty() {
            ParamType::Tuple(vec![])
        } else if function_input.len() == 1 {
            let input_param =
                function_input
                    .get(DEFAULT_FN_INPUT_INDEX)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                        DEFAULT_FN_INPUT_INDEX,
                    )))?;
            input_param.kind.clone()
        } else {
            let input_param_types: Vec<ParamType> = function_input
                .iter()
                .map(|param| param.kind.clone())
                .collect();
            ParamType::Tuple(input_param_types)
        };

        Ok(input_param_type)
    }

    /// # Description
    /// This function gets the output parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the output parameter type.
    pub fn get_output_param_type(&self) -> Result<ParamType, WorkerError> {
        let function = self.get_function()?;

        let function_output = &function.outputs;

        let output_param_types: Vec<ParamType> = function_output
            .iter()
            .map(|param| param.kind.clone())
            .collect();

        let output_param_type = ParamType::Tuple(output_param_types);

        Ok(output_param_type)
    }

    /// # Description
    /// This function gets the method parameter token.
    ///
    /// # Returns
    ///
    /// A result containing the method parameter token.
    pub fn get_method_param_token(&self) -> Result<Token, WorkerError> {
        let method_params = self.rule.method_params.clone();
        let input_param_type = self.get_input_param_type()?;

        encode_token(method_params, &input_param_type)
    }

    /// # Description
    /// This function gets the method call.
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

        self.client
            .contracts_call(
                self.contracts.clone(),
                function_name,
                method_params,
                block_id,
            )
            .await
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tracing_subscriber;

    use watch_tower_lib::{
        cli::db::postgres::PostgresClient,
        utils::{error::DatabaseError, DbRuleType},
    };

    #[tokio::test]
    async fn test_postgres_client() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt::init();

        let client = PostgresClient::new("<YOUR_DATABASE_URL>").await?;

        // client.initiate().await?;
        let db_result = client.select_table(DbRuleType::ContractCall).await?;

        let raw_rule = ContractCallRule::try_from(&db_result[0]).unwrap();

        println!("{:?}", raw_rule);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_output_param_type() -> Result<(), WorkerError> {
        let client = PostgresClient::new("<YOUR_DATABASE_URL>").await.unwrap();

        let db_result = client.select_table(DbRuleType::ContractCall).await.unwrap();

        let raw_rule = ContractCallRule::try_from(&db_result[0]).unwrap();

        let function_output = raw_rule.abi.functions().next().unwrap().outputs.clone();

        let output_param_types: Vec<ParamType> = function_output
            .iter()
            .map(|param| param.kind.clone())
            .collect();

        let output_param_type = ParamType::Tuple(output_param_types);

        println!("{:?}", output_param_type);

        Ok(())
    }
}
