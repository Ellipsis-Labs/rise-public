use serde::Serialize;
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    PhoenixHttpError, PnlPoint, PnlQueryParams, TraderKey, TraderStateResponse, TraderView,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TraderStateQuery {
    pda_index: u8,
}

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
                &format!("/trader/{authority}/state"),
                &TraderStateQuery { pda_index },
            )
            .await?;

        Ok(resp.traders)
    }

    pub async fn get_trader_subaccount(
        &self,
        authority: &Pubkey,
        pda_index: u8,
        subaccount_index: u8,
    ) -> Result<Option<TraderView>, PhoenixHttpError> {
        Ok(self
            .get_trader_internal(authority, pda_index)
            .await?
            .into_iter()
            .find(|trader| trader.trader_subaccount_index == subaccount_index))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trader_state_request_uses_public_route_and_pda_index_query() {
        assert_eq!(
            serde_urlencoded::to_string(TraderStateQuery { pda_index: 7 }).unwrap(),
            "pdaIndex=7"
        );
    }
}
