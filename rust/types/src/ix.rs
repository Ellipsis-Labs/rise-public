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
    pub tp_sl: Option<TpSlOrderConfig>,
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
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
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
    pub tp_sl: Option<TpSlOrderConfig>,
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
    use super::CancelConditionalOrderRequest;

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
}
