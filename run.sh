#!/usr/bin/env bash
# Runs kalshi-bot every 45 seconds in a loop.
# Usage: nohup ./run.sh &  (or run inside tmux/screen)

cd "$(dirname "$0")"

# Clear any stale env vars so dotenv .env takes precedence
unset OPENROUTER_API_KEY 2>/dev/null

export RUST_LOG=info
INTERVAL=300
LOG="logs/cron.log"

echo "[$(date -u +%FT%TZ)] Bot loop started (every ${INTERVAL}s)" >> "$LOG"

while true; do
    ./target/release/kalshi-bot >> "$LOG" 2>&1 || true
    sleep "$INTERVAL"
done
