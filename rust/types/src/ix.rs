use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Account metadata returned from instruction-building endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// API representation of a Solana instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInstructionResponse {
    pub data: Vec<u8>,
    pub keys: Vec<ApiAccountMeta>,
    pub program_id: String,
}

/// TP/SL configuration shared across isolated order endpoints.
///
/// Deprecated: legacy stop-loss flow. Prefer conditional orders
/// (`PlacePositionConditionalOrderRequest`, `CancelConditionalOrderRequest`,
/// or the `*_with_conditionals` requests).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TpSlOrderConfig {
    #[serde(default)]
    pub take_profit_trigger_price: Option<f64>,
    #[serde(default)]
    pub take_profit_trigger_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub take_profit_execution_price: Option<f64>,
    #[serde(default)]
    pub take_profit_execution_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub stop_loss_trigger_price: Option<f64>,
    #[serde(default)]
    pub stop_loss_trigger_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub stop_loss_execution_price: Option<f64>,
    #[serde(default)]
    pub stop_loss_execution_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub order_kind: Option<String>,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
}

/// Request payload for `/v1/ix/place-stop-loss-order`.
///
/// Deprecated: legacy stop-loss flow. Prefer conditional orders
/// (`PlacePositionConditionalOrderRequest`, `CancelConditionalOrderRequest`,
/// or the `*_with_conditionals` requests).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceStopLossOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub trader_pda_index: u8,
    #[serde(default)]
    pub trader_subaccount_index: Option<u8>,
    #[serde(default)]
    pub is_isolated: bool,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
    #[serde(flatten)]
    pub tp_sl: TpSlOrderConfig,
}

/// Trigger direction for `/v1/ix/cancel-stop-loss-order`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopLossExecutionDirection {
    GreaterThan,
    LessThan,
}

/// Request payload for `/v1/ix/cancel-stop-loss-order`.
/// Request payload for `/v1/ix/cancel-stop-loss-order`.
///
/// Deprecated: legacy stop-loss flow. Prefer conditional orders
/// (`PlacePositionConditionalOrderRequest`, `CancelConditionalOrderRequest`,
/// or the `*_with_conditionals` requests).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelStopLossOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub trader_pda_index: u8,
    #[serde(default)]
    pub trader_subaccount_index: Option<u8>,
    #[serde(default)]
    pub is_isolated: bool,
    pub symbol: String,
    /// Which trigger direction to cancel: e.g. "greater_than" or "less_than".
    pub execution_direction: StopLossExecutionDirection,
}

/// Trigger configuration for attached and position conditional orders.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalTriggerRequest {
    pub side: String,
    #[serde(default)]
    pub order_kind: Option<String>,
    #[serde(default)]
    pub trigger_price: Option<f64>,
    #[serde(default)]
    pub trigger_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub execution_price: Option<f64>,
    #[serde(default)]
    pub execution_price_in_ticks: Option<u64>,
}

/// Request payload for `/v1/ix/place-attached-conditional-order`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceAttachedConditionalOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub trader_pda_index: u8,
    #[serde(default)]
    pub trader_subaccount_index: Option<u8>,
    #[serde(default)]
    pub is_isolated: bool,
    pub symbol: String,
    #[serde(default)]
    pub fee_payer: Option<String>,
    pub order_sequence_number: String,
    pub order_price_in_ticks: u64,
    #[serde(default)]
    pub greater_trigger: Option<ConditionalTriggerRequest>,
    #[serde(default)]
    pub less_trigger: Option<ConditionalTriggerRequest>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
}

/// Request payload for `/v1/ix/place-position-conditional-order`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlacePositionConditionalOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub trader_pda_index: u8,
    #[serde(default)]
    pub trader_subaccount_index: Option<u8>,
    #[serde(default)]
    pub is_isolated: bool,
    pub symbol: String,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub greater_trigger: Option<ConditionalTriggerRequest>,
    #[serde(default)]
    pub less_trigger: Option<ConditionalTriggerRequest>,
    #[serde(default)]
    pub size_percent: Option<u64>,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
}

/// Request payload for `/v1/ix/cancel-conditional-order`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CancelConditionalOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub trader_pda_index: u8,
    #[serde(default)]
    pub trader_subaccount_index: Option<u8>,
    #[serde(default)]
    pub is_isolated: bool,
    pub symbol: String,
    pub conditional_order_index: u8,
    pub execution_direction: String,
}

