use crate::core::types::*;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Brain: Send + Sync {
    async fn decide(&self, context: &DecisionContext) -> Result<TradeDecision>;
}
