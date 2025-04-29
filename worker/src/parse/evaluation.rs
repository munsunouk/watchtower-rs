use ethers::abi::{ParamType, Token};

use ethers::types::U256;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

use watch_tower_lib::cli::db::postgres::{
    select_assign_data_sync, select_fetched_raw_data_with_filter, PostgresClient, Query, SelectData,
};
use watch_tower_lib::utils::{
    constants::{
        BOOLEAN_LITERAL_FALSE, BOOLEAN_LITERAL_TRUE, DB_EXPECTED_VALUE_COLUMN, DB_ID_COLUMN,
        DB_RULE_FILTER_COLUMN, DEFAULT_INDEX, LOGIC_OPERATOR_AND, LOGIC_OPERATOR_OR,
        RULE_FILTER_SPLIT_CHAR, RULE_ID_SPLIT_INDEX, VALUE_ID_SPLIT_INDEX,
    },
    error::GeneralError,
    parse_i32_to_usize, parse_string_to_uint, parse_to_abi,
    types::RuleID,
    DbRuleType,
};

use watch_tower_lib::utils::error::IndexType;

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::{collections::HashMap, str::FromStr};

use watch_tower_lib::utils::{
    arithmetic_token, compare_token,
    constants::{
        ADDRESS_SPLIT_INDEX, BLOCK_NUMBER_SPLIT_INDEX, CHAIN_SPLIT_INDEX, EVENT_INDEX_SPLIT_INDEX,
        INTERVAL_SPLIT_INDEX, URL_SPLIT_INDEX,
    },
    parse_string_to_i32,
};

use futures::executor::block_on;

use crate::rule::store::{assign, SymbolTable, TokenConvert};

/// # Description
/// This struct represents an evaluation rule.
/// # Fields
/// * `id` - The ID of the rule.
/// * `rule_filter` - The rule filter.
/// * `expected_value` - The expected value.
#[derive(Clone, PartialEq, Debug)]
pub struct EvaluationRule {
    pub id: RuleID,
    pub rule_filter: String,
    pub expected_value: String,
}

impl TryFrom<&PgRow> for EvaluationRule {
    type Error = GeneralError;
    fn try_from(row: &PgRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_i32_to_usize(row.get(DB_ID_COLUMN))?,
            rule_filter: row.get(DB_RULE_FILTER_COLUMN),
            expected_value: row.get(DB_EXPECTED_VALUE_COLUMN),
        })
    }
}

#[derive(Parser)]
#[grammar = "parse/evaluation.pest"]
pub struct RuleEvaluationParser;

