use crate::{option_or_err, rule::parse_u256_to_i64, utils::error::ClientError};
use chrono::{DateTime, Duration, Utc};
use ethers::types::U256;
use rustls::crypto::ring::default_provider;
use slack_morphism::prelude::*;

#[derive(Clone)]
pub struct SlackNotifier {
    token: String,
    channel: Option<String>,
    time_interval: Option<i64>,
    latest_notification_timestamp: Option<String>,
}

impl SlackNotifier {
    pub fn new(token: &str) -> Self {
        let _ = default_provider().install_default();

        Self {
            token: token.to_string(),
            channel: None,
            time_interval: None,
            latest_notification_timestamp: None,
        }
    }

    pub fn set_channel(&mut self, channel: &str) {
        self.channel = Some(channel.to_string());
    }

    pub fn set_time_interval(&mut self, time_interval: &U256) -> Result<(), ClientError> {
        self.time_interval = Some(parse_u256_to_i64(time_interval)?);
        Ok(())
    }

    pub async fn send_message(&mut self, text: &str) -> Result<(), ClientError> {
        if !self.check_should_post()? {
            return Ok(());
        }

        let client = slack_morphism::SlackClient::new(SlackClientHyperConnector::new()?);
        let token_value: SlackApiTokenValue = self.token.as_str().into();
        let token = SlackApiToken::new(token_value);
        let session = client.open_session(&token);

        let formatted_text = text.replace("\\n", "\n");
        let message = SlackMessageContent::new().with_text(formatted_text);

        let request = SlackApiChatPostMessageRequest::new(
            SlackChannelId(option_or_err!(self.channel.as_ref()).to_string()),
            message,
        );

        session.chat_post_message(&request).await?;

        self.latest_notification_timestamp = Some(Utc::now().to_rfc3339());

        Ok(())
    }

    fn check_should_post(&self) -> Result<bool, ClientError> {
        match (self.time_interval, &self.latest_notification_timestamp) {
            (Some(interval), Some(latest)) => {
                let now = Utc::now().to_rfc3339();
                let now_dt = now.parse::<DateTime<Utc>>()?;
                let latest_dt = latest.parse::<DateTime<Utc>>()?;

                Ok(now_dt - latest_dt > Duration::seconds(interval))
            }
            _ => Ok(true),
        }
    }
}