/// Request payload for `/v1/ix/place-isolated-limit-order`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedLimitOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub price_in_ticks: Option<u64>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
    /// Native collateral amounts keyed by symbol; SOL amounts are lamports.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub transfer_spot_collateral_amounts: HashMap<String, u64>,
    #[serde(default)]
    pub pda_index: Option<u8>,
    #[serde(default)]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub is_reduce_only: Option<bool>,
    #[serde(default)]
    pub is_post_only: Option<bool>,
    #[serde(default)]
    pub slide: Option<bool>,
    #[serde(default)]
    pub skip_transfer_to_parent: Option<bool>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
    /// Deprecated: legacy stop-loss protection. Prefer `greater_trigger` /
    /// `less_trigger` or the `*_with_conditionals` requests.
    #[serde(default)]
    pub tp_sl: Option<TpSlOrderConfig>,
}

/// Request payload for `/v1/ix/place-isolated-limit-order-with-conditionals`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedLimitOrderWithConditionalsRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub price_in_ticks: Option<u64>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
    /// Native collateral amounts keyed by symbol; SOL amounts are lamports.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub transfer_spot_collateral_amounts: HashMap<String, u64>,
    #[serde(default)]
    pub pda_index: Option<u8>,
    #[serde(default)]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub is_post_only: Option<bool>,
    #[serde(default)]
    pub slide: Option<bool>,
    #[serde(default)]
    pub skip_transfer_to_parent: Option<bool>,
    #[serde(default)]
    pub greater_trigger: Option<ConditionalTriggerRequest>,
    #[serde(default)]
    pub less_trigger: Option<ConditionalTriggerRequest>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
}

/// Request payload for `/v1/ix/place-isolated-market-order`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedMarketOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub min_base_lots_to_fill: Option<u64>,
    #[serde(default)]
    pub min_quote_lots_to_fill: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
    /// Native collateral amounts keyed by symbol; SOL amounts are lamports.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub transfer_spot_collateral_amounts: HashMap<String, u64>,
    #[serde(default)]
    pub max_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub pda_index: Option<u8>,
    #[serde(default)]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub is_reduce_only: Option<bool>,
    #[serde(default)]
    pub skip_transfer_to_parent: Option<bool>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
    /// Deprecated: legacy stop-loss protection. Prefer `greater_trigger` /
    /// `less_trigger` or the `*_with_conditionals` requests.
    #[serde(default)]
    pub tp_sl: Option<TpSlOrderConfig>,
}

/// V2-capable request payload for `/v1/ix/place-isolated-market-order`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedMarketOrderV2Request {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub min_base_lots_to_fill: Option<u64>,
    #[serde(default)]
    pub min_quote_lots_to_fill: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
    /// Native collateral amounts keyed by symbol; SOL amounts are lamports.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub transfer_spot_collateral_amounts: HashMap<String, u64>,
    #[serde(default)]
    pub max_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub pda_index: Option<u8>,
    #[serde(default)]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub is_reduce_only: Option<bool>,
    #[serde(default)]
    pub skip_transfer_to_parent: Option<bool>,
    #[serde(default)]
    pub flight_builder_authority: Option<String>,
    #[serde(default)]
    pub flight_fee_collector_trader: Option<String>,
    /// Deprecated: legacy stop-loss protection. Prefer `greater_trigger` /
    /// `less_trigger` or the `*_with_conditionals` requests.
    #[serde(default)]
    pub tp_sl: Option<TpSlOrderConfig>,
    /// Trigger when price exceeds the threshold. Requires sizePercent: 100;
    /// cannot be combined with tpSl. May be paired with lessTrigger.
    #[serde(default)]
    pub greater_trigger: Option<ConditionalTriggerRequest>,
    /// Trigger when price falls below the threshold. Requires sizePercent: 100;
    /// cannot be combined with tpSl. May be paired with greaterTrigger.
    #[serde(default)]
    pub less_trigger: Option<ConditionalTriggerRequest>,
    /// Full-position protection percentage. Must be 100 with at least one of
    /// greaterTrigger or lessTrigger, and cannot be combined with tpSl.
    #[serde(default)]
    pub size_percent: Option<u64>,
}

