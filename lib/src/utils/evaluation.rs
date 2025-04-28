use ethers::abi::{ParamType, Token};

use ethers::types::U256;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

use crate::cli::db::postgres::{
    select_assign_data_sync, select_fetched_raw_data_with_filter, PostgresClient, Query, SelectData,
};
use crate::utils::{
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

use crate::utils::error::IndexType;

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::{collections::HashMap, str::FromStr};

use super::{
    arithmetic_token, compare_token,
    constants::{
        ADDRESS_SPLIT_INDEX, BLOCK_NUMBER_SPLIT_INDEX, CHAIN_SPLIT_INDEX, EVENT_INDEX_SPLIT_INDEX,
        INTERVAL_SPLIT_INDEX, URL_SPLIT_INDEX,
    },
    parse_string_to_i32,
};

use futures::executor::block_on;

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
#[grammar = "utils/evaluation.pest"]
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
// pub fn parse_values(
//     pair: Pair<Rule>,
//     rule_type: &str,
//     rule_name: &str,
// ) -> Result<(HashMap<(String, String), String>, Vec<Token>), GeneralError> {
//     let mut rule_values = HashMap::new();
//     let mut values = Vec::new();

//     match pair.as_rule() {
//         Rule::values => {
//             let inner = pair.into_inner();
//             for (value_id, value_pair) in inner.enumerate() {
//                 let mut value_inner = value_pair.into_inner();
//                 if let (Some(value_pair_key), Some(version)) =
//                     (value_inner.next(), value_inner.next())
//                 {
//                     let parse_value = format!("{}_{}_{}", rule_type, rule_name, value_id);

//                     rule_values.insert(
//                         (rule_name.to_string(), value_pair_key.as_str().to_string()),
//                         parse_value,
//                     );

//                     values.push(Token::String(version.as_str().to_string()));
//                 }
//             }
//             Ok((rule_values, values))
//         }

//         _ => Err(GeneralError::InvalidRuleDecode(format!(
//             "Unexpected rule: {:?}",
//             pair.as_rule()
//         ))),
//     }
// }

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

/// ParseRules is the function to parse the rules.
/// # Arguments
/// * `input` - The input.
/// # Returns
/// * `Result<(Vec<Option<HashMap<String, Token>>>, Vec<Option<HashMap<String, Token>>>), GeneralError>` - The result.
pub fn parse_rules(
    input: &str,
    last_ids: &mut (usize, usize, usize),
) -> Result<
    (
        Vec<Option<HashMap<String, Token>>>,
        Vec<Option<HashMap<String, Token>>>,
        Vec<Option<HashMap<String, Token>>>,
    ),
    GeneralError,
> {
    // let pairs = RuleEvaluationParser::parse(Rule::rules, input)
    let pairs = RuleEvaluationParser::parse(Rule::evaluation, input)
        .map_err(|e| GeneralError::InvalidRuleDecode(e.to_string()))?;

    let mut rules: Vec<Option<HashMap<String, Token>>> = Vec::new();
    let mut eval_rules: Vec<Option<HashMap<String, Token>>> = Vec::new();
    let mut assign_rules: Vec<Option<HashMap<String, Token>>> = Vec::new();
    let mut assign_rule = HashMap::new();
    for pair in pairs {
        let mut rule = HashMap::new();
        let mut current_rule_values = HashMap::new();

        let inner = pair.into_inner();

        for unwrapped_pair in inner {
            parse_rule(
                unwrapped_pair,
                &mut rule,
                &mut current_rule_values,
                &mut rules,
                &mut eval_rules,
                &mut assign_rule,
                &mut assign_rules,
                last_ids,
            )?;
        }
    }

    println!("rules: {:?}", rules);

    Ok((rules, eval_rules, assign_rules))
}

/// ParseRule is the function to parse the rule.
/// # Arguments
/// * `pair` - The pair.
/// * `result` - The result.
/// * `current_rule_values` - The current rule values.
/// * `rules` - The rules.
/// # Returns
/// * `Result<(), GeneralError>` - The result.
pub fn parse_rule(
    pair: Pair<Rule>,
    result: &mut HashMap<String, Token>,
    current_rule_values: &mut HashMap<(String, String), String>,
    rules: &mut Vec<Option<HashMap<String, Token>>>,
    eval_rules: &mut Vec<Option<HashMap<String, Token>>>,
    assign_rule: &mut HashMap<String, Token>,
    assign_rules: &mut Vec<Option<HashMap<String, Token>>>,
    last_ids: &mut (usize, usize, usize),
) -> Result<(), GeneralError> {
    println!("pair: {:?}", pair.as_rule());
    match pair.as_rule() {
        Rule::rule => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_rule(
                    unwrapped_pair,
                    result,
                    current_rule_values,
                    rules,
                    eval_rules,
                    assign_rule,
                    assign_rules,
                    last_ids,
                )?;
            }

            rules.push(Some(result.clone()));
        }

        Rule::get_command => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_rule(
                    unwrapped_pair,
                    result,
                    current_rule_values,
                    rules,
                    eval_rules,
                    assign_rule,
                    assign_rules,
                    last_ids,
                )?;
            }

            rules.push(Some(result.clone()));
        }

        Rule::return_command => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_rule(
                    unwrapped_pair,
                    result,
                    current_rule_values,
                    rules,
                    eval_rules,
                    assign_rule,
                    assign_rules,
                    last_ids,
                )?;
            }
        }

        Rule::get_return_command => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_rule(
                    unwrapped_pair,
                    result,
                    current_rule_values,
                    rules,
                    eval_rules,
                    assign_rule,
                    assign_rules,
                    last_ids,
                )?;
            }

            eval_rules.push(Some(result.clone()));
        }

        Rule::assign_command => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_rule(
                    unwrapped_pair,
                    result,
                    current_rule_values,
                    rules,
                    eval_rules,
                    assign_rule,
                    assign_rules,
                    last_ids,
                )?;
            }

            assign_rules.push(Some(assign_rule.clone()));
            assign_rule.clear();
        }

        Rule::eval_literal => {
            let mut result = HashMap::new();
            let component = pair.as_str().to_string();
            result.insert("eval_literal".to_string(), Token::String(component));
            eval_rules.push(Some(result.clone()));
        }

        // New format handlers
        Rule::key_value_pair => {
            let mut inners = pair.into_inner();
            let key = inners
                .next()
                .ok_or(GeneralError::InvalidRuleDecode("Missing key".to_string()))?
                .as_str()
                .to_string();
            let value = inners
                .next()
                .ok_or(GeneralError::InvalidRuleDecode("Missing value".to_string()))?;

            if assign_rule.get("name").is_some() && key == "type" {
                assign_rule.insert(
                    "type".to_string(),
                    Token::String(value.as_str().to_string()),
                );

                last_ids.1 += 1;

                assign_rule.insert("rule_id".to_string(), Token::Int(last_ids.1.into()));
            }

            println!("key: {:?}", key);
            println!("value: {:?}", value.as_str().to_string());

            if key == "target_block_number" {
                result.insert(key.clone(), Token::String(value.as_str().to_string()));
            } else {
                println!("value: {:?}", value.as_rule());
                match value.as_rule() {
                    Rule::number => {
                        result.insert(
                            key,
                            Token::Int(parse_string_to_i32(value.as_str().to_string())?.into()),
                        );
                    }

                    Rule::ethereum_address => {
                        result.insert(key, Token::String(value.as_str().to_string()));
                    }
                    Rule::abi_list => {
                        let abi_text = value.as_str().to_string();

                        result.insert(key, Token::String(abi_text.clone()));

                        if let Token::String(ref token_type) =
                            result.get("type").ok_or(GeneralError::InvalidRuleName)?
                        {
                            if (token_type == "contractcall"
                                && !check_function_length(&abi_text).unwrap_or(false))
                                || (token_type == "contractevent"
                                    && !check_event_length(&abi_text).unwrap_or(false))
                            {
                                return Ok(());
                            }
                        }
                    }
                    Rule::json_values => {
                        result.insert(key, Token::String(value.as_str().to_string()));
                    }
                    // Rule::rule_literal => {
                    //     result.insert(key, Token::String(value.as_str().to_string()));
                    // }
                    Rule::identifier => {
                        result.insert(key, Token::String(value.as_str().to_string()));
                    }
                    _ => {
                        // return Err(GeneralError::InvalidRuleDecode(format!(
                        //     "Unsupported value type: {:?}",
                        //     value.as_rule()
                        // )));
                        return Ok(());
                    }
                }
            }
        }

        Rule::identifier => {
            let component = pair.as_str().to_string();
            assign_rule.insert("name".to_string(), Token::String(component));
        }
        // _ => {
        //     return Err(GeneralError::InvalidRuleDecode(format!(
        //         "Unsupported value type: {:?}",
        //         pair.as_rule()
        //     )));
        // }

        // _ => {
        // return Err(GeneralError::InvalidRuleDecode(format!(
        //     "Unsupported value type: {:?}",
        //     pair.as_rule()
        // )));
        // }
        _ => {
            let rule_str = format!("{:?}", pair.as_rule());
            println!("rule_str: {:?}", rule_str);
            // result.insert(rule_str, Token::Bool(false));
            return Ok(());
        }
    }

    Ok(())
}

