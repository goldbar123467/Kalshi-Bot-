# kalshi-bot

Autonomous Rust bot that trades BTC 15-minute binary contracts on Kalshi, powered by Claude Opus 4.6.

## How It Works

Every 5 minutes, the bot:

1. Checks if the previous trade settled and updates the ledger
2. Runs risk checks (balance floor, daily loss cap, stop loss, streak limit)
3. Fetches the active BTC Up/Down market, orderbook, and live BTC price from Binance
4. Sends everything to Claude Opus 4.6 — market state, orderbook, BTC momentum, performance stats, trade history
5. Claude returns **BUY** (side, shares, price), **SELL** (early exit), or **PASS** with reasoning
6. Places the order on Kalshi (or logs it in paper mode)

The AI never writes files. All stats are computed deterministically in Rust from an append-only markdown ledger.

## Setup

### Prerequisites

- Rust toolchain (stable)
- Kalshi account with API access + RSA key pair
- OpenRouter API key
- (Optional) Telegram bot token for alerts

### Configure

Create a `.env` file:

```bash
# Kalshi
KALSHI_API_KEY_ID=your-api-key-uuid
KALSHI_PRIVATE_KEY_PATH=./kalshi_private_key.pem
KALSHI_BASE_URL=https://api.elections.kalshi.com
KALSHI_SERIES_TICKER=KXBTC15M

# AI (OpenRouter)
OPENROUTER_API_KEY=sk-or-v1-...

# Telegram (optional)
# TELEGRAM_BOT_TOKEN=
# TELEGRAM_CHAT_ID=

# Safety — paper trading by default
PAPER_TRADE=true
CONFIRM_LIVE=false
```

### Build

```bash
cargo build --release
```

## Running

### Paper trading (no real orders)

```bash
./run.sh
```

This runs the bot in a 5-minute loop. Paper mode is the default — set `PAPER_TRADE=true` in `.env` (or just don't change it).

### Live trading (real money)

Set both flags in `.env`:

```bash
PAPER_TRADE=false
CONFIRM_LIVE=true
```

Then start the loop:

```bash
./run.sh
```

### systemd (run forever)

Copy the included service file and enable it:

```bash
sudo cp kalshi-bot.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now kalshi-bot
```

Check status:

```bash
sudo systemctl status kalshi-bot
journalctl -u kalshi-bot -f
```

## Risk Limits

All hardcoded — no config knobs to accidentally blow up:

| Limit | Value | What It Does |
|-------|-------|--------------|
| Max shares per trade | 5 | Position size cap |
| Max daily loss | $10 | Stop trading for the day |
| Stop loss | 20% | Halt if total P&L drops 20% of starting balance |
| Max consecutive losses | 7 | Stop trading until a win |
| Min balance | $5 | Don't trade below this floor |
| Min time to expiry | 2 min | Don't enter dying markets |

## How the AI Decides

Claude gets a full context package each cycle:

- **Market data**: yes/no bid/ask, last price, volume, open interest
- **Orderbook**: full depth on both sides
- **BTC price data**: spot, 15m/1h momentum, SMA, volatility, recent candles (from Binance US)
- **Performance**: win rate, streak, P&L, max drawdown
- **Trade history**: last 20 trades with outcomes

The system prompt (`brain/prompt.md`) teaches Claude to evaluate asymmetric risk/reward, find mispricings between BTC momentum and Kalshi implied probability, and size positions based on edge magnitude (5-9pt edge = 1-2 shares, 10-15pt = 3, 15+ = 4-5).

One Opus call per cycle. If Claude returns garbage JSON, the bot does nothing.

## Early Exit

If the bot already holds a position, it asks Claude whether to **sell** or **hold**:

- **Sell to take profit** — momentum reversing, lock in gains
- **Sell to cut losses** — momentum flipped, exit at a small loss instead of riding to expiry
- **Hold** — momentum still supports the position

Selling means placing a sell order on the same side (not buying the opposite side). This closes the position without extra capital.

## Project Structure

```
kalshi-bot/
├── src/
│   ├── main.rs                    # Entry point, config, lockfile
│   ├── safety.rs                  # Lockfile, startup validation, live-mode gate
│   ├── storage.rs                 # Read/write brain/*.md files
│   ├── core/
│   │   ├── engine.rs              # The trading cycle
│   │   ├── risk.rs                # Pure risk checks
│   │   ├── stats.rs               # Compute stats from ledger
│   │   ├── indicators.rs          # BTC technical indicators
│   │   └── types.rs               # All domain types
│   ├── ports/
│   │   ├── exchange.rs            # Exchange trait
│   │   ├── brain.rs               # Brain trait
│   │   └── price_feed.rs          # Price feed trait
│   └── adapters/
│       ├── kalshi/                 # Kalshi API + RSA-PSS auth
│       ├── openrouter.rs          # Claude via OpenRouter
│       └── binance.rs             # BTC price from Binance US
├── brain/
│   ├── prompt.md                  # System prompt (you edit, AI reads)
│   ├── ledger.md                  # Append-only trade log
│   └── stats.md                   # Computed performance stats
├── run.sh                         # 5-minute loop runner
├── kalshi-bot.service             # systemd unit file
└── logs/
    └── cron.log                   # Bot output
```

## Cost

~$0.05 per cycle via OpenRouter. At one cycle every 5 minutes, that's roughly 288 cycles/day = ~$14/day if it runs 24/7. In practice it PASSes on many cycles and markets aren't always open, so real cost is lower.

## License

MIT
