//! Market types for Phoenix API.
//!
//! These types represent market configuration, status, orderbook data,
//! statistics, and market views from both WebSocket and HTTP APIs.

use serde::{Deserialize, Serialize};

use crate::types::core::{Decimal, Price};

// ============================================================================
// Market Configuration
// ============================================================================

/// Market unit configuration.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketUnitConfig {
    pub tick_size_in_quote_lots_per_base_lot: u64,
    pub base_lots_decimals: i8,
}

/// Market fee configuration.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketFeeConfig {
    pub taker_fee_micro: u32,
    pub maker_fee_micro: i32,
}

/// Leverage tier for a market.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageTier {
    pub max_leverage: f64,
    pub max_size_base_lots: u64,
    pub limit_order_risk_factor: u16,
}

/// Risk factors for a market.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactors {
    pub maintenance: u16,
    pub backstop: u16,
    pub high_risk: u16,
    pub upnl: u16,
    pub upnl_for_withdrawals: u16,
    pub cancel_order: u16,
}

// ============================================================================
// Market Status Enums
// ============================================================================

/// Market status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketStatus {
    #[default]
    Uninitialized,
    Active,
    PostOnly,
    Paused,
    Closed,
    Tombstoned,
}

/// Risk state enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskState {
    Healthy,
    Unhealthy,
    Underwater,
    ZeroCollateralNoPositions,
}

/// Risk tier enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskTier {
    Safe,
    AtRisk,
    Cancellable,
    Liquidatable,
    BackstopLiquidatable,
    HighRisk,
}

// ============================================================================
// Orderbook Types
// ============================================================================

/// L2 orderbook (HTTP API response format).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L2Orderbook {
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub mid: Option<f64>,
}

/// HTTP orderbook response returned by `/market/{symbol}/orderbook`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookView {
    pub slot: u64,
    pub symbol: String,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub mid: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splines: Option<serde_json::Value>,
}

/// L2 orderbook update from the WebSocket server.
///
/// Contains bid and ask levels for a specific market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L2BookUpdate {
    /// Market symbol (e.g., "SOL")
    pub symbol: String,
    /// The orderbook data
    pub orderbook: L2Orderbook,
    /// True when the snapshot bypasses the commodities after-hours
    /// execution-price-band filter. Clients that subscribe to both
    /// modes for the same symbol use this to route messages.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bypass_execution_band: bool,
}

// ============================================================================
// Market Statistics
// ============================================================================

/// Market statistics update from the WebSocket server.
///
/// Contains real-time pricing and market data for a specific perpetual market.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStatsUpdate {
    /// Market symbol (e.g., "SOL")
    pub symbol: String,
    /// Total open interest in the market
    #[serde(rename = "openInterest")]
    pub open_interest: f64,
    /// Current mark price
    #[serde(rename = "markPx")]
    pub mark_price: f64,
    /// Current mid price
    #[serde(rename = "midPx")]
    pub mid_price: f64,
    /// Current oracle price
    #[serde(rename = "oraclePx")]
    pub oracle_price: f64,
    /// Mark price from 24 hours ago
    #[serde(rename = "prevDayPx")]
    pub prev_day_mark_price: f64,
    /// 24-hour notional trading volume in USD
    #[serde(rename = "dayNtlVlm")]
    pub day_volume_usd: f64,
    /// Current funding rate
    #[serde(rename = "funding")]
    pub funding_rate: f64,
}

/// Current mark price response returned by `/v1/market/{symbol}/mark-price`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkPriceResponse {
    pub slot: u64,
    pub slot_index: u32,
    pub symbol: String,
    pub mark_price: Option<Price>,
}

// ============================================================================
// Market Views (HTTP API)
// ============================================================================

/// Full market information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketInfo {
    pub symbol: String,
    pub asset_id: u64,
    pub market_status: MarketStatus,
    pub market_key: String,
    pub units: MarketUnitConfig,
    pub fees: MarketFeeConfig,
    pub risk_action_price_validity_rules: [[[u8; 8]; 4]; 8],

    pub open_interest: Decimal,
    pub leverage_tiers: Vec<LeverageTier>,

    pub risk_factors: RiskFactors,

    pub spot_price: Option<Price>,
    pub mark_price: Option<Price>,

    pub funding_interval_seconds: u64,
    pub funding_period_seconds: u64,
    pub funding_start_interval_timestamp: u64,
    pub cumulative_funding_rate: i64,
    pub max_funding_rate_per_interval: i64,

    pub current_funding_rate_percentage: f64,
    pub annualized_funding_rate_percentage: f64,

    pub l2_orderbook: L2Orderbook,
}

/// Market snapshot response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketView {
    pub slot: u64,
    pub market: MarketInfo,
}

