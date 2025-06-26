use ethers::{
    abi::{Function, Param, ParamType, Token},
    prelude::*,
};

use super::{create_contracts, encode_token};
use watch_tower_lib::{cli::eth::EthClient, rule::contract_call::ContractCallRule};

use crate::{
    option_or_err,
    utils::{constants::DEFAULT_FN_INPUT_INDEX, error::WorkerError},
};

/// Represents a contract call.
#[derive(Clone)]
pub struct ContractCall<T> {
    pub rule: ContractCallRule,
    pub client: EthClient<T>,
    contracts: Vec<Contract<Provider<T>>>,
}

impl<T: JsonRpcClient + Clone> ContractCall<T> {
    /// Helper function to extract ParamTypes from function inputs
    fn extract_param_types(inputs: &[Param]) -> Vec<ParamType> {
        inputs.iter().map(|param| param.kind.clone()).collect()
    }

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
    pub fn new(client: &EthClient<T>, rule: &ContractCallRule) -> Self {
        let contracts: Vec<Contract<Provider<T>>> =
            create_contracts(&rule.address, &rule.abi, client.get_providers());

        Self {
            client: client.to_owned(),
            rule: rule.to_owned(),
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
        let abi = option_or_err!(self.contracts.first()).abi();

        let function = option_or_err!(abi.functions().next());

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

        match function_input.len() {
            0 => Ok(ParamType::Tuple(vec![])),
            1 => {
                let input_param = option_or_err!(function_input.get(DEFAULT_FN_INPUT_INDEX));
                Ok(input_param.kind.clone())
            }
            _ => {
                let param_types = Self::extract_param_types(function_input);
                Ok(ParamType::Tuple(param_types))
            }
        }
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

        let output_param_types = Self::extract_param_types(function_output);
        Ok(ParamType::Tuple(output_param_types))
    }

    /// # Description
    /// This function gets the method parameter token.
    ///
    /// # Returns
    ///
    /// A result containing the method parameter token.
    pub fn get_method_param_token(&self) -> Result<Token, WorkerError> {
        let method_params = &self.rule.method_params;
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
    pub async fn get_method_call(&self, block_id: BlockId) -> Result<Token, WorkerError> {
        let function_name = self.get_function_name()?;
        let method_params = self.get_method_param_token()?;

        Ok(self
            .client
            .contracts_call(&self.contracts, function_name, &method_params, block_id)
            .await?)
    }
}
