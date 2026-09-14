//! WebSocket protocol types for trader state synchronization.
//!
//! These types represent snapshots and deltas for trader positions,
//! orders, splines, trade history, and capabilities received via WebSocket.
//!
//! For HTTP API types (views, history), see [`crate::trader_http`].

use serde::{Deserialize, Serialize};

use crate::core::Side;

// ============================================================================
// State Envelope Types
// ============================================================================

/// Complete server message envelope for trader-state updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateServerMessage {
    pub authority: String,
    pub trader_pda_index: u8,
    pub slot: u64,
    #[serde(flatten)]
    pub content: TraderStatePayload,
}

/// Trader-state payload variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "messageType", rename_all = "camelCase")]
pub enum TraderStatePayload {
    #[serde(rename = "snapshot")]
    Snapshot(TraderStateSnapshot),
    #[serde(rename = "delta")]
    Delta(TraderStateDelta),
}

/// Snapshot payload covering every subaccount belonging to a trader PDA.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSnapshot {
    pub version: u32,
    pub capabilities: TraderStateCapabilities,
    pub maker_fee_override_multiplier: f64,
    pub taker_fee_override_multiplier: f64,
    pub subaccounts: Vec<TraderStateSubaccountSnapshot>,
}

/// Batch of row-level deltas grouped by subaccount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateDelta {
    pub deltas: Vec<TraderStateSubaccountDelta>,
}

// ============================================================================
// Subaccount Types
// ============================================================================

/// Raw spot collateral balance row (native units, no valuation). The list is
/// always the complete current set for the subaccount; an empty list means no
/// spot collateral.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSpotCollateralSnapshot {
    pub asset_index: u32,
    pub symbol: String,
    /// Balance in the asset's native units (lamports for SOL), as a decimal
    /// integer string.
    pub balance: String,
}

/// Complete subaccount view contained in a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSubaccountSnapshot {
    pub subaccount_index: u8,
    pub sequence: u64,
    pub collateral: String,
    #[serde(default)]
    pub spot_collaterals: Vec<TraderStateSpotCollateralSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<TraderStateCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_status: Option<CooldownStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<TraderStatePositionSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<TraderStateLimitOrderEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splines: Vec<TraderStateSplineSnapshot>,
}

/// Row-level delta set for a specific subaccount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSubaccountDelta {
    pub subaccount_index: u8,
    pub sequence: u64,
    pub collateral: String,
    #[serde(default)]
    pub spot_collaterals: Vec<TraderStateSpotCollateralSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<TraderStateCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_status: Option<CooldownStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<TraderStatePositionDelta>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<TraderStateLimitOrderEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splines: Vec<TraderStateSplineDelta>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trade_history: Vec<TradeHistoryDelta>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_history: Vec<OrderHistoryDelta>,
}

// ============================================================================
// Position Types
// ============================================================================

/// Snapshot entry keyed by market symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStatePositionSnapshot {
    pub symbol: String,
    #[serde(flatten)]
    pub position: TraderStatePositionRow,
}

/// Position row used for snapshots and deltas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStatePositionRow {
    pub position_sequence_number: String,
    pub base_position_lots: String,
    pub entry_price_ticks: String,
    pub entry_price_usd: String,
    pub virtual_quote_position_lots: String,
    pub unsettled_funding_quote_lots: String,
    pub accumulated_funding_quote_lots: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub take_profit_triggers: Vec<TraderStateTakeProfitTrigger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_loss_triggers: Vec<TraderStateStopLossTrigger>,
}

/// Trigger configuration for TP/SL orders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateTrigger {
    pub trigger_price_ticks: String,
    pub execution_price_ticks: String,
    pub side: Side,
    pub kind: String,
}

/// Position delta grouped by symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStatePositionDelta {
    pub symbol: String,
    pub change: TraderStateRowChangeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<TraderStatePositionRow>,
}

/// Change indicator used for row-level deltas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TraderStateRowChangeKind {
    Updated,
    Closed,
}

/// Stop-loss trigger rows scoped to a position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateStopLossTrigger {
    pub stop_loss_id: String,
    pub trigger: TraderStateTrigger,
    pub status: String,
}

/// Take-profit trigger rows scoped to a position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateTakeProfitTrigger {
    pub take_profit_id: String,
    pub trigger: TraderStateTrigger,
    pub status: String,
}

// ============================================================================
// Order Types
// ============================================================================

/// Order grouping used for snapshots/deltas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateLimitOrderEvent {
    pub symbol: String,
    pub orders: Vec<TraderStateMarketLimitOrderEvent>,
}

/// Detailed limit order representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateMarketLimitOrderEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change: Option<TraderStateRowChangeKind>,
    pub order_sequence_number: String,
    pub side: Side,
    pub order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional_kind: Option<String>,
    pub price_ticks: String,
    pub price_usd: String,
    pub size_remaining_lots: String,
    pub initial_size_lots: String,
    pub reduce_only: bool,
    #[serde(default)]
    pub is_stop_loss: bool,
    pub status: String,
}

// ============================================================================
// Spline Types
// ============================================================================

/// Spline snapshot grouped by market symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSplineSnapshot {
    pub symbol: String,
    #[serde(flatten)]
    pub spline: TraderStateSplineRow,
}

