use ethers::{
    abi::{Abi, Event, ParamType},
    prelude::*,
    types::U64,
};

use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

use crate::{
    cli::eth::EthClient,
    utils::{
        constants::{
            DB_ABI_COLUMN, DB_ADDRESS_COLUMN, DB_BLOCK_NUMBER_COLUMN, DB_CHAIN_ID_COLUMN,
            DB_EVENT_INDEX_COLUMN, DB_ID_COLUMN, DB_NAME_COLUMN, DB_VALUES_COLUMN, DEFAULT_INDEX,
        },
        error::{GeneralError, IndexType},
        parse_i32_to_usize, parse_i64_to_u64, parse_to_abi, parse_to_address, parse_u256_to_u64,
        types::{ChainID, RuleID},
    },
};

use super::parse_string_to_index;

/// # Description
/// This struct represents a log of contract events.
/// # Fields
/// * `id` - The ID of the rule.
/// * `block_number` - The block number.
#[derive(Clone, Debug)]
pub struct ContractEventBlockLog {
    pub id: RuleID,
    pub block_number: U64,
    pub chain_id: ChainID,
}

impl TryFrom<&PgRow> for ContractEventBlockLog {
    type Error = GeneralError;

    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        let id = parse_i32_to_usize(row.get(DB_ID_COLUMN))
            .map_err(|e| GeneralError::InvalidTypeConvertError(e.to_string()))?;

        let block_number = parse_i32_to_usize(row.get(DB_BLOCK_NUMBER_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse block number: {}", e))
        })?;

        let chain_id = parse_i32_to_usize(row.get(DB_CHAIN_ID_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse chain ID: {}", e))
        })?;

        Ok(Self {
            id,
            block_number: U64::from(block_number),
            chain_id: chain_id as ChainID,
        })
    }
}

/// # Description
/// This struct represents a rule for contract events.
/// # Fields
/// * `id` - The ID of the rule.
/// * `chain_id` - The chain ID.
/// * `address` - The address.
/// * `abi` - The ABI.
/// * `event_index` - The event index.
/// * `values` - The values.
#[derive(Clone, Debug, PartialEq)]
pub struct ContractEventRule {
    pub chain_id: ChainID,
    pub address: Address,
    pub abi: Abi,
    pub event_index: usize,
    pub target_index: Vec<usize>,
    pub target_block_number: U64,
}

impl TryFrom<&PgRow> for ContractEventRule {
    type Error = GeneralError;

    /// # Description
    /// This function creates a `ContractEventRule` from a database row.
    ///
    /// # Arguments
    ///
    /// * `row` - A reference to a `PgRow`.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractEventRule`.
    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        let chain_id = parse_i32_to_usize(row.get(DB_CHAIN_ID_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse chain ID: {}", e))
        })?;

        let event_index = parse_i32_to_usize(row.get(DB_EVENT_INDEX_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse event index: {}", e))
        })?;

        Ok(ContractEventRule {
            chain_id: chain_id as ChainID,
            address: parse_to_address(row.get(DB_ADDRESS_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            abi: parse_to_abi(row.get(DB_ABI_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            event_index,
            target_index: parse_string_to_index(row.get(DB_VALUES_COLUMN))?,
            target_block_number: parse_i64_to_u64(row.get(DB_BLOCK_NUMBER_COLUMN)),
        })
    }
}

impl ContractEventRule {
    pub fn new(
        chain_id: i32,
        address: String,
        abi: Value,
        event_index: i32,
        target_index: String,
        target_block_number: U256,
    ) -> Result<Self, GeneralError> {
        let target_block_number = parse_u256_to_u64(target_block_number);

        let chain_id = parse_i32_to_usize(chain_id).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse chain ID: {}", e))
        })? as ChainID;

        let target_index = parse_string_to_index(target_index).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
        })?;

        let event_index = parse_i32_to_usize(event_index).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse event index: {}", e))
        })?;

        Ok(Self {
            chain_id,
            address: parse_to_address(address).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            abi: parse_to_abi(abi).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            event_index,
            target_index,
            target_block_number,
        })
    }
}
