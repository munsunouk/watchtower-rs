use ethers::{
    abi::{ParamType, Token},
    types::{U256, U64},
};
use reqwest::Method;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};
use watch_tower_lib::{
    cli::rpc::RpcClient,
    rule::rpc_call::RpcCallRule,
    utils::{
        constants::{
            DB_API_BODY_TYPE_COLUMN, DB_CALL_TIME_INTERVAL_COLUMN, DB_CALL_TYPE_COLUMN,
            DB_ID_COLUMN, DB_METHOD_TYPE_COLUMN, DB_NAME_COLUMN, DB_URL_COLUMN, DB_VALUES_COLUMN,
        },
        parse_i32_to_usize, parse_json_to_value, parse_string_to_method,
        parse_string_to_rpc_call_type,
        types::RuleID,
        RpcCallType,
    },
};

use crate::{
    rule::{convert_value_to_param_type, convert_value_to_token},
    utils::error::WorkerError,
};

use super::parse_string_to_values;

/// # Description
/// This struct represents an RPC call.
/// # Arguments
/// * `rule` - The RPC call rule.
/// * `client` - The HTTP client.
/// * `request` - The JSON-RPC request.
#[derive(Clone)]
pub struct RpcCall {
    pub rule: RpcCallRule,
    pub client: RpcClient,
}

impl RpcCall {
    /// # Description
    /// This function creates a new `RpcCall` instance.
    /// # Arguments
    /// * `client` - The HTTP client.
    /// * `rule` - The RPC call rule.
    /// # Returns
    ///
    /// A new instance of `RpcCall`.
    pub fn new(client: RpcClient, rule: RpcCallRule) -> Self {
        Self { rule, client }
    }

    pub async fn fetch_api_call_with_query(&self) -> Result<(Token, ParamType), WorkerError> {
        if self.rule.api_query.is_none() {
            return Err(WorkerError::InvalidEmptyToken);
        }

        let query = self.rule.api_query.as_ref().unwrap();

        let response = self
            .client
            .request_with_query(self.rule.method_type.clone(), &self.rule.url, &query)
            .await
            .map_err(|e| WorkerError::InternalProviderError(e.to_string()))?;

        let status: U64 = response.status().as_u16().into();
        let body = response
            .json::<Value>()
            .await
            .map_err(|e| WorkerError::InternalProviderError(e.to_string()))?;

        let status_token = Token::Uint(U256::from(status.as_u64()));
        let body_token = convert_value_to_token(&body)?;
        let body_param_type = convert_value_to_param_type(&body)?;

        let param_type = ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);
        let tokens = Token::Tuple(vec![status_token, body_token]);

        Ok((tokens, param_type))
    }

    /// # Description
    /// This function fetches the RPC call status.
    /// # Returns
    ///
    /// A result containing the status as `U64`.
    pub async fn fetch_api_call_with_body(&self) -> Result<(Token, ParamType), WorkerError> {
        if self.rule.api_body.is_none() {
            return Err(WorkerError::InvalidEmptyToken);
        }

        let api_body = self.rule.api_body.as_ref().unwrap();

        let response = self
            .client
            .request_with_json(self.rule.method_type.clone(), &self.rule.url, &api_body)
            .await;

        match response {
            Ok(resp) => {
                let status: U64 = resp.status().as_u16().into();

                let body = parse_json_to_value(resp.json().await.map_err(|_| {
                    WorkerError::InternalProviderError("Failed to parse JSON response".to_string())
                })?)
                .map_err(|e| {
                    WorkerError::InvalidTypeConvertError(format!("Failed to parse values: {}", e))
                })?;

                let status_token = Token::Uint(U256::from(status.as_u64()));

                let body_token = convert_value_to_token(&body)?;

                let body_param_type = convert_value_to_param_type(&body)?;

                let param_type = ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);

                let tokens = Token::Tuple(vec![status_token, body_token]);

                Ok((tokens, param_type))
            }
            Err(error) => Err(WorkerError::InternalProviderError(error.to_string())),
        }
    }
}

