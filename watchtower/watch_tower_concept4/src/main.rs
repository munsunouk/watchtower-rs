mod call;
mod event;
mod utils;

use anyhow::Ok;
use ethers::providers::{Http, Provider};
use std::{iter::zip, sync::Arc};

use crate::call::call_contract;
use crate::event::event_contract;
use crate::utils::{ContractCallRule, ContractEventRule};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let node = "https://public-01.testnet.bifrostnetwork.com/rpc";

    let contract_call_rules = vec![
        ContractCallRule::new(
            //BIFI
            "0xb871966e866F684681f9F44A69BF19652C0c462c",
            "./src/abi/callproxy.json",
            vec![],
            vec!["0.0.0-0"],
            "0.0.2",
            "100000000",
            "uint256",
            "<",
        )?,
        ContractCallRule::new(
            //EVERDEX
            "0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423",
            "./src/abi/poolinfo.json",
            vec!["0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb"],
            vec![],
            "0.2",
            "100000000",
            "uint256",
            "<",
        )?,
    ];

    let contract_event_rules = vec![
        //CCCP - INBOUND
        ContractEventRule::new(
            "0x0218371b18340aBD460961bdF3Bd5F01858dAB53",
            "./src/abi/socket.json",
            vec!["0.0.0-00014a34", "0.2.0-0000bfc0"],
            "0.3.4",
            "100000000",
            "uint256",
            "<",
        )?,
        //BRP - INBOUND
        ContractEventRule::new(
            "0xc292D9d5c31D5246cfAC67ba91202bbCF0AA8108",
            "./src/abi/socket.json",
            vec!["0.0.0-00002711", "0.2.0-0000bfc0"],
            "0.3.4",
            "100000000",
            "uint256",
            "<",
        )?,
    ];

    let contract_event_block_numbers = vec![18657523, 18862506];

    for contract_call_rule in contract_call_rules {
        let provider = Provider::<Http>::try_from(node)?;
        let client = Arc::new(provider);

        let contract_address = contract_call_rule.address;
        let abi = contract_call_rule.abi;
        let method_params = contract_call_rule.method_params;
        let rule_filter = contract_call_rule.rule_filter;
        let expected_value_index = contract_call_rule.expected_value_index;

        let value = call_contract(
            client,
            &contract_address,
            &abi,
            method_params,
            &rule_filter,
            &expected_value_index,
        )
        .await
        .unwrap();

        println!("value: {:?}", value);
    }

    for (contract_event_rule, block_number) in
        zip(contract_event_rules, contract_event_block_numbers)
    {
        let provider = Provider::<Http>::try_from(node)?;
        let client = Arc::new(provider);

        let contract_address = contract_event_rule.address;
        let abi = contract_event_rule.abi;
        let rule_filter = contract_event_rule.rule_filter;
        let expected_value_index = contract_event_rule.expected_value_index;

        let value = event_contract(
            client,
            &contract_address,
            &abi,
            block_number,
            &rule_filter,
            &expected_value_index,
        )
        .await
        .unwrap();

        println!("value: {:?}", value);
    }

    Ok(())
}
