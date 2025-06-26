use crate::utils::{constants::DEFAULT_CALL_RETRY_INTERVAL_MS, error::ClientError};
use reqwest::{Client, Method, Response};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

#[derive(Clone)]
pub struct RpcClient {
    providers: Vec<Arc<Client>>,
}

impl RpcClient {
    pub fn new(providers: Vec<Arc<Client>>) -> Self {
        Self { providers }
    }

    pub async fn request_with_query(
        &self,
        method: &Method,
        url: &String,
        url_token: &Option<String>,
        query: &Value,
    ) -> Result<Response, ClientError> {
        let mut error_msg = String::default();

        for provider in &self.providers {
            let mut request = provider.request(method.to_owned(), url);

            // Add bearer token if provided
            if let Some(token) = url_token {
                request = request.header("Authorization", format!("Bearer {}", token.trim()));
            }

            match request.query(query).send().await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    error_msg = format!("❗️ [method: {:?}] [Error: {}]", method, error.to_string());
                }
            }
            sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
        }

        Err(ClientError::InternalProviderError(error_msg))
    }

    pub async fn request_with_json(
        &self,
        method: &Method,
        url: &String,
        url_token: &Option<String>,
        body: &Value,
    ) -> Result<Response, ClientError> {
        let mut error_msg = String::default();

        for provider in &self.providers {
            let mut request = provider.request(method.to_owned(), url);

            // Add bearer token if provided
            if let Some(token) = url_token {
                request = request.header("Authorization", format!("Bearer {}", token.trim()));
            }

            match request.json(body).send().await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    error_msg = format!(
                        "❗️ [method: {:?}] [url: {}] [Error: {}]",
                        method,
                        url,
                        error.to_string()
                    );
                }
            }
            sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
        }

        Err(ClientError::InternalProviderError(error_msg))
    }
}