/// ParseValues is the function to parse the values.
/// # Description
/// This function parses the values of the rule.
/// # Arguments
/// * `pair` - The pair.
/// * `rule_type` - The rule type.
/// * `rule_name` - The rule name.
/// # Returns
/// * `(HashMap<(String, String), String>, Vec<Token>)` - The rule values and values.
pub fn parse_pair(
    symbol_table: &mut SymbolTable,
    result_vec: &mut HashMap<String, String>,
    pair: Pair<Rule>,
) -> Result<Token, GeneralError> {
    let mut rule_values = HashMap::new();
    let mut values = Vec::new();

    match pair.as_rule() {
        Rule::expression_stmt => {
            let inner = pair.into_inner();

            let result = for unwrapped_pair in inner {
                parse_pair(symbol_table, result_vec, unwrapped_pair)?;
            };

            Ok(result)
        }

        Rule::assignment_stmt => {
            let mut inner = pair.into_inner();

            let identifer = inner
                .next()
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

            let _equal = inner
                .next()
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

            let expression = while let Some(op) = inner.next() {
                let next = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

                parse_pair(symbol_table, result_vec, next)?;
            };

            assign(symbol_table, identifer.as_str().to_string(), expression);

            Ok(Token::Bool(true))
        }

        // operation level parsing
        Rule::expression => {
            let mut inner = pair.into_inner();
            let first = inner
                .next()
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
            let mut result = parse_pair(symbol_table, result_vec, first)?;

            while let Some(op) = inner.next() {
                let next = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let right = parse_pair(symbol_table, result_vec, next)?;

                result = match op.as_str() {
                    LOGIC_OPERATOR_AND => {
                        if result.type_check(&ParamType::Bool) && right.type_check(&ParamType::Bool)
                        {
                            Token::Bool(
                                result.into_bool().ok_or(GeneralError::InvalidTypeConvert)?
                                    && right.into_bool().ok_or(GeneralError::InvalidTypeConvert)?,
                            )
                        } else {
                            return Err(GeneralError::InvalidTypeConvert);
                        }
                    }
                    LOGIC_OPERATOR_OR => {
                        if result.type_check(&ParamType::Bool) && right.type_check(&ParamType::Bool)
                        {
                            Token::Bool(
                                result.into_bool().ok_or(GeneralError::InvalidTypeConvert)?
                                    || right.into_bool().ok_or(GeneralError::InvalidTypeConvert)?,
                            )
                        } else {
                            return Err(GeneralError::InvalidTypeConvert);
                        }
                    }
                    _ => return Err(GeneralError::InvalidOperator(op.as_str().to_string())),
                };
            }
            Ok(result)
        }

        Rule::number => Ok(Token::Uint(parse_string_to_uint(
            pair.as_str().to_string(),
        )?)),

        Rule::boolean_literal => match pair.as_str() {
            BOOLEAN_LITERAL_TRUE => Ok(Token::Bool(true)),
            BOOLEAN_LITERAL_FALSE => Ok(Token::Bool(false)),
            _ => Err(GeneralError::InvalidOperator(pair.as_str().to_string())),
        },

        _ => Err(GeneralError::InvalidRuleDecode(format!(
            "Unexpected rule: {:?}",
            pair.as_rule()
        ))),
    }
}

/// CheckFunctionLength is the function to check the function length.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<bool, GeneralError>` - The result.
pub fn check_function_length(abi_text: &str) -> Result<bool, GeneralError> {
    let abi_value = parse_abi_text(abi_text)?;

    let abi = parse_to_abi(abi_value)?;

    let function_count = abi.functions().count();

    if function_count != 1 {
        return Ok(false);
    }
    Ok(true)
}

/// CheckEventLength is the function to check the event length.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<bool, GeneralError>` - The result.
pub fn check_event_length(abi_text: &str) -> Result<bool, GeneralError> {
    let abi_value = parse_abi_text(abi_text)?;

    let abi = parse_to_abi(abi_value)?;

    let event_count = abi.events().count();

    if event_count != 1 {
        return Ok(false);
    }

    Ok(true)
}

/// ParseABIText is the function to parse the ABI text.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<Value, GeneralError>` - The result.
pub fn parse_abi_text(abi_text: &str) -> Result<Value, GeneralError> {
    serde_json::from_str(abi_text).map_err(|_| GeneralError::InvalidTypeABI)
}

/// # Description
/// This function evaluates a rule filter.
/// # Arguments
/// * `rule_filter` - The rule filter.
/// * `values` - The values.
/// # Returns
/// A `Result` struct.
pub fn parse_result(
    symbol_table: &mut SymbolTable,
    program_input: &str,
) -> Result<Token, GeneralError> {
    let pairs = match RuleEvaluationParser::parse(Rule::program, program_input) {
        Ok(pairs) => pairs,
        Err(_) => {
            return Err(GeneralError::InvalidRuleDecode(program_input.to_string()));
        }
    };
    let mut result = Token::Bool(false);
    let mut result_vec = Vec::new();

    for pair in pairs {
        result = parse_pair(symbol_table, result_vec, pair);
    }
    Ok(result)
}

