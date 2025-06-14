use crate::utils::error::ClientError;
use rustls::crypto::ring::default_provider;
use slack_morphism::prelude::*;

#[derive(Clone)]
pub struct SlackNotifier {
    token: String,
    channel: String,
}

impl SlackNotifier {
    pub fn new(token: &str, channel: &str) -> Self {
        // Try to install default provider, but continue if it fails
        let _ = default_provider().install_default();

        Self {
            token: token.to_string(),
            channel: channel.to_string(),
        }
    }

    pub async fn send_message(&self, text: &str) -> Result<(), ClientError> {
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
}
