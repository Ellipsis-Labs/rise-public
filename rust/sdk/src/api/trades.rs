use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    PhoenixHttpError, TradeHistoryQueryParams, TradeHistoryResponse, TraderKey,
};

pub struct TradesClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradesClient<'_> {
    pub async fn get_trader_trade_history(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.get_trade_history_internal(authority, params).await
    }

    pub async fn get_trader_trade_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_trade_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_trade_history_internal(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/trader/{}/trades-history", authority), &params)
            .await
    }
}
