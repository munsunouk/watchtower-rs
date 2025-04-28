use std::collections::HashMap;

use ethers::abi::Token;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::error::GeneralError;

/// RpcCallRuleData
///
///  * Feature: RpcCallRuleData
///  * Description: This struct represents the data for an RPC call rule.
///  * Fields:
///    * id: Option<i32> - The ID of the rule.
///    * name: String - The name of the rule.
///    * url: String - The URL of the RPC provider.
///    * values: Vec<String> - The values to be used in the RPC call.
///    * call_time_interval: i32 - The time interval for the RPC call.
#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct RpcCallRuleData {
    pub url: String,
    pub call_type: String,
    pub method_type: String,
    pub api_body: Value,
    pub values: Vec<String>,
    pub call_time_interval: i32,
}

impl RpcCallRuleData {
    pub fn from_tokens(tokens: HashMap<String, Token>) -> Result<Self, GeneralError> {
        Ok(Self {
            url: decode_string_token(
                tokens
                    .get("url")
                    .ok_or(GeneralError::InvalidRuleDecode("url".to_string()))?,
            )?,
            call_type: decode_string_token(
                tokens
                    .get("call_type")
                    .ok_or(GeneralError::InvalidRuleDecode("call_type".to_string()))?,
            )?,
            method_type: decode_string_token(
                tokens
                    .get("method_type")
                    .ok_or(GeneralError::InvalidRuleDecode("method_type".to_string()))?,
            )?,
            api_body: decode_string_value_token(
                tokens
                    .get("api_body")
                    .ok_or(GeneralError::InvalidRuleDecode("api_body".to_string()))?,
            ),
            values: decode_string_vec_token(
                tokens
                    .get("values")
                    .ok_or(GeneralError::InvalidRuleDecode("values".to_string()))?,
            )?,
            call_time_interval: decode_int_token(tokens.get("call_time_interval").ok_or(
                GeneralError::InvalidRuleDecode("call_time_interval".to_string()),
            )?)?,
        })
    }
}

/// ContractCallRuleData
///
///  * Feature: ContractCallRuleData
///  * Description: This struct represents the data for a contract call rule.
///  * Fields:
///    * id: Option<i32> - The ID of the rule.
///    * name: String - The name of the rule.
///    * chain_id: i32 - The ID of the chain.
///    * address: String - The address of the contract.
///    * abi: Value - The ABI of the contract.
///    * method_params: Vec<String> - The method parameters of the contract.
///    * values: Vec<String> - The values to be used in the contract call.
///    * check_block_interval: i32 - The time interval for the contract call.
///    * target_block_number: i32 - The target block number from latest block.
#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct ContractCallRuleData {
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub method_params: Vec<String>,
    pub values: Vec<String>,
    pub check_block_interval: i32,
    pub target_block_number: String,
}

impl ContractCallRuleData {
    pub fn from_tokens(tokens: HashMap<String, Token>) -> Result<Self, GeneralError> {
        Ok(Self {
            chain_id: decode_int_token(
                tokens
                    .get("chain_id")
                    .ok_or(GeneralError::InvalidRuleDecode("chain_id".to_string()))?,
            )?,
            address: decode_string_token(
                tokens
                    .get("address")
                    .ok_or(GeneralError::InvalidRuleDecode("address".to_string()))?,
            )?,
            abi: decode_string_value_token(
                tokens
                    .get("abi")
                    .ok_or(GeneralError::InvalidRuleDecode("abi".to_string()))?,
            ),
            method_params: decode_string_vec_token(
                tokens.get("method_params").unwrap_or(&Token::Array(vec![])),
            )?,
            values: decode_string_vec_token(
                tokens
                    .get("values")
                    .ok_or(GeneralError::InvalidRuleDecode("values".to_string()))?,
            )?,
            check_block_interval: decode_int_token(tokens.get("check_block_interval").ok_or(
                GeneralError::InvalidRuleDecode("check_block_interval".to_string()),
            )?)?,
            target_block_number: decode_string_token(tokens.get("target_block_number").ok_or(
                GeneralError::InvalidRuleDecode("target_block_number".to_string()),
            )?)?,
        })
    }
}

/// ContractEventRuleData
///
///  * Feature: ContractEventRuleData
///  * Description: This struct represents the data for a contract event rule.
///  * Fields:
///    * id: Option<i32> - The ID of the rule.
///    * name: String - The name of the rule.
///    * chain_id: i32 - The ID of the chain.
#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct ContractEventRuleData {
    pub chain_id: i32,
    pub address: String,
    pub abi: Value,
    pub event_index: i32,
    pub values: Vec<String>,
}

