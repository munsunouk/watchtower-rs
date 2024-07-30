use std::sync::Arc;

use ethers::{
    abi::{Abi, ParamType, Token},
    prelude::*,
    providers::{Http, Provider},
};

use crate::utils::{create_contract, encode_token, parse_decode_token};

pub async fn call_contract<'a>(
    client: Arc<Provider<Http>>,
    contract_address: &Address,
    abi: &Abi,
    method_params: Vec<String>,
    rule_filter: &'a [&str],
    expected_value_index: &str,
) -> anyhow::Result<Option<String>> {
    let contract = create_contract(&contract_address, &abi, client);

    let abi = contract.abi();

    let function = abi.functions().next().unwrap();

    let function_name = function.name.clone();

    let function_input = function.inputs.clone();

    let input_param_type = if !function_input.is_empty() {
        let input_param = function_input.get(0).unwrap();

        input_param.kind.clone()
    } else {
        ParamType::Tuple(vec![])
    };

    let function_output = function.outputs.clone();

    let output_param_types: Vec<ParamType> = function_output
        .iter()
        .map(|param| param.kind.clone())
        .collect();

    let output_param_type = ParamType::Tuple(output_param_types);

    let method_params = encode_token(method_params, &input_param_type);

    let call = contract.method::<_, Token>(&function_name, method_params)?;
    let token = call.call().await?;

    let result = parse_decode_token(
        &token,
        &output_param_type,
        &rule_filter,
        &expected_value_index,
    )?;

    // let result = if let Token::Tuple(tokens) = token {
    //     parse_decode_token(
    //         &tokens,
    //         &output_param_type,
    //         &rule_filter,
    //         &expected_value_index,
    //     )?
    // } else {
    //     return Err(anyhow::anyhow!("Failed to extract value"));
    // };

    Ok(result)
}
