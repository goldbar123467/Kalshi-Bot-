mod adapters;
mod core;
mod ports;
mod safety;
mod storage;

use adapters::binance::BinanceClient;
use adapters::kalshi::client::KalshiClient;
use adapters::openrouter::OpenRouterClient;
use adapters::telegram::TelegramClient;
use core::types::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;

    safety::validate_startup(&config)?;

    let _lock = safety::Lockfile::acquire(&config.lockfile_path)?;

    let exchange = KalshiClient::new(&config)?;
    let brain = OpenRouterClient::new(&config)?;
    let notifier = TelegramClient::new(&config)?;
    let price_feed = BinanceClient::new(&config)?;

    core::engine::run_cycle(&exchange, &brain, &notifier, &price_feed, &config).await
}
