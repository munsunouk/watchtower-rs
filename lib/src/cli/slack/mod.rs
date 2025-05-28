use crate::utils::error::ClientError;
use slack::api::chat::PostMessageRequest;

#[derive(Clone)]
pub struct SlackClient {
    token: String,
    channel: String,
}

impl SlackClient {
    pub fn new(token: &str, channel: &str) -> Self {
        Self {
            token: token.to_string(),
            channel: channel.to_string(),
        }
    }

    pub async fn send_message(&self, text: &str) -> Result<(), ClientError> {
        let request = PostMessageRequest {
            channel: &self.channel,
            text,
            ..Default::default()
        };

        let client = slack::api::requests::default_client()
            .map_err(|e| ClientError::InternalProviderError(e.to_string()))?;

        slack::api::chat::post_message(&client, &self.token, &request)
            .map_err(|e| ClientError::InternalProviderError(e.to_string()))?;

        Ok(())
    }

    pub async fn send_alert(
        &self,
        title: &str,
        message: &str,
        hashtag: Option<&str>,
    ) -> Result<(), ClientError> {
        let alert_text = match hashtag {
            Some(tag) => format!("*{}*\n{} {}", title, tag, message),
            None => format!("*{}*\n{}", title, message),
        };
        let request = PostMessageRequest {
            channel: &self.channel,
            text: &alert_text,
            ..Default::default()
        };

        let client = slack::api::requests::default_client()
            .map_err(|e| ClientError::InternalProviderError(e.to_string()))?;

        slack::api::chat::post_message(&client, &self.token, &request)
            .map_err(|e| ClientError::InternalProviderError(e.to_string()))?;

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::cli::slack::send_message;
//     use crate::config::Config;
//     use crate::utils::error::GeneralError;
//     use std::env;

//     #[tokio::test]
//     async fn test_send_message() -> Result<(), GeneralError> {
//         let config = Config::default();
//         let result = send_message(&config, "test message").await;
//         assert!(result.is_ok());
//         Ok(())
//     }

//     #[test]
//     fn test_parse_rules_new_format() {
//         let input = r#"get(type=contractcall, name=test2, chain=3068, address=0x0000000000000000000000000000000000000100, abi=[{"type":"function","name":"current_round","stateMutability":"view","inputs":[],"outputs":[{"internalType":"uint32","name":"","type":"uint32"}]}], params={pool:0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}, value={status:0}, check_block=3, target_block=0)"#;
//         let result = parse_rules(input).unwrap();
//         assert_eq!(result.len(), 1);
//         let rule = result[0].as_ref().unwrap();
//         assert_eq!(rule.get("type").unwrap(), &Token::String("contractcall".to_string()));
//         assert_eq!(rule.get("name").unwrap(), &Token::String("test2".to_string()));
//         assert_eq!(rule.get("chain").unwrap(), &Token::Uint(U256::from(3068)));
//     }
// }
