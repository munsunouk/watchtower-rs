use crate::utils::error::ClientError;
use rustls::crypto::ring::default_provider;
use slack_morphism::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct SlackNotifier {
    token: String,
    channel: String,
}

impl SlackNotifier {
    pub fn new(token: &str, channel: &str) -> Self {
        Self {
            token: token.to_string(),
            channel: channel.to_string(),
        }
    }

    pub async fn send_message(&self, text: &str) -> Result<(), ClientError> {
        default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let client = slack_morphism::SlackClient::new(SlackClientHyperConnector::new().unwrap());
        let token_value: SlackApiTokenValue = self.token.clone().into();
        let token = SlackApiToken::new(token_value);
        let session = client.open_session(&token);

        let formatted_text = text.replace("\\n", "\n");
        let message = SlackMessageContent::new().with_text(formatted_text);

        println!("{}", text);

        let request =
            SlackApiChatPostMessageRequest::new(SlackChannelId(self.channel.clone()), message);

        session
            .chat_post_message(&request)
            .await
            .map_err(|e| ClientError::InternalProviderError(e.to_string()))?;

        Ok(())
    }

    pub async fn send_alert(
        &self,
        title: &str,
        message: &str,
        hashtag: Option<&str>,
    ) -> Result<(), ClientError> {
        default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let alert_text = match hashtag {
            Some(tag) => format!("{}\n{}", message, tag),
            None => message.to_string(),
        };
        let formatted_alert_text = alert_text.replace("\\n", "\n").replace("\\>", "> ");
        let message = SlackMessageContent::new().with_text(formatted_alert_text);

        let client = slack_morphism::SlackClient::new(SlackClientHyperConnector::new().unwrap());
        let token_value: SlackApiTokenValue = self.token.clone().into();
        let token = SlackApiToken::new(token_value);
        let session = client.open_session(&token);

        let request =
            SlackApiChatPostMessageRequest::new(SlackChannelId(self.channel.clone()), message);

        session
            .chat_post_message(&request)
            .await
            .map_err(|e| ClientError::InternalProviderError(e.to_string()))?;

        Ok(())
    }
}
