//! HTTP API types for funding history routes.

use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query parameters for `/v1/users/{user_pubkey}/funding-hourly`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHourlyQuery {
    /// Market symbol to filter by, for example `SOL-PERP`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Maximum number of hourly rows to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Opaque pagination cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Trader PDA index, defaulting to 0 server-side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<i32>,
}

impl FundingHourlyQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    pub fn with_trader_pda_index(mut self, trader_pda_index: i32) -> Self {
        self.trader_pda_index = Some(trader_pda_index);
        self
    }
}

/// Individual hourly funding accrual entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHourlyEvent {
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub funding_payment: String,
    pub funding_rate_percentage: String,
    pub position_size: String,
    pub position_side: String,
}

/// Response containing hourly funding accruals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHourlyHistoryResponse {
    pub events: Vec<FundingHourlyEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// Query parameters for `/v1/funding/{symbol}/rates`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRateHistoryQuery {
    /// Start time in milliseconds since Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    /// End time in milliseconds since Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    /// Maximum number of funding rate points to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

impl FundingRateHistoryQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    pub fn with_end_time(mut self, end_time: i64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Individual funding rate data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRatePoint {
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub funding_rate_percentage: String,
}

/// Response containing historical funding rates for one market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRateHistoryResponse {
    pub market_id: i64,
    pub symbol: String,
    pub rates: Vec<FundingRatePoint>,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;

    #[test]
    fn funding_hourly_query_serializes_camel_case() {
        let query = FundingHourlyQuery::new()
            .with_symbol("SOL-PERP")
            .with_limit(24)
            .with_cursor("opaque")
            .with_trader_pda_index(2);

        let encoded = serde_urlencoded::to_string(query).unwrap();
        assert!(encoded.contains("symbol=SOL-PERP"));
        assert!(encoded.contains("limit=24"));
        assert!(encoded.contains("cursor=opaque"));
        assert!(encoded.contains("traderPdaIndex=2"));
    }

    #[test]
    fn funding_rate_query_serializes_camel_case() {
        let query = FundingRateHistoryQuery::new()
            .with_start_time(1_700_000_000_000)
            .with_end_time(1_700_086_400_000)
            .with_limit(500);

        assert_eq!(
            serde_json::to_value(query).unwrap(),
            json!({
                "startTime": 1_700_000_000_000_i64,
                "endTime": 1_700_086_400_000_i64,
                "limit": 500
            })
        );
    }

    #[test]
    fn funding_rate_point_uses_second_timestamps() {
        let raw = r#"{"timestamp":1700000000,"fundingRatePercentage":"0.001"}"#;
        let point: FundingRatePoint = serde_json::from_str(raw).unwrap();

        assert_eq!(
            point.timestamp,
            Utc.timestamp_opt(1_700_000_000, 0).unwrap()
        );
        assert_eq!(
            serde_json::to_value(point).unwrap(),
            json!({"timestamp": 1_700_000_000, "fundingRatePercentage": "0.001"})
        );
    }
}
