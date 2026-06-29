//! HTTP API types for trader state, views, and history.
//!
//! These types represent responses from Phoenix REST API endpoints
//! for trader views, order history, collateral history, and funding history.

use std::fmt::{self, Display};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::core::{Decimal, Side};
use crate::market::{RiskState, RiskTier};
use crate::trader::TraderCapabilitiesView;

// ============================================================================
// Order History Types
// ============================================================================

/// Order status in order history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    /// Order is open and active on the book.
    Open,
    /// Order was filled completely.
    Filled,
    /// Order was cancelled.
    Cancelled,
    /// Order expired.
    Expired,
}

/// Individual order in order history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryItem {
    /// Order sequence number.
    pub order_sequence_number: String,
    /// Market symbol (e.g., "SOL").
    pub market_symbol: String,
    /// Order status.
    pub status: OrderStatus,
    /// Order side ("buy" or "sell").
    pub side: Side,
    /// Indicates whether the order was marked reduce-only at placement time.
    pub is_reduce_only: bool,
    /// Indicates whether the order originated from a stop loss trigger.
    #[serde(default)]
    pub is_stop_loss: bool,
    /// Order price (human-readable, decimal format).
    pub price: String,
    /// Base quantity (human-readable, decimal format).
    pub base_qty: String,
    /// Remaining base quantity (human-readable, decimal format).
    pub remaining_base_qty: String,
    /// Total filled base quantity (human-readable, decimal format).
    pub filled_base_qty: String,
    /// Timestamp when the order was placed (ISO 8601).
    pub placed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp when the order was completed (ISO 8601).
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Response from the order history endpoint.
pub type OrderHistoryResponse = crate::core::PaginatedResponse<Vec<OrderHistoryItem>>;

// ============================================================================
// User Liquidation History Types
// ============================================================================

/// Query parameters for fetching user-scoped liquidation, backstop, and ADL
/// history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLiquidationHistoryQueryParams {
    /// Trader PDA index to scope history. Defaults to 0.
    #[serde(default, alias = "pdaIndex", alias = "pda_index")]
    pub pda_index: i32,
    /// Optional subaccount index. If omitted, all subaccounts under the PDA are
    /// included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subaccount_index: Option<i32>,
    /// Optional market symbol filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Number of items to return (max 100, API default 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Optional cursor for older or newer pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl Default for UserLiquidationHistoryQueryParams {
    fn default() -> Self {
        Self {
            pda_index: 0,
            subaccount_index: None,
            symbol: None,
            limit: None,
            cursor: None,
        }
    }
}

impl UserLiquidationHistoryQueryParams {
    /// Creates new query params with API defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the PDA index.
    pub fn with_pda_index(mut self, pda_index: i32) -> Self {
        self.pda_index = pda_index;
        self
    }

    /// Sets the subaccount index.
    pub fn with_subaccount_index(mut self, subaccount_index: i32) -> Self {
        self.subaccount_index = Some(subaccount_index);
        self
    }

    /// Sets the market symbol filter.
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Sets the result limit.
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the pagination cursor.
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

/// User-scoped liquidation history event kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserLiquidationHistoryKind {
    MarketOrder,
    Adl,
    Backstop,
}

/// High-level liquidation-related event type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserLiquidationHistoryType {
    Market,
    Adl,
    Backstop,
}

/// Role the requested user played in a liquidation-related event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserLiquidationHistoryRole {
    Liquidatee,
    BackstopLiquidatee,
    AdlClosedShort,
    AdlClosedLong,
    AdlInProfit,
    AdlCaller,
}

/// Normalized user-scoped liquidation, backstop, or ADL history point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLiquidationHistoryPoint {
    pub kind: UserLiquidationHistoryKind,
    pub r#type: UserLiquidationHistoryType,
    pub ix_name: String,
    pub role: UserLiquidationHistoryRole,
    pub slot: i64,
    pub slot_index: i32,
    pub event_index: i32,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub symbol: String,
    pub market: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subaccount_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidatee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_closed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_lots_filled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_lots_filled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haircut_rate_bps: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidatee_collateral_change: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidator_collateral_change: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_profit_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_loss_close_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_profit_close_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_loss_collateral_change: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_profit_collateral_change: Option<String>,
}

/// Response from the user liquidation history endpoint.
pub type UserLiquidationHistoryResponse =
    crate::core::PaginatedResponse<Vec<UserLiquidationHistoryPoint>>;

