use crate::core::types::*;
use crate::ports::brain::Brain;
use anyhow::Result;
use async_trait::async_trait;

pub struct OpenRouterClient {
    client: reqwest::Client,
    api_key: String,
}

impl OpenRouterClient {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            api_key: config.openrouter_api_key.clone(),
        })
    }
}

#[async_trait]
impl Brain for OpenRouterClient {
    async fn decide(&self, ctx: &DecisionContext) -> Result<TradeDecision> {
        let btc_section = match &ctx.btc_price {
            Some(snap) => format!(
                "\n\n---\n## BTC PRICE (Binance BTCUSDT)\n{}",
                format_btc_price(snap)
            ),
            None => "\n\n---\n## BTC PRICE\nUnavailable this cycle.".into(),
        };

        let prompt = format!(
            "{prompt}\n\n---\n## STATS\n{stats}\n\n---\n## LAST {n} TRADES\n{ledger}\n\n---\n## MARKET\n{market}\n\n---\n## ORDERBOOK\nYes bids: {yes_ob}\nNo bids: {no_ob}{btc}",
            prompt = ctx.prompt_md,
            stats = format_stats(&ctx.stats),
            n = ctx.last_n_trades.len(),
            ledger = format_ledger(&ctx.last_n_trades),
            market = format_market(&ctx.market),
            yes_ob = format_ob_side(&ctx.orderbook.yes),
            no_ob = format_ob_side(&ctx.orderbook.no),
            btc = btc_section,
        );

        let content = self.call_model(
            "anthropic/claude-opus-4-6",
            &prompt,
            1200,
        ).await?;

        parse_decision(&content)
    }

    async fn decide_exit(
        &self,
        ctx: &DecisionContext,
        entry_side: &str,
        entry_price: u32,
        position_shares: u32,
    ) -> Result<TradeDecision> {
        let btc_section = match &ctx.btc_price {
            Some(snap) => format!(
                "\n\n## BTC PRICE\n{}",
                format_btc_price(snap)
            ),
            None => "\n\n## BTC PRICE\nUnavailable.".into(),
        };

        let exit_prompt = format!(
            "You hold {shares}x {side} @ {price}¢ on {ticker}.\n\
             The contract expires in {expiry:.1} minutes.\n\n\
             ## CURRENT MARKET\n{market}\n\n\
             ## ORDERBOOK\nYes bids: {yes_ob}\nNo bids: {no_ob}{btc}\n\n\
             ## DECISION\n\
             Should you SELL your {side} contracts to lock in profit/cut loss, or HOLD to expiry?\n\
             If SELL, set max_price_cents to the price you'd sell your {side} at (look at the {side} bid side of the orderbook).\n\
             Respond with JSON: {{\"action\": \"SELL\" or \"PASS\", \"shares\": {shares}, \"max_price_cents\": <sell price for your {side} contracts>, \"reasoning\": \"...\"}}\n\
             SELL = close now. PASS = hold to expiry.",
            shares = position_shares,
            side = entry_side.to_uppercase(),
            price = entry_price,
            ticker = ctx.market.ticker,
            expiry = ctx.market.minutes_to_expiry,
            market = format_market(&ctx.market),
            yes_ob = format_ob_side(&ctx.orderbook.yes),
            no_ob = format_ob_side(&ctx.orderbook.no),
            btc = btc_section,
        );

        let content = self.call_model(
            "anthropic/claude-opus-4-6",
            &exit_prompt,
            800,
        ).await?;

        parse_decision(&content)
    }
}

impl OpenRouterClient {
    async fn call_model(&self, model: &str, prompt: &str, max_tokens: u32) -> Result<String> {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "temperature": 0.2,
            "messages": [{"role": "user", "content": prompt}]
        });

        let resp = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://kyzlolabs.com")
            .header("X-Title", "Kalshi BTC Bot")
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let msg = &resp["choices"][0]["message"];
        msg["content"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| msg["reasoning"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No content in OpenRouter response: {}", resp))
    }
}

