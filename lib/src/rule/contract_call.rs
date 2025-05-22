use ethers::{
    abi::{Abi, Function, Param, ParamType, Token},
    prelude::*,
};
use serde_json::Value;

use std::convert::TryFrom;

use super::{parse_string_to_index, parse_string_to_target_index, TargetIndex};
use crate::{
    cli::eth::EthClient,
    utils::{
        constants::{
            DB_ABI_COLUMN, DB_ADDRESS_COLUMN, DB_BLOCK_NUMBER_COLUMN, DB_CHAIN_ID_COLUMN,
            DB_CHECK_BLOCK_INTERVAL_COLUMN, DB_ID_COLUMN, DB_METHOD_PARAMS_COLUMN, DB_NAME_COLUMN,
            DB_TARGET_BLOCK_NUMBER_COLUMN, DB_VALUES_COLUMN, DEFAULT_INDEX,
        },
        error::{ClientError, GeneralError, IndexType},
        parse_i32_to_usize, parse_i64_to_u64, parse_string_to_u64, parse_to_abi, parse_to_address,
        parse_u256_to_u64,
        types::{ChainID, RuleID},
    },
};
use sqlx::{postgres::PgRow, Row};

/// # Description
/// This struct represents a log of contract calls.
/// # Fields
/// * `id` - The ID of the rule.
/// * `block_number` - The block number.
#[derive(Clone, Debug)]
pub struct ContractCallBlockLog {
    pub id: RuleID,
    pub block_number: U64,
}

impl TryFrom<&PgRow> for ContractCallBlockLog {
    type Error = GeneralError;

    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN))
                .map_err(|e| GeneralError::InvalidTypeConvertError(e.to_string()))?,
            block_number: U64::from(
                parse_i32_to_usize(row.get(DB_BLOCK_NUMBER_COLUMN))
                    .map_err(|e| GeneralError::InvalidTypeConvertError(e.to_string()))?,
            ),
        })
    }
}

/// # Description
/// This struct represents a rule for contract calls.
/// # Fields
/// * `chain_id` - The chain ID.
/// * `address` - The address.
/// * `abi` - The ABI.
/// * `method_params` - The method parameters.
/// * `target_index` - The target_index.
/// * `target_block_number` - The target block number from latest block.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractCallRule {
    pub chain_id: ChainID,
    pub address: Address,
    pub abi: Abi,
    pub method_params: Vec<Option<Token>>,
    pub target_index: Vec<TargetIndex>,
    pub target_block_number: U64,
}

impl TryFrom<&PgRow> for ContractCallRule {
    type Error = GeneralError;

    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        let chain_id = parse_i32_to_usize(row.get(DB_CHAIN_ID_COLUMN)).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse chain ID: {}", e))
        })? as ChainID;

        let check_block_interval = parse_i32_to_usize(row.get(DB_CHECK_BLOCK_INTERVAL_COLUMN))
            .map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!(
                    "Failed to parse block interval: {}",
                    e
                ))
            })?;

        let target_block_number = parse_string_to_u64(row.get(DB_TARGET_BLOCK_NUMBER_COLUMN))
            .map_err(|e| GeneralError::InvalidTypeConvertError(e.to_string()))?;

        let target_index =
            parse_string_to_target_index(row.get(DB_VALUES_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?;

        Ok(Self {
            chain_id,
            address: parse_to_address(row.get(DB_ADDRESS_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            abi: parse_to_abi(row.get(DB_ABI_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            method_params: serde_json::from_str(row.get(DB_METHOD_PARAMS_COLUMN)).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!(
                    "Failed to parse method params: {}",
                    e
                ))
            })?,
            target_index,
            target_block_number: target_block_number,
        })
    }
}

impl ContractCallRule {
    pub fn new(
        chain_id: i32,
        address: String,
        abi: Value,
        params: Vec<Option<Token>>,
        target_index: String,
        target_block_number: U256,
    ) -> Result<Self, GeneralError> {
        let chain_id = parse_i32_to_usize(chain_id).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse chain ID: {}", e))
        })? as ChainID;

        let target_block_number = parse_u256_to_u64(target_block_number);

        let target_index = parse_string_to_target_index(target_index).map_err(|e| {
            GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
        })?;

        Ok(Self {
            chain_id,
            address: parse_to_address(address).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            abi: parse_to_abi(abi).map_err(|e| {
                GeneralError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
            })?,
            method_params: params,
            target_index,
            target_block_number: target_block_number,
        })
    }
}
