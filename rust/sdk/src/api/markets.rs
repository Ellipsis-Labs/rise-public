use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    ExchangeMarketConfig, NextCommodityMarketTransition, PhoenixHttpError,
};

pub struct MarketsClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl MarketsClient<'_> {
    pub async fn get_markets(&self) -> Result<Vec<ExchangeMarketConfig>, PhoenixHttpError> {
        self.http.get_json("/exchange/markets").await
    }

    pub async fn get_market(&self, symbol: &str) -> Result<ExchangeMarketConfig, PhoenixHttpError> {
        let symbol_upper = symbol.to_ascii_uppercase();
        self.http
            .get_json(&format!("/exchange/market/{}", symbol_upper))
            .await
    }

    pub async fn get_next_commodity_market_transition(
        &self,
    ) -> Result<NextCommodityMarketTransition, PhoenixHttpError> {
        self.http
            .get_json("/v1/market/next-commodity-market-transition")
            .await
    }
}
