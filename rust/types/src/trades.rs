//! Trade types for Phoenix WebSocket protocol.
//!
//! These types represent real-time trade events streamed via WebSocket.

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampSeconds, serde_as};

use crate::core::Side;
use crate::js_safe_ints::JsSafeU64;

/// Liquidity role in a trade (maker or taker).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LiquidityRole {
    Maker,
    Taker,
}

/// Type of trade.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeType {
    Limit,
    Market,
    Liquidation,
    Adl,
}

/// Trades message from the trades channel (wrapper with array of events).
///
/// The trades channel sends messages containing the symbol and an array of
/// trade events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradesMessage {
    /// Market symbol (e.g., "SOL").
    pub symbol: String,
    /// Array of trade events.
    pub trades: Vec<TradeEvent>,
}

/// Individual trade event from the trades channel.
///
/// Represents a single trade with price, quantity, and metadata.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeEvent {
    /// Slot when the trade occurred.
    pub slot: JsSafeU64,
    /// Index within the slot.
    pub slot_index: u32,
    /// Timestamp of the trade (unix seconds, string-encoded).
    #[serde_as(as = "TimestampSeconds<String>")]
    pub timestamp: SystemTime,
    /// Market symbol (e.g., "SOL").
    pub symbol: String,
    /// Taker authority pubkey.
    pub taker: String,
    /// Monotonically increasing trade sequence number.
    pub trade_sequence_number: JsSafeU64,
    /// Side of the taker order.
    pub side: Side,
    /// Base lots filled.
    pub base_lots_filled: JsSafeU64,
    /// Quote lots filled.
    pub quote_lots_filled: JsSafeU64,
    /// Fee in quote lots.
    pub fee_in_quote_lots: JsSafeU64,
    /// Human-readable base amount.
    pub base_amount: f64,
    /// Human-readable quote amount.
    pub quote_amount: f64,
    /// Number of fills in this trade.
    pub num_fills: u32,
}

/// Subscription request for the trades channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TradesSubscriptionRequest {
    /// Market symbol to filter trades (e.g., "SOL").
    pub symbol: String,
}

// ============================================================================
// Trade History Types (HTTP API)
// ============================================================================

/// Individual trade record from the `trades-history` and `trades_v2`
/// endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryItem {
    /// User ID
    pub user_id: i64,
    /// Trader ID
    pub trader_id: i64,
    /// Trader PDA index
    pub trader_pda_index: i32,
    /// Subaccount index
    pub subaccount_index: i32,
    /// Market symbol
    pub market_symbol: String,
    /// Transaction signature
    pub signature: Option<String>,
    /// Deterministic UUID v3 derived from the raw fill coordinates.
    #[serde(default)]
    pub fill_id: Option<String>,
    /// Formatted datetime string (ISO 8601).
    pub timestamp: DateTime<Utc>,
    /// Slot coordinates for cursor
    pub slot: i64,
    pub slot_index: i32,
    pub event_index: i32,
    pub instruction_index: i32,
    /// Instruction type (e.g., "PlaceLimitOrder", "PlaceMarketOrder")
    pub instruction_type: String,
    // Position info
    /// Base lots before the trade (human readable)
    pub base_lots_before: String,
    /// Base lots after the trade (human readable)
    pub base_lots_after: String,
    /// Base lots delta (human readable, signed)
    pub base_lots_delta: String,
    /// Virtual quote lots before (human readable)
    pub virtual_quote_lots_before: String,
    /// Virtual quote lots after (human readable)
    pub virtual_quote_lots_after: String,
    /// Virtual quote lots delta (human readable, signed)
    pub virtual_quote_lots_delta: String,
    pub price: String,
    // PnL info
    /// Realized PnL from this trade (human readable USD)
    pub realized_pnl: String,
    pub fees: String,
    pub liquidity: LiquidityRole,
    pub order_sequence_number: Option<i64>,
    pub spline_sequence_number: Option<i64>,
    pub trade_type: TradeType,
}

/// Response from the trade history endpoint.
pub type TradeHistoryResponse = crate::core::PaginatedResponse<Vec<TradeHistoryItem>>;

