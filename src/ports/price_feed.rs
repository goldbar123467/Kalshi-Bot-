use crate::core::types::Candle;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PriceFeed: Send + Sync {
    async fn candles(
        &self,
        symbol: &str,
        interval: &str,
        limit: u32,
    ) -> Result<Option<Vec<Candle>>>;

    async fn spot_price(&self, symbol: &str) -> Result<Option<f64>>;
}
