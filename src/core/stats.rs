use crate::core::types::{LedgerRow, Stats};

pub fn compute(ledger: &[LedgerRow]) -> Stats {
    let done: Vec<&LedgerRow> = ledger
        .iter()
        .filter(|r| r.result == "win" || r.result == "loss")
        .collect();

    let wins = done.iter().filter(|r| r.result == "win").count() as u32;
    let losses = done.iter().filter(|r| r.result == "loss").count() as u32;
    let total = wins + losses;
    let total_pnl: i64 = done.iter().map(|r| r.pnl_cents).sum();

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_pnl: i64 = done
        .iter()
        .filter(|r| r.timestamp.starts_with(&today))
        .map(|r| r.pnl_cents)
        .sum();

    let mut streak: i32 = 0;
    for row in done.iter().rev() {
        let is_win = row.result == "win";
        if streak == 0 {
            streak = if is_win { 1 } else { -1 };
        } else if (streak > 0) == is_win {
            streak += if is_win { 1 } else { -1 };
        } else {
            break;
        }
    }

    let win_pnl: Vec<i64> = done
        .iter()
        .filter(|r| r.result == "win")
        .map(|r| r.pnl_cents)
        .collect();
    let loss_pnl: Vec<i64> = done
        .iter()
        .filter(|r| r.result == "loss")
        .map(|r| r.pnl_cents)
        .collect();

    Stats {
        total_trades: total,
        wins,
        losses,
        win_rate: if total > 0 {
            wins as f64 / total as f64
        } else {
            0.0
        },
        total_pnl_cents: total_pnl,
        today_pnl_cents: today_pnl,
        current_streak: streak,
        max_drawdown_cents: max_drawdown(&done),
        avg_win_cents: if wins > 0 {
            win_pnl.iter().sum::<i64>() as f64 / wins as f64
        } else {
            0.0
        },
        avg_loss_cents: if losses > 0 {
            loss_pnl.iter().sum::<i64>() as f64 / losses as f64
        } else {
            0.0
        },
    }
}

fn max_drawdown(trades: &[&LedgerRow]) -> i64 {
    let mut peak: i64 = 0;
    let mut running: i64 = 0;
    let mut worst: i64 = 0;
    for t in trades {
        running += t.pnl_cents;
        if running > peak {
            peak = running;
        }
        let dd = peak - running;
        if dd > worst {
            worst = dd;
        }
    }
    worst
}