/// Summary market information.
/// This is a simpler structure than `MarketInfo` without orderbook/price data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSummary {
    pub symbol: String,
    pub asset_id: u64,
    pub market_status: MarketStatus,
    pub units: MarketUnitConfig,
    pub fees: MarketFeeConfig,
    pub open_interest: Decimal,
    pub open_interest_cap: Decimal,
    pub leverage_tiers: Vec<LeverageTier>,
    pub funding_interval_in_slots: u64,
    pub funding_period_in_slots: u64,
    pub funding_start_interval_slot: u64,
    pub cumulative_funding_rate: i64,
    pub max_liquidation_size: Decimal,
    pub risk_factors: RiskFactors,
    pub isolated_only: bool,
}

/// Markets snapshot response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketsView {
    pub slot: u64,
    pub markets: Vec<MarketSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_l2_book_update() {
        let json = r#"{
            "symbol": "SOL",
            "orderbook": {
                "bids": [[150.25, 100.0], [150.20, 200.0], [150.15, 300.0]],
                "asks": [[150.30, 150.0], [150.35, 250.0], [150.40, 400.0]],
                "mid": 150.275
            }
        }"#;

        let update: L2BookUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.symbol, "SOL");
        assert_eq!(update.orderbook.bids.len(), 3);
        assert_eq!(update.orderbook.asks.len(), 3);
        assert_eq!(update.orderbook.bids[0], (150.25, 100.0));
        assert_eq!(update.orderbook.asks[0], (150.30, 150.0));
        assert_eq!(update.orderbook.mid, Some(150.275));
    }

    #[test]
    fn test_serialize_l2_book_update() {
        let update = L2BookUpdate {
            symbol: "BTC".to_string(),
            orderbook: L2Orderbook {
                bids: vec![(65000.0, 1.5), (64990.0, 2.0)],
                asks: vec![(65010.0, 1.0), (65020.0, 2.5)],
                mid: Some(65005.0),
            },
            bypass_execution_band: false,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("\"symbol\":\"BTC\""));
        assert!(json.contains("\"orderbook\""));
        assert!(json.contains("\"mid\":65005"));
    }

    #[test]
    fn test_deserialize_market_stats_update() {
        let json = r#"{
            "symbol": "SOL",
            "openInterest": 367.51,
            "markPx": 97.35,
            "midPx": 97.315,
            "oraclePx": 97.4,
            "prevDayPx": 104.14,
            "dayNtlVlm": 243491.15,
            "funding": -0.00014956533318115242
        }"#;

        let update: MarketStatsUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.symbol, "SOL");
        assert_eq!(update.mark_price, 97.35);
        assert_eq!(update.mid_price, 97.315);
        assert_eq!(update.oracle_price, 97.4);
        assert_eq!(update.prev_day_mark_price, 104.14);
    }

    #[test]
    fn test_serialize_market_stats_update() {
        let update = MarketStatsUpdate {
            symbol: "BTC".to_string(),
            open_interest: 500000.0,
            mark_price: 65000.0,
            mid_price: 64995.0,
            oracle_price: 64990.0,
            prev_day_mark_price: 64000.0,
            day_volume_usd: 1000000000.0,
            funding_rate: 0.00005,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("\"symbol\":\"BTC\""));
        assert!(json.contains("\"markPx\":65000"));
        assert!(json.contains("\"oraclePx\":64990"));
    }

    #[test]
    fn test_deserialize_market_status() {
        let json = r#""active""#;
        let status: MarketStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, MarketStatus::Active);

        let json = r#""postOnly""#;
        let status: MarketStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, MarketStatus::PostOnly);
    }

    #[test]
    fn test_deserialize_markets_view() {
        let json = r#"{"slot":396561188,"markets":[{"symbol":"ETH","assetId":2,"marketStatus":"active","units":{"tickSizeInQuoteLotsPerBaseLot":100,"baseLotsDecimals":3},"fees":{"takerFeeMicro":200,"makerFeeMicro":0},"openInterest":{"value":1392,"decimals":3,"ui":"1.392"},"openInterestCap":{"value":312000000,"decimals":3,"ui":"312000.000"},"leverageTiers":[{"maxLeverage":20.0,"maxSizeBaseLots":8000,"limitOrderRiskFactor":6000}],"fundingIntervalInSlots":3600,"fundingPeriodInSlots":86400,"fundingStartIntervalSlot":1769634000,"cumulativeFundingRate":901,"maxLiquidationSize":{"value":20000,"decimals":3,"ui":"20.000"},"riskFactors":{"maintenance":5000,"backstop":2000,"highRisk":1000,"upnl":10000,"upnlForWithdrawals":100,"cancelOrder":7000},"isolatedOnly":false}]}"#;

        let view: MarketsView = serde_json::from_str(json).unwrap();
        assert_eq!(view.slot, 396561188);
        assert_eq!(view.markets.len(), 1);
        assert_eq!(view.markets[0].symbol, "ETH");
        assert_eq!(view.markets[0].asset_id, 2);
        assert_eq!(view.markets[0].market_status, MarketStatus::Active);
        assert_eq!(
            view.markets[0].leverage_tiers[0].limit_order_risk_factor,
            6000
        );
        assert_eq!(view.markets[0].risk_factors.maintenance, 5000);
        assert_eq!(view.markets[0].risk_factors.upnl_for_withdrawals, 100);
        assert!(!view.markets[0].isolated_only);
    }
}
