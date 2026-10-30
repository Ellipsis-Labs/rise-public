use phoenix_rise_types::prelude::{PnlPoint, PnlQueryParams, TraderActivityState, TraderView};
use phoenix_rise_types::twap::{TwapOrdersQueryParams, TwapSnapshot};
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::http_error::PhoenixHttpError;
use crate::trader_key::TraderKey;

pub struct TradersClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradersClient<'_> {
    /// Returns the trader's TWAP accounts, including dust execution parameters.
    pub async fn get_twap(
        &self,
        authority: &Pubkey,
        params: TwapOrdersQueryParams,
    ) -> Result<TwapSnapshot, PhoenixHttpError> {
        self.http
            .get_json_with_query(&format!("/v1/trader/{authority}/twap"), &params)
            .await
    }

    pub async fn get_trader_by_pubkey(
        &self,
        trader_pubkey: &Pubkey,
    ) -> Result<TraderView, PhoenixHttpError> {
        self.http
            .get_json(&format!("/v1/view/trader/{trader_pubkey}"))
            .await
    }

    /// Returns the trader subaccount for `authority` at `pda_index` and
    /// `subaccount_index`, or `None` when that subaccount does not exist
    /// on-chain: it has not been created yet, or it was closed.
    pub async fn get_trader_subaccount(
        &self,
        authority: &Pubkey,
        pda_index: u8,
        subaccount_index: u8,
    ) -> Result<Option<TraderView>, PhoenixHttpError> {
        let trader_key =
            TraderKey::from_authority_with_idx(*authority, pda_index, subaccount_index);
        match self.get_trader_by_pubkey(&trader_key.pda()).await {
            Ok(trader) if trader.state == TraderActivityState::Uninitialized => Ok(None),
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    use serde_json::{Value, json};

    use super::*;
    use crate::http_client::PhoenixHttpClient;

    /// Serves one request with `status` and `body`; the handle yields the
    /// requested path.
    fn serve_once(status: &'static str, body: String) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request should connect");
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).expect("request should read");
            let request = String::from_utf8_lossy(&buffer[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_string();
            let response = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\nconnection: \
                 close\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("response should write");
            path
        });
        (base_url, handle)
    }

    fn trader_view_json(state: &str, trader_key: &Pubkey, authority: &Pubkey) -> String {
        let zero = json!({ "value": 0, "decimals": 6, "ui": "0.000000" });
        let access = json!({ "immediate": false, "viaColdActivation": false });
        let mut view = json!({
            "flags": 0,
            "state": state,
            "capabilities": {
                "placeLimitOrder": access,
                "placeMarketOrder": access,
                "riskIncreasingTrade": access,
                "riskReducingTrade": access,
                "depositCollateral": access,
                "withdrawCollateral": access,
            },
            "traderKey": trader_key.to_string(),
            "traderPdaIndex": 0,
            "traderSubaccountIndex": 1,
            "authority": authority.to_string(),
            "riskState": "zeroCollateralNoPositions",
            "riskTier": "safe",
            "positions": [],
            "limitOrders": {},
            "makerFeeOverrideMultiplier": 1.0,
            "takerFeeOverrideMultiplier": 1.0,
            "maxPositions": 0,
            "lastDepositSlot": 0,
            "isInActiveTraders": false,
        });
        let Value::Object(fields) = &mut view else {
            unreachable!("trader view is a JSON object");
        };
        for field in [
            "collateralBalance",
            "effectiveCollateral",
            "effectiveCollateralForWithdrawals",
            "unrealizedPnl",
            "discountedUnrealizedPnl",
            "unsettledFundingOwed",
            "accumulatedFunding",
            "portfolioValue",
            "maintenanceMargin",
            "cancelMargin",
            "initialMargin",
            "initialMarginForWithdrawals",
        ] {
            fields.insert(field.to_string(), zero.clone());
        }
        view.to_string()
    }

    async fn get_subaccount(
        status: &'static str,
        body: impl FnOnce(&Pubkey, &Pubkey) -> String,
    ) -> Option<TraderView> {
        let authority = Pubkey::new_unique();
        let trader_key = TraderKey::from_authority_with_idx(authority, 0, 1).pda();
        let (base_url, server) = serve_once(status, body(&trader_key, &authority));
        let client = PhoenixHttpClient::new(base_url).expect("client should build");

        let trader = client
            .traders()
            .get_trader_subaccount(&authority, 0, 1)
            .await
            .expect("request should succeed");

        assert_eq!(
            server.join().expect("test server should finish"),
            format!("/v1/view/trader/{trader_key}")
        );
        trader
    }

    #[tokio::test]
    async fn get_trader_subaccount_returns_none_for_unknown_trader() {
        let trader = get_subaccount("404 Not Found", |_, _| {
            json!({ "error": "Trader not found" }).to_string()
        })
        .await;

        assert!(trader.is_none());
    }

    #[tokio::test]
    async fn get_trader_subaccount_returns_none_for_uninitialized_trader() {
        let trader = get_subaccount("200 OK", |trader_key, authority| {
            trader_view_json("uninitialized", trader_key, authority)
        })
        .await;

        assert!(trader.is_none());
    }

    #[tokio::test]
    async fn get_trader_subaccount_returns_initialized_trader() {
        let trader = get_subaccount("200 OK", |trader_key, authority| {
            trader_view_json("cold", trader_key, authority)
        })
        .await
        .expect("initialized trader should be returned");

        assert_eq!(trader.state, TraderActivityState::Cold);
        assert_eq!(trader.trader_subaccount_index, 1);
    }
}