#[derive(Clone, PartialEq, Debug)]
pub enum TargetData {
    Value,
    Block,
}

/// # Description
/// This function evaluates an expression.
/// # Arguments
/// * `pair` - The pair.
/// * `values` - The values.
/// # Returns
/// A `Token` struct.
pub fn parse_operation<'a>(
    pair: Pair<'a, Rule>,
    db_client: &PostgresClient,
    target_data: TargetData,
) -> Result<Token, GeneralError> {
    match pair.as_rule() {
        Rule::operation => {
            let mut inner = pair.into_inner();
            let first = inner
                .next()
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
            let mut result = parse_operation(first, db_client, target_data.clone())?;

            while let Some(op) = inner.next() {
                let next = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let right = parse_operation(next, db_client, target_data.clone())?;

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

        Rule::assign_identifier => {
            let pair_str = pair.as_str();

            let identifier = pair_str
                .strip_prefix("eval(")
                .and_then(|s| s.strip_suffix(")"))
                .ok_or(GeneralError::InvalidRuleDecode(
                    "Invalid eval format".to_string(),
                ))?;

            // let data = block_on(db_client.select_assign_data(identifier)).unwrap();
            let data = select_assign_data_sync(identifier).unwrap();
            // let value = data.get::<usize, i32>(4);

            Ok(Token::Int(data.into()))
        }
        Rule::identifier => {
            let pair_split = pair
                .as_str()
                .split(RULE_FILTER_SPLIT_CHAR)
                .collect::<Vec<&str>>();

            let rule_type = pair_split
                .get(DEFAULT_INDEX)
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;

            let rule_id = pair_split
                .get(RULE_ID_SPLIT_INDEX)
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(
                    RULE_ID_SPLIT_INDEX,
                )))?
                .parse::<usize>()
                .map_err(|_| GeneralError::InvalidTypeConvert)?;

            // let value_id = pair_split
            //     .get(VALUE_ID_SPLIT_INDEX)
            //     .ok_or(GeneralError::InvalidIndex(IndexType::USize(
            //         VALUE_ID_SPLIT_INDEX,
            //     )))?
            //     .parse::<i32>()
            //     .map_err(|_| GeneralError::InvalidTypeConvert)?;

            // let value_id = 0;

            // let rule_type = DbRuleType::from_str(rule_type)
            //     .map_err(|_| GeneralError::InvalidOperator(rule_type.to_string()))?;

            // let value_id = parse_i32_to_usize(value_id)?;

            let value = select_fetched_raw_data_with_filter(rule_type, rule_id as i32).unwrap();

            return Ok(value);

            // let value = data.get::<usize, i32>(3);
            // println!("value: {:?}", value);

            // let handle = tokio::runtime::Handle::current();
            // let data = block_on(db_client.get_fetched_raw_data_with_filter(
            //     "values",
            //     Query {
            //         rule_type: Some(rule_type),
            //         rule_id: Some(rule_id as i32),
            //         value_id: Some(value_id),
            //         start_block_number: None,
            //         end_block_number: None,
            //         start_timestamp: None,
            //         end_timestamp: None,
            //     },
            // ))
            // .unwrap();

            // for d in data {
            //     if let SelectData::Values(values) = d {
            //         let value = values.get(value_id).and_then(|v| v.clone()).ok_or(
            //             GeneralError::InvalidOperator(format!(
            //                 "identifier Failed : {:?}, {}, {}",
            //                 rule_type,
            //                 rule_id.to_string(),
            //                 value_id
            //             )),
            //         )?;

            //         return Ok(value);
            //     }
            // }

            // Err(GeneralError::InvalidOperator(format!(
            //     "identifier Failed : {:?}, {}, {}",
            //     rule_type,
            //     rule_id.to_string(),
            //     value_id
            // )))
        }

        Rule::expression => {
            let mut inner = pair.into_inner();
            let left = parse_operation(
                inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?,
                db_client,
                target_data.clone(),
            )?;

            if let Some(op) = inner.next() {
                let right = parse_operation(
                    inner
                        .next()
                        .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?,
                    db_client,
                    target_data.clone(),
                )?;

                let err_msg = format!("{:?}, {:?}, {}", left, right, op.as_str().to_string());

                compare_token(&left, &right, op.as_str())
                    .ok_or(GeneralError::InvalidOperator(err_msg))
            } else {
                Ok(left)
            }
        }

        Rule::term | Rule::factor => {
            let mut inner = pair.into_inner();
            let first = inner
                .next()
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
            let mut result = parse_operation(first, db_client, target_data.clone())?;

            while let Some(op) = inner.next() {
                let next = inner
                    .next()
                    .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
                let right = parse_operation(next, db_client, target_data.clone())?;

                // let result_uint = result.into_uint().unwrap();
                // let right_uint = right.into_uint().unwrap();

                // result = Token::Uint(result_uint - right_uint);

                // println!("result: {:?}", result);

                result = arithmetic_token(&result, &right, op.as_str())
                    .unwrap_or_else(|| Token::Uint(U256::from(0)));
            }
            Ok(result)
        }

        Rule::primary => {
            let mut inner = pair.into_inner();
            let first = inner
                .next()
                .ok_or(GeneralError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?;
            parse_operation(first, db_client, target_data)
        }

        _ => Err(GeneralError::InvalidOperator(pair.as_str().to_string())),
    }
}

/// # Description
/// This function evaluates a rule filter.
/// # Arguments
/// * `rule_filter` - The rule filter.
/// * `values` - The values.
/// # Returns
/// A `Result` struct.
pub fn evaluate_tokens(
    rule_filter: &str,
    db_client: &PostgresClient,
) -> Result<Token, GeneralError> {
    let pairs = match RuleEvaluationParser::parse(Rule::operation, rule_filter) {
        Ok(pairs) => pairs,
        Err(_) => {
            return Err(GeneralError::InvalidRuleDecode(rule_filter.to_string()));
        }
    };
    let mut result = Token::Bool(false);
    let target_data = TargetData::Value;

    for pair in pairs {
        result = parse_operation(pair, db_client, target_data.clone())?;
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

    use crate::{
        cli::db::data::{
            ContractCallRuleData, ContractEventRuleData, EvaluationRuleData, RpcCallRuleData,
        },
        config::set_config,
        utils::parse_compare,
    };
    use ethers::{abi::Int, types::U256};

    use super::*;

    fn setup() -> String {
        let config = set_config("/Users/munseon-ug/rust/watchtower/config.yaml");
        config.postgres_config.url
    }

    #[tokio::test]
    async fn test_evaluate_rule_filter() {
        let rule_filter = "contractcall_1_0 ";

        let mut values: HashMap<(DbRuleType, RuleID), Vec<Option<Token>>> = HashMap::new();

        let db_client = PostgresClient::new(&setup()).await.unwrap();
        values.insert(
            (DbRuleType::ContractCall, 1),
            vec![
                Some(Token::Uint(
                    U256::from_dec_str("3905527246000000000000").unwrap(),
                )),
                Some(Token::Uint(U256::from(15))),
            ],
        );

        let result = evaluate_tokens(rule_filter, &db_client).unwrap();
    }

    #[test]
    fn test_set_rule_dependencies() {
        let evaluations = HashMap::from([
            (
                1,
                EvaluationRule {
                    id: 1,
                    rule_filter: "contractcall_3_0 && contractcall_4_0".to_string(),
                    expected_value: "contractcall_3_0".to_string(),
                },
            ),
            (
                2,
                EvaluationRule {
                    id: 2,
                    rule_filter: "contractcall_3_1 || contractcall_4_1".to_string(),
                    expected_value: "contractcall_3_1".to_string(),
                },
            ),
            (
                3,
                EvaluationRule {
                    id: 3,
                    rule_filter: "contractcall_3_0 || contractcall_3_1".to_string(),
                    expected_value: "contractcall_3_0 * contractcall_3_1".to_string(),
                },
            ),
            (
                4,
                EvaluationRule {
                    id: 4,
                    rule_filter: "contractcall_1_0 == 0 && contractcall_1_1 < 500000000000000"
                        .to_string(),
                    expected_value: "contractcall_3_0".to_string(),
                },
            ),
            (
                5,
                EvaluationRule {
                    id: 5,
                    rule_filter: "rpccall_aggregator-balance_0 >= 0".to_string(),
                    expected_value: "rpccall_aggregator-balance_0".to_string(),
                },
            ),
        ]);

        let rule_to_evaluations = set_rule_dependencies(&evaluations).unwrap();
        println!("rule_to_evaluations: {:?}", rule_to_evaluations);
    }

    #[test]
    fn test_parse_rule() {
        let test_input =
        "assign(usdc, get(type=contractcall, chain_id=3068, address=0x5BaBC26813898543EC4467b46411a605882b767B, abi=[{\"name\": \"latestRoundData\", \"type\": \"function\", \"inputs\": [], \"outputs\": [{\"name\": \"roundId\", \"type\": \"uint80\", \"internalType\": \"uint80\"}, {\"name\": \"answer\", \"type\": \"int256\", \"internalType\": \"int256\"}, {\"name\": \"startedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"updatedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"answeredInRound\", \"type\": \"uint80\", \"internalType\": \"uint80\"}], \"stateMutability\": \"view\"}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={1}, check_block_interval=3, target_block_number=0)) + get(type=contractcall, chain_id=3068, address=0x5BaBC26813898543EC4467b46411a605882b767B, abi=[{\"name\": \"latestRoundData\", \"type\": \"function\", \"inputs\": [], \"outputs\": [{\"name\": \"roundId\", \"type\": \"uint80\", \"internalType\": \"uint80\"}, {\"name\": \"answer\", \"type\": \"int256\", \"internalType\": \"int256\"}, {\"name\": \"startedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"updatedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"answeredInRound\", \"type\": \"uint80\", \"internalType\": \"uint80\"}], \"stateMutability\": \"view\"}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=usdc-1) + get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={1}, check_block_interval=3, target_block_number=usdc-2), get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={1}, check_block_interval=3, target_block_number=usdc-1))";
        // "assign(usdc, get(type=contractcall, chain_id=3068, address=0x5BaBC26813898543EC4467b46411a605882b767B, abi=[{\"name\": \"latestRoundData\", \"type\": \"function\", \"inputs\": [], \"outputs\": [{\"name\": \"roundId\", \"type\": \"uint80\", \"internalType\": \"uint80\"}, {\"name\": \"answer\", \"type\": \"int256\", \"internalType\": \"int256\"}, {\"name\": \"startedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"updatedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"answeredInRound\", \"type\": \"uint80\", \"internalType\": \"uint80\"}], \"stateMutability\": \"view\"}], method_params={}, values={1}, check_block_interval=3, target_block_number=0)) + get(type=contractcall, chain_id=3068, address=0x5BaBC26813898543EC4467b46411a605882b767B, abi=[{\"name\": \"latestRoundData\", \"type\": \"function\", \"inputs\": [], \"outputs\": [{\"name\": \"roundId\", \"type\": \"uint80\", \"internalType\": \"uint80\"}, {\"name\": \"answer\", \"type\": \"int256\", \"internalType\": \"int256\"}, {\"name\": \"startedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"updatedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"answeredInRound\", \"type\": \"uint80\", \"internalType\": \"uint80\"}], \"stateMutability\": \"view\"}], method_params={}, values={1}, check_block_interval=3, target_block_number=usdc-1) + get(type=contractcall, chain_id=3068, address=0x5BaBC26813898543EC4467b46411a605882b767B, abi=[{\"name\": \"latestRoundData\", \"type\": \"function\", \"inputs\": [], \"outputs\": [{\"name\": \"roundId\", \"type\": \"uint80\", \"internalType\": \"uint80\"}, {\"name\": \"answer\", \"type\": \"int256\", \"internalType\": \"int256\"}, {\"name\": \"startedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"updatedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"answeredInRound\", \"type\": \"uint80\", \"internalType\": \"uint80\"}], \"stateMutability\": \"view\"}], method_params={}, values={1}, check_block_interval=3, target_block_number=usdc - 2) / 3 > 0, get(type=contractcall, chain_id=3068, address=0x5BaBC26813898543EC4467b46411a605882b767B, abi=[{\"name\": \"latestRoundData\", \"type\": \"function\", \"inputs\": [], \"outputs\": [{\"name\": \"roundId\", \"type\": \"uint80\", \"internalType\": \"uint80\"}, {\"name\": \"answer\", \"type\": \"int256\", \"internalType\": \"int256\"}, {\"name\": \"startedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"updatedAt\", \"type\": \"uint256\", \"internalType\": \"uint256\"}, {\"name\": \"answeredInRound\", \"type\": \"uint80\", \"internalType\": \"uint80\"}], \"stateMutability\": \"view\"}], method_params={}, values={1}, check_block_interval=3, target_block_number=usdc - 2)";

        let pairs = RuleEvaluationParser::parse(Rule::evaluation, test_input).unwrap();

        println!("pairs: {:?}", pairs);
        // let mut last_ids = (0, 0, 0);
        // let (rules, eval_rules, assign_rules) = parse_rules(test_input, &mut last_ids).unwrap();

        // println!("rules: {:?}", rules);
        // println!("eval_rules: {:?}", eval_rules);
        // println!("assign_rules: {:?}", assign_rules);
    }

    #[test]
    fn test_parse_rule1() {
        let test_input =
        "get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=0)";
        // let test_input =
        // "assign(a, get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=0))";

        let pairs = RuleEvaluationParser::parse(Rule::assign_command, test_input).unwrap();

        println!("pairs: {:?}", pairs);
    }

    #[test]
    fn test_parse_rule_data() {
        let test_input = "filter (contractcall_Bifrost-Chainlink-Oracle-usdc_0 - contractcall_Bifrost-bifnet-Oracle-usdc_0) / contractcall_Bifrost-Chainlink-Oracle-usdc_0 * 100 > 5 || (contractcall_Bifrost-bifnet-Oracle-usdc_0 - contractcall_Bifrost-Chainlink-Oracle-usdc_0) / contractcall_Bifrost-Chainlink-Oracle-usdc_0 * 100 > 5 move contractcall_Bifrost-Chainlink-Oracle-usdc_0";

        let mut last_ids = (0, 0, 0);
        let rules = parse_rules(test_input, &mut last_ids).unwrap();

        // for rule in rules {
        //     if let Some(rule) = rule {
        //         if let Some(Token::String(ref token_type)) = rule.get("type") {
        //             if token_type == "contractevent" {
        //                 let rule_data = ContractEventRuleData::from_tokens(rule).unwrap();
        //                 println!("rule_data: {:?}", rule_data);
        //             } else if token_type == "contractcall" {
        //                 let rule_data = ContractCallRuleData::from_tokens(rule).unwrap();
        //                 println!("rule_data: {:?}", rule_data);
        //             } else if token_type == "rpccall" {
        //                 let rule_data = RpcCallRuleData::from_tokens(rule).unwrap();
        //                 println!("rule_data: {:?}", rule_data);
        //             } else if token_type == "evaluation" {
        //                 let rule_data = EvaluationRuleData::from_tokens(rule).unwrap();
        //                 println!("rule_data: {:?}", rule_data);
        //             }
        //         }
        //     }
        // }
    }

    #[test]
    fn test_compare_token() {
        let result = parse_compare(
            &Int::from(158655396170i64),
            &Int::from(158655396171i64),
            "<",
        );
        println!("result: {:?}", result);

        println!(
            "158655396170i64 < 158655396171i64: {}",
            Int::from(158655396170i64) < Int::from(158655396171i64)
        );
    }

    #[test]
    fn test_arithmetic_token() {
        let result = arithmetic_token(
            &Token::Uint(U256::from(15)),
            &Token::Uint(U256::from(10)),
            "-",
        );
        println!("result: {:?}", result);
    }

    #[test]
    fn test_parse_rules() {
        // let input = "assign(a, get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=0) > get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=0), get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=0)";
        let input = "get(type=contractcall, chain_id=49088, address=0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423, abi=[{\"type\":\"function\",\"name\":\"get_pool_info\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"_pool\",\"type\":\"address\"}],\"outputs\":[{\"internalType\":\"uint256[8]\",\"name\":\"balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_balances\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"underlying_decimals\",\"type\":\"uint256[8]\"},{\"internalType\":\"uint256[8]\",\"name\":\"rates\",\"type\":\"uint256[8]\"},{\"internalType\":\"address\",\"name\":\"lp_token\",\"type\":\"address\"},{\"internalType\":\"tuple\",\"name\":\"params\",\"type\":\"tuple\",\"components\":[{\"internalType\":\"uint256\",\"name\":\"A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_fee\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_admin_fee\",\"type\":\"uint256\"},{\"internalType\":\"address\",\"name\":\"future_owner\",\"type\":\"address\"},{\"internalType\":\"uint256\",\"name\":\"initial_A\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"initial_A_time\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"future_A_time\",\"type\":\"uint256\"}]},{\"internalType\":\"bool\",\"name\":\"is_meta\",\"type\":\"bool\"},{\"internalType\":\"string\",\"name\":\"name\",\"type\":\"string\"}]}], method_params={0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, values={0.0.2}, check_block_interval=3, target_block_number=0)";

        let mut last_ids = (0, 0, 0);
        let (rules, eval_rules, assign_rules) = parse_rules(input, &mut last_ids).unwrap();
        println!("rules: {:?}", rules);
        println!("eval_rules: {:?}", eval_rules);
        // assert_eq!(rules.len(), 1);
        // assert!(rules[0].is_some());

        // let rule = rules[0].as_ref().unwrap();
        // println!("rule : {:?}", rule);
        // assert_eq!(
        //     rule.get("type").unwrap(),
        //     &Token::String("contractcall".to_string())
        // );
        // assert_eq!(
        //     rule.get("name").unwrap(),
        //     &Token::String("test2".to_string())
        // );
        // assert_eq!(rule.get("chain_id").unwrap(), &Token::Int(3068.into()));
        // assert_eq!(
        //     rule.get("address").unwrap(),
        //     &Token::String("0x0000000000000000000000000000000000000100".to_string())
        // );
        // assert_eq!(
        //     rule.get("check_block_interval").unwrap(),
        //     &Token::Int(3.into())
        // );
        // assert_eq!(
        //     rule.get("target_block_number").unwrap(),
        //     &Token::Int(0.into())
        // );
    }

    #[test]
    fn test_parse_rules_new_format() {
        let input = r#"get(type=contractcall, name=test2, chain=3068, address=0x0000000000000000000000000000000000000100, abi=[{"type":"function","name":"current_round","stateMutability":"view","inputs":[],"outputs":[{"internalType":"uint32","name":"","type":"uint32"}]}], params={pool:0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, value={status:0}, check_block=3, target_block=0)"#;
        // let input = r#"get(type=contractcall, name=test2, chain=3068, address=0x0000000000000000000000000000000000000100, abi=[{'type':'function','name':'current_round','stateMutability':'view','inputs':[],'outputs':[{'internalType':'uint32','name':'','type':'uint32'}]}], params={pool:0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, value={status:0}, check_block=3, target_block=0)"#;
        let mut last_ids = (0, 0, 0);
        let rules = parse_rules(input, &mut last_ids).unwrap();

        println!("rules: {:?}", rules);

        // assert!(rules[0].is_some());

        // let rule = rules[0].as_ref().unwrap();

        // assert_eq!(
        //     rule.get("type").unwrap(),
        //     &Token::String("contractcall".to_string())
        // );
        // assert_eq!(
        //     rule.get("name").unwrap(),
        //     &Token::String("test2".to_string())
        // );
        // assert_eq!(rule.get("chain").unwrap(), &Token::Int(3068.into()));
        // assert_eq!(
        //     rule.get("address").unwrap(),
        //     &Token::String("0x0000000000000000000000000000000000000100".to_string())
        // );
        // assert_eq!(rule.get("check_block").unwrap(), &Token::Int(3.into()));
        // assert_eq!(rule.get("target_block").unwrap(), &Token::Int(0.into()));
    }
}
