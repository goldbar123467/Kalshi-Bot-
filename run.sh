#!/usr/bin/env bash
# Runs kalshi-bot at 20 seconds past each 15-minute mark (:00:20, :15:20, :30:20, :45:20).
# Usage: nohup ./run.sh &  (or run via systemd)

cd "$(dirname "$0")"

# Clear any stale env vars so dotenv .env takes precedence
unset OPENROUTER_API_KEY 2>/dev/null

export RUST_LOG=info
OFFSET=20   # seconds after each 15-min boundary
LOG="logs/cron.log"

echo "[$(date -u +%FT%TZ)] Bot loop started (synced to :XX:${OFFSET})" >> "$LOG"

while true; do
    # Sleep until 20s past the next 15-minute mark
    now=$(date +%s)
    secs_past=$(( (now - OFFSET) % 900 ))
    sleep_secs=$(( 900 - secs_past ))
    sleep "$sleep_secs"

    ./target/release/kalshi-bot >> "$LOG" 2>&1 || true
done
