use ethers::{
    abi::{Abi, Int, ParamType, Token, Uint},
    prelude::*,
    providers::Provider,
    utils::hex,
};
use std::{fs::File, io::BufReader, sync::Arc};

#[allow(dead_code)]
pub struct ContractCallRule {
    pub address: Address,
    pub abi: Abi,
    pub method_params: Vec<String>,
    pub rule_filter: Vec<&'static str>,
    pub expected_value_index: &'static str,
    expected_value: &'static str,
    expected_value_type: &'static str,
    comparison_operator: &'static str,
}

impl ContractCallRule {
    pub fn new(
        raw_address: &'static str,
        abi_path: &'static str,
        raw_method_params: Vec<&'static str>,
        rule_filter: Vec<&'static str>,
        expected_value_index: &'static str,
        expected_value: &'static str,
        expected_value_type: &'static str,
        comparison_operator: &'static str,
    ) -> anyhow::Result<Self> {
        let file = File::open(abi_path)?;
        let reader = BufReader::new(file);
        let abi: Abi = serde_json::from_reader(reader)?;

        let address = raw_address.parse::<Address>()?;

        let method_params = raw_method_params.iter().map(|s| s.to_string()).collect();

        Ok(Self {
            address,
            abi,
            method_params,
            rule_filter,
            expected_value_index,
            expected_value,
            expected_value_type,
            comparison_operator,
        })
    }
}

#[allow(dead_code)]
pub struct ContractEventRule {
    pub address: Address,
    pub abi: Abi,
    pub event_index: usize,
    pub rule_filter: Vec<&'static str>,
    pub expected_value_index: &'static str,
    expected_value: &'static str,
    expected_value_type: &'static str,
    comparison_operator: &'static str,
}

impl ContractEventRule {
    pub fn new(
        raw_address: &'static str,
        abi_path: &'static str,
        event_index: usize,
        rule_filter: Vec<&'static str>,
        expected_value_index: &'static str,
        expected_value: &'static str,
        expected_value_type: &'static str,
        comparison_operator: &'static str,
    ) -> anyhow::Result<Self> {
        let file = File::open(abi_path)?;
        let reader = BufReader::new(file);
        let abi: Abi = serde_json::from_reader(reader)?;

        let address = raw_address.parse::<Address>()?;

        Ok(Self {
            address,
            abi,
            event_index,
            rule_filter,
            expected_value_index,
            expected_value,
            expected_value_type,
            comparison_operator,
        })
    }
}

pub fn create_contract<T: JsonRpcClient>(
    address: &Address,
    abi: &Abi,
    provider: Arc<Provider<T>>,
) -> Contract<Provider<T>> {
    Contract::new(address.clone(), abi.clone(), provider)
}

pub fn parse_rule_filter<'a>(rule_filters: &'a [&str]) -> Vec<(Vec<usize>, &'a str)> {
    rule_filters
        .iter()
        .map(|&rule_filter| {
            let parts: Vec<&str> = rule_filter.split('-').collect();
            let indices: Vec<usize> = parts[0].split('.').map(|s| s.parse().unwrap()).collect();
            let value = parts[1];
            (indices, value)
        })
        .collect()
}

pub fn parse_expected_value_index(expected_value_index: &str) -> Vec<usize> {
    expected_value_index
        .split('.')
        .map(|s| s.parse().unwrap())
        .collect()
}

pub fn encode_token(params: Vec<String>, param_type: &ParamType) -> Token {
    if params.is_empty() {
        return Token::Tuple(vec![]); // Default case for empty params
    }

    match param_type {
        ParamType::String => Token::String(params[0].clone()),
        ParamType::Address => Token::Address(params[0].parse::<Address>().unwrap()),
        ParamType::Bool => Token::Bool(params[0].parse::<bool>().unwrap()),
        ParamType::Uint(_) => Token::Uint(params[0].parse::<Uint>().unwrap()),
        ParamType::Int(_) => Token::Int(params[0].parse::<Int>().unwrap()),
        ParamType::Bytes => Token::Bytes(hex::decode(&params[0]).unwrap()),
        ParamType::FixedBytes(size) => {
            let bytes = hex::decode(&params[0]).unwrap();
            assert_eq!(bytes.len(), *size);
            Token::FixedBytes(bytes)
        }
        ParamType::Array(inner_type) => {
            let tokens: Vec<Token> = params
                .into_iter()
                .map(|p| encode_token(vec![p], inner_type))
                .collect();
            Token::Array(tokens)
        }
        ParamType::FixedArray(inner_type, size) => {
            let tokens: Vec<Token> = params
                .into_iter()
                .take(*size)
                .map(|p| encode_token(vec![p], inner_type))
                .collect();
            Token::FixedArray(tokens)
        }
        ParamType::Tuple(inner_types) => {
            let tokens: Vec<Token> = inner_types
                .iter()
                .zip(params.into_iter())
                .map(|(t, p)| encode_token(vec![p], t))
                .collect();
            Token::Tuple(tokens)
        }
    }
}

pub fn decode_token(
    token: &Token,
    param_type: &ParamType,
    expected_value_path: &[usize],
) -> Option<String> {
    if expected_value_path.is_empty() {
        return match (param_type, token) {
            (ParamType::Uint(_), Token::Uint(value)) => Some(value.to_string()),
            (ParamType::Address, Token::Address(value)) => Some(format!("{:?}", value)),
            (ParamType::Bool, Token::Bool(value)) => Some(value.to_string()),
            (ParamType::String, Token::String(value)) => Some(value.clone()),
            (ParamType::Bytes, Token::Bytes(value)) => Some(hex::encode(value)),
            (ParamType::Int(_), Token::Int(value)) => Some(value.to_string()),
            (ParamType::FixedBytes(_), Token::FixedBytes(value)) => Some(hex::encode(value)),
            _ => None,
        };
    }

    let (index, rest) = expected_value_path.split_first().unwrap();

    match param_type {
        ParamType::Tuple(inner_types) => {
            if let Token::Tuple(tokens) = token {
                if let Some(inner_type) = inner_types.get(*index) {
                    if let Some(inner_token) = tokens.get(*index) {
                        return decode_token(inner_token, inner_type, rest);
                    }
                }
            }
        }
        ParamType::Array(inner_type) => {
            if let Token::Array(array_tokens) = token {
                if let Some(inner_token) = array_tokens.get(*index) {
                    return decode_token(inner_token, inner_type, rest);
                }
            }
        }
        ParamType::FixedArray(inner_type, _) => {
            if let Token::FixedArray(array_tokens) = token {
                if let Some(inner_token) = array_tokens.get(*index) {
                    return decode_token(inner_token, inner_type, rest);
                }
            }
        }
        _ => {}
    }
    None
}

pub fn parse_decode_token<'a>(
    token: &Token,
    param_type: &ParamType,
    rule_filter: &'a [&str],
    expected_value_index: &'a str,
) -> anyhow::Result<Option<String>> {
    let parsed_rule_filter = parse_rule_filter(&rule_filter);
    let parsed_expected_value_index_key = parse_expected_value_index(&expected_value_index);

    for (parsed_rule_key, parsed_rule_value) in parsed_rule_filter {
        if let Some(value) = decode_token(&token, &param_type, &parsed_rule_key) {
            if value != parsed_rule_value {
                return Ok(None);
            }
        } else {
            return Err(anyhow::anyhow!("Failed to extract value"));
        }
    }

    if let Some(value) = decode_token(&token, &param_type, &parsed_expected_value_index_key) {
        return Ok(Some(value));
    } else {
        return Err(anyhow::anyhow!("Failed to extract value"));
    }
}
