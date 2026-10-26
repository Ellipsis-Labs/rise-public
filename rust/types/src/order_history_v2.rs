//! Order History V2 packets and terminal TWAP parents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::PaginatedResponse;

/// Flat history packets, including individual TWAP executions.
pub type OrderHistoryV2Response = PaginatedResponse<Vec<OrderHistoryV2Item>>;

/// Terminal TWAP parents and standalone packets sharing a top-level page limit.
pub type OrganizedOrderHistoryV2Response = PaginatedResponse<OrganizedOrderHistoryV2Data>;

/// Filters shared by flat and organized history methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderHistoryV2QueryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_symbol: Option<String>,
    pub limit: i64,
    /// Opaque view-specific cursor. Start without a cursor when changing views.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl OrderHistoryV2QueryParams {
    pub fn new(limit: i64) -> Self {
        Self {
            trader_pda_index: None,
            market_symbol: None,
            limit,
            cursor: None,
        }
    }

    pub fn with_pda_index(mut self, pda_index: u8) -> Self {
        self.trader_pda_index = Some(pda_index);
        self
    }

    pub fn with_market_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.market_symbol = Some(symbol.into());
        self
    }

    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct OrderHistoryV2Item {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_order_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_key: Option<String>,
    pub user_id: i64,
    pub trader_id: i64,
    pub trader_pda_index: i32,
    pub trader_subaccount_index: i32,
    pub slot: i64,
    pub slot_index: i32,
    pub instruction_index: i32,
    pub event_index: i32,
    pub market_symbol: String,
    pub instruction_type: String,
    pub order_type: String,
    pub created_at: DateTime<Utc>,
    pub price: String,
    pub intended: IntendedOrder,
    pub fill_before_placement: Option<AggregatedTrades>,
    pub placed_order: Option<PlacedOrder>,
    pub current_order_state: Option<CurrentOrderState>,
    pub is_reduce_only: bool,
    pub is_stop_loss: bool,
    #[serde(default)]
    pub is_stop_loss_direction: bool,
    #[serde(default)]
    pub is_conditional_order: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct IntendedOrder {
    pub order_kind: String,
    pub side: String,
    pub price_in_ticks: Option<i64>,
    pub price_usd: Option<String>,
    pub has_price_limit: bool,
    pub base_lots: i64,
    pub base_qty: String,
    pub quote_lot_budget: Option<i64>,
    pub match_limit: Option<i64>,
    pub last_valid_slot: Option<i64>,
    pub client_order_id: Option<String>,
    pub order_flags: i16,
    pub is_reduce_only: bool,
    pub is_stop_loss: bool,
    #[serde(default)]
    pub is_stop_loss_direction: bool,
    #[serde(default)]
    pub is_conditional_order: bool,
    pub is_post_only: bool,
    pub is_take_only: bool,
    pub cancel_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlacedOrder {
    pub order_sequence_number: i64,
    pub event_index: i32,
    pub side: String,
    pub price: String,
    pub base_qty: String,
    pub last_valid_slot: Option<i64>,
    pub initial_slot: Option<i64>,
    pub order_flags: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_set_id: Option<i16>,
    pub transaction_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CurrentOrderState {
    pub order_sequence_number: i64,
    pub side: String,
    pub price: String,
    pub base_qty: String,
    pub remaining_base_qty: String,
    pub filled_base_qty: String,
    pub released_base_qty: String,
    pub order_status: String,
    pub placed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub last_fill_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_set_id: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct AggregatedTrades {
    pub total_base_qty_filled: String,
    pub total_quote_qty_filled: String,
    pub total_taker_fee: String,
    pub total_num_fills: i32,
    pub num_trades: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct OrganizedOrderHistoryV2Data {
    pub twap_orders: Vec<TwapParentOrderHistoryItem>,
    pub single_orders: Vec<OrderHistoryV2Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TwapParentOrderHistoryItem {
    pub parent_order_id: String,
    pub parent_order_type: String,
    pub market_symbol: String,
    pub trader_pda_index: i32,
    pub trader_subaccount_index: i32,
    pub side: Option<String>,
    pub is_reduce_only: Option<bool>,
    /// Total scheduled executions, including additional dust executions.
    pub n_child_orders: Option<i64>,
    /// Additional dust executions; zero also covers legacy final-child dust.
    pub n_dust_orders: Option<i64>,
    pub cooldown_slots: Option<i64>,
    pub status: String,
    pub requested_base_qty: Option<String>,
    pub filled_base_qty: Option<String>,
    pub filled_quote_qty: Option<String>,
    pub placed_at: Option<DateTime<Utc>>,
    pub terminal_at: DateTime<Utc>,
    pub sort_key: String,
    pub children: Vec<OrderHistoryV2Item>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        OrderHistoryV2QueryParams, OrderHistoryV2Response, OrganizedOrderHistoryV2Response,
    };

    #[test]
    fn v2_query_preserves_optional_filters_and_opaque_cursor() {
        let defaults = OrderHistoryV2QueryParams::new(25);
        assert_eq!(serde_urlencoded::to_string(defaults).unwrap(), "limit=25");

        let params = OrderHistoryV2QueryParams::new(25)
            .with_pda_index(7)
            .with_market_symbol("SOL")
            .with_cursor("parent:42+opaque=");
        assert_eq!(
            serde_urlencoded::to_string(params).unwrap(),
            "trader_pda_index=7&market_symbol=SOL&limit=25&cursor=parent%3A42%2Bopaque%3D"
        );
    }

    #[test]
    fn history_responses_reject_the_other_views_data_shape() {
        let flat = json!({
            "data": [], "prevCursor": null, "nextCursor": null, "hasMore": false
        });
        let organized = json!({
            "data": { "twapOrders": [], "singleOrders": [] },
            "prevCursor": null, "nextCursor": null, "hasMore": false
        });

        assert!(serde_json::from_value::<OrderHistoryV2Response>(flat.clone()).is_ok());
        assert!(
            serde_json::from_value::<OrganizedOrderHistoryV2Response>(organized.clone()).is_ok()
        );
        assert!(serde_json::from_value::<OrganizedOrderHistoryV2Response>(flat).is_err());
        assert!(serde_json::from_value::<OrderHistoryV2Response>(organized).is_err());
    }

    #[test]
    fn terminal_parent_accepts_unknown_metadata_but_requires_identity_and_terminal_time() {
        let mut parent = json!({
            "parentOrderId": "twap-account:42",
            "parentOrderType": "twap",
            "marketSymbol": "SOL",
            "traderPdaIndex": 7,
            "traderSubaccountIndex": 0,
            "status": "cancelled",
            "requestedBaseQty": null,
            "filledBaseQty": null,
            "filledQuoteQty": null,
            "terminalAt": "2026-10-12T16:00:00Z",
            "sortKey": "opaque-sort-key",
            "children": []
        });
        let response = json!({
            "data": { "twapOrders": [parent.clone()], "singleOrders": [] },
            "prevCursor": "newer", "nextCursor": "older", "hasMore": true
        });
        let parsed: OrganizedOrderHistoryV2Response = serde_json::from_value(response).unwrap();
        let historical = &parsed.data.twap_orders[0];
        assert!(historical.side.is_none());
        assert!(historical.n_child_orders.is_none());
        assert!(historical.requested_base_qty.is_none());
        assert!(historical.filled_base_qty.is_none());
        assert_eq!(parsed.prev_cursor.as_deref(), Some("newer"));
        assert_eq!(parsed.next_cursor.as_deref(), Some("older"));

        for field in ["parentOrderId", "terminalAt"] {
            let original = parent.as_object_mut().unwrap().remove(field).unwrap();
            assert!(
                serde_json::from_value::<super::TwapParentOrderHistoryItem>(parent.clone())
                    .is_err()
            );
            parent[field] = original;
        }
        parent["terminalAt"] = json!("not-a-timestamp");
        assert!(serde_json::from_value::<super::TwapParentOrderHistoryItem>(parent).is_err());
    }
}
