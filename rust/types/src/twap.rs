use serde::{Deserialize, Serialize};

/// Active TWAP snapshot for a trader PDA.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TwapSnapshot {
    pub authority: String,
    pub trader_pda_index: u8,
    pub slot: u64,
    pub accounts: Vec<TwapAccountSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TwapAccountSnapshot {
    pub twap_account: String,
    pub trader_subaccount_index: u8,
    pub asset_id: u32,
    pub sequence_number: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<TwapOrderSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TwapOrderSnapshot {
    pub trader_account: String,
    pub trader_authority: String,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,
    pub asset_id: u32,
    pub order_sequence_number: u64,
    pub cooldown_slots: u64,
    /// Total executions, including additional dust executions.
    pub n_child_orders: u64,
    /// Additional dust executions. Zero also covers legacy final-child dust.
    /// Omitted by older servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_dust_orders: Option<u64>,
    /// Size of each dust execution in base lots; null when unavailable.
    pub dust_order_size: Option<u64>,
    pub child_orders_executed: u64,
    pub child_orders_remaining: u64,
    /// Cumulative quote lots filled across executed child orders.
    pub quote_lots_filled: u64,
    /// Cumulative base lots filled across executed child orders.
    pub base_lots_filled: u64,
    pub due_slot: u64,
    pub last_executed_slot: u64,
    pub last_valid_slot: u64,
    pub latest_sequence_number: u64,
    pub child_order_max_slippage_bps: Option<u64>,
    pub child_order_min_price_in_ticks: Option<u64>,
    pub child_order_max_price_in_ticks: Option<u64>,
    pub child_order_collateral_quote_lots_to_transfer: Option<u64>,
    pub transfer_collateral_account_count: u8,
    pub side: Option<String>,
    pub price_in_ticks: Option<u64>,
    pub num_base_lots: Option<u64>,
    pub num_quote_lots: Option<u64>,
    pub min_base_lots_to_fill: Option<u64>,
    pub min_quote_lots_to_fill: Option<u64>,
    pub self_trade_behavior: Option<String>,
    pub match_limit: Option<u64>,
    pub client_order_id: Option<String>,
    pub last_valid_slot_from_packet: Option<u64>,
    pub order_flags: Option<u8>,
    pub cancel_existing: Option<bool>,
}

/// Filters for a trader's TWAP accounts. The PDA index defaults to zero.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwapOrdersQueryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_subaccount_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::{TwapOrdersQueryParams, TwapSnapshot};

    #[test]
    fn snapshot_preserves_dust_parameters_and_accepts_older_servers() {
        let mut payload: serde_json::Value =
            serde_json::from_str(include_str!("../../../ts/tests/mocks/twap-snapshot.json"))
                .unwrap();
        for (count, size) in [(0, Some(0)), (0, Some(7)), (2, Some(7)), (2, None)] {
            payload["accounts"][0]["order"]["nDustOrders"] = serde_json::json!(count);
            payload["accounts"][0]["order"]["dustOrderSize"] = serde_json::json!(size);
            let snapshot: TwapSnapshot = serde_json::from_value(payload.clone()).unwrap();
            let order = snapshot.accounts[0].order.as_ref().unwrap();
            assert_eq!(order.n_child_orders, 5);
            assert_eq!(order.n_dust_orders, Some(count));
            assert_eq!(order.dust_order_size, size);
            assert_eq!(serde_json::to_value(snapshot).unwrap(), payload);
        }
        let order = payload["accounts"][0]["order"].as_object_mut().unwrap();
        order.remove("nDustOrders");
        order.remove("dustOrderSize");
        let snapshot: TwapSnapshot = serde_json::from_value(payload).unwrap();
        let order = snapshot.accounts[0].order.as_ref().unwrap();
        assert_eq!(order.n_dust_orders, None);
        assert_eq!(order.dust_order_size, None);
    }

    #[test]
    fn query_uses_camel_case_and_preserves_zero_filters() {
        assert_eq!(
            serde_urlencoded::to_string(TwapOrdersQueryParams::default()).unwrap(),
            ""
        );
        let query = TwapOrdersQueryParams {
            trader_pda_index: Some(0),
            trader_subaccount_index: Some(0),
            asset_id: Some(0),
        };
        assert_eq!(
            serde_urlencoded::to_string(query).unwrap(),
            "traderPdaIndex=0&traderSubaccountIndex=0&assetId=0"
        );
    }
}