/// Query parameters for fetching trader trade history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryQueryParams {
    /// PDA index for the trader account.
    #[serde(default, alias = "pdaIndex", alias = "pda_index")]
    pub pda_index: u8,
    /// Optional market symbol filter (e.g., "SOL").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_symbol: Option<String>,
    /// Number of items to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Cursor for pagination (base64-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl TradeHistoryQueryParams {
    /// Creates new query params with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the PDA index.
    pub fn with_pda_index(mut self, pda_index: u8) -> Self {
        self.pda_index = pda_index;
        self
    }

    /// Sets the market symbol filter.
    pub fn with_market_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.market_symbol = Some(symbol.into());
        self
    }

    /// Sets the limit.
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the cursor for pagination.
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_deserialize_trade_event() {
        let json = r#"{
            "slot": "123456789",
            "slotIndex": 5,
            "timestamp": "1775578550",
            "symbol": "SOL",
            "taker": "ABC123pubkey",
            "tradeSequenceNumber": "100",
            "side": "bid",
            "baseLotsFilled": "1000",
            "quoteLotsFilled": "150000",
            "feeInQuoteLots": "30",
            "baseAmount": 10.0,
            "quoteAmount": 1500.0,
            "numFills": 2
        }"#;

        let trade: TradeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(trade.symbol, "SOL");
        assert_eq!(trade.base_amount, 10.0);
        assert_eq!(trade.quote_amount, 1500.0);
        assert_eq!(trade.num_fills, 2);
        assert_eq!(trade.side, Side::Bid);
        assert_eq!(
            trade.timestamp,
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1775578550)
        );
    }

    #[test]
    fn test_serialize_trade_event() {
        let trade = TradeEvent {
            slot: 123456789u64.into(),
            slot_index: 5,
            timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1775578550),
            symbol: "SOL".to_string(),
            taker: "ABC123pubkey".to_string(),
            trade_sequence_number: 100u64.into(),
            side: Side::Ask,
            base_lots_filled: 1000u64.into(),
            quote_lots_filled: 150000u64.into(),
            fee_in_quote_lots: 30u64.into(),
            base_amount: 10.0,
            quote_amount: 1500.0,
            num_fills: 2,
        };

        let json = serde_json::to_string(&trade).unwrap();
        assert!(json.contains("\"symbol\":\"SOL\""));
        assert!(json.contains("\"baseAmount\":10"));
        assert!(json.contains("\"side\":\"ask\""));
    }

    #[test]
    fn test_trades_subscription_request() {
        let req = TradesSubscriptionRequest {
            symbol: "SOL".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"symbol\":\"SOL\""));
    }

    #[test]
    fn test_deserialize_trade_history_item_with_fill_id() {
        let json = json!({
            "userId": 1,
            "traderId": 2,
            "traderPdaIndex": 0,
            "subaccountIndex": 0,
            "marketSymbol": "SOL-PERP",
            "signature": "5PRu7zP_mnhT5c",
            "fillId": "43148c7f-1389-34f5-99f0-c7296f5858c2",
            "timestamp": "2026-04-21T12:00:00Z",
            "slot": 377700000,
            "slotIndex": 2442,
            "eventIndex": 0,
            "instructionIndex": 4,
            "instructionType": "PlaceMarketOrder",
            "baseLotsBefore": "0",
            "baseLotsAfter": "1",
            "baseLotsDelta": "1",
            "virtualQuoteLotsBefore": "0",
            "virtualQuoteLotsAfter": "150000",
            "virtualQuoteLotsDelta": "150000",
            "price": "150",
            "realizedPnl": "1",
            "fees": "0.1",
            "liquidity": "taker",
            "orderSequenceNumber": null,
            "splineSequenceNumber": null,
            "tradeType": "market"
        });

        let item: TradeHistoryItem = serde_json::from_value(json).unwrap();

        assert_eq!(
            item.fill_id.as_deref(),
            Some("43148c7f-1389-34f5-99f0-c7296f5858c2")
        );
    }

    #[test]
    fn test_deserialize_trade_history_item_without_fill_id() {
        let json = json!({
            "userId": 1,
            "traderId": 2,
            "traderPdaIndex": 0,
            "subaccountIndex": 0,
            "marketSymbol": "SOL-PERP",
            "signature": "5PRu7zP_mnhT5c",
            "timestamp": "2026-04-21T12:00:00Z",
            "slot": 377700000,
            "slotIndex": 2442,
            "eventIndex": 0,
            "instructionIndex": 4,
            "instructionType": "PlaceMarketOrder",
            "baseLotsBefore": "0",
            "baseLotsAfter": "1",
            "baseLotsDelta": "1",
            "virtualQuoteLotsBefore": "0",
            "virtualQuoteLotsAfter": "150000",
            "virtualQuoteLotsDelta": "150000",
            "price": "150",
            "realizedPnl": "1",
            "fees": "0.1",
            "liquidity": "taker",
            "orderSequenceNumber": null,
            "splineSequenceNumber": null,
            "tradeType": "market"
        });

        let item: TradeHistoryItem = serde_json::from_value(json).unwrap();

        assert_eq!(item.fill_id, None);
    }
}
