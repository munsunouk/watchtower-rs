use ethers::{
    abi::{Abi, Function, ParamType, Token},
    prelude::*,
};

use std::convert::TryFrom;

use super::{create_contracts, encode_token, parse_string_to_values};
use sqlx::{postgres::PgRow, Row};
use watch_tower_lib::{
    cli::eth::EthClient,
    rule::contract_call::ContractCallRule,
    utils::{
        constants::{
            DB_ABI_COLUMN, DB_ADDRESS_COLUMN, DB_BLOCK_NUMBER_COLUMN, DB_CHAIN_ID_COLUMN,
            DB_CHECK_BLOCK_INTERVAL_COLUMN, DB_ID_COLUMN, DB_METHOD_PARAMS_COLUMN, DB_NAME_COLUMN,
            DB_TARGET_BLOCK_NUMBER_COLUMN, DB_VALUES_COLUMN, DEFAULT_INDEX,
        },
        error::{ClientError, IndexType},
        parse_i32_to_usize, parse_to_abi, parse_to_address,
        types::{ChainID, RuleID},
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

        let input_param_type = if !function_input.is_empty() {
            let input_param =
                function_input
                    .get(DEFAULT_FN_INPUT_INDEX)
                    .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                        DEFAULT_FN_INPUT_INDEX,
                    )))?;

            input_param.kind.clone()
        } else {
            ParamType::Tuple(vec![])
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
    use std::sync::Arc;

    use super::*;
    use std::str::FromStr;
    use tracing_subscriber;

    use watch_tower_lib::{
        cli::{db::postgres::PostgresClient, eth::ProviderMetadata},
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

    #[tokio::test]
    async fn test_contract_call() {
        let rpc = "<YOUR_RPC_URL>";

        let client = EthClient::<Http>::new(
            ProviderMetadata::new(
                "bifrost".to_string(),
                vec![rpc.to_string()],
                3068 as ChainID,
            ),
            vec![Arc::new(Provider::try_from(rpc).unwrap())],
        );

        let abi_str = r#"
[{"name": "latestRoundData", "type": "function", "inputs": [], "outputs": [{"name": "roundId", "type": "uint80", "internalType": "uint80"}, {"name": "answer", "type": "int256", "internalType": "int256"}, {"name": "startedAt", "type": "uint256", "internalType": "uint256"}, {"name": "updatedAt", "type": "uint256", "internalType": "uint256"}, {"name": "answeredInRound", "type": "uint80", "internalType": "uint80"}], "stateMutability": "view"}]"#;

        let abi = parse_to_abi(serde_json::from_str(abi_str).unwrap()).unwrap();

        let address = Address::from_str("0x77348eAee88F7bce55D0ff3cd74f69E91c2A7165").unwrap();

        let contracts: Vec<Contract<Provider<Http>>> =
            create_contracts(&address, &abi, client.get_providers());

        let method_call = client
            .contracts_call(
                contracts,
                "latestRoundData",
                Token::Tuple(vec![]),
                BlockId::Number(BlockNumber::Number(U64::from(21149110))),
            )
            .await
            .unwrap();

        println!("{:?}", method_call);
    }
}
