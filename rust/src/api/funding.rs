use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    FundingHistoryQueryParams, FundingHistoryResponse, FundingHourlyHistoryResponse,
    FundingHourlyQuery, FundingRateHistoryQuery, FundingRateHistoryResponse, PhoenixHttpError,
    TraderKey,
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
            .get_json_with_query(
                &format!("/v1/trader/{}/funding-history", authority),
                &params,
            )
            .await
    }

    pub async fn get_user_hourly_funding_history(
        &self,
        user_pubkey: &Pubkey,
        params: FundingHourlyQuery,
    ) -> Result<FundingHourlyHistoryResponse, PhoenixHttpError> {
        self.http
            .get_json_with_query(
                &format!("/v1/users/{}/funding-hourly", user_pubkey),
                &params,
            )
            .await
    }

    pub async fn get_market_funding_rate_history(
        &self,
        symbol: &str,
        params: FundingRateHistoryQuery,
    ) -> Result<FundingRateHistoryResponse, PhoenixHttpError> {
        let symbol = symbol.trim().to_ascii_uppercase();
        self.http
            .get_json_with_query(&format!("/v1/funding/{}/rates", symbol), &params)
            .await
    }
}
