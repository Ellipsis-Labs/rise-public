use phoenix_rise_types::prelude::{
    CollateralAssetsResponse, CollateralHistoryQueryParams, CollateralHistoryResponse,
};
use serde::Serialize;
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::http_error::PhoenixHttpError;
use crate::trader_key::TraderKey;

pub struct CollateralClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl CollateralClient<'_> {
    pub async fn get_assets(&self) -> Result<CollateralAssetsResponse, PhoenixHttpError> {
        self.http.get_json("/v1/collateral/assets").await
    }

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
        let trader_pda = trader_key.pda();
        let query = CollateralHistoryRequestQuery {
            limit: params.request.limit,
            next_cursor: params.request.next_cursor.as_deref(),
            prev_cursor: params.request.prev_cursor.as_deref(),
            cursor: params.request.cursor.as_deref(),
        };

        self.http
            .get_json_with_query(
                &format!("/v1/traders/{}/collateral-history", trader_pda),
                &query,
            )
            .await
    }

    async fn get_collateral_history_internal(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        let query = CollateralHistoryRequestQuery {
            limit: params.request.limit,
            next_cursor: params.request.next_cursor.as_deref(),
            prev_cursor: params.request.prev_cursor.as_deref(),
            cursor: params.request.cursor.as_deref(),
        };

        self.http
            .get_json_with_query(
                &format!("/v1/users/{}/collateral-history", authority),
                &query,
            )
            .await
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollateralHistoryRequestQuery<'a> {
    limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prev_cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
}
