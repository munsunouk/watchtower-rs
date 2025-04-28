use ethers::{
    abi::{Abi, Event, ParamType},
    prelude::*,
    types::U64,
};

use sqlx::{postgres::PgRow, Row};

use watch_tower_lib::{
    cli::eth::EthClient,
    rule::contract_event::ContractEventRule,
    utils::{
        constants::{
            DB_ABI_COLUMN, DB_ADDRESS_COLUMN, DB_BLOCK_NUMBER_COLUMN, DB_CHAIN_ID_COLUMN,
            DB_EVENT_INDEX_COLUMN, DB_ID_COLUMN, DB_NAME_COLUMN, DB_VALUES_COLUMN, DEFAULT_INDEX,
        },
        error::IndexType,
        parse_i32_to_usize, parse_to_abi, parse_to_address,
        types::{ChainID, RuleID},
    },
};

use super::{create_contracts, parse_string_to_values};

use crate::utils::error::WorkerError;

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

impl<T: JsonRpcClient> ContractEvent<T> {
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
    pub fn new(client: EthClient<T>, rule: ContractEventRule) -> Self {
        let contracts: Vec<Contract<Provider<T>>> =
            create_contracts(&rule.address, &rule.abi, client.get_providers());
        Self {
            rule,
            client,
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
        tracing::debug!(
            "Getting event for contract at address: {:?}",
            self.rule.address
        );

        let abi = self
            .contracts
            .first()
            .ok_or_else(|| {
                tracing::error!(
                    "No contracts available for address: {:?}",
                    self.rule.address
                );
                WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX))
            })?
            .abi();

        tracing::debug!("ABI loaded, checking events");

        let event = abi.events().next().ok_or_else(|| {
            tracing::error!(
                "No events found in ABI for contract: {:?}",
                self.rule.address
            );
            WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX))
        })?;

        tracing::debug!("Event found: {:?}", event.name);
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

        tracing::info!("event: {:?}", event);

        let signature = event.signature();

        tracing::info!("signature: {:?}", signature);

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

        let input_param_type =
            input_param_types
                .get(self.rule.event_index)
                .ok_or(WorkerError::InvalidIndex(IndexType::USize(
                    self.rule.event_index,
                )))?;

        Ok(input_param_type.clone())
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
        let db_result = client.select_table(DbRuleType::ContractEvent).await?;

        let raw_rule = ContractEventRule::try_from(&db_result[0]).unwrap();

        println!("{:?}", raw_rule);

        Ok(())
    }
}
