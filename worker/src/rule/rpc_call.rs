use ethers::types::U64;
use serde_json::{json, Value};
use watch_tower_lib::{
    cli::rpc::RpcClient, rule::rpc_call::RpcCallRule, utils::parse_json_to_value,
};

use crate::utils::error::WorkerError;

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
    pub fn new(client: &RpcClient, rule: &RpcCallRule) -> Self {
        Self {
            rule: rule.to_owned(),
            client: client.to_owned(),
        }
    }

    pub async fn fetch_api_call_with_query(&self) -> Result<(U64, Value), WorkerError> {
        let empty_json = json!({});
        let query = self.rule.api_query.as_ref().unwrap_or(&empty_json);

        let response = self
            .client
            .request_with_query(
                &self.rule.method_type,
                &self.rule.url,
                &self.rule.url_token,
                query,
            )
            .await?;

        let status: U64 = response.status().as_u16().into();
        let body = response.json::<Value>().await?;

        Ok((status, body))
    }

    /// # Description
    /// This function fetches the RPC call status.
    /// # Returns
    ///
    /// A result containing the status as `U64`.
    pub async fn fetch_api_call_with_body(&self) -> Result<(U64, Value), WorkerError> {
        let empty_json = json!({});
        let api_body = self.rule.api_body.as_ref().unwrap_or(&empty_json);

        let response = self
            .client
            .request_with_json(
                &self.rule.method_type,
                &self.rule.url,
                &self.rule.url_token,
                api_body,
            )
            .await;

        match response {
            Ok(resp) => {
                let status: U64 = resp.status().as_u16().into();

                let body = parse_json_to_value(resp.json().await?)?;

                Ok((status, body))
            }
            Err(error) => Err(WorkerError::InternalProviderError(error.to_string())),
        }
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;
    use std::sync::Arc;

    use ethers::{
        abi::{ParamType, Token},
        types::{Address, U256},
    };
    use reqwest::{Client, Method};
    use serde_json::json;
    use watch_tower_lib::rule::parse_string_to_target_index;

    use crate::rule::{
        convert_target_index_to_indices, convert_value_to_param_type, convert_value_to_token,
        decodes_token,
    };

    use super::*;

    #[tokio::test]
    async fn test_fetch_api_call_with_query() {
        let client = Arc::new(Client::new());
        let method_type = Method::GET;
        let url = "https://reference-data-directory.vercel.app/feeds-mainnet.json".to_string();
        let url_token: Option<String> = Some("<TOKEN>".to_string());
        let query = json!({});
        let raw_target_index =
            "1.{proxyAddress: 0x2665701293fCbEB223D11A08D826563EDcCE423A}".to_string();

        let target_index = parse_string_to_target_index(raw_target_index).unwrap();

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

        let mut param_type = ParamType::Tuple(vec![ParamType::Uint(256), body_param_type]);
        let mut tokens = Token::Tuple(vec![status_token, body_token]);
        let mut key_store = Vec::new();

        let (mut indices, mut foreach_positions) =
            convert_target_index_to_indices(&target_index, Some(&body), Some(&mut key_store))
                .unwrap();

        println!("indices: {indices:?}");

        // Test extraction using found indices
        let usdc_path_result = decodes_token(
            &mut tokens,
            &mut param_type,
            &mut indices,
            &mut foreach_positions,
        )
        .unwrap();

        assert!(
            matches!(usdc_path_result, Token::Address(s) if s == Address::from_str("0x2665701293fCbEB223D11A08D826563EDcCE423A").unwrap())
        );
    }
}
