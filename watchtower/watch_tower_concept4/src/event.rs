use std::sync::Arc;

use ethers::{
    abi::{Abi, ParamType, Token},
    prelude::*,
    providers::{Http, Provider},
};

use crate::utils::{create_contract, parse_decode_token};

pub async fn event_contract<'a>(
    client: Arc<Provider<Http>>,
    contract_address: &Address,
    abi: &Abi,
    block_number: u64,
    rule_filter: &'a [&str],
    expected_value_index: &str,
) -> anyhow::Result<Vec<Option<String>>> {
    let contract: ContractInstance<Arc<Provider<Http>>, Provider<Http>> =
        create_contract(&contract_address, &abi, client.clone());

    let abi = contract.abi();

    let filter = Filter::new()
        .address(contract_address.clone())
        .from_block(block_number)
        .to_block(block_number);

    let event: &abi::Event = abi.events().next().unwrap();

    let event_input = event.inputs.clone();

    let input_param_types: Vec<ParamType> =
        event_input.iter().map(|param| param.kind.clone()).collect();

    let input_param_types_cloned = input_param_types.clone();

    let input_param_type = input_param_types_cloned.get(0).unwrap();
    let parsing_input_param_type = ParamType::Tuple(input_param_types);

    let logs: Vec<Log> = client.get_logs(&filter).await?;
    let mut result_vec = Vec::new();

    for log in logs {
        let tokens = Token::Tuple(ethers::abi::decode(&[input_param_type.clone()], &log.data)?);

        let result = parse_decode_token(
            &tokens,
            &parsing_input_param_type,
            &rule_filter,
            &expected_value_index,
        )?;

        result_vec.push(result);
    }

    Ok(result_vec)
}
