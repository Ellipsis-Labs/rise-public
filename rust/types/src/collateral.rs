//! Multi-asset collateral history and lifetime totals.

use serde::{Deserialize, Serialize};

/// A single event in the mixed collateral history timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralEventV2 {
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
    /// On-chain `AssetIndex` of the collateral asset (e.g. 4294967295 for the
    /// quote asset / USDC, 4294901760 for native SOL).
    pub asset_index: i64,
    /// Asset symbol (e.g. `SOL`, `USDC`) from the collateral asset registry.
    pub symbol: String,
    /// Unified classification: `deposit`, `withdrawal`, `transfer`, `swap`, or
    /// `liquidation`.
    pub category: String,
    /// Signed amount in the asset's native units (outflows negative). The sign
    /// is the direction.
    pub amount: i64,
    /// Balance after this event, native units of the asset.
    pub balance_after: i64,
    /// Uncounted native units beyond `amount` (spot only: clamped-off credit or
    /// swept excess). Zero for quote events.
    pub excess: i64,
    /// Transaction signature, absent when unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Event timestamp (ISO 8601).
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response for mixed collateral history queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralHistoryV2Response {
    /// The data payload (events, newest first).
    pub data: Vec<CollateralEventV2>,
    /// Cursor for fetching older results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Cursor for fetching newer results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Whether there are more results in the requested direction.
    pub has_more: bool,
}

/// One direction (deposits or withdrawals) of a single collateral asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralFlowTotal {
    /// Native units of the asset moved in this direction, as a positive
    /// magnitude (same scale as `CollateralEventV2::amount`).
    pub amount: i64,
    /// Quote-unit (USD) value: each event valued at its own event-time price.
    pub value: f64,
}

/// Deposit and withdrawal totals for one collateral asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralAssetTotals {
    /// On-chain `AssetIndex` of the collateral asset.
    pub asset_index: i64,
    /// Asset symbol from the asset registry.
    pub symbol: String,
    /// Native decimals of the asset.
    pub decimals: i32,
    pub deposited: CollateralFlowTotal,
    pub withdrawn: CollateralFlowTotal,
}

/// Lifetime deposit and withdrawal totals for a user across all subaccounts
/// and collateral assets. Spot events are valued at the index price of their
/// pricing market at event time; quote events are 1:1. Transfers, swaps and
/// liquidations are excluded, matching the rows classified as `deposit` /
/// `withdrawal` in the mixed collateral history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollateralTotalsResponse {
    /// Sum of `assets[].deposited.value`.
    pub total_deposited: f64,
    /// Sum of `assets[].withdrawn.value`.
    pub total_withdrawn: f64,
    /// Per-asset breakdown; only assets with at least one deposit or
    /// withdrawal appear.
    pub assets: Vec<CollateralAssetTotals>,
}
