use crate::core::types::{Config, Stats};

pub fn check(
    stats: &Stats,
    balance_cents: u64,
    config: &Config,
) -> Option<String> {
    if balance_cents < config.min_balance_cents {
        return Some(format!(
            "Balance {}¢ < {}¢ minimum",
            balance_cents, config.min_balance_cents
        ));
    }
    // 20% stop loss: if total P&L has dropped >= stop_loss_pct of starting balance, halt
    let starting_balance = balance_cents as i64 - stats.total_pnl_cents;
    if starting_balance > 0 {
        let max_loss = (starting_balance as f64 * config.stop_loss_pct) as i64;
        if stats.total_pnl_cents <= -max_loss {
            return Some(format!(
                "Stop loss: P&L {}¢ exceeds {:.0}% of starting balance ({}¢)",
                stats.total_pnl_cents, config.stop_loss_pct * 100.0, starting_balance
            ));
        }
    }
    if stats.today_pnl_cents <= -config.max_daily_loss_cents {
        return Some(format!("Daily loss: {}¢", stats.today_pnl_cents));
    }
    if stats.current_streak <= -(config.max_consecutive_losses as i32) {
        return Some(format!(
            "{}× consecutive losses",
            stats.current_streak.abs()
        ));
    }
    None
}
