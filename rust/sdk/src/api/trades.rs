use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    PhoenixHttpError, TradeHistoryQueryParams, TradeHistoryResponse, TraderKey,
    UserLiquidationHistoryQueryParams, UserLiquidationHistoryResponse,
};

#[derive(serde::Serialize)]
struct TradeHistoryV2Query<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    market_symbol: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trader_pda_index: Option<i32>,
    limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
}

pub struct TradesClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradesClient<'_> {
    pub async fn get_user_liquidation_history(
        &self,
        authority: &Pubkey,
        params: UserLiquidationHistoryQueryParams,
    ) -> Result<UserLiquidationHistoryResponse, PhoenixHttpError> {
        self.http
            .get_json_with_query(
                &format!("/v1/users/{authority}/liquidation-history"),
                &params,
            )
            .await
    }

    pub async fn get_user_trade_history(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        let query = trade_history_v2_query(&params, Some(i32::from(params.pda_index)));
        self.http
            .get_json_with_query(&format!("/v1/users/{authority}/trades_v2"), &query)
            .await
    }

    pub async fn get_trader_trade_history(
        &self,
        trader_pubkey: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.get_legacy_trader_trade_history(trader_pubkey, params)
            .await
    }

    pub async fn get_trader_trade_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.get_trader_trade_history_by_pda(&trader_key.pda(), params)
            .await
    }

    pub async fn get_trader_trade_history_by_pda(
        &self,
        trader_pda: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        let query = trade_history_v2_query(&params, None);
        self.http
            .get_json_with_query(&format!("/v1/traders/{trader_pda}/trades_v2"), &query)
            .await
    }

    async fn get_legacy_trader_trade_history(
        &self,
        trader_pubkey: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.http
            .get_json_with_query(
                &format!("/v1/trader/{trader_pubkey}/trades-history"),
                &params,
            )
            .await
    }
}

fn trade_history_v2_query(
    params: &TradeHistoryQueryParams,
    trader_pda_index: Option<i32>,
) -> TradeHistoryV2Query<'_> {
    TradeHistoryV2Query {
        market_symbol: params.market_symbol.as_deref(),
        trader_pda_index,
        limit: params.limit.unwrap_or(100),
        cursor: params.cursor.as_deref(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_trade_history_v2_query_uses_trader_pda_index_name() {
        let params = TradeHistoryQueryParams::new()
            .with_pda_index(7)
            .with_limit(1);
        let query = serde_urlencoded::to_string(trade_history_v2_query(
            &params,
            Some(i32::from(params.pda_index)),
        ))
        .unwrap();

        assert!(query.contains("trader_pda_index=7"));
        assert!(query.contains("limit=1"));
        assert!(!query.contains("pdaIndex"));
    }
}
