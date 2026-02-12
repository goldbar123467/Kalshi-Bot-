use crate::core::types::*;

pub fn compute(candles_1m: &[Candle], candles_5m: &[Candle], spot: f64) -> PriceIndicators {
    let pct_change_15m = if !candles_1m.is_empty() {
        let first_open = candles_1m.first().unwrap().open;
        ((spot - first_open) / first_open) * 100.0
    } else {
        0.0
    };

    let pct_change_1h = if !candles_5m.is_empty() {
        let first_open = candles_5m.first().unwrap().open;
        ((spot - first_open) / first_open) * 100.0
    } else {
        0.0
    };

    let momentum = if pct_change_15m > 0.05 {
        MomentumDirection::Up
    } else if pct_change_15m < -0.05 {
        MomentumDirection::Down
    } else {
        MomentumDirection::Flat
    };

    let sma_15m = if !candles_1m.is_empty() {
        candles_1m.iter().map(|c| c.close).sum::<f64>() / candles_1m.len() as f64
    } else {
        spot
    };

    let sma_diff_pct = ((spot - sma_15m) / sma_15m) * 100.0;
    let price_vs_sma = if sma_diff_pct.abs() < 0.01 {
        "at SMA".into()
    } else if sma_diff_pct > 0.0 {
        format!("above +{:.3}%", sma_diff_pct)
    } else {
        format!("below {:.3}%", sma_diff_pct)
    };

    let returns: Vec<f64> = candles_1m
        .windows(2)
        .map(|w| (w[1].close - w[0].close) / w[0].close * 100.0)
        .collect();
    let volatility_1m = if returns.len() >= 2 {
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let last_3_candles: Vec<Candle> = candles_1m
        .iter()
        .rev()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    PriceIndicators {
        spot_price: spot,
        pct_change_15m,
        pct_change_1h,
        momentum,
        sma_15m,
        price_vs_sma,
        volatility_1m,
        last_3_candles,
    }
}
