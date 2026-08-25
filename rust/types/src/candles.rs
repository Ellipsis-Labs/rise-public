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

impl Timeframe {
    pub fn as_seconds(self) -> i64 {
        match self {
            Timeframe::Minute1 => 60,
            Timeframe::Minute5 => 5 * 60,
            Timeframe::Minute15 => 15 * 60,
            Timeframe::Minute30 => 30 * 60,
            Timeframe::Hour1 => 60 * 60,
            Timeframe::Hour4 => 4 * 60 * 60,
            Timeframe::Day1 => 24 * 60 * 60,
        }
    }
}

/// API Candle structure (Trading View Bar interface).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCandle {
    /// Unix timestamp (UTC). REST candle history uses milliseconds; WebSocket
    /// candle messages use seconds.
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
    /// Maximum number of candles to return (server default and max: 2,500).
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

/// Canonical candle returned by the candles v2 endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCandleV2 {
    /// Candle start timestamp in milliseconds since Unix epoch (UTC).
    pub time: i64,
    pub low: f64,
    pub high: f64,
    pub open: f64,
    pub close: f64,
    pub mark_open: f64,
    pub mark_high: f64,
    pub mark_low: f64,
    pub mark_close: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_quote: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_source: Option<String>,
    /// Whether the bucket is closed and should no longer update.
    pub is_final: bool,
}

/// Pagination metadata returned by candles v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandlesV2Page {
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Canonical paginated candles v2 response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandlesV2Response {
    pub symbol: String,
    pub timeframe: Timeframe,
    /// Echoed inclusive lower bound in milliseconds since Unix epoch.
    pub from: i64,
    /// Echoed exclusive upper bound in milliseconds since Unix epoch.
    pub to: i64,
    pub bars: Vec<ApiCandleV2>,
    pub page: CandlesV2Page,
}

/// Initial explicit-window query for candles v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandlesV2InitialQueryParams {
    /// Trading symbol used in the request path, not the query string.
    #[serde(skip)]
    pub symbol: String,
    pub timeframe: String,
    /// Inclusive lower bound in milliseconds since Unix epoch.
    pub from: i64,
    /// Exclusive upper bound in milliseconds since Unix epoch.
    pub to: i64,
    /// Maximum bars returned in this page (SDK default: 1,000; server max:
    /// 10,000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Include the current open bucket when available (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_partial: Option<bool>,
}

impl CandlesV2InitialQueryParams {
    pub fn new(symbol: impl Into<String>, timeframe: Timeframe, from_ms: i64, to_ms: i64) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: timeframe.to_string(),
            from: from_ms,
            to: to_ms,
            limit: None,
            include_partial: None,
        }
    }

    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_include_partial(mut self, include_partial: bool) -> Self {
        self.include_partial = Some(include_partial);
        self
    }
}

/// Opaque-cursor continuation query for candles v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandlesV2CursorQueryParams {
    /// Trading symbol used in the request path, not the query string.
    #[serde(skip)]
    pub symbol: String,
    pub cursor: String,
}

impl CandlesV2CursorQueryParams {
    pub fn new(symbol: impl Into<String>, cursor: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            cursor: cursor.into(),
        }
    }
}

/// Valid initial or continuation query for candles v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CandlesV2QueryParams {
    Initial(CandlesV2InitialQueryParams),
    Cursor(CandlesV2CursorQueryParams),
}

impl CandlesV2QueryParams {
    pub fn symbol(&self) -> &str {
        match self {
            Self::Initial(params) => &params.symbol,
            Self::Cursor(params) => &params.symbol,
        }
    }
}

impl From<CandlesV2InitialQueryParams> for CandlesV2QueryParams {
    fn from(params: CandlesV2InitialQueryParams) -> Self {
        Self::Initial(params)
    }
}

impl From<CandlesV2CursorQueryParams> for CandlesV2QueryParams {
    fn from(params: CandlesV2CursorQueryParams) -> Self {
        Self::Cursor(params)
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

    #[test]
    fn candles_v2_initial_query_serializes_new_controls_without_symbol() {
        let query = CandlesV2QueryParams::from(
            CandlesV2InitialQueryParams::new("SOL", Timeframe::Minute1, 0, 60_000)
                .with_limit(10_000)
                .with_include_partial(false),
        );

        let encoded = serde_urlencoded::to_string(query).unwrap();

        assert_eq!(
            encoded,
            "timeframe=1m&from=0&to=60000&limit=10000&includePartial=false"
        );
    }

    #[test]
    fn candles_v2_cursor_query_serializes_only_cursor_controls() {
        let query =
            CandlesV2QueryParams::from(CandlesV2CursorQueryParams::new("SOL", "opaque-cursor"));

        let encoded = serde_urlencoded::to_string(query).unwrap();

        assert_eq!(encoded, "cursor=opaque-cursor");
    }

    #[test]
    fn candles_v2_response_preserves_metadata_and_legacy_timestamp_units() {
        let json = r#"{
            "symbol": "SOL",
            "timeframe": "1m",
            "from": 0,
            "to": 60000,
            "bars": [{
                "time": 60000,
                "low": 9.0,
                "high": 12.0,
                "open": 10.0,
                "close": 11.0,
                "markOpen": 10.5,
                "markHigh": 12.5,
                "markLow": 8.5,
                "markClose": 11.5,
                "volume": 5.0,
                "volumeQuote": 55.0,
                "tradeCount": 3,
                "externalSource": "binance",
                "isFinal": true
            }],
            "page": { "hasMore": true, "nextCursor": "next-page" }
        }"#;

        let response: CandlesV2Response = serde_json::from_str(json).unwrap();

        assert_eq!(response.page.next_cursor.as_deref(), Some("next-page"));
        assert!(response.bars[0].is_final);
        assert_eq!(response.bars[0].external_source.as_deref(), Some("binance"));
    }
}