/// Response payload for `/v1/ix/place-isolated-limit-order-enhanced`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedLimitOrderEnhancedResponse {
    pub instructions: Vec<ApiInstructionResponse>,
    #[serde(default)]
    pub estimated_liquidation_price_usd: Option<f64>,
}

/// Response payload for `/v1/ix/place-isolated-market-order-enhanced`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedMarketOrderEnhancedResponse {
    pub instructions: Vec<ApiInstructionResponse>,
    #[serde(default)]
    pub estimated_liquidation_price_usd: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::{
        CancelConditionalOrderRequest, CancelStopLossOrderRequest, ConditionalTriggerRequest,
        PlaceAttachedConditionalOrderRequest, PlaceIsolatedLimitOrderWithConditionalsRequest,
        PlaceIsolatedMarketOrderRequest, PlacePositionConditionalOrderRequest,
        PlaceStopLossOrderRequest, StopLossExecutionDirection, TpSlOrderConfig,
    };

    #[test]
    fn cancel_conditional_order_request_deserializes_camel_case_json() {
        let json = r#"{
            "authority": "11111111111111111111111111111112",
            "positionAuthority": "11111111111111111111111111111113",
            "traderPdaIndex": 0,
            "traderSubaccountIndex": 1,
            "isIsolated": true,
            "symbol": "SOL-PERP",
            "conditionalOrderIndex": 7,
            "executionDirection": "greater_than"
        }"#;

        let request: CancelConditionalOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.authority, "11111111111111111111111111111112");
        assert_eq!(
            request.position_authority.as_deref(),
            Some("11111111111111111111111111111113")
        );
        assert_eq!(request.trader_pda_index, 0);
        assert_eq!(request.trader_subaccount_index, Some(1));
        assert!(request.is_isolated);
        assert_eq!(request.symbol, "SOL-PERP");
        assert_eq!(request.conditional_order_index, 7);
        assert_eq!(request.execution_direction, "greater_than");
    }

    #[test]
    fn place_stop_loss_order_request_serializes_flattened_tpsl_fields() {
        let request = PlaceStopLossOrderRequest {
            authority: "11111111111111111111111111111112".to_string(),
            trader_pda_index: 0,
            symbol: "SOL-PERP".to_string(),
            side: "ask".to_string(),
            tp_sl: TpSlOrderConfig {
                stop_loss_trigger_price: Some(90.0),
                stop_loss_execution_price: Some(89.5),
                order_kind: Some("ioc".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let json = serde_json::to_value(request).unwrap();
        assert_eq!(json["authority"], "11111111111111111111111111111112");
        assert_eq!(json["traderPdaIndex"], 0);
        assert_eq!(json["symbol"], "SOL-PERP");
        assert_eq!(json["side"], "ask");
        assert_eq!(json["stopLossTriggerPrice"], 90.0);
        assert_eq!(json["stopLossExecutionPrice"], 89.5);
        assert_eq!(json["orderKind"], "ioc");
        assert!(json.get("tpSl").is_none());
    }

    #[test]
    fn isolated_market_order_request_serializes_minimum_fill() {
        let request = PlaceIsolatedMarketOrderRequest {
            authority: "11111111111111111111111111111112".to_string(),
            symbol: "SOL-PERP".to_string(),
            side: "buy".to_string(),
            num_base_lots: Some(25),
            min_base_lots_to_fill: Some(0),
            min_quote_lots_to_fill: Some(0),
            ..Default::default()
        };

        let json = serde_json::to_value(request).unwrap();
        assert_eq!(json["numBaseLots"], 25);
        assert_eq!(json["minBaseLotsToFill"], 0);
        assert_eq!(json["minQuoteLotsToFill"], 0);
    }

    #[test]
    fn cancel_stop_loss_order_request_uses_typed_execution_direction() {
        let json = r#"{
            "authority": "11111111111111111111111111111112",
            "traderPdaIndex": 0,
            "symbol": "SOL-PERP",
            "executionDirection": "less_than"
        }"#;

        let request: CancelStopLossOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.execution_direction,
            StopLossExecutionDirection::LessThan
        );

        let serialized = serde_json::to_value(request).unwrap();
        assert_eq!(serialized["executionDirection"], "less_than");
    }

    #[test]
    fn cancel_stop_loss_order_request_rejects_invalid_execution_direction() {
        let json = r#"{
            "authority": "11111111111111111111111111111112",
            "traderPdaIndex": 0,
            "symbol": "SOL-PERP",
            "executionDirection": "invalid"
        }"#;

        let error = serde_json::from_str::<CancelStopLossOrderRequest>(json).unwrap_err();
        assert!(error.to_string().contains("unknown variant"));
    }

    #[test]
    fn attached_conditional_order_request_deserializes_nested_triggers() {
        let json = r#"{
            "authority": "11111111111111111111111111111112",
            "traderPdaIndex": 0,
            "symbol": "SOL-PERP",
            "orderSequenceNumber": "42",
            "orderPriceInTicks": 1234,
            "greaterTrigger": {
                "side": "ask",
                "orderKind": "limit",
                "triggerPrice": 120.5,
                "executionPriceInTicks": 119
            }
        }"#;

        let request: PlaceAttachedConditionalOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.authority, "11111111111111111111111111111112");
        assert_eq!(request.order_sequence_number, "42");
        assert_eq!(request.order_price_in_ticks, 1234);
        assert!(request.less_trigger.is_none());

        let greater: ConditionalTriggerRequest = request.greater_trigger.unwrap();
        assert_eq!(greater.side, "ask");
        assert_eq!(greater.order_kind.as_deref(), Some("limit"));
        assert_eq!(greater.trigger_price, Some(120.5));
        assert_eq!(greater.execution_price_in_ticks, Some(119));
    }

    #[test]
    fn flight_order_routing_fields_serialize_on_routable_requests() {
        let stop_loss = serde_json::to_value(PlaceStopLossOrderRequest {
            authority: "11111111111111111111111111111112".to_string(),
            trader_pda_index: 0,
            symbol: "SOL-PERP".to_string(),
            side: "ask".to_string(),
            flight_builder_authority: Some("builder".to_string()),
            flight_fee_collector_trader: Some("collector".to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(stop_loss["flightBuilderAuthority"], "builder");
        assert_eq!(stop_loss["flightFeeCollectorTrader"], "collector");

        let attached = serde_json::to_value(PlaceAttachedConditionalOrderRequest {
            authority: "11111111111111111111111111111112".to_string(),
            trader_pda_index: 0,
            symbol: "SOL-PERP".to_string(),
            order_sequence_number: "42".to_string(),
            order_price_in_ticks: 1234,
            flight_builder_authority: Some("builder".to_string()),
            flight_fee_collector_trader: Some("collector".to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(attached["flightBuilderAuthority"], "builder");
        assert_eq!(attached["flightFeeCollectorTrader"], "collector");

        let position = serde_json::to_value(PlacePositionConditionalOrderRequest {
            authority: "11111111111111111111111111111112".to_string(),
            trader_pda_index: 0,
            symbol: "SOL-PERP".to_string(),
            size_percent: Some(50),
            flight_builder_authority: Some("builder".to_string()),
            flight_fee_collector_trader: Some("collector".to_string()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(position["flightBuilderAuthority"], "builder");
        assert_eq!(position["flightFeeCollectorTrader"], "collector");

        let isolated_with_conditionals =
            serde_json::to_value(PlaceIsolatedLimitOrderWithConditionalsRequest {
                authority: "11111111111111111111111111111112".to_string(),
                symbol: "SOL-PERP".to_string(),
                side: "bid".to_string(),
                flight_builder_authority: Some("builder".to_string()),
                flight_fee_collector_trader: Some("collector".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            isolated_with_conditionals["flightBuilderAuthority"],
            "builder"
        );
        assert_eq!(
            isolated_with_conditionals["flightFeeCollectorTrader"],
            "collector"
        );
    }
}

#[cfg(test)]
mod isolated_v2_tests {
    use super::PlaceIsolatedMarketOrderV2Request;

    #[test]
    fn isolated_market_order_v2_round_trips_protection_and_authorities() {
        for (tp, sl) in [(true, false), (false, true), (true, true)] {
            let mut value = serde_json::json!({
                "authority": "owner", "positionAuthority": "delegate", "feePayer": "sponsor",
                "symbol": "SOL-PERP", "side": "buy", "numBaseLots": 50,
                "sizePercent": 100, "flightBuilderAuthority": "builder",
                "flightFeeCollectorTrader": "collector"
            });
            if tp {
                value["greaterTrigger"] =
                    serde_json::json!({"side": "sell", "triggerPriceInTicks": 1200});
            }
            if sl {
                value["lessTrigger"] =
                    serde_json::json!({"side": "sell", "triggerPriceInTicks": 800});
            }
            let request: PlaceIsolatedMarketOrderV2Request = serde_json::from_value(value).unwrap();
            assert_eq!(request.size_percent, Some(100));
            assert_eq!(request.greater_trigger.is_some(), tp);
            assert_eq!(request.less_trigger.is_some(), sl);
            let serialized = serde_json::to_value(request).unwrap();
            assert_eq!(serialized["sizePercent"], 100);
            assert_eq!(serialized["positionAuthority"], "delegate");
            assert_eq!(serialized["feePayer"], "sponsor");
            assert_eq!(serialized["flightBuilderAuthority"], "builder");
            assert_eq!(serialized["flightFeeCollectorTrader"], "collector");
            assert_eq!(!serialized["greaterTrigger"].is_null(), tp);
            assert_eq!(!serialized["lessTrigger"].is_null(), sl);
        }
    }
}

#[cfg(test)]
mod isolated_market_compatibility_tests {
    use super::{PlaceIsolatedMarketOrderRequest, PlaceIsolatedMarketOrderV2Request};

    #[test]
    fn isolated_market_order_without_sol_map_preserves_legacy_wire_shape() {
        let legacy = PlaceIsolatedMarketOrderRequest {
            authority: "owner".to_string(),
            position_authority: None,
            symbol: "SOL-PERP".to_string(),
            side: "buy".to_string(),
            num_base_lots: None,
            min_base_lots_to_fill: None,
            min_quote_lots_to_fill: None,
            quantity: None,
            transfer_amount: 0,
            transfer_spot_collateral_amounts: Default::default(),
            max_price_in_ticks: None,
            pda_index: None,
            allow_cross_and_isolated_for_asset: None,
            fee_payer: None,
            is_reduce_only: None,
            skip_transfer_to_parent: None,
            flight_builder_authority: None,
            flight_fee_collector_trader: None,
            tp_sl: None,
        };
        let serialized = serde_json::to_value(&legacy).unwrap();
        assert!(serialized.get("sizePercent").is_none());
        assert!(serialized.get("greaterTrigger").is_none());
        assert!(serialized.get("lessTrigger").is_none());
        let current: PlaceIsolatedMarketOrderV2Request =
            serde_json::from_value(serialized).unwrap();
        assert_eq!(current.authority, legacy.authority);
        assert!(current.size_percent.is_none());
        assert!(current.greater_trigger.is_none());
        assert!(current.less_trigger.is_none());
    }
}

#[cfg(test)]
mod isolated_spot_collateral_tests {
    use super::{
        PlaceIsolatedLimitOrderRequest, PlaceIsolatedLimitOrderWithConditionalsRequest,
        PlaceIsolatedMarketOrderRequest, PlaceIsolatedMarketOrderV2Request,
    };

    #[test]
    fn every_isolated_request_round_trips_native_sol_funding() {
        let payload = serde_json::json!({"authority":"owner", "symbol":"SOL", "side":"buy", "transferAmount":0,
            "transferSpotCollateralAmounts":{"SOL":1_000_000_000_u64}});
        let encoded = [
            serde_json::to_value(
                serde_json::from_value::<PlaceIsolatedLimitOrderRequest>(payload.clone()).unwrap(),
            )
            .unwrap(),
            serde_json::to_value(
                serde_json::from_value::<PlaceIsolatedLimitOrderWithConditionalsRequest>(
                    payload.clone(),
                )
                .unwrap(),
            )
            .unwrap(),
            serde_json::to_value(
                serde_json::from_value::<PlaceIsolatedMarketOrderRequest>(payload.clone()).unwrap(),
            )
            .unwrap(),
            serde_json::to_value(
                serde_json::from_value::<PlaceIsolatedMarketOrderV2Request>(payload.clone())
                    .unwrap(),
            )
            .unwrap(),
        ];
        for value in encoded {
            assert_eq!(
                value["transferSpotCollateralAmounts"],
                payload["transferSpotCollateralAmounts"]
            );
            assert_eq!(value["transferAmount"], 0);
        }
        let mut legacy = payload;
        legacy
            .as_object_mut()
            .unwrap()
            .remove("transferSpotCollateralAmounts");
        assert!(
            serde_json::from_value::<PlaceIsolatedMarketOrderRequest>(legacy)
                .unwrap()
                .transfer_spot_collateral_amounts
                .is_empty()
        );
    }
}
