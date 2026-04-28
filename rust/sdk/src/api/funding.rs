use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    FundingHistoryQueryParams, FundingHistoryResponse, PhoenixHttpError, TraderKey,
};

pub struct FundingClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl FundingClient<'_> {
    pub async fn get_user_funding_history(
        &self,
        authority: &Pubkey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.get_funding_history_internal(authority, params).await
    }

    pub async fn get_trader_funding_history(
        &self,
        trader_key: &TraderKey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_funding_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_funding_history_internal(
        &self,
        authority: &Pubkey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/trader/{}/funding-history", authority), &params)
            .await
    }
}
