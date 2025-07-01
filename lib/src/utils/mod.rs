pub mod constants;
pub mod error;
pub mod types;

use crate::cli::db::data::YamlRule;
use crate::utils::error::{DatabaseError, GeneralError};
use crate::{cli::db::data::RuleData, option_or_err};

use abi::ParamType;
use constants::{
    ADDRESS_COMPARATOR_TYPE, BOOL_COMPARATOR_TYPE, BYTES_COMPARATOR_TYPE, COMPARATOR_EQUAL,
    COMPARATOR_GREATER, COMPARATOR_GREATER_EQUAL, COMPARATOR_LESS, COMPARATOR_LESS_EQUAL,
    COMPARATOR_NOT_EQUAL, DECIMAL_RADIX, ETH_ADDRESS_LENGTH, FIXED_BYTES_COMPARATOR_TYPE,
    FLOAT_ARITHMETIC_TYPE, FLOAT_COMPARATOR_TYPE, FLOAT_PRECISION_MULTIPLIER, HEX_PREFIX_LENGTH,
    HEX_RADIX, INT_ARITHMETIC_TYPE, INT_COMPARATOR_TYPE, OPERATOR_ADD, OPERATOR_DIV, OPERATOR_MUL,
    OPERATOR_POW, OPERATOR_SUB, RULE, STRING_ARITHMETIC_TYPE, STRING_COMPARATOR_TYPE,
    UINT_ARITHMETIC_TYPE, UINT_COMPARATOR_TYPE,
};
use reqwest::Method;
use serde_json::{from_str, Value};
use sqlx::types::Json;
use url;

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ethers::{
    abi::{Abi, Token, Uint},
    prelude::*,
    utils::hex,
};
use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::rule::parse_token_to_string;
use crate::utils::constants::{
    RPC_CALL_TYPE_BODY, RPC_CALL_TYPE_QUERY, SERVICE_DIR, YAML_EXTENSION,
};
use crate::utils::types::GeneralToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcCallType {
    Body,
    Query,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DbTable {
    Rule,
}

impl FromStr for DbTable {
    type Err = GeneralError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            RULE => Ok(Self::Rule),
            _ => Err(GeneralError::InvalidRuleDecode(
                "Invalid rule type".to_string(),
            )),
        }
    }
}

impl DbTable {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Rule => RULE,
        }
    }

    pub fn to_wrapped_str(&self) -> Result<String, GeneralError> {
        match self {
            Self::Rule => Ok(RULE.to_string()),
        }
    }
}

/// Converts an i32 to usize.
///
/// # Arguments
///
/// * `input` - An i32 value.
///
/// # Returns
///
/// A usize value.
pub fn parse_i32_to_usize(input: i32) -> Result<usize, GeneralError> {
    Ok(input.try_into()?)
}

pub fn parse_u256_to_u64(input: &U256) -> U64 {
    U64::from(input.as_u64())
}

pub fn parse_string_to_u64(input: String) -> Result<U64, GeneralError> {
    Ok(input.parse::<U64>()?)
}

pub fn parse_i64_to_u64(input: i64) -> U64 {
    U64::from(input)
}

pub fn parse_string_to_method(input: String) -> Method {
    Method::from_bytes(input.as_bytes()).unwrap_or(Method::POST)
}

pub fn parse_string_to_rpc_call_type(input: String) -> Result<RpcCallType, GeneralError> {
    match input.as_str() {
        RPC_CALL_TYPE_BODY => Ok(RpcCallType::Body),
        RPC_CALL_TYPE_QUERY => Ok(RpcCallType::Query),
        _ => Err(GeneralError::InvalidTypeConvertError(input)),
    }
}

pub fn parse_json_to_value(input: Json<Value>) -> Result<Value, GeneralError> {
    Ok(serde_json::to_value(input)?)
}

/// Parses a JSON value into an ABI.
///
/// # Arguments
///
/// * `input` - A JSON value representing the ABI.
///
/// # Returns
///
/// A Result containing either an `Abi` instance or a `GeneralError`.
pub fn parse_to_abi(input: Value) -> Result<Abi, GeneralError> {
    Ok(from_str(&input.to_string())?)
}

