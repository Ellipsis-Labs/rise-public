//! Combined market state container for Phoenix markets.

use crate::l2book::L2Book;
use crate::market::{L2BookUpdate, MarketStatsUpdate};
use crate::market_stats::MarketStats;
use crate::ws::ServerMessage;

/// Combined container for market state including statistics and orderbook.
#[derive(Debug, Clone)]
pub struct Market {
    stats: MarketStats,
    book: L2Book,
}

impl Market {
    pub fn new(stats_symbol: String, orderbook_symbol: String) -> Self {
        Self {
            stats: MarketStats::new(stats_symbol),
            book: L2Book::new(orderbook_symbol),
        }
    }

    pub fn from_symbol(symbol: String) -> Self {
        Self::new(symbol.clone(), symbol)
    }

    pub fn apply_server_message(&mut self, msg: &ServerMessage) -> &mut Self {
        match msg {
            ServerMessage::Market(update) => self.apply_market_stats_update(update),
            ServerMessage::Orderbook(update) => self.apply_l2_book_update(update),
            _ => self,
        }
    }

    pub fn apply_market_stats_update(&mut self, msg: &MarketStatsUpdate) -> &mut Self {
        self.stats.apply_update(msg);
        self
    }

    pub fn apply_l2_book_update(&mut self, msg: &L2BookUpdate) -> &mut Self {
        self.book.apply_update(msg);
        self
    }

    pub fn stats(&self) -> &MarketStats {
        &self.stats
    }

    pub fn stats_mut(&mut self) -> &mut MarketStats {
        &mut self.stats
    }

    pub fn book(&self) -> &L2Book {
        &self.book
    }

    pub fn book_mut(&mut self) -> &mut L2Book {
        &mut self.book
    }

    pub fn symbol(&self) -> &str {
        self.stats.symbol()
    }

    pub fn orderbook_symbol(&self) -> &str {
        self.book.symbol()
    }

    pub fn mark_price(&self) -> Option<f64> {
        self.stats.mark_price()
    }

    pub fn oracle_price(&self) -> Option<f64> {
        self.stats.oracle_price()
    }

    pub fn open_interest(&self) -> Option<f64> {
        self.stats.open_interest()
    }

    pub fn funding_rate(&self) -> Option<f64> {
        self.stats.funding_rate()
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.book.best_bid()
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.book.best_ask()
    }

    pub fn spread(&self) -> Option<f64> {
        self.book.spread()
    }

    pub fn mid_price(&self) -> Option<f64> {
        self.book.mid_price()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::L2Orderbook;

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

    fn make_l2_update(symbol: &str) -> L2BookUpdate {
        L2BookUpdate {
            symbol: symbol.to_string(),
            orderbook: L2Orderbook {
                bids: vec![(150.25, 100.0), (150.20, 200.0), (150.15, 300.0)],
                asks: vec![(150.30, 150.0), (150.35, 250.0), (150.40, 400.0)],
                mid: Some(150.275),
            },
            bypass_execution_band: false,
        }
    }

    #[test]
    fn test_new_market() {
        let market = Market::new("SOL".to_string(), "SOL".to_string());
        assert_eq!(market.symbol(), "SOL");
        assert_eq!(market.orderbook_symbol(), "SOL");
        assert!(market.mark_price().is_none());
        assert!(market.best_bid().is_none());
    }

    #[test]
    fn test_from_symbol() {
        let market = Market::from_symbol("SOL".to_string());
        assert_eq!(market.symbol(), "SOL");
        assert_eq!(market.orderbook_symbol(), "SOL");
    }

    #[test]
    fn test_apply_server_message_market_stats() {
        let mut market = Market::new("SOL".to_string(), "SOL".to_string());
        let update = make_stats_update("SOL", 150.0);
        let msg = ServerMessage::Market(update);

        market.apply_server_message(&msg);

        assert_eq!(market.mark_price(), Some(150.0));
        assert_eq!(market.oracle_price(), Some(149.95));
    }

    #[test]
    fn test_apply_server_message_l2_book() {
        let mut market = Market::new("SOL".to_string(), "SOL".to_string());
        let update = make_l2_update("SOL");
        let msg = ServerMessage::Orderbook(update);

        market.apply_server_message(&msg);

        assert_eq!(market.best_bid(), Some(150.25));
        assert_eq!(market.best_ask(), Some(150.30));
    }

    #[test]
    fn test_apply_both_updates() {
        let mut market = Market::new("SOL".to_string(), "SOL".to_string());

        let stats_update = make_stats_update("SOL", 150.0);
        let l2_update = make_l2_update("SOL");

        market
            .apply_server_message(&ServerMessage::Market(stats_update))
            .apply_server_message(&ServerMessage::Orderbook(l2_update));

        assert_eq!(market.mark_price(), Some(150.0));
        assert!(market.open_interest().is_some());
        assert_eq!(market.best_bid(), Some(150.25));
        assert!(market.spread().is_some());
    }

    #[test]
    fn test_ignore_wrong_symbol_and_coin() {
        let mut market = Market::new("SOL".to_string(), "SOL".to_string());

        let wrong_stats = make_stats_update("BTC", 65000.0);
        let wrong_l2 = make_l2_update("BTC");

        market
            .apply_server_message(&ServerMessage::Market(wrong_stats))
            .apply_server_message(&ServerMessage::Orderbook(wrong_l2));

        assert!(market.mark_price().is_none());
        assert!(market.best_bid().is_none());
    }

    #[test]
    fn test_access_inner_containers() {
        let mut market = Market::new("SOL".to_string(), "SOL".to_string());
        let l2_update = make_l2_update("SOL");

        market.apply_l2_book_update(&l2_update);

        assert_eq!(market.book().bid_depth(), 3);
        assert_eq!(market.book().ask_depth(), 3);
        assert_eq!(market.stats().symbol(), "SOL");
    }
}
