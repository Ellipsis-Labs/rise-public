//! Mark-price history types for Phoenix API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query parameters for `GET /v1/prices/{symbol}/history`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriceHistoryQuery {
    /// Start time (defaults to one hour ago).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    /// End time (defaults to now).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    /// Maximum number of price points (default 1000, max 10000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Aggregation timeframe (e.g. "1m", "1h", "1d").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
}

impl PriceHistoryQuery {
    /// Creates an empty query using server defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the start time.
    pub fn with_start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Sets the end time.
    pub fn with_end_time(mut self, end_time: DateTime<Utc>) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Sets the maximum number of price points.
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the aggregation timeframe.
    pub fn with_timeframe(mut self, timeframe: impl Into<String>) -> Self {
        self.timeframe = Some(timeframe.into());
        self
    }
}

/// Response for the price history endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistoryResponse {
    pub market_id: i64,
    pub symbol: String,
    pub timeframe: Option<String>,
    pub prices: Vec<PricePoint>,
}

/// One price data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub mark_price: f64,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: i64,
    pub exchange_spot_price: Option<f64>,
    pub exchange_perp_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub last_trade: Option<f64>,
    pub mid_spot_diff_ema_ticks: Option<i64>,
    pub cumulative_funding_rate: Option<i64>,
    pub settled_contribution: Option<i64>,
    pub interval_accumulator: Option<i128>,
}