impl ContractEventRuleData {
    pub fn from_tokens(tokens: HashMap<String, Token>) -> Result<Self, GeneralError> {
        Ok(Self {
            chain_id: decode_int_token(
                tokens
                    .get("chain_id")
                    .ok_or(GeneralError::InvalidRuleDecode("chain_id".to_string()))?,
            )?,
            address: decode_string_token(
                tokens
                    .get("address")
                    .ok_or(GeneralError::InvalidRuleDecode("address".to_string()))?,
            )?,
            abi: decode_string_value_token(
                tokens
                    .get("abi")
                    .ok_or(GeneralError::InvalidRuleDecode("abi".to_string()))?,
            ),
            event_index: decode_int_token(
                tokens
                    .get("event_index")
                    .ok_or(GeneralError::InvalidRuleDecode("event_index".to_string()))?,
            )?,
            values: decode_string_vec_token(
                tokens
                    .get("values")
                    .ok_or(GeneralError::InvalidRuleDecode("values".to_string()))?,
            )?,
        })
    }
}

/// EvaluationRuleData
///
///  * Feature: EvaluationRuleData
///  * Description: This struct represents the data for an evaluation rule.
///  * Fields:
///    * id: Option<i32> - The ID of the rule.
///    * name: String - The name of the rule.
///    * rule_filter: String - The filter for the rule.
///    * expected_value: String - The expected value for the rule.
#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct EvaluationRuleData {
    pub rule_filter: String,
    pub expected_value: String,
}

impl EvaluationRuleData {
    pub fn from_tokens(tokens: HashMap<String, Token>) -> Result<Self, GeneralError> {
        Ok(Self {
            rule_filter: decode_string_token(
                tokens
                    .get("rule_filter")
                    .ok_or(GeneralError::InvalidRuleDecode("rule_filter".to_string()))?,
            )?,
            expected_value: decode_string_token(tokens.get("expected_value").ok_or(
                GeneralError::InvalidRuleDecode("expected_value".to_string()),
            )?)?,
        })
    }
}

pub fn decode_string_token(token: &Token) -> Result<String, GeneralError> {
    if let Token::String(string) = token {
        Ok(string.to_string())
    } else {
        Err(GeneralError::InvalidRuleDecode(
            "Invalid string token".to_string(),
        ))
    }
}

pub fn decode_int_token(token: &Token) -> Result<i32, GeneralError> {
    if let Token::Int(int) = token {
        i32::try_from(int.as_u128())
            .map_err(|_| GeneralError::InvalidRuleDecode("Invalid int token".to_string()))
    } else {
        Err(GeneralError::InvalidRuleDecode(
            "Invalid int token".to_string(),
        ))
    }
}

pub fn decode_string_vec_token(token: &Token) -> Result<Vec<String>, GeneralError> {
    match token {
        Token::Array(array) => array
            .iter()
            .map(decode_string_token)
            .collect::<Result<Vec<String>, GeneralError>>(),
        Token::String(string) => {
            // Try to parse the string as a JSON array or single value
            if string.starts_with('{') && string.ends_with('}') {
                // Handle single value in curly braces
                let value = string.trim_start_matches('{').trim_end_matches('}');
                Ok(vec![value.to_string()])
            } else {
                // Try to parse as JSON array
                let parsed: Vec<String> = serde_json::from_str(string).map_err(|_| {
                    GeneralError::InvalidRuleDecode("Invalid JSON array string".to_string())
                })?;
                Ok(parsed)
            }
        }
        _ => Err(GeneralError::InvalidRuleDecode(
            "Invalid string vec token".to_string(),
        )),
    }
}

pub fn decode_string_value_token(token: &Token) -> Value {
    if let Token::String(string) = token {
        serde_json::from_str(string).unwrap()
    } else {
        Value::Null
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_string_value_token() {
        let token = Token::String(
            "{\n            \"active\": \"bc1p2cmsnvtvxxvvyxm055vc45827zdyvawsyps6ctqta7lapuh2hepqsp5qas|bc1q6ylrskh4p6u983kx8f0mp0ztwer850u0xzeszj\"\n        }"
                .to_string(),
        );
        let value = decode_string_value_token(&token);
        println!("value: {:?}", value);
    }
}