/// # Description
/// This function sets the rule dependencies.
/// # Arguments
/// * `evaluations` - The evaluations.
/// # Returns
/// A `HashMap` struct.
pub fn set_rule_dependencies(
    evaluations: &HashMap<RuleID, EvaluationRule>,
) -> Result<HashMap<(DbRuleType, RuleID), Vec<RuleID>>, GeneralError> {
    let mut rule_to_evaluations = HashMap::new();

    for (eval_id, eval) in evaluations {
        let rule_pairs = RuleEvaluationParser::parse(Rule::operation, &eval.rule_filter)
            .map_err(|_| GeneralError::InvalidTypeConvert)?;

        for pair in rule_pairs {
            extract_rule_dependencies(pair, &mut rule_to_evaluations, *eval_id)?;
        }

        let value_pairs = RuleEvaluationParser::parse(Rule::operation, &eval.expected_value)
            .map_err(|_| GeneralError::InvalidTypeConvert)?;

        for pair in value_pairs {
            extract_rule_dependencies(pair, &mut rule_to_evaluations, *eval_id)?;
        }
    }

    Ok(rule_to_evaluations)
}

/// # Description
/// This function extracts the rule dependencies.
/// # Arguments
/// * `pair` - The pair.
/// * `rule_to_evaluations` - The rule to evaluations.
/// * `eval_id` - The evaluation ID.
pub fn extract_rule_dependencies(
    pair: Pair<Rule>,
    rule_to_evaluations: &mut HashMap<(DbRuleType, RuleID), Vec<RuleID>>,
    eval_id: RuleID,
) -> Result<(), GeneralError> {
    match pair.as_rule() {
        Rule::identifier => {
            let pair_split = pair
                .as_str()
                .split(RULE_FILTER_SPLIT_CHAR)
                .collect::<Vec<&str>>();

            let rule_type_str = match pair_split.get(DEFAULT_INDEX) {
                Some(val) => val,
                None => {
                    return Err(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)));
                }
            };

            let rule_type = match DbRuleType::from_str(rule_type_str) {
                Ok(val) => val,
                Err(_) => {
                    return Err(GeneralError::InvalidRuleDecode(rule_type_str.to_string()));
                }
            };

            let rule_id_str = match pair_split.get(RULE_ID_SPLIT_INDEX) {
                Some(val) => val,
                None => {
                    return Err(GeneralError::InvalidIndex(IndexType::USize(
                        RULE_ID_SPLIT_INDEX,
                    )));
                }
            };

            let rule_id = rule_id_str.parse::<RuleID>().map_err(|_| {
                GeneralError::InvalidRuleDecode(format!("Invalid rule ID: {}", rule_id_str))
            })?;

            let entry = rule_to_evaluations.entry((rule_type, rule_id)).or_default();

            if !entry.iter().any(|&id| id == eval_id) {
                entry.push(eval_id);
            }
            Ok(())
        }
        _ => {
            for inner_pair in pair.into_inner() {
                extract_rule_dependencies(inner_pair, rule_to_evaluations, eval_id)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {

    use ethers::{abi::Int, types::U256};
    use watch_tower_lib::{config::set_config, utils::parse_compare};

    use super::*;

    fn setup() -> String {
        let config = set_config("/Users/munseon-ug/rust/watchtower/config.yaml");
        config.postgres_config.url
    }

    #[test]

    fn test_new_parse_rule() {
        // 1) 유동성 조회 후 저장
        let test_input = "bifrostBN = Bifrost.LatestBlock(); ChainlinkBTC = Bifrost.ChainlinkOracle.BTC.LatestPrice(bifrostBN); BifnetBTC = Bifrost.BifnetOracle.BTC.LatestPrice(bifrostBN -1); BifaggBTC = Bifrost.Bifagg.BTC.LatestPrice(bifrostBN -2); result = (ChainlinkBTC + BifnetBTC + BifaggBTC) / 3";

        let pairs = RuleEvaluationParser::parse(Rule::program, test_input).unwrap();
        println!("pairs: {:?}", pairs);
    }

    #[tokio::test]
    async fn test_parse_result() {
        let test_input = "bifrostBN = Bifrost.LatestBlock();";

        let mut symbol_table = SymbolTable::new();

        let result = parse_result(&mut symbol_table, test_input).unwrap();

        println!("result: {:?}", result);
    }
}
