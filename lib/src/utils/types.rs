use ethers::abi::{Address, Bytes, FixedBytes, Int, ParamType, Token as EthToken, Uint};
use ethers::utils::hex;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::rule::parse_int_to_uint;
use crate::utils::error::GeneralError;
use crate::utils::parse_u256_to_bigint;

/// The type of EVM chain ID's.
pub type ChainID = u32;
pub type RuleID = usize;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeneralToken {
    None,
    Address(Address),
    FixedBytes(FixedBytes),
    Bytes(Bytes),
    Int(BigInt),
    Uint(Uint),
    Float(f64),
    Bool(bool),
    String(String),
    FixedArray(Vec<GeneralToken>),
    Array(Vec<GeneralToken>),
    Tuple(Vec<GeneralToken>),
}

impl GeneralToken {
    pub fn from_eth_token(token: EthToken) -> Result<Self, GeneralError> {
        match token {
            EthToken::Address(addr) => Ok(GeneralToken::Address(addr)),
            EthToken::FixedBytes(bytes) => Ok(GeneralToken::FixedBytes(bytes)),
            EthToken::Bytes(bytes) => Ok(GeneralToken::Bytes(bytes)),
            EthToken::Int(i) => Ok(GeneralToken::Int(parse_u256_to_bigint(&i)?)),
            EthToken::Uint(u) => Ok(GeneralToken::Uint(u)),
            EthToken::Bool(b) => Ok(GeneralToken::Bool(b)),
            EthToken::String(s) => Ok(GeneralToken::String(s)),
            EthToken::FixedArray(tokens) => {
                let general_tokens: Result<Vec<GeneralToken>, GeneralError> = tokens
                    .into_iter()
                    .map(GeneralToken::from_eth_token)
                    .collect();
                Ok(GeneralToken::FixedArray(general_tokens?))
            }
            EthToken::Array(tokens) => {
                let general_tokens: Result<Vec<GeneralToken>, GeneralError> = tokens
                    .into_iter()
                    .map(GeneralToken::from_eth_token)
                    .collect();
                Ok(GeneralToken::Array(general_tokens?))
            }
            EthToken::Tuple(tokens) => {
                let general_tokens: Result<Vec<GeneralToken>, GeneralError> = tokens
                    .into_iter()
                    .map(GeneralToken::from_eth_token)
                    .collect();
                Ok(GeneralToken::Tuple(general_tokens?))
            }
        }
    }

