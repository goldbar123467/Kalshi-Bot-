# CLAUDE.md — Kalshi BTC 15-Min Trading Bot (Rust)

## What This Is

A Rust cron job that runs every 15 minutes, asks Claude Opus 4.6 whether to buy YES or NO on Kalshi's BTC Up/Down 15-minute binary contract, places the order, and exits. Stats are computed deterministically in Rust. The AI never writes files. Hexagonal architecture so every external boundary is a swappable trait.

## Architecture

```
                    ┌─────────────────────────────┐
                    │         CORE DOMAIN          │
                    │  (pure Rust, no IO, no deps) │
                    │                              │
                    │  • risk.rs    (limit checks)  │
                    │  • stats.rs   (ledger math)   │
                    │  • types.rs   (domain types)  │
                    │  • engine.rs  (orchestration)  │
                    └──────────┬──────────────────┘
                               │ uses traits (ports)
            ┌──────────────────┼──────────────────────┐
            │                  │                       │
    ┌───────▼──────┐   ┌──────▼───────┐   ┌──────────▼────────┐
    │  Port:       │   │  Port:       │   │  Port:            │
    │  Exchange    │   │  Brain       │   │  Notifier         │
    │              │   │              │   │                    │
    │  • market()  │   │  • decide()  │   │  • alert()        │
    │  • orderbook │   │              │   │                    │
    │  • order()   │   │              │   │                    │
    │  • positions │   │              │   │                    │
    │  • cancel()  │   │              │   │                    │
    │  • settle()  │   │              │   │                    │
    │  • balance() │   │              │   │                    │
    └───────┬──────┘   └──────┬───────┘   └──────────┬────────┘
            │                 │                       │
    ┌───────▼──────┐   ┌──────▼───────┐   ┌──────────▼────────┐
    │  Adapter:    │   │  Adapter:    │   │  Adapter:         │
    │  KalshiApi   │   │  OpenRouter  │   │  Telegram         │
    └──────────────┘   └──────────────┘   └───────────────────┘

    Storage is plain filesystem — read/append to markdown files.
    Not behind a trait in v1. Promote to trait when you need SQLite.
```

### Why Hexagonal

- **Testing**: Mock every adapter. Core domain is pure functions — unit test with zero network.
- **Expansion**: New exchange? Implement `Exchange` trait. Swap AI? Implement `Brain` trait. Add Discord? Implement `Notifier` trait.
- **Clarity**: If it touches the network, it's an adapter. If it's pure logic, it's core. No ambiguity.

## Tech Stack

- **Language**: Rust 2021
- **Async**: Tokio
- **HTTP**: reqwest
- **Serialization**: serde / serde_json
- **Crypto**: rsa (RSA-PSS SHA-256), sha2, base64
- **Time**: chrono
- **Config**: dotenv
- **Error handling**: anyhow

## Project Structure

```
kalshi-bot/
├── Cargo.toml
├── .env
├── .gitignore
├── CLAUDE.md
├── brain/
│   ├── prompt.md                 # Static system prompt (you edit, AI reads)
│   ├── ledger.md                 # Append-only trade log (Rust writes, AI reads)
│   └── stats.md                  # Computed stats (Rust writes, AI reads)
├── src/
│   ├── main.rs                   # Entry point — wires adapters, startup checks, lockfile
│   ├── safety.rs                 # Lockfile, startup validation, live-mode gate
│   ├── core/
│   │   ├── mod.rs
│   │   ├── engine.rs             # Orchestration: the 10-step cycle
│   │   ├── risk.rs               # Pure risk checks — no IO
│   │   ├── stats.rs              # Compute stats from ledger — no IO
│   │   └── types.rs              # All domain types, enums, structs
│   ├── ports/
│   │   ├── mod.rs
│   │   ├── exchange.rs           # Exchange trait
│   │   ├── brain.rs              # Brain trait
│   │   └── notifier.rs           # Notifier trait
│   ├── adapters/
│   │   ├── mod.rs
│   │   ├── kalshi/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs           # RSA-PSS signing
│   │   │   ├── client.rs         # Implements Exchange trait
│   │   │   └── types.rs          # Kalshi-specific API response structs
│   │   ├── openrouter.rs         # Implements Brain trait
│   │   └── telegram.rs           # Implements Notifier trait
│   └── storage.rs                # Read/write brain/*.md files
└── logs/
    └── .gitkeep
```