#[cfg(test)]
mod test {

    use std::sync::Arc;

    use ethers::abi::ParamType;
    use reqwest::{Client, Method};
    use serde_json::json;
    use watch_tower_lib::{
        cli::db::data::RpcCallRuleData,
        utils::{constants::RPC_CALL_RULE_TYPE, evaluation::parse_rules},
    };

    use crate::rule::{convert_value_to_param_type, decodes_token};

    use super::*;

    #[tokio::test]
    async fn test_fetch_api_call_with_body() {
        let test_input = "rpccall { Public_Call1 to url <YOUR_RPC_URL> call type body method type POST body { \"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBalance\", \"params\": [\"0x51c9abb01e2ef6495daafc56778b499e8d3992ff\", \"latest\"] } values{balance is 1.0.0} call every 10 seconds } filter balance >= 0 move balance";

        // let (rules, _eval_rules) = parse_rules(test_input).unwrap();
        // let client = Arc::new(Client::new());

        // for rule in rules {
        //     if let Some(rule) = rule {
        //         if let Some(Token::String(ref token_type)) = rule.get("type") {
        //             match token_type.as_str() {
        //                 RPC_CALL_RULE_TYPE => {
        //                     let rule_data = RpcCallRuleData::from_tokens(rule).unwrap();

        //                     let method_type = parse_string_to_method(rule_data.method_type);
        //                     let url = rule_data.url;
        //                     let params = rule_data.api_body;

        //                     let res = client
        //                         .request(method_type, url.clone())
        //                         .json(&params)
        //                         .send()
        //                         .await
        //                         .unwrap();

        //                     let status: U64 = res.status().as_u16().into();
        //                     let body = res.json::<Value>().await.unwrap();
        //                     println!("{:?}", body);

        //                     let status_token = Token::Uint(U256::from(status.as_u64()));
        //                     let body_token = convert_value_to_token(&body).unwrap();
        //                     println!("body_token: {:?}", body_token);

        //                     let tokens = Token::Tuple(vec![status_token, body_token]);
        //                     let body_param_type = convert_value_to_param_type(&body).unwrap();
        //                     let param_type =
        //                         ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);
        //                     println!("tokens:{:?}", tokens);

        //                     let result =
        //                         decodes_token(&tokens, &param_type, &vec![vec![1, 2]]).unwrap();
        //                     println!("result: {:?}", result);
        //                 }
        //                 _ => {}
        //             }
        //         }
        //     }
        // }
    }

    // #[tokio::test]
    // async fn test_fetch_api_call_with_query() {
    //     let client = Arc::new(Client::new());
    //     let method_type = Method::GET;
    //     let url = "https://blockchain.info/balance".to_string();
    //     let query = json!({
    //         "active": "bc1p2cmsnvtvxxvvyxm055vc45827zdyvawsyps6ctqta7lapuh2hepqsp5qas|bc1q6ylrskh4p6u983kx8f0mp0ztwer850u0xzeszj"
    //     });

    //     let res = client
    //         .request(method_type, url)
    //         .query(&query)
    //         .send()
    //         .await
    //         .unwrap();

    //     let status: U64 = res.status().as_u16().into();

    //     let status_token = Token::Uint(U256::from(status.as_u64()));

    //     let body = res.json::<Value>().await.unwrap();

    //     println!("body: {:?}", body);

    //     let body_token = convert_value_to_token(&body).unwrap();
    //     let body_param_type = convert_value_to_param_type(&body).unwrap();

    //     let param_type = ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);
    //     let tokens = Token::Tuple(vec![status_token, body_token]);

    //     let result =
    //         decodes_token(&tokens, &param_type, &vec![vec![1, 0, 0], vec![1, 1, 0]]).unwrap();

    //     println!("result: {:?}", result);
    // }
}
