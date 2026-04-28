use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    PhoenixHttpError, PnlPoint, PnlQueryParams, TraderStateResponse, TraderView,
};

pub struct TradersClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradersClient<'_> {
    pub async fn get_trader(
        &self,
        authority: &Pubkey,
    ) -> Result<Vec<TraderView>, PhoenixHttpError> {
        self.get_trader_internal(authority, 0).await
    }

    pub async fn get_trader_internal(
        &self,
        authority: &Pubkey,
        pda_index: u8,
    ) -> Result<Vec<TraderView>, PhoenixHttpError> {
        let resp: TraderStateResponse = self
            .http
            .get_json_with_query(
                &format!("/trader/{}/state", authority),
                &serde_json::json!({ "pdaIndex": pda_index }),
            )
            .await?;

        Ok(resp.traders)
    }

    pub async fn get_trader_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/trader/{}/pnl", authority), &params)
            .await
    }
}