/// Query parameters for fetching order history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryQueryParams {
    /// PDA index for the trader (default 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<u8>,
    /// Optional market symbol filter (e.g., "SOL").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_symbol: Option<String>,
    /// Number of items to return (max 1000).
    pub limit: i64,
    /// Optional cursor for pagination (format: "slot,slot_index,event_index").
    /// Returns items older than (exclusive of) this cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Optional Privy user ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privy_id: Option<String>,
}

impl OrderHistoryQueryParams {
    /// Creates new query params with the specified limit.
    pub fn new(limit: i64) -> Self {
        Self {
            trader_pda_index: None,
            market_symbol: None,
            limit,
            cursor: None,
            privy_id: None,
        }
    }

    /// Sets the PDA index for the trader.
    pub fn with_pda_index(mut self, pda_index: u8) -> Self {
        self.trader_pda_index = Some(pda_index);
        self
    }

    /// Sets the market symbol filter.
    pub fn with_market_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.market_symbol = Some(symbol.into());
        self
    }

    /// Sets the cursor for pagination (returns items older than cursor).
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Sets the Privy user ID.
    pub fn with_privy_id(mut self, privy_id: impl Into<String>) -> Self {
        self.privy_id = Some(privy_id.into());
        self
    }
}

// ============================================================================
// Collateral History Types
// ============================================================================

/// Pagination parameters for collateral history requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralHistoryRequest {
    /// Number of items to return (max 1000).
    pub limit: i64,
    /// Cursor for older events (base64-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Cursor for newer events (base64-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Deprecated cursor parameter (older events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl CollateralHistoryRequest {
    /// Creates new request params with the specified limit.
    pub fn new(limit: i64) -> Self {
        Self {
            limit,
            next_cursor: None,
            prev_cursor: None,
            cursor: None,
        }
    }

    /// Sets the cursor for fetching older events.
    pub fn with_next_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.next_cursor = Some(cursor.into());
        self
    }

    /// Sets the cursor for fetching newer events.
    pub fn with_prev_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.prev_cursor = Some(cursor.into());
        self
    }
}

/// Query parameters for fetching collateral history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralHistoryQueryParams {
    /// PDA index for the trader account.
    #[serde(default, alias = "pdaIndex", alias = "pda_index")]
    pub pda_index: u8,
    /// Pagination and filter parameters.
    pub request: CollateralHistoryRequest,
}

impl CollateralHistoryQueryParams {
    /// Creates new query params with the specified limit.
    pub fn new(limit: i64) -> Self {
        Self {
            pda_index: 0,
            request: CollateralHistoryRequest::new(limit),
        }
    }

    /// Sets the PDA index.
    pub fn with_pda_index(mut self, pda_index: u8) -> Self {
        self.pda_index = pda_index;
        self
    }

    /// Sets the cursor for fetching older events.
    pub fn with_next_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.request.next_cursor = Some(cursor.into());
        self
    }

    /// Sets the cursor for fetching newer events.
    pub fn with_prev_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.request.prev_cursor = Some(cursor.into());
        self
    }
}

/// Response from the collateral history endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralHistoryResponse {
    /// The data payload (array of collateral events).
    pub data: Vec<CollateralEvent>,
    /// Cursor for fetching older results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Cursor for fetching newer results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Whether there are more results in the requested direction.
    pub has_more: bool,
}

/// A single collateral event (deposit or withdrawal).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralEvent {
    /// Solana slot when the event occurred.
    pub slot: i64,
    /// Index within the slot.
    pub slot_index: i32,
    /// Event index for ordering within the slot.
    pub event_index: i32,
    /// Trader PDA index (usually 0).
    pub trader_pda_index: i32,
    /// Trader subaccount index.
    pub trader_subaccount_index: i32,
    /// Event type: "deposit" or "withdrawal".
    pub event_type: String,
    /// Amount deposited or withdrawn (in quote lots, 6 decimals).
    pub amount: i64,
    /// Collateral balance after this event (in quote lots, 6 decimals).
    pub collateral_after: i64,
    /// Timestamp when the transaction was processed.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Funding History Types
// ============================================================================

