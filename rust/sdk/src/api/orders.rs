use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::BracketLegOrders;
use crate::http_client::HttpClientInner;
use crate::phoenix_rise_ix::{IsolatedCollateralFlow, Side};
use crate::phoenix_rise_types::{
    ApiInstructionResponse, CancelConditionalOrderRequest, OrderHistoryQueryParams,
    OrderHistoryResponse, PhoenixHttpError, PlaceIsolatedLimitOrderEnhancedResponse,
    PlaceIsolatedLimitOrderRequest, PlaceIsolatedMarketOrderEnhancedResponse,
    PlaceIsolatedMarketOrderRequest, TraderKey,
};

pub struct OrdersClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl OrdersClient<'_> {
    pub async fn get_trader_order_history(
        &self,
        authority: &Pubkey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.get_order_history_internal(authority, params).await
    }

    pub async fn get_trader_order_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_order_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_order_history_internal(
        &self,
        authority: &Pubkey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/v1/trader/{}/order-history", authority), &params)
            .await
    }

    pub async fn build_isolated_limit_order_tx(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        price: f64,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let transfer_amount = collateral_transfer_amount(&collateral)?;

        let request = PlaceIsolatedLimitOrderRequest {
            authority: authority.to_string(),
            symbol: symbol.to_string(),
            side: side.to_api_string().to_string(),
            price: Some(price),
            num_base_lots: Some(num_base_lots),
            transfer_amount,
            allow_cross_and_isolated_for_asset: Some(allow_cross_and_isolated),
            ..Default::default()
        };

        self.build_isolated_limit_order_tx_with_request(request)
            .await
    }

    pub async fn build_isolated_limit_order_tx_with_request(
        &self,
        request: PlaceIsolatedLimitOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let api_ixs: Vec<ApiInstructionResponse> = self
            .http
            .post_json("/v1/ix/place-isolated-limit-order", &request)
            .await?;

        api_ixs.into_iter().map(try_into_instruction).collect()
    }

    pub async fn cancel_conditional_order(
        &self,
        request: CancelConditionalOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let api_ixs: Vec<ApiInstructionResponse> = self
            .http
            .post_json("/v1/ix/cancel-conditional-order", &request)
            .await?;
        api_ixs.into_iter().map(try_into_instruction).collect()
    }

    pub async fn build_isolated_limit_order_tx_enhanced(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        price: f64,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        let transfer_amount = collateral_transfer_amount(&collateral)?;

        let request = PlaceIsolatedLimitOrderRequest {
            authority: authority.to_string(),
            symbol: symbol.to_string(),
            side: side.to_api_string().to_string(),
            price: Some(price),
            num_base_lots: Some(num_base_lots),
            transfer_amount,
            allow_cross_and_isolated_for_asset: Some(allow_cross_and_isolated),
            ..Default::default()
        };

        self.build_isolated_limit_order_tx_enhanced_with_request(request)
            .await
    }

    pub async fn build_isolated_limit_order_tx_enhanced_with_request(
        &self,
        request: PlaceIsolatedLimitOrderRequest,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        let enhanced: PlaceIsolatedLimitOrderEnhancedResponse = self
            .http
            .post_json("/v1/ix/place-isolated-limit-order-enhanced", &request)
            .await?;

        let instructions = enhanced
            .instructions
            .into_iter()
            .map(try_into_instruction)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((instructions, enhanced.estimated_liquidation_price_usd))
    }

    pub async fn build_isolated_market_order_tx(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let transfer_amount = collateral_transfer_amount(&collateral)?;
        let tp_sl = match bracket {
            Some(bracket) => Some(
                bracket
                    .try_to_tp_sl_config_for_side(side)
                    .map_err(PhoenixHttpError::UnsupportedFeature)?,
            ),
            None => None,
        };

        let request = PlaceIsolatedMarketOrderRequest {
            authority: authority.to_string(),
            symbol: symbol.to_string(),
            side: side.to_api_string().to_string(),
            num_base_lots: Some(num_base_lots),
            transfer_amount,
            allow_cross_and_isolated_for_asset: Some(allow_cross_and_isolated),
            tp_sl,
            ..Default::default()
        };

        self.build_isolated_market_order_tx_with_request(request)
            .await
    }

    pub async fn build_isolated_market_order_tx_with_request(
        &self,
        request: PlaceIsolatedMarketOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let api_ixs: Vec<ApiInstructionResponse> = self
            .http
            .post_json("/v1/ix/place-isolated-market-order", &request)
            .await?;

        api_ixs.into_iter().map(try_into_instruction).collect()
    }

    pub async fn build_isolated_market_order_tx_enhanced(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        let transfer_amount = collateral_transfer_amount(&collateral)?;
        let tp_sl = match bracket {
            Some(bracket) => Some(
                bracket
                    .try_to_tp_sl_config_for_side(side)
                    .map_err(PhoenixHttpError::UnsupportedFeature)?,
            ),
            None => None,
        };

        let request = PlaceIsolatedMarketOrderRequest {
            authority: authority.to_string(),
            symbol: symbol.to_string(),
            side: side.to_api_string().to_string(),
            num_base_lots: Some(num_base_lots),
            transfer_amount,
            allow_cross_and_isolated_for_asset: Some(allow_cross_and_isolated),
            tp_sl,
            ..Default::default()
        };

        self.build_isolated_market_order_tx_enhanced_with_request(request)
            .await
    }

    pub async fn build_isolated_market_order_tx_enhanced_with_request(
        &self,
        request: PlaceIsolatedMarketOrderRequest,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        let enhanced: PlaceIsolatedMarketOrderEnhancedResponse = self
            .http
            .post_json("/v1/ix/place-isolated-market-order-enhanced", &request)
            .await?;

        let instructions = enhanced
            .instructions
            .into_iter()
            .map(try_into_instruction)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((instructions, enhanced.estimated_liquidation_price_usd))
    }
}

fn collateral_transfer_amount(
    collateral: &Option<IsolatedCollateralFlow>,
) -> Result<u64, PhoenixHttpError> {
    match collateral {
        Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }) => Ok(*collateral),
        Some(IsolatedCollateralFlow::Deposit { .. }) => Err(PhoenixHttpError::ApiError {
            status: 0,
            message: "IsolatedCollateralFlow::Deposit is not supported by the server-side \
                      endpoint; use TransferFromCrossMargin instead"
                .to_string(),
            error_code: None,
        }),
        None => Ok(0),
    }
}

fn try_into_instruction(api_ix: ApiInstructionResponse) -> Result<Instruction, PhoenixHttpError> {
    let program_id: Pubkey = api_ix
        .program_id
        .parse()
        .map_err(|e| PhoenixHttpError::ParseFailed(format!("Invalid program_id pubkey: {}", e)))?;

    let accounts = api_ix
        .keys
        .into_iter()
        .map(|meta| {
            let pubkey: Pubkey = meta.pubkey.parse().map_err(|e| {
                PhoenixHttpError::ParseFailed(format!("Invalid account pubkey: {}", e))
            })?;
            Ok(if meta.is_writable {
                AccountMeta::new(pubkey, meta.is_signer)
            } else {
                AccountMeta::new_readonly(pubkey, meta.is_signer)
            })
        })
        .collect::<Result<Vec<_>, PhoenixHttpError>>()?;

    Ok(Instruction::new_with_bytes(
        program_id,
        &api_ix.data,
        accounts,
    ))
}
