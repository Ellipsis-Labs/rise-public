use serde::Serialize;
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    CollateralHistoryQueryParams, CollateralHistoryResponse, PhoenixHttpError, TraderKey,
};

pub struct CollateralClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl CollateralClient<'_> {
    pub async fn get_user_collateral_history(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.get_collateral_history_internal(authority, params)
            .await
    }

    pub async fn get_trader_collateral_history(
        &self,
        trader_key: &TraderKey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_collateral_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_collateral_history_internal(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        let query = CollateralHistoryRequestQuery {
            pda_index: params.pda_index,
            limit: params.request.limit,
            next_cursor: params.request.next_cursor.as_deref(),
            prev_cursor: params.request.prev_cursor.as_deref(),
            cursor: params.request.cursor.as_deref(),
        };

        self.http
            .get_json_with_query(&format!("/trader/{}/collateral-history", authority), &query)
            .await
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollateralHistoryRequestQuery<'a> {
    pda_index: u8,
    limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prev_cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
}
