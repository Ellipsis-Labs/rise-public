//! Candle types for Phoenix API.
//!
//! These types represent candlestick (OHLCV) data streamed via WebSocket.

use std::fmt::{self, Display};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Timeframe enumeration for candlestick data aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    #[serde(rename = "1s")]
    Second1,
    #[serde(rename = "5s")]
    Second5,
    #[serde(rename = "1m")]
    Minute1,
    #[serde(rename = "5m")]
    Minute5,
    #[serde(rename = "15m")]
    Minute15,
    #[serde(rename = "30m")]
    Minute30,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "4h")]
    Hour4,
    #[serde(rename = "1d")]
    Day1,
}

impl Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Timeframe::Second1 => write!(f, "1s"),
            Timeframe::Second5 => write!(f, "5s"),
            Timeframe::Minute1 => write!(f, "1m"),
            Timeframe::Minute5 => write!(f, "5m"),
            Timeframe::Minute15 => write!(f, "15m"),
            Timeframe::Minute30 => write!(f, "30m"),
            Timeframe::Hour1 => write!(f, "1h"),
            Timeframe::Hour4 => write!(f, "4h"),
            Timeframe::Day1 => write!(f, "1d"),
        }
    }
}

impl FromStr for Timeframe {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1s" => Ok(Timeframe::Second1),
            "5s" => Ok(Timeframe::Second5),
            "1m" => Ok(Timeframe::Minute1),
            "5m" => Ok(Timeframe::Minute5),
            "15m" => Ok(Timeframe::Minute15),
            "30m" => Ok(Timeframe::Minute30),
            "1h" => Ok(Timeframe::Hour1),
            "4h" => Ok(Timeframe::Hour4),
            "1d" => Ok(Timeframe::Day1),
            _ => Err(format!("Unknown timeframe: {s}")),
        }
    }
}

/// API Candle structure (Trading View Bar interface).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCandle {
    /// Time in seconds since Unix epoch (UTC).
    pub time: i64,
    pub low: f64,
    pub high: f64,
    pub open: f64,
    pub close: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_count: Option<u64>,
}

/// Candle data with symbol and timeframe metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleData {
    pub candle: ApiCandle,
    pub symbol: String,
    pub timeframe: String,
}

/// Query parameters for the candles endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CandlesQueryParams {
    /// Trading symbol (e.g., "SOL", "BTC", "ETH").
    pub symbol: String,
    /// Candle timeframe (e.g., "1m", "5m", "1h", "1d").
    pub timeframe: String,
    /// Start time in milliseconds since Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    /// End time in milliseconds since Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    /// Maximum number of candles to return (default: 2500).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl CandlesQueryParams {
    /// Creates a new query with the symbol and timeframe.
    pub fn new(symbol: impl Into<String>, timeframe: Timeframe) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: timeframe.to_string(),
            ..Default::default()
        }
    }

    /// Sets the start time.
    pub fn with_start_time(mut self, start_time_ms: i64) -> Self {
        self.start_time = Some(start_time_ms);
        self
    }

    /// Sets the end time.
    pub fn with_end_time(mut self, end_time_ms: i64) -> Self {
        self.end_time = Some(end_time_ms);
        self
    }

    /// Sets the limit.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeframe_display() {
        assert_eq!(Timeframe::Minute1.to_string(), "1m");
        assert_eq!(Timeframe::Hour4.to_string(), "4h");
        assert_eq!(Timeframe::Day1.to_string(), "1d");
    }

    #[test]
    fn test_timeframe_from_str() {
        assert_eq!("1m".parse::<Timeframe>().unwrap(), Timeframe::Minute1);
        assert_eq!("4h".parse::<Timeframe>().unwrap(), Timeframe::Hour4);
        assert_eq!("1d".parse::<Timeframe>().unwrap(), Timeframe::Day1);
        assert!("invalid".parse::<Timeframe>().is_err());
    }

    #[test]
    fn test_timeframe_serde() {
        let tf = Timeframe::Minute5;
        let json = serde_json::to_string(&tf).unwrap();
        assert_eq!(json, r#""5m""#);

        let parsed: Timeframe = serde_json::from_str(r#""5m""#).unwrap();
        assert_eq!(parsed, Timeframe::Minute5);
    }

    #[test]
    fn test_deserialize_candle_data() {
        let json = r#"{
            "candle": {
                "time": 1727181985,
                "low": 149.80,
                "high": 151.50,
                "open": 150.25,
                "close": 150.90,
                "volume": 1234.56,
                "tradeCount": 89
            },
            "symbol": "SOL",
            "timeframe": "1m"
        }"#;

        let data: CandleData = serde_json::from_str(json).unwrap();
        assert_eq!(data.symbol, "SOL");
        assert_eq!(data.timeframe, "1m");
        assert_eq!(data.candle.time, 1727181985);
        assert_eq!(data.candle.open, 150.25);
        assert_eq!(data.candle.close, 150.90);
        assert_eq!(data.candle.volume, Some(1234.56));
        assert_eq!(data.candle.trade_count, Some(89));
    }
}
