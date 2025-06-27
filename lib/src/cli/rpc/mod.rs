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
        for (index, provider) in self.providers.iter().enumerate() {
            let mut request = provider.request(method.to_owned(), url);

            // Add bearer token if provided
            if let Some(token) = url_token {
                request = request.bearer_auth(token.trim());
            }

            match request.query(query).send().await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // If this is not the last provider, sleep and try the next one
                    if index < self.providers.len() - 1 {
                        sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
                        continue;
                    } else {
                        // This was the last provider, return the error
                        return Err(ClientError::from(e));
                    }
                }
            }
        }

        Err(ClientError::InternalProviderError(format!(
            "All providers failed for URL: {}, query: {:?}",
            url, query
        )))
    }

    pub async fn request_with_json(
        &self,
        method: &Method,
        url: &String,
        url_token: &Option<String>,
        body: &Value,
    ) -> Result<Response, ClientError> {
        for (index, provider) in self.providers.iter().enumerate() {
            let mut request = provider.request(method.to_owned(), url);

            // Add bearer token if provided
            if let Some(token) = url_token {
                request = request.bearer_auth(token.trim());
            }

            match request.json(body).send().await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // If this is not the last provider, sleep and try the next one
                    if index < self.providers.len() - 1 {
                        sleep(Duration::from_millis(DEFAULT_CALL_RETRY_INTERVAL_MS)).await;
                        continue;
                    } else {
                        // This was the last provider, return the error
                        return Err(ClientError::from(e));
                    }
                }
            }
        }

        Err(ClientError::InternalProviderError(format!(
            "All providers failed for URL: {}, body: {:?}",
            url, body
        )))
    }
}