    pub fn to_eth_token(&self) -> Result<EthToken, GeneralError> {
        match self {
            GeneralToken::Address(addr) => Ok(EthToken::Address(*addr)),
            GeneralToken::FixedBytes(bytes) => Ok(EthToken::FixedBytes(bytes.to_vec())),
            GeneralToken::Bytes(bytes) => Ok(EthToken::Bytes(bytes.to_vec())),
            GeneralToken::Int(i) => {
                if let Some(int) = i.to_i128() {
                    Ok(EthToken::Int(Int::from(int)))
                } else {
                    Err(GeneralError::InvalidTypeConvertError(format!("{:?}", self)))
                }
            }
            GeneralToken::Uint(u) => Ok(EthToken::Uint(*u)),
            GeneralToken::Float(_) => Err(GeneralError::InvalidTypeConvertError(
                "Float cannot be converted to EthToken".to_string(),
            )),
            GeneralToken::Bool(b) => Ok(EthToken::Bool(*b)),
            GeneralToken::String(s) => Ok(EthToken::String(s.to_string())),
            GeneralToken::FixedArray(tokens) => {
                let eth_tokens: Result<Vec<EthToken>, GeneralError> =
                    tokens.iter().map(|t| t.to_eth_token()).collect();
                Ok(EthToken::FixedArray(eth_tokens?))
            }
            GeneralToken::Array(tokens) => {
                let eth_tokens: Result<Vec<EthToken>, GeneralError> =
                    tokens.iter().map(|t| t.to_eth_token()).collect();
                Ok(EthToken::Array(eth_tokens?))
            }
            GeneralToken::Tuple(tokens) => {
                let eth_tokens: Result<Vec<EthToken>, GeneralError> =
                    tokens.iter().map(|t| t.to_eth_token()).collect();
                Ok(EthToken::Tuple(eth_tokens?))
            }
            GeneralToken::None => {
                return Err(GeneralError::InvalidTypeConvertError(
                    "None cannot be converted to EthToken".to_string(),
                ))
            }
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            GeneralToken::Int(_) | GeneralToken::Uint(_) | GeneralToken::Float(_)
        )
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            GeneralToken::Int(i) => i.to_f64(),
            GeneralToken::Uint(u) => u.to_string().parse::<f64>().ok(),
            GeneralToken::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn into_bool(&self) -> Option<bool> {
        match self {
            GeneralToken::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn into_string(self) -> Result<String, GeneralError> {
        match self {
            GeneralToken::Uint(value) => Ok(value.to_string()),
            GeneralToken::Int(value) => Ok(value.to_string()),
            GeneralToken::Address(value) => Ok(format!("{:#x}", value)),
            GeneralToken::Bool(value) => Ok(value.to_string()),
            GeneralToken::Bytes(value) => Ok(hex::encode(value)),
            GeneralToken::FixedBytes(value) => Ok(hex::encode(value)),
            GeneralToken::String(value) => Ok(value),
            GeneralToken::Float(value) => Ok(value.to_string()),
            _ => Err(GeneralError::InvalidTypeConvertError(format!("{:?}", self))),
        }
    }

    pub fn type_check(&self, param_type: &ParamType) -> bool {
        match (self, param_type) {
            (GeneralToken::Address(_), ParamType::Address) => true,
            (GeneralToken::FixedBytes(bytes), ParamType::FixedBytes(size)) => bytes.len() <= *size,
            (GeneralToken::Bytes(_), ParamType::Bytes) => true,
            (GeneralToken::Int(_), ParamType::Int(_)) => true,
            (GeneralToken::Uint(_), ParamType::Uint(_)) => true,
            (GeneralToken::Float(_), _) => false, // Float is not supported in ParamType
            (GeneralToken::Bool(_), ParamType::Bool) => true,
            (GeneralToken::String(_), ParamType::String) => true,
            (GeneralToken::Array(tokens), ParamType::Array(param_type)) => {
                tokens.iter().all(|t| t.type_check(param_type))
            }
            (GeneralToken::FixedArray(tokens), ParamType::FixedArray(param_type, size)) => {
                tokens.len() == *size && tokens.iter().all(|t| t.type_check(param_type))
            }
            (GeneralToken::Tuple(tokens), ParamType::Tuple(param_types)) => {
                tokens.len() == param_types.len()
                    && tokens
                        .iter()
                        .zip(param_types)
                        .all(|(token, param_type)| token.type_check(param_type))
            }
            _ => false,
        }
    }

    pub fn types_check(tokens: &[GeneralToken], param_types: &[ParamType]) -> bool {
        tokens.len() == param_types.len()
            && tokens
                .iter()
                .zip(param_types)
                .all(|(token, param_type)| token.type_check(param_type))
    }

    pub fn into_uint(&self) -> Result<Uint, GeneralError> {
        match self {
            GeneralToken::Uint(uint) => Ok(*uint),
            GeneralToken::Int(int) => parse_int_to_uint(&int),
            _ => Err(GeneralError::InvalidTypeConvertError(
                "Failed to convert to uint".to_string(),
            )),
        }
    }
}

// Implement From traits for common conversions
impl TryFrom<EthToken> for GeneralToken {
    type Error = GeneralError;

    fn try_from(token: EthToken) -> Result<Self, Self::Error> {
        GeneralToken::from_eth_token(token)
    }
}

impl From<f64> for GeneralToken {
    fn from(value: f64) -> Self {
        GeneralToken::Float(value)
    }
}

impl From<BigInt> for GeneralToken {
    fn from(value: BigInt) -> Self {
        GeneralToken::Int(value)
    }
}

// Add convenience From implementations for common integer types
impl From<i32> for GeneralToken {
    fn from(value: i32) -> Self {
        GeneralToken::Int(BigInt::from(value))
    }
}

impl From<i64> for GeneralToken {
    fn from(value: i64) -> Self {
        GeneralToken::Int(BigInt::from(value))
    }
}

impl From<u32> for GeneralToken {
    fn from(value: u32) -> Self {
        GeneralToken::Uint(Uint::from(value))
    }
}

impl From<u64> for GeneralToken {
    fn from(value: u64) -> Self {
        GeneralToken::Uint(Uint::from(value))
    }
}