/// Spline row containing the spline parameters and fill state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSplineRow {
    pub mid_price_ticks: String,
    pub bid_filled_amount_lots: String,
    pub ask_filled_amount_lots: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bid_regions: Vec<TraderStateTickRegion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ask_regions: Vec<TraderStateTickRegion>,
}

/// Tick region for spline configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateTickRegion {
    pub start_price_ticks: String,
    pub end_price_ticks: String,
    pub density_lots_per_tick: String,
    pub total_size_lots: String,
    pub filled_size_lots: String,
}

/// Spline delta for row-level changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSplineDelta {
    pub symbol: String,
    pub change: TraderStateRowChangeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spline: Option<TraderStateSplineRow>,
}

/// Trade history row generated from PnL events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryDelta {
    pub signature: String,
    /// Deterministic UUID v3 derived from the raw fill coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_id: Option<String>,
    pub timestamp: u64,
    pub slot: i64,
    pub slot_index: i32,
    pub instruction_index: i32,
    pub event_index: i32,
    pub market: String,
    pub instruction_type: String,
    pub trade_type: String,
    pub base_qty_before: String,
    pub base_qty_after: String,
    pub size: String,
    pub liquidity: String,
    pub price: String,
    pub fee: String,
    pub realized_pnl: String,
}

/// Order history row generated from market order packets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryDelta {
    pub timestamp: u64,
    pub slot: u64,
    pub slot_index: u32,
    pub instruction_index: u32,
    pub event_index: u32,
    pub market: String,
    pub instruction_type: String,
    pub order_type: String,
    pub status: String,
    pub size: String,
    pub price: String,
    pub filled_size: String,
}

// ============================================================================
// Capabilities Types
// ============================================================================

/// Withdrawal cooldown status for a trader PDA.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CooldownStatus {
    pub last_deposit_slot: u64,
    pub cooldown_period_in_slots: u64,
}

/// Trader capability flags and derived views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateCapabilities {
    /// Raw capability flags as a bitmask integer.
    pub flags: u16,
    /// Activity state as a string (e.g., "active", "reduceOnly",
    /// "liquidatable").
    pub state: String,
    /// Capability access levels for various actions.
    pub capabilities: TraderCapabilitiesView,
}

/// Human-readable capabilities derived from flags.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraderCapabilitiesView {
    pub place_limit_order: CapabilityAccess,
    pub place_market_order: CapabilityAccess,
    pub risk_increasing_trade: CapabilityAccess,
    pub risk_reducing_trade: CapabilityAccess,
    pub deposit_collateral: CapabilityAccess,
    pub withdraw_collateral: CapabilityAccess,
}

/// Capability access levels.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityAccess {
    #[serde(default)]
    pub immediate: bool,
    #[serde(default)]
    pub via_cold_activation: bool,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_deserialize_trader_state_snapshot() {
        let json = r#"{
            "authority": "ABC123",
            "traderPdaIndex": 0,
            "slot": 12345,
            "messageType": "snapshot",
            "version": 1,
            "capabilities": {
                "flags": 63,
                "state": "active",
                "capabilities": {
                    "placeLimitOrder": {"immediate": true},
                    "placeMarketOrder": {"immediate": true},
                    "riskIncreasingTrade": {"immediate": true},
                    "riskReducingTrade": {"immediate": true},
                    "depositCollateral": {"immediate": true},
                    "withdrawCollateral": {"immediate": true}
                }
            },
            "makerFeeOverrideMultiplier": 1.0,
            "takerFeeOverrideMultiplier": 1.0,
            "subaccounts": []
        }"#;

        let msg: TraderStateServerMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.authority, "ABC123");
        assert_eq!(msg.slot, 12345);
        assert!(matches!(msg.content, TraderStatePayload::Snapshot(_)));
    }

    #[test]
    fn test_deserialize_trader_state_delta_trade_history_fill_id() {
        let json = json!({
            "authority": "ABC123",
            "traderPdaIndex": 0,
            "slot": 12346,
            "messageType": "delta",
            "deltas": [{
                "subaccountIndex": 0,
                "sequence": 101,
                "collateral": "1000",
                "positions": [],
                "orders": [],
                "splines": [],
                "tradeHistory": [{
                    "signature": "test_signature",
                    "fillId": "43148c7f-1389-34f5-99f0-c7296f5858c2",
                    "timestamp": 1000,
                    "slot": 100,
                    "slotIndex": 1,
                    "instructionIndex": 2,
                    "eventIndex": 3,
                    "market": "SOL-PERP",
                    "instructionType": "PlaceMarketOrder",
                    "tradeType": "market",
                    "baseQtyBefore": "0",
                    "baseQtyAfter": "1",
                    "size": "1",
                    "liquidity": "taker",
                    "price": "150",
                    "fee": "0.1",
                    "realizedPnl": "1"
                }],
                "orderHistory": []
            }]
        });

        let msg: TraderStateServerMessage = serde_json::from_value(json).unwrap();

        let TraderStatePayload::Delta(delta) = msg.content else {
            panic!("expected delta payload");
        };

        assert_eq!(
            delta.deltas[0].trade_history[0].fill_id,
            Some("43148c7f-1389-34f5-99f0-c7296f5858c2".to_string())
        );
    }
}