pub fn parse_string_to_number(input: &str) -> Result<GeneralToken, GeneralError> {
    if input.contains('-') {
        Ok(GeneralToken::Int(parse_string_to_int(input)?))
    } else if input.contains('.') {
        Ok(GeneralToken::Float(parse_string_to_float(input)?))
    } else {
        Ok(GeneralToken::Uint(parse_string_to_uint(input)?))
    }
}

/// Parses a string into a uint.
///
/// # Arguments
///
/// * `input` - A string representing the uint.
///
/// # Returns
///
/// A `Uint` instance.
pub fn parse_string_to_uint(input: &str) -> Result<Uint, GeneralError> {
    Ok(U256::from_dec_str(input)?)
}

pub fn parse_f64_to_uint(input: f64) -> Result<Uint, GeneralError> {
    Ok(U256::from_dec_str(&input.to_string())?)
}

pub fn parse_token_to_i64(token: Token) -> Result<i64, GeneralError> {
    match token {
        Token::Uint(value) => Ok(value.as_u64() as i64),
        Token::Int(value) => Ok(value.as_u64() as i64),
        _ => Err(GeneralError::InvalidTypeConvertError(format!(
            "{:?}",
            token
        ))),
    }
}

pub fn parse_string_to_address(input: &str) -> Result<Address, GeneralError> {
    Ok(input.parse::<Address>()?)
}

pub fn parse_string_to_url(input: &str) -> Result<url::Url, GeneralError> {
    Ok(url::Url::parse(input)?)
}

pub fn parse_u256_to_bigint(input: &U256) -> Result<BigInt, GeneralError> {
    Ok(option_or_err!(BigInt::parse_bytes(
        input.to_string().as_bytes(),
        DECIMAL_RADIX
    )))
}

pub fn parse_hex_or_decimal_to_uint_token(input: &str) -> Option<Token> {
    // Try parsing as hex first, then as decimal
    if let Ok(num) = U256::from_str_radix(input, HEX_RADIX) {
        Some(Token::Uint(num))
    } else if let Ok(num) = U256::from_dec_str(input) {
        Some(Token::Uint(num))
    } else {
        None
    }
}

pub fn parse_address_to_token(input: &str) -> Option<Token> {
    if let Ok(addr) = H160::from_str(input) {
        Some(Token::Address(addr))
    } else {
        None
    }
}

/// Parses a string into an int.
///
/// # Arguments
///
/// * `input` - A string representing the int.
///
/// # Returns
///
/// An `Int` instance.
pub fn parse_string_to_int(input: &str) -> Result<BigInt, GeneralError> {
    Ok(input.parse::<BigInt>()?)
}

pub fn parse_string_to_float(input: &str) -> Result<f64, GeneralError> {
    Ok(input.parse::<f64>()?)
}

/// Parses a string into a bool.
///
/// # Arguments
///
/// * `input` - A string representing the bool.
///
/// # Returns
///
/// A `bool` instance.
pub fn parse_string_to_bool(input: String) -> Result<bool, GeneralError> {
    Ok(input.parse::<bool>()?)
}

/// Parses a string into an i32.
///
/// # Arguments
///
/// * `input` - A string representing the i32.
///
/// # Returns
///
/// An `i32` instance.
pub fn parse_string_to_i32(input: String) -> Result<i32, GeneralError> {
    Ok(input.parse::<i32>()?)
}

/// Parses a string into an Ethereum address.
///
/// # Arguments
///
/// * `input` - A string representing the address.
///
/// # Returns
///
/// A Result containing either an `Address` or a `GeneralError`.
pub fn parse_to_address(input: &str) -> Result<Address, GeneralError> {
    Ok(input.parse::<Address>()?)
}

