use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    PhoenixHttpError, PnlPoint, PnlQueryParams, TraderKey, TraderView,
};

pub struct TradersClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradersClient<'_> {
    pub async fn get_trader_by_pubkey(
        &self,
        trader_pubkey: &Pubkey,
    ) -> Result<TraderView, PhoenixHttpError> {
        self.http
            .get_json(&format!("/v1/view/trader/{trader_pubkey}"))
            .await
    }

    pub async fn get_trader_subaccount(
        &self,
        authority: &Pubkey,
        pda_index: u8,
        subaccount_index: u8,
    ) -> Result<Option<TraderView>, PhoenixHttpError> {
        let trader_key =
            TraderKey::from_authority_with_idx(*authority, pda_index, subaccount_index);
        match self.get_trader_by_pubkey(&trader_key.pda()).await {
            Ok(trader) => Ok(Some(trader)),
            Err(err) if err.status() == Some(404) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub async fn get_user_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/v1/users/{authority}/pnl"), &params)
            .await
    }

    pub async fn get_trader_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.get_user_pnl(authority, params).await
    }

    pub async fn get_trader_pda_pnl(
        &self,
        trader_pda: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/v1/traders/{trader_pda}/pnl"), &params)
            .await
    }

    pub async fn get_trader_pnl_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.get_trader_pda_pnl(&trader_key.pda(), params).await
    }
}
