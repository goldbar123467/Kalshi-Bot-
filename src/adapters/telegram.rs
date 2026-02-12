use crate::core::types::Config;
use crate::ports::notifier::Notifier;
use anyhow::Result;
use async_trait::async_trait;

pub struct TelegramClient {
    client: reqwest::Client,
    token: String,
    chat_id: String,
}

impl TelegramClient {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            token: config.telegram_token.clone(),
            chat_id: config.telegram_chat_id.clone(),
        })
    }
}

#[async_trait]
impl Notifier for TelegramClient {
    async fn alert(&self, message: &str) -> Result<()> {
        self.client
            .post(format!(
                "https://api.telegram.org/bot{}/sendMessage",
                self.token
            ))
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": message,
            }))
            .send()
            .await?;
        Ok(())
    }
}
