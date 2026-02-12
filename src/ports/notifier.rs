use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn alert(&self, message: &str) -> Result<()>;
}