/// # Description
/// This function compares a token with an expected value based on a comparator.
/// # Arguments
///
/// * `token` - The token to compare.
/// * `expected_value` - The expected value as a string.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// An optional string containing the value if the comparison is true.
pub fn compare_token(
    left: &GeneralToken,
    right: &GeneralToken,
    comparator: &str,
) -> Result<GeneralToken, GeneralError> {
    if !check_type_comparator(left, right, comparator) {
        let error_msg = format!("{:?} {} {:?}", left, comparator, right);
        return Err(GeneralError::InvalidOperator(error_msg));
    }
    match (left, right) {
        (GeneralToken::Uint(value), GeneralToken::Uint(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Uint(value), GeneralToken::Int(expected_value)) => {
            let left_big = BigInt::from_str(&value.to_string())?;
            parse_compare(&left_big, expected_value, comparator)
        }
        (GeneralToken::Int(value), GeneralToken::Int(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Int(value), GeneralToken::Uint(expected_value)) => {
            let right_big = BigInt::from_str(&expected_value.to_string())?;
            parse_compare(value, &right_big, comparator)
        }
        (GeneralToken::Float(value), GeneralToken::Float(expected_value)) => {
            Ok(parse_compare(value, expected_value, comparator)?)
        }
        (GeneralToken::Float(value), GeneralToken::Int(expected_value)) => {
            let right_float = option_or_err!(expected_value.to_f64());
            Ok(parse_compare(value, &right_float, comparator)?)
        }
        (GeneralToken::Int(value), GeneralToken::Float(expected_value)) => {
            let left_float = option_or_err!(value.to_f64());
            parse_float_arithmetic(&left_float, expected_value, comparator)
        }
        (GeneralToken::Float(value), GeneralToken::Uint(expected_value)) => {
            let right_float = expected_value.to_string().parse::<f64>()?;
            parse_compare(value, &right_float, comparator)
        }
        (GeneralToken::Uint(value), GeneralToken::Float(expected_value)) => {
            let left_float = value.to_string().parse::<f64>()?;
            parse_compare(&left_float, expected_value, comparator)
        }
        (GeneralToken::Bool(value), GeneralToken::Bool(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::String(value), GeneralToken::String(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Address(value), GeneralToken::Address(expected_value)) => {
            parse_compare(value, expected_value, comparator)
        }
        (GeneralToken::Address(value), GeneralToken::String(expected_value)) => {
            let expected_address = parse_to_address(expected_value)?;
            parse_compare(value, &expected_address, comparator)
        }
        (GeneralToken::String(value), GeneralToken::Address(expected_value)) => {
            let value_address = parse_to_address(value)?;
            parse_compare(&value_address, expected_value, comparator)
        }
        (GeneralToken::Bytes(value), GeneralToken::Bytes(expected_value))
        | (GeneralToken::FixedBytes(value), GeneralToken::FixedBytes(expected_value)) => {
            let parsing_value = hex::encode(value);
            let parsing_expected_value = hex::encode(expected_value);
            parse_compare(&parsing_value, &parsing_expected_value, comparator)
        }
        _ => Err(GeneralError::InvalidOperator(comparator.to_string())),
    }
}

/// # Description
/// This function compares two values based on a comparator.
/// # Arguments
///
/// * `value` - The value to compare.
/// * `expected_value` - The expected value.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// An optional string containing the value if the comparison is true.
pub fn parse_compare<T: PartialOrd>(
    value: &T,
    expected_value: &T,
    comparator: &str,
) -> Result<GeneralToken, GeneralError> {
    match comparator {
        COMPARATOR_EQUAL => Ok(GeneralToken::Bool(value == expected_value)),
        COMPARATOR_GREATER => Ok(GeneralToken::Bool(value > expected_value)),
        COMPARATOR_GREATER_EQUAL => Ok(GeneralToken::Bool(value >= expected_value)),
        COMPARATOR_LESS => Ok(GeneralToken::Bool(value < expected_value)),
        COMPARATOR_LESS_EQUAL => Ok(GeneralToken::Bool(value <= expected_value)),
        COMPARATOR_NOT_EQUAL => Ok(GeneralToken::Bool(value != expected_value)),
        _ => Err(GeneralError::InvalidOperator(comparator.to_string())),
    }
}

/// # Description
/// This function checks if the type comparator is valid.
/// # Arguments
///
/// * `value` - The token to check.
/// * `comparator` - The comparator string.
///
/// # Returns
///
/// A boolean indicating if the type comparator is valid.
pub fn check_type_comparator(left: &GeneralToken, right: &GeneralToken, comparator: &str) -> bool {
    match (left, right) {
        (GeneralToken::Uint(_), GeneralToken::Uint(_)) => {
            UINT_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Int(_), GeneralToken::Int(_))
        | (GeneralToken::Uint(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Uint(_)) => {
            INT_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Float(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Uint(_))
        | (GeneralToken::Uint(_), GeneralToken::Float(_)) => {
            FLOAT_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Address(_), GeneralToken::Address(_))
        | (GeneralToken::Address(_), GeneralToken::String(_))
        | (GeneralToken::String(_), GeneralToken::Address(_)) => {
            ADDRESS_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Bool(_), GeneralToken::Bool(_)) => {
            BOOL_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::Bytes(_), GeneralToken::Bytes(_)) => {
            BYTES_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::FixedBytes(_), GeneralToken::FixedBytes(_)) => {
            FIXED_BYTES_COMPARATOR_TYPE.contains(&comparator)
        }
        (GeneralToken::String(_), GeneralToken::String(_)) => {
            STRING_COMPARATOR_TYPE.contains(&comparator)
        }
        _ => false,
    }
}

/// # Description
/// This function performs arithmetic operations on tokens.
/// # Arguments
///
/// * `left` - The left token.
/// * `right` - The right token.
/// * `operator` - The operator string.
///
pub fn arithmetic_token(
    left: &GeneralToken,
    right: &GeneralToken,
    operator: &str,
) -> Result<GeneralToken, GeneralError> {
    if !check_type_arithmetic(left, right, operator) {
        return Err(GeneralError::InvalidOperator(operator.to_string()));
    }
    match (left, right) {
        (GeneralToken::Uint(value), GeneralToken::Uint(expected_value)) => {
            if value < expected_value {
                let left_big = BigInt::from_str(&value.to_string())?;
                let right_big = BigInt::from_str(&expected_value.to_string())?;
                parse_bigint_arithmetic(&left_big, &right_big, operator)
            } else {
                parse_u256_arithmetic(value, expected_value, operator)
            }
        }
        (GeneralToken::Uint(value), GeneralToken::Int(expected_value)) => {
            let left_big = BigInt::from_str(&value.to_string())?;
            parse_bigint_arithmetic(&left_big, expected_value, operator)
        }
        (GeneralToken::Int(value), GeneralToken::Int(expected_value)) => {
            parse_bigint_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::Int(value), GeneralToken::Uint(expected_value)) => {
            let right_big = BigInt::from_str(&expected_value.to_string())?;
            parse_bigint_arithmetic(value, &right_big, operator)
        }
        (GeneralToken::Float(value), GeneralToken::Float(expected_value)) => {
            parse_float_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::Float(value), GeneralToken::Int(expected_value)) => {
            let right_float = option_or_err!(expected_value.to_f64());
            parse_float_arithmetic(value, &right_float, operator)
        }
        (GeneralToken::Float(value), GeneralToken::Uint(expected_value)) => {
            let right_float = expected_value.to_string().parse::<f64>()?;
            parse_float_arithmetic(value, &right_float, operator)
        }
        (GeneralToken::Int(value), GeneralToken::Float(expected_value)) => {
            let left_float = option_or_err!(value.to_f64());
            parse_float_arithmetic(&left_float, expected_value, operator)
        }
        (GeneralToken::Uint(value), GeneralToken::Float(expected_value)) => {
            let left_float = value.to_string().parse::<f64>()?;
            parse_float_arithmetic(&left_float, expected_value, operator)
        }
        (GeneralToken::String(value), GeneralToken::String(expected_value)) => {
            parse_string_arithmetic(value, expected_value, operator)
        }
        (GeneralToken::String(value), other) | (other, GeneralToken::String(value)) => {
            let other_str = parse_token_to_string(other)?;
            parse_string_arithmetic(value, &other_str, operator)
        }
        _ => Err(GeneralError::InvalidOperator(format!(
            "{:?} {} {:?}",
            left, operator, right
        ))),
    }
}

pub fn format_float_to_4_decimal(value: f64) -> f64 {
    (value * FLOAT_PRECISION_MULTIPLIER).trunc() / FLOAT_PRECISION_MULTIPLIER
}

fn parse_float_arithmetic(
    value: &f64,
    expected_value: &f64,
    operator: &str,
) -> Result<GeneralToken, GeneralError> {
    match operator {
        OPERATOR_ADD => Ok(GeneralToken::Float(value + expected_value)),
        OPERATOR_SUB => Ok(GeneralToken::Float(value - expected_value)),
        OPERATOR_MUL => Ok(GeneralToken::Float(value * expected_value)),
        OPERATOR_DIV => Ok(GeneralToken::Float(value / expected_value)),
        OPERATOR_POW => Ok(GeneralToken::Float(value.powf(*expected_value))),
        _ => Err(GeneralError::InvalidOperator(operator.to_string())),
    }
}

/// # Description
/// This function checks if the type arithmetic is valid.
/// # Arguments
///
/// * `left` - The left token.
/// * `right` - The right token.
/// * `operator` - The operator string.
///
pub fn check_type_arithmetic(left: &GeneralToken, right: &GeneralToken, operator: &str) -> bool {
    match (left, right) {
        (GeneralToken::Uint(_), GeneralToken::Uint(_)) => UINT_ARITHMETIC_TYPE.contains(&operator),
        (GeneralToken::Int(_), GeneralToken::Int(_))
        | (GeneralToken::Uint(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Uint(_)) => INT_ARITHMETIC_TYPE.contains(&operator),
        (GeneralToken::Float(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Int(_))
        | (GeneralToken::Int(_), GeneralToken::Float(_))
        | (GeneralToken::Float(_), GeneralToken::Uint(_))
        | (GeneralToken::Uint(_), GeneralToken::Float(_)) => {
            FLOAT_ARITHMETIC_TYPE.contains(&operator)
        }
        (GeneralToken::String(_), _) | (_, GeneralToken::String(_)) => {
            STRING_ARITHMETIC_TYPE.contains(&operator)
        }

        _ => false,
    }
}

/// # Description
/// This function parses arithmetic values.
/// # Arguments
///
/// * `value` - The value to parse.
/// * `expected_value` - The expected value.
/// * `operator` - The operator string.
///
/// # Returns
///
/// An optional token containing the parsed value.
pub fn parse_bigint_arithmetic(
    value: &BigInt,
    expected_value: &BigInt,
    operator: &str,
) -> Result<GeneralToken, GeneralError> {
    match operator {
        OPERATOR_ADD => Ok(GeneralToken::Int(value + expected_value)),
        OPERATOR_SUB => Ok(GeneralToken::Int(value - expected_value)),
        OPERATOR_MUL => Ok(GeneralToken::Int(value * expected_value)),
        OPERATOR_DIV => {
            let result = option_or_err!(value.to_f64()) / option_or_err!(expected_value.to_f64());
            Ok(GeneralToken::Float(result))
        }
        OPERATOR_POW => Ok(GeneralToken::Int(
            value.pow(option_or_err!(expected_value.to_u32())),
        )),
        _ => Err(GeneralError::InvalidOperator(operator.to_string())),
    }
}

pub fn parse_u256_arithmetic(
    value: &U256,
    expected_value: &U256,
    operator: &str,
) -> Result<GeneralToken, GeneralError> {
    match operator {
        OPERATOR_ADD => Ok(GeneralToken::Uint(option_or_err!(
            value.checked_add(*expected_value)
        ))),
        OPERATOR_SUB => Ok(GeneralToken::Uint(option_or_err!(
            value.checked_sub(*expected_value)
        ))),
        OPERATOR_MUL => Ok(GeneralToken::Uint(option_or_err!(
            value.checked_mul(*expected_value)
        ))),
        OPERATOR_DIV => Ok(GeneralToken::Uint(option_or_err!(
            value.checked_div(*expected_value)
        ))),
        OPERATOR_POW => Ok(GeneralToken::Uint(value.pow(*expected_value))),
        _ => Err(GeneralError::InvalidOperator(operator.to_string())),
    }
}

/// # Description
/// This function parses string arithmetic operations.
/// # Arguments
///
/// * `value` - The string value to parse.
/// * `expected_value` - The expected value to concatenate with.
/// * `operator` - The operator string.
///
/// # Returns
///
/// An optional token containing the parsed value.
pub fn parse_string_arithmetic(
    value: &str,
    expected_value: &str,
    operator: &str,
) -> Result<GeneralToken, GeneralError> {
    match operator {
        OPERATOR_ADD => Ok(GeneralToken::String(value.to_string() + expected_value)),
        _ => Err(GeneralError::InvalidOperator(operator.to_string())),
    }
}

/// Validates if a string is a properly formatted hex string
pub fn is_valid_hex_string(hex: &str) -> bool {
    if !hex.starts_with("0x") {
        return false;
    }
    let hex_content = &hex[HEX_PREFIX_LENGTH..];
    !hex_content.is_empty() && hex_content.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validates if a string is a properly formatted Ethereum address
pub fn is_valid_ethereum_address(addr: &str) -> bool {
    if !addr.starts_with("0x") {
        return false;
    }
    let addr_content = &addr[HEX_PREFIX_LENGTH..];
    addr_content.len() == ETH_ADDRESS_LENGTH && addr_content.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn convert_hex_token(str_param: &str) -> Result<Token, GeneralError> {
    if str_param.starts_with("0x") {
        let trimmed_hex = str_param.strip_prefix("0x").unwrap_or(str_param);

        // Validate hex string format
        if !trimmed_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(GeneralError::InvalidTypeConvertError(format!(
                "Invalid hex string format: '{}' contains non-hex characters",
                str_param
            )));
        }

        match trimmed_hex.len() {
            ETH_ADDRESS_LENGTH => {
                // Validate Ethereum address format
                if is_valid_ethereum_address(str_param) {
                    Ok(Token::Address(Address::from_str(str_param)?))
                } else {
                    Err(GeneralError::InvalidTypeConvertError(format!(
                        "Invalid Ethereum address format: '{}'",
                        str_param
                    )))
                }
            }
            _ => {
                // For other hex values, ensure they're valid
                if trimmed_hex.is_empty() {
                    return Err(GeneralError::InvalidTypeConvertError(
                        "Empty hex string after '0x' prefix".to_string(),
                    ));
                }
                Ok(Token::Uint(hex_to_eth_amount(str_param)?))
            }
        }
    } else {
        Ok(Token::String(str_param.to_string()))
    }
}

pub fn convert_hex_param(str_param: &str) -> Result<ParamType, GeneralError> {
    if str_param.starts_with("0x") {
        // Validate hex format before determining type
        if !is_valid_hex_string(str_param) {
            return Err(GeneralError::InvalidTypeConvertError(format!(
                "Invalid hex string format: '{}'",
                str_param
            )));
        }

        if is_valid_ethereum_address(str_param) {
            Ok(ParamType::Address)
        } else {
            Ok(ParamType::String)
        }
    } else {
        Ok(ParamType::String)
    }
}

pub fn hex_to_eth_amount(hex: &str) -> Result<U256, GeneralError> {
    // Validate hex string format before parsing
    if !hex.starts_with("0x") {
        return Err(GeneralError::InvalidTypeConvertError(format!(
            "Hex string must start with '0x': '{}'",
            hex
        )));
    }

    let hex_content = &hex[HEX_PREFIX_LENGTH..];
    if hex_content.is_empty() {
        return Err(GeneralError::InvalidTypeConvertError(
            "Hex string is empty after '0x' prefix".to_string(),
        ));
    }

    // Validate that all characters are valid hex digits
    if !hex_content.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(GeneralError::InvalidTypeConvertError(format!(
            "Invalid hex characters in '{}'",
            hex
        )));
    }

    // Parse the hex string
    U256::from_str_radix(hex_content, HEX_RADIX).map_err(|e| {
        GeneralError::InvalidTypeConvertError(format!(
            "Failed to parse hex string '{}': {}",
            hex, e
        ))
    })
}

// Read and parse service files
///
/// # Description
/// This function reads and parses service files from the service directory.
/// It recursively traverses all subdirectories to find YAML files.
/// The category is constructed from the directory structure (e.g., "apy_monitor/info").
///
/// # Returns
/// A Result<Vec<RuleData>, DatabaseError> containing the parsed rule data.
pub async fn read_service_files(project_root: &PathBuf) -> Result<Vec<RuleData>, DatabaseError> {
    // Construct the path to the service directory
    let service_dir = project_root.join(SERVICE_DIR);

    // Check if the service directory exists
    if !service_dir.exists() {
        return Err(DatabaseError::GenericInitError(format!(
            "Service directory not found at: {:?}",
            service_dir
        )));
    }

    let mut rules = Vec::new();
    let mut category_parts = Vec::new();
    read_directory_recursive(&service_dir, &mut rules, &mut category_parts)?;

    Ok(rules)
}

/// Recursively read directory and parse YAML files
///
/// # Arguments
/// * `dir` - The directory to read
/// * `rules` - Vector to store parsed rules
/// * `category_parts` - Vector of category parts for building the full category path
fn read_directory_recursive(
    dir: &Path,
    rules: &mut Vec<RuleData>,
    category_parts: &mut Vec<String>,
) -> Result<(), DatabaseError> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                category_parts.push(
                    option_or_err!(path.file_name())
                        .to_string_lossy()
                        .to_string(),
                );
                read_directory_recursive(&path, rules, category_parts)?;
                category_parts.pop();
            } else if path.is_file() && path.extension().map_or(false, |ext| ext == YAML_EXTENSION)
            {
                let category = category_parts.join("/");

                if let Ok(contents) = fs::read_to_string(&path) {
                    match serde_yaml::from_str::<YamlRule>(&contents) {
                        Ok(yaml_rule) => {
                            let rule_data = RuleData {
                                category: category.clone(),
                                name: yaml_rule.name,
                                time_interval: yaml_rule.time_interval,
                                script: yaml_rule.script,
                            };
                            rules.push(rule_data);
                        }
                        Err(e) => {
                            return Err(DatabaseError::GenericInitError(format!(
                                "Failed to parse YAML file {:?}: {}",
                                path, e
                            )));
                        }
                    }
                } else {
                    return Err(DatabaseError::GenericInitError(format!(
                        "Failed to read file: {:?}",
                        path
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_parse_string_to_method() {
        let method_type = "GET";

        let result = parse_string_to_method(method_type.to_string());

        println!("{}", result);
    }

    #[tokio::test]
    pub async fn test_read_service_files() {
        let lib_dir = std::env::current_dir().unwrap();
        let project_root = lib_dir.parent().unwrap();
        println!("Lib directory: {:?}", lib_dir);
        println!("Project root: {:?}", project_root);

        let service_dir = project_root.join(SERVICE_DIR);
        println!("Service directory: {:?}", service_dir);
        println!("Service directory exists: {}", service_dir.exists());

        if let Ok(entries) = fs::read_dir(&service_dir) {
            let mut file_count = 0;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == YAML_EXTENSION) {
                    file_count += 1;
                    if file_count <= 5 {
                        // Only print first 5 files to avoid spam
                        println!("Found YAML file: {:?}", path);
                    }
                }
            }
            println!("Total YAML files found: {}", file_count);
        }

        match read_service_files(&project_root.to_path_buf()).await {
            Ok(rules) => {
                println!(
                    "Successfully loaded {} rules from service files",
                    rules.len()
                );
                for (i, rule) in rules.iter().enumerate() {
                    println!(
                        "Rule {}: {} (category: {})",
                        i + 1,
                        rule.name,
                        rule.category
                    );
                }
            }
            Err(e) => {
                println!("Error reading service files: {:?}", e);
            }
        }
    }
}
