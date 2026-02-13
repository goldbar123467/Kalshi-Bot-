#!/usr/bin/env bash
# Two-shot pattern: runs twice per 15-min market, then sleeps until the next one.
#   Shot 1 — :XX:20  → enter a trade if there's edge
#   Shot 2 — :XX:65  → manage position (sell/hold) 45s later
# Max 2 API calls per market. Usage: systemd or nohup ./run.sh &

cd "$(dirname "$0")"

# Clear any stale env vars so dotenv .env takes precedence
unset OPENROUTER_API_KEY 2>/dev/null

export RUST_LOG=info
OFFSET=20   # seconds after each 15-min boundary
FOLLOWUP=45 # seconds between shot 1 and shot 2
LOG="logs/cron.log"

echo "[$(date -u +%FT%TZ)] Bot loop started (two-shot at :XX:${OFFSET} + ${FOLLOWUP}s)" >> "$LOG"

while true; do
    # Sleep until 20s past the next 15-minute mark
    now=$(date +%s)
    secs_past=$(( (now - OFFSET) % 900 ))
    sleep_secs=$(( 900 - secs_past ))
    sleep "$sleep_secs"

    # Shot 1: enter trade
    ./target/release/kalshi-bot >> "$LOG" 2>&1 || true

    # Shot 2: manage position (sell/hold check)
    sleep "$FOLLOWUP"
    ./target/release/kalshi-bot >> "$LOG" 2>&1 || true
done