fn format_stats(s: &Stats) -> String {
    format!(
        "Trades: {} | W/L: {}/{} | Win rate: {:.1}% | P&L: {}¢ | Today: {}¢ | Streak: {} | Drawdown: {}¢",
        s.total_trades, s.wins, s.losses, s.win_rate * 100.0,
        s.total_pnl_cents, s.today_pnl_cents, s.current_streak, s.max_drawdown_cents
    )
}

fn format_ledger(trades: &[LedgerRow]) -> String {
    if trades.is_empty() {
        return "No trades yet.".into();
    }
    trades
        .iter()
        .map(|t| {
            format!(
                "{} | {} | {} | {}x @ {}¢ | {} | {}¢",
                t.timestamp, t.ticker, t.side, t.shares, t.price, t.result, t.pnl_cents
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_market(m: &MarketState) -> String {
    format!(
        "Ticker: {} | Title: {} | Yes bid/ask: {:?}/{:?} | No bid/ask: {:?}/{:?} | Last: {:?} | Vol: {} | 24h Vol: {} | OI: {} | Expiry: {} ({:.1}min)",
        m.ticker, m.title, m.yes_bid, m.yes_ask, m.no_bid, m.no_ask,
        m.last_price, m.volume, m.volume_24h, m.open_interest,
        m.expiration_time, m.minutes_to_expiry
    )
}

fn format_ob_side(levels: &[(u32, u32)]) -> String {
    if levels.is_empty() {
        return "empty".into();
    }
    levels
        .iter()
        .take(5)
        .map(|(p, q)| format!("{}¢ x{}", p, q))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_btc_price(snap: &PriceSnapshot) -> String {
    let ind = &snap.indicators;
    let momentum_str = match ind.momentum {
        MomentumDirection::Up => "UP",
        MomentumDirection::Down => "DOWN",
        MomentumDirection::Flat => "FLAT",
    };

    let mut s = format!(
        "Spot: ${:.2} | 15m change: {:+.3}% | 1h change: {:+.3}% | Momentum: {}\n\
         SMA(15x1m): ${:.2} | Price vs SMA: {} | 1m volatility: {:.4}%",
        ind.spot_price,
        ind.pct_change_15m,
        ind.pct_change_1h,
        momentum_str,
        ind.sma_15m,
        ind.price_vs_sma,
        ind.volatility_1m,
    );

    if !ind.last_3_candles.is_empty() {
        s.push_str("\nLast 3 candles (1m): ");
        let candle_strs: Vec<String> = ind
            .last_3_candles
            .iter()
            .map(|c| {
                format!(
                    "O:{:.0} H:{:.0} L:{:.0} C:{:.0} V:{:.1}",
                    c.open, c.high, c.low, c.close, c.volume
                )
            })
            .collect();
        s.push_str(&candle_strs.join(" | "));
    }

    s
}

fn parse_decision(raw: &str) -> Result<TradeDecision> {
    let json_str = if let Some(s) = raw.find("```json") {
        let start = s + 7;
        let end = raw[start..]
            .find("```")
            .map(|i| start + i)
            .unwrap_or(raw.len());
        &raw[start..end]
    } else if raw.trim().starts_with('{') {
        raw.trim()
    } else if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) {
        &raw[s..=e]
    } else {
        return Ok(TradeDecision {
            action: Action::Pass,
            side: None,
            shares: None,
            max_price_cents: None,
            reasoning: "Failed to parse AI response".into(),
        });
    };

    match serde_json::from_str(json_str.trim()) {
        Ok(decision) => Ok(decision),
        Err(e) => {
            tracing::warn!("JSON parse failed ({}), defaulting to PASS", e);
            Ok(TradeDecision {
                action: Action::Pass,
                side: None,
                shares: None,
                max_price_cents: None,
                reasoning: "Failed to parse AI response".into(),
            })
        }
    }
}
