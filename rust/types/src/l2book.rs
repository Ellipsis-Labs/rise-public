//! L2 orderbook state container for Phoenix markets.

use crate::market::L2BookUpdate;

/// A single price level in the orderbook.
#[derive(Debug, Clone, Copy)]
pub struct PriceLevel {
    /// Price at this level
    pub price: f64,
    /// Total quantity at this level
    pub quantity: f64,
}

/// Container for L2 orderbook data.
#[derive(Debug, Clone)]
pub struct L2Book {
    symbol: String,
    data: Option<L2BookUpdate>,
}

impl L2Book {
    pub fn new(symbol: String) -> Self {
        Self { symbol, data: None }
    }

    pub fn apply_update(&mut self, msg: &L2BookUpdate) {
        if msg.symbol != self.symbol {
            return;
        }
        self.data = Some(msg.clone());
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn raw(&self) -> Option<&L2BookUpdate> {
        self.data.as_ref()
    }

    pub fn bids(&self) -> Vec<PriceLevel> {
        self.data
            .as_ref()
            .map(|d| {
                d.orderbook
                    .bids
                    .iter()
                    .map(|(price, quantity)| PriceLevel {
                        price: *price,
                        quantity: *quantity,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn asks(&self) -> Vec<PriceLevel> {
        self.data
            .as_ref()
            .map(|d| {
                d.orderbook
                    .asks
                    .iter()
                    .map(|(price, quantity)| PriceLevel {
                        price: *price,
                        quantity: *quantity,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.data
            .as_ref()
            .and_then(|d| d.orderbook.bids.first().map(|(p, _)| *p))
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.data
            .as_ref()
            .and_then(|d| d.orderbook.asks.first().map(|(p, _)| *p))
    }

    pub fn best_bid_quantity(&self) -> Option<f64> {
        self.data
            .as_ref()
            .and_then(|d| d.orderbook.bids.first().map(|(_, q)| *q))
    }

    pub fn best_ask_quantity(&self) -> Option<f64> {
        self.data
            .as_ref()
            .and_then(|d| d.orderbook.asks.first().map(|(_, q)| *q))
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<f64> {
        if let Some(mid) = self.data.as_ref().and_then(|d| d.orderbook.mid) {
            return Some(mid);
        }
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    pub fn spread_percent(&self) -> Option<f64> {
        match (self.spread(), self.mid_price()) {
            (Some(spread), Some(mid)) if mid != 0.0 => Some(spread / mid * 100.0),
            _ => None,
        }
    }

    pub fn total_bid_liquidity(&self) -> f64 {
        self.data
            .as_ref()
            .map(|d| d.orderbook.bids.iter().map(|(_, q)| q).sum())
            .unwrap_or(0.0)
    }

    pub fn total_ask_liquidity(&self) -> f64 {
        self.data
            .as_ref()
            .map(|d| d.orderbook.asks.iter().map(|(_, q)| q).sum())
            .unwrap_or(0.0)
    }

    pub fn bid_depth(&self) -> usize {
        self.data
            .as_ref()
            .map(|d| d.orderbook.bids.len())
            .unwrap_or(0)
    }

    pub fn ask_depth(&self) -> usize {
        self.data
            .as_ref()
            .map(|d| d.orderbook.asks.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::L2Orderbook;

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
    fn test_new_book() {
        let book = L2Book::new("SOL".to_string());
        assert_eq!(book.symbol(), "SOL");
        assert!(book.raw().is_none());
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn test_apply_update() {
        let mut book = L2Book::new("SOL".to_string());
        let update = make_l2_update("SOL");

        book.apply_update(&update);

        assert!(book.raw().is_some());
        assert_eq!(book.best_bid(), Some(150.25));
        assert_eq!(book.best_ask(), Some(150.30));
    }

    #[test]
    fn test_ignore_wrong_symbol() {
        let mut book = L2Book::new("SOL".to_string());
        let update = make_l2_update("BTC");

        book.apply_update(&update);

        assert!(book.raw().is_none());
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn test_spread_and_mid_price() {
        let mut book = L2Book::new("SOL".to_string());
        let update = make_l2_update("SOL");

        book.apply_update(&update);

        let spread = book.spread().unwrap();
        assert!((spread - 0.05).abs() < 0.0001);

        let mid = book.mid_price().unwrap();
        assert!((mid - 150.275).abs() < 0.0001);
    }

    #[test]
    fn test_liquidity() {
        let mut book = L2Book::new("SOL".to_string());
        let update = make_l2_update("SOL");

        book.apply_update(&update);

        assert_eq!(book.total_bid_liquidity(), 600.0);
        assert_eq!(book.total_ask_liquidity(), 800.0);
        assert_eq!(book.bid_depth(), 3);
        assert_eq!(book.ask_depth(), 3);
    }

    #[test]
    fn test_price_levels() {
        let mut book = L2Book::new("SOL".to_string());
        let update = make_l2_update("SOL");

        book.apply_update(&update);

        let bids = book.bids();
        assert_eq!(bids.len(), 3);
        assert_eq!(bids[0].price, 150.25);
        assert_eq!(bids[0].quantity, 100.0);

        let asks = book.asks();
        assert_eq!(asks.len(), 3);
        assert_eq!(asks[0].price, 150.30);
        assert_eq!(asks[0].quantity, 150.0);
    }
}
