use ethers::abi::{Address, Bytes, FixedBytes, Int, ParamType, Token as EthToken, Uint};
use ethers::utils::hex;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::utils::error::GeneralError;

/// The type of EVM chain ID's.
pub type ChainID = u32;
pub type RuleID = usize;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeneralToken {
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
    pub fn from_eth_token(token: EthToken) -> Self {
        match token {
            EthToken::Address(addr) => GeneralToken::Address(addr),
            EthToken::FixedBytes(bytes) => GeneralToken::FixedBytes(bytes),
            EthToken::Bytes(bytes) => GeneralToken::Bytes(bytes),
            EthToken::Int(i) => {
                GeneralToken::Int(BigInt::parse_bytes(i.to_string().as_bytes(), 10).unwrap())
            }
            EthToken::Uint(u) => GeneralToken::Uint(u),
            EthToken::Bool(b) => GeneralToken::Bool(b),
            EthToken::String(s) => GeneralToken::String(s),
            EthToken::FixedArray(tokens) => GeneralToken::FixedArray(
                tokens
                    .into_iter()
                    .map(GeneralToken::from_eth_token)
                    .collect(),
            ),
            EthToken::Array(tokens) => GeneralToken::Array(
                tokens
                    .into_iter()
                    .map(GeneralToken::from_eth_token)
                    .collect(),
            ),
            EthToken::Tuple(tokens) => GeneralToken::Tuple(
                tokens
                    .into_iter()
                    .map(GeneralToken::from_eth_token)
                    .collect(),
            ),
        }
    }

    pub fn to_eth_token(&self) -> Option<EthToken> {
        match self {
            GeneralToken::Address(addr) => Some(EthToken::Address(*addr)),
            GeneralToken::FixedBytes(bytes) => Some(EthToken::FixedBytes(bytes.clone())),
            GeneralToken::Bytes(bytes) => Some(EthToken::Bytes(bytes.clone())),
            GeneralToken::Int(i) => {
                if let Some(int) = i.to_i128() {
                    Some(EthToken::Int(Int::from(int)))
                } else {
                    None
                }
            }
            GeneralToken::Uint(u) => Some(EthToken::Uint(*u)),
            GeneralToken::Float(_) => None, // Can't convert float to EthToken
            GeneralToken::Bool(b) => Some(EthToken::Bool(*b)),
            GeneralToken::String(s) => Some(EthToken::String(s.clone())),
            GeneralToken::FixedArray(tokens) => {
                let eth_tokens: Option<Vec<EthToken>> =
                    tokens.iter().map(|t| t.to_eth_token()).collect();
                eth_tokens.map(EthToken::FixedArray)
            }
            GeneralToken::Array(tokens) => {
                let eth_tokens: Option<Vec<EthToken>> =
                    tokens.iter().map(|t| t.to_eth_token()).collect();
                eth_tokens.map(EthToken::Array)
            }
            GeneralToken::Tuple(tokens) => {
                let eth_tokens: Option<Vec<EthToken>> =
                    tokens.iter().map(|t| t.to_eth_token()).collect();
                eth_tokens.map(EthToken::Tuple)
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

    pub fn into_bool(self) -> Option<bool> {
        match self {
            GeneralToken::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn into_string(self) -> Result<String, GeneralError> {
        match self {
            GeneralToken::Uint(value) => Ok(value.to_string()),
            GeneralToken::Int(value) => Ok(value.to_string()),
            GeneralToken::Address(value) => Ok(value.to_string()),
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

    pub fn into_uint(self) -> Option<Uint> {
        match self {
            GeneralToken::Uint(uint) => Some(uint),
            GeneralToken::Int(int) => {
                // Convert BigInt to Uint if possible
                int.to_string().parse::<Uint>().ok()
            }
            _ => None,
        }
    }
}

// Implement From traits for common conversions
impl From<EthToken> for GeneralToken {
    fn from(token: EthToken) -> Self {
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
