use crate::core::types::*;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Brain: Send + Sync {
    async fn decide(&self, context: &DecisionContext) -> Result<TradeDecision>;
    async fn decide_exit(
        &self,
        context: &DecisionContext,
        entry_side: &str,
        entry_price: u32,
        position_shares: u32,
    ) -> Result<TradeDecision>;
}
