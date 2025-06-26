use ethers::{
    abi::{Event, ParamType},
    prelude::*,
};

use watch_tower_lib::{cli::eth::EthClient, rule::contract_event::ContractEventRule};

use super::create_contracts;

use crate::{option_or_err, utils::error::WorkerError};

/// # Description
/// This struct represents a contract event.
/// # Fields
/// * `rule` - The rule.
/// * `contracts` - The contracts.
#[derive(Clone)]
pub struct ContractEvent<T> {
    pub rule: ContractEventRule,
    pub client: EthClient<T>,
    contracts: Vec<Contract<Provider<T>>>,
}

impl<T: JsonRpcClient + Clone> ContractEvent<T> {
    /// # Description
    /// This function creates a new `ContractEvent` instance.
    ///
    /// # Arguments
    ///
    /// * `client` - The Ethereum client.
    /// * `rule` - The contract event rule.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEvent`.
    pub fn new(client: &EthClient<T>, rule: &ContractEventRule) -> Self {
        let contracts: Vec<Contract<Provider<T>>> =
            create_contracts(&rule.address, &rule.abi, client.get_providers());
        Self {
            rule: rule.to_owned(),
            client: client.to_owned(),
            contracts,
        }
    }

    /// # Description
    /// This function gets the event from the contract ABI.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the event.
    pub fn get_event(&self) -> Result<&Event, WorkerError> {
        let abi = option_or_err!(self.contracts.first()).abi();

        let event = option_or_err!(abi.events().next());

        Ok(event)
    }

    /// # Description
    /// This function gets the event signature.
    ///
    /// # Returns
    ///
    /// A result containing the event signature as `H256`.
    pub fn get_event_signature(&self) -> Result<H256, WorkerError> {
        let event = self.get_event()?;

        let signature = event.signature();

        Ok(signature)
    }

    /// # Description
    /// This function gets the raw input parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the raw input parameter type.
    pub fn get_raw_input_param_type(&self) -> Result<ParamType, WorkerError> {
        let event: &Event = self.get_event()?;

        let event_input = &event.inputs;

        let input_param_types: Vec<ParamType> =
            event_input.iter().map(|param| param.kind.clone()).collect();

        let input_param_type = option_or_err!(input_param_types.get(self.rule.event_index));

        Ok(input_param_type.to_owned())
    }

    /// # Description
    /// This function gets the input parameter type.
    ///
    /// # Returns
    ///
    /// A result containing the input parameter type.
    pub fn get_input_param_type(&self) -> Result<ParamType, WorkerError> {
        let event = self.get_event()?;

        let event_input = &event.inputs;

        let input_param_types: Vec<ParamType> =
            event_input.iter().map(|param| param.kind.clone()).collect();

        let parsing_input_param_type = ParamType::Tuple(input_param_types);

        Ok(parsing_input_param_type)
    }

    /// # Description
    /// This function checks if the provided signature matches the target event signature.
    ///
    /// # Arguments
    ///
    /// * `signature` - The event signature to check.
    ///
    /// # Returns
    ///
    /// `true` if the signature matches the target event signature, otherwise `false`.
    pub fn is_target_event(&self, signature: &H256) -> Result<bool, WorkerError> {
        let target_signature = self.get_event_signature()?;

        Ok(*signature == target_signature)
    }
}