## Ports (Traits)

### ports/exchange.rs

```rust
#[async_trait]
pub trait Exchange: Send + Sync {
    async fn active_market(&self) -> Result<Option<MarketState>>;
    async fn orderbook(&self, ticker: &str) -> Result<Orderbook>;
    async fn resting_orders(&self) -> Result<Vec<RestingOrder>>;
    async fn cancel_order(&self, order_id: &str) -> Result<()>;
    async fn place_order(&self, order: &OrderRequest) -> Result<String>;
    async fn positions(&self) -> Result<Vec<Position>>;
    async fn settlements(&self, since: &str) -> Result<Vec<Settlement>>;
    async fn balance(&self) -> Result<u64>;
}
```

### ports/brain.rs

```rust
#[async_trait]
pub trait Brain: Send + Sync {
    async fn decide(&self, context: &DecisionContext) -> Result<TradeDecision>;
}
```

### ports/notifier.rs

```rust
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn alert(&self, message: &str) -> Result<()>;
}
```

## Core Engine — The 10-Step Cycle

1. **CANCEL** stale resting orders from previous cycles
2. **SETTLE** — check if previous trade settled, update ledger + stats
3. **RISK** — deterministic checks (balance, daily loss, streak, open position)
4. **MARKET** — fetch active market by series ticker
5. **ORDERBOOK** — fetch orderbook depth
6. **BRAIN** — one AI call with full context
7. **VALIDATE** — clamp shares/price, handle PASS
8. **FINAL POSITION CHECK** — abort if position appeared during AI call
9. **EXECUTE** — order first, ledger second (never phantom trades)
10. **EXIT**

## Risk Limits (hardcoded defaults)

- max_shares: 2
- max_daily_loss_cents: 1000 ($10)
- max_consecutive_losses: 7
- min_balance_cents: 500 ($5)
- min_minutes_to_expiry: 2.0

## Safety

- **Lockfile**: `/tmp/kalshi-bot.lock` — PID-based, prevents double execution from cron overlap
- **Live mode gate**: PAPER_TRADE=true by default; must set CONFIRM_LIVE=true to go live
- **Startup validation**: Checks all config before any network calls
- **Ledger backup**: `brain/ledger.md.bak` before every write
- **Atomic stats**: Write to `.tmp` then rename
- **Order-first**: Order placed before ledger write; if order fails, ledger stays clean

## Kalshi Auth

RSA-PSS with SHA-256, MGF1(SHA-256), salt length = digest length (32 bytes).
Message format: `{timestamp_ms}{METHOD}{path}`
Handles both PKCS#1 and PKCS#8 PEM formats.

### Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/trade-api/v2/exchange/status` | GET | Verify exchange is open |
| `/trade-api/v2/markets` | GET | Market discovery |
| `/trade-api/v2/markets/{ticker}/orderbook` | GET | Orderbook |
| `/trade-api/v2/portfolio/orders` | GET | Resting orders |
| `/trade-api/v2/portfolio/orders` | POST | Place order |
| `/trade-api/v2/portfolio/orders/{id}` | DELETE | Cancel order |
| `/trade-api/v2/portfolio/positions` | GET | Open positions |
| `/trade-api/v2/portfolio/settlements` | GET | Settled trades |
| `/trade-api/v2/portfolio/balance` | GET | Balance in cents |

### Base URLs

- **Production**: `https://api.elections.kalshi.com`
- **Demo**: `https://demo-api.kalshi.co`

## OpenRouter / Brain

- Model: `anthropic/claude-opus-4-6` (or `anthropic/claude-sonnet-4-5-20250929` for cost savings)
- Parse fail = PASS (never trade on garbage)
- ~$0.05/cycle, ~$5/day at 96 cycles

## Cron

```bash
1,16,31,46 * * * * cd /path/to/kalshi-bot && ./target/release/kalshi-bot >> logs/cron.log 2>&1
```

## First Run — Discover Series Ticker

Query `/trade-api/v2/markets?status=open&limit=200`, filter for BTC + 15-min titles, note the `series_ticker`, put it in `.env`.

## What's NOT in v1

- AI writing to any file
- Confidence scores
- External BTC price feeds
- Debug/self-critique AI loop
- WebSocket streaming
- Multiple positions
- Early exit / selling

All are v2+ features that plug into existing traits.