/// Query parameters for fetching funding history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistoryQueryParams {
    /// PDA index for the trader (default: 0).
    #[serde(default)]
    pub pda_index: u8,
    /// Optional market symbol to filter by (e.g., "SOL").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Start time in milliseconds since Unix epoch.
    /// Mutually exclusive with cursor. Max range: 1 year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    /// End time in milliseconds since Unix epoch.
    /// Max range: 1 year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    /// Max number of events (default: 100, max: 1000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Opaque cursor for pagination.
    /// Mutually exclusive with start_time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl FundingHistoryQueryParams {
    /// Creates new empty query params with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the PDA index for the trader.
    pub fn with_pda_index(mut self, pda_index: u8) -> Self {
        self.pda_index = pda_index;
        self
    }

    /// Sets the market symbol filter.
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// Sets the start time in milliseconds since Unix epoch.
    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Sets the end time in milliseconds since Unix epoch.
    pub fn with_end_time(mut self, end_time: i64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Sets the maximum number of events to return.
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the pagination cursor.
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

/// Response from the funding history endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistoryResponse {
    /// List of funding payment events.
    pub events: Vec<FundingHistoryEvent>,
    /// Opaque cursor for fetching newer results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Opaque cursor for fetching older results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Whether more results exist beyond the current page.
    pub has_more: bool,
}

/// A single funding payment event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistoryEvent {
    /// Timestamp when funding was settled.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Market symbol (e.g., "SOL").
    pub symbol: String,
    /// Funding payment amount in USDC (negative = paid, positive = received).
    pub funding_payment: String,
    /// Funding rate percentage at the time of payment.
    pub funding_rate_percentage: String,
    /// Position size at the time of payment.
    pub position_size: String,
    /// Position side ("Long" or "Short").
    pub position_side: String,
}

#[cfg(test)]
mod tests {
    use super::FundingHistoryEvent;

    #[test]
    fn funding_history_timestamp_accepts_rfc3339() {
        let raw = r#"{
            "timestamp":"2026-02-11T16:00:00Z",
            "symbol":"SOL",
            "fundingPayment":"-0.123",
            "fundingRatePercentage":"0.001",
            "positionSize":"10",
            "positionSide":"Long"
        }"#;

        let event: FundingHistoryEvent =
            serde_json::from_str(raw).expect("RFC3339 timestamp should deserialize");
        assert_eq!(event.timestamp.to_rfc3339(), "2026-02-11T16:00:00+00:00");
    }

    #[test]
    fn funding_history_timestamp_rejects_integer() {
        let raw = r#"{
            "timestamp":1770825600,
            "symbol":"SOL",
            "fundingPayment":"-0.123",
            "fundingRatePercentage":"0.001",
            "positionSize":"10",
            "positionSide":"Long"
        }"#;

        let result = serde_json::from_str::<FundingHistoryEvent>(raw);
        assert!(
            result.is_err(),
            "integer timestamp should not deserialize for FundingHistoryEvent"
        );
    }
}

// ============================================================================
// PnL Types
// ============================================================================

/// Resolution for PnL time-series data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PnlResolution {
    #[serde(rename = "1m")]
    Minute1,
    #[serde(rename = "5m")]
    Minute5,
    #[serde(rename = "15m")]
    Minute15,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "4h")]
    Hour4,
    #[serde(rename = "1d")]
    Day1,
    #[serde(rename = "1w")]
    Week1,
    #[serde(rename = "1M")]
    Month1,
}

impl Display for PnlResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PnlResolution::Minute1 => write!(f, "1m"),
            PnlResolution::Minute5 => write!(f, "5m"),
            PnlResolution::Minute15 => write!(f, "15m"),
            PnlResolution::Hour1 => write!(f, "1h"),
            PnlResolution::Hour4 => write!(f, "4h"),
            PnlResolution::Day1 => write!(f, "1d"),
            PnlResolution::Week1 => write!(f, "1w"),
            PnlResolution::Month1 => write!(f, "1M"),
        }
    }
}

impl FromStr for PnlResolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1m" => Ok(PnlResolution::Minute1),
            "5m" => Ok(PnlResolution::Minute5),
            "15m" => Ok(PnlResolution::Minute15),
            "1h" => Ok(PnlResolution::Hour1),
            "4h" => Ok(PnlResolution::Hour4),
            "1d" => Ok(PnlResolution::Day1),
            "1w" => Ok(PnlResolution::Week1),
            "1M" => Ok(PnlResolution::Month1),
            _ => Err(format!("Unknown PnL resolution: {s}")),
        }
    }
}

/// Query parameters for fetching PnL time-series data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlQueryParams {
    /// Resolution for the PnL buckets.
    pub resolution: PnlResolution,
    /// Start time in milliseconds since Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    /// End time in milliseconds since Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    /// Maximum number of data points to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

