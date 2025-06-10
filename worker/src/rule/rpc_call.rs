use ethers::{
    abi::{ParamType, Token},
    types::{U256, U64},
};
use serde_json::{json, Value};
use watch_tower_lib::{
    cli::rpc::RpcClient, rule::rpc_call::RpcCallRule, utils::parse_json_to_value,
};

use crate::{
    rule::{convert_value_to_param_type, convert_value_to_token},
    utils::error::WorkerError,
};

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
        let empty_json = json!({});
        let query = self.rule.api_query.as_ref().unwrap_or_else(|| &empty_json);

        let response = self
            .client
            .request_with_query(
                self.rule.method_type.clone(),
                &self.rule.url,
                &self.rule.url_token,
                &query,
            )
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
        let empty_json = json!({});
        let api_body = self.rule.api_body.as_ref().unwrap_or_else(|| &empty_json);

        let response = self
            .client
            .request_with_json(
                self.rule.method_type.clone(),
                &self.rule.url,
                &self.rule.url_token,
                &api_body,
            )
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
    use watch_tower_lib::rule::TargetIndex;

    use crate::rule::{convert_value_to_param_type, decodes_token};

    use super::*;

    #[tokio::test]
    async fn test_fetch_api_call_with_query() {
        let client = Arc::new(Client::new());
        let method_type = Method::GET;
        let url = "<URL>".to_string();
        let url_token: Option<String> = Some("<TOKEN>".to_string());
        let query = json!({});

        let res = client
            .request(method_type, url)
            .header(
                "Authorization",
                format!("Bearer {}", url_token.as_ref().unwrap()),
            )
            .query(&query)
            .send()
            .await
            .unwrap();

        let status: U64 = res.status().as_u16().into();

        let status_token = Token::Uint(U256::from(status.as_u64()));

        let body = res.json::<Value>().await.unwrap();

        let body_token = convert_value_to_token(&body).unwrap();
        let body_param_type = convert_value_to_param_type(&body).unwrap();

        let param_type = ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);
        let tokens = Token::Tuple(vec![status_token, body_token]);

        let result = decodes_token(
            &tokens,
            &param_type,
            &vec![
                TargetIndex::Index(1),  // value
                TargetIndex::Index(1),  // data
                TargetIndex::Index(0),  // first item
                TargetIndex::Index(12), // tvl
            ],
        )
        .unwrap();

        println!("result: {:?}", result);
    }
}
