//! Market statistics state container for Phoenix markets.

use crate::types::MarketStatsUpdate;

/// Container for market statistics.
#[derive(Debug, Clone)]
pub struct MarketStats {
    symbol: String,
    stats: Option<MarketStatsUpdate>,
}

impl MarketStats {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            stats: None,
        }
    }

    pub fn apply_update(&mut self, msg: &MarketStatsUpdate) {
        if msg.symbol != self.symbol {
            return;
        }
        self.stats = Some(msg.clone());
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn stats(&self) -> Option<&MarketStatsUpdate> {
        self.stats.as_ref()
    }

    pub fn mark_price(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.mark_price)
    }

    pub fn mid_price(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.mid_price)
    }

    pub fn oracle_price(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.oracle_price)
    }

    pub fn open_interest(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.open_interest)
    }

    pub fn day_volume_usd(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.day_volume_usd)
    }

    pub fn funding_rate(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.funding_rate)
    }

    pub fn prev_day_mark_price(&self) -> Option<f64> {
        self.stats.as_ref().map(|s| s.prev_day_mark_price)
    }

    pub fn price_change_24h_percent(&self) -> Option<f64> {
        self.stats.as_ref().and_then(|s| {
            if s.prev_day_mark_price == 0.0 {
                return None;
            }
            Some((s.mark_price - s.prev_day_mark_price) / s.prev_day_mark_price * 100.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats_update(symbol: &str, mark_price: f64) -> MarketStatsUpdate {
        MarketStatsUpdate {
            symbol: symbol.to_string(),
            open_interest: 1000000.0,
            mark_price,
            mid_price: mark_price - 0.025,
            oracle_price: mark_price - 0.05,
            prev_day_mark_price: mark_price * 0.98,
            day_volume_usd: 50000000.0,
            funding_rate: 0.0001,
        }
    }

    #[test]
    fn test_new_market_stats() {
        let stats = MarketStats::new("SOL".to_string());
        assert_eq!(stats.symbol(), "SOL");
        assert!(stats.stats().is_none());
        assert!(stats.mark_price().is_none());
    }

    #[test]
    fn test_apply_update() {
        let mut stats = MarketStats::new("SOL".to_string());
        let update = make_stats_update("SOL", 150.0);

        stats.apply_update(&update);

        assert!(stats.stats().is_some());
        assert_eq!(stats.mark_price(), Some(150.0));
        assert_eq!(stats.oracle_price(), Some(149.95));
    }

    #[test]
    fn test_ignore_wrong_symbol() {
        let mut stats = MarketStats::new("SOL".to_string());
        let update = make_stats_update("BTC", 65000.0);

        stats.apply_update(&update);

        assert!(stats.stats().is_none());
        assert!(stats.mark_price().is_none());
    }

    #[test]
    fn test_price_change_24h_percent() {
        let mut stats = MarketStats::new("SOL".to_string());
        let update = make_stats_update("SOL", 150.0);

        stats.apply_update(&update);

        let change = stats.price_change_24h_percent().unwrap();
        assert!((change - 2.04).abs() < 0.1);
    }
}