impl PnlQueryParams {
    /// Creates new query params with the specified resolution.
    pub fn new(resolution: PnlResolution) -> Self {
        Self {
            resolution,
            start_time: None,
            end_time: None,
            limit: None,
        }
    }

    /// Sets the start time in milliseconds since Unix epoch.
    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Sets the end time in milliseconds since Unix epoch.
    pub fn with_end_time(mut self, end_time: i64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Sets the maximum number of data points to return.
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// A single PnL data point in the time-series.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlPoint {
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp: i64,
    /// Start time of the bucket in milliseconds since Unix epoch.
    pub start_time: i64,
    /// End time of the bucket in milliseconds since Unix epoch.
    pub end_time: i64,
    /// Cumulative realized PnL.
    pub cumulative_pnl: f64,
    /// Current unrealized PnL.
    pub unrealized_pnl: f64,
    /// Cumulative funding payments.
    pub cumulative_funding_payment: f64,
    /// Cumulative taker fees paid.
    pub cumulative_taker_fee: f64,
}

/// Response from the PnL endpoint.
pub type PnlResponse = Vec<PnlPoint>;

// ============================================================================
// Trader View Types
// ============================================================================

/// Trader activity state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TraderActivityState {
    Uninitialized,
    Cold,
    Active,
    ReduceOnly,
    Frozen,
}

/// A trader's position view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderPositionView {
    pub symbol: String,
    pub position_size: Decimal,
    pub virtual_quote_position: Decimal,
    pub entry_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub discounted_unrealized_pnl: Decimal,
    pub position_initial_margin: Decimal,
    pub initial_margin: Decimal,
    pub maintenance_margin: Decimal,
    pub backstop_margin: Decimal,
    pub limit_order_margin: Decimal,
    pub position_value: Decimal,
    pub unsettled_funding: Decimal,
    pub accumulated_funding: Decimal,
    pub liquidation_price: Decimal,
    #[serde(default)]
    pub take_profit_price: Option<Decimal>,
    #[serde(default)]
    pub stop_loss_price: Option<Decimal>,
}

/// A trader's limit order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitOrder {
    pub price: Decimal,
    pub side: Side,
    pub order_sequence_number: String,
    pub initial_trade_size: Decimal,
    pub trade_size_remaining: Decimal,
    pub margin_requirement: Decimal,
    pub margin_factor: Decimal,
    pub is_reduce_only: bool,
    #[serde(default)]
    pub is_stop_loss: bool,
}

/// Trader view with all trading information (HTTP API response).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderView {
    pub flags: u16,
    pub state: TraderActivityState,
    pub capabilities: TraderCapabilitiesView,
    pub trader_key: String,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,
    pub authority: String,

    pub collateral_balance: Decimal,
    pub effective_collateral: Decimal,
    pub effective_collateral_for_withdrawals: Decimal,
    pub unrealized_pnl: Decimal,
    pub discounted_unrealized_pnl: Decimal,
    pub unsettled_funding_owed: Decimal,
    pub accumulated_funding: Decimal,
    pub portfolio_value: Decimal,
    pub maintenance_margin: Decimal,
    pub cancel_margin: Decimal,
    pub initial_margin: Decimal,
    pub initial_margin_for_withdrawals: Decimal,
    pub risk_state: RiskState,
    pub risk_tier: RiskTier,

    pub positions: Vec<TraderPositionView>,
    pub limit_orders: std::collections::HashMap<String, Vec<LimitOrder>>,
    pub maker_fee_override_multiplier: f64,
    pub taker_fee_override_multiplier: f64,

    pub max_positions: u64,
    pub last_deposit_slot: u64,

    pub is_in_active_traders: bool,
}

/// Response wrapper for the `/trader/{authority}/state` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateResponse {
    pub slot: u64,
    pub slot_index: u32,
    pub authority: String,
    pub pda_index: u8,
    pub traders: Vec<TraderView>,
}

impl TraderStateResponse {
    /// Find an isolated subaccount for the given asset.
    ///
    /// Prefers a subaccount with an existing position in this asset. Falls back
    /// to the first empty isolated subaccount if none match.
    pub fn isolated_subaccount_for_asset(&self, symbol: &str) -> Option<&TraderView> {
        if let Some(t) = self.traders.iter().find(|t| {
            t.trader_subaccount_index > 0 && t.positions.iter().any(|p| p.symbol == symbol)
        }) {
            return Some(t);
        }
        self.traders
            .iter()
            .find(|t| t.trader_subaccount_index > 0 && t.positions.is_empty())
    }
}
