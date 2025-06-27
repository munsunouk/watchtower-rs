use ethers::abi::Token;
use serde::{Deserialize, Serialize};

use sqlx::postgres::PgRow;
use sqlx::Row;

use crate::utils::constants::{
    DB_CATEGORY_COLUMN, DB_NAME_COLUMN, DB_SCRIPT_COLUMN, DB_TIME_INTERVAL_COLUMN,
};
use crate::utils::error::GeneralError;

/// RuleData
///
/// * Feature: RuleData
/// * Description: This struct represents the data for a unified rule.
/// * Fields:
///   * category: String - The category of the rule
///   * name: String - The name of the rule
///   * time_interval: i32 - The time interval in seconds
///   * script: String - The script content
#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct RuleData {
    pub category: String,
    pub name: String,
    pub time_interval: i32,
    pub script: String,
}

#[derive(Deserialize)]
pub struct YamlRule {
    pub name: String,
    pub time_interval: i32,
    pub script: String,
}

impl TryFrom<&PgRow> for RuleData {
    type Error = GeneralError;

    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        Ok(Self {
            category: row.try_get::<String, _>(DB_CATEGORY_COLUMN)?,
            name: row.try_get::<String, _>(DB_NAME_COLUMN)?,
            time_interval: row.try_get::<i32, _>(DB_TIME_INTERVAL_COLUMN)?,
            script: row.try_get::<String, _>(DB_SCRIPT_COLUMN)?,
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
        Ok(i32::try_from(int.as_u128())?)
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
                let parsed: Vec<String> = serde_json::from_str(string)?;
                Ok(parsed)
            }
        }
        _ => Err(GeneralError::InvalidRuleDecode(
            "Invalid string vec token".to_string(),
        )),
    }
}
