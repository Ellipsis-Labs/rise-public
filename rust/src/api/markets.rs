use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    CommodityMarketCalendarResponse, ExchangeMarketConfig, MarkPriceResponse,
    MarketCalendarResponse, NextCommodityMarketTransition, NextMarketCalendarTransition,
    OrderbookView, PhoenixHttpError,
};

pub struct MarketsClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl MarketsClient<'_> {
    pub async fn get_markets(&self) -> Result<Vec<ExchangeMarketConfig>, PhoenixHttpError> {
        self.http.get_json("/v1/view/exchange/markets").await
    }

    pub async fn get_market(&self, symbol: &str) -> Result<ExchangeMarketConfig, PhoenixHttpError> {
        let symbol_upper = symbol.to_ascii_uppercase();
        self.http
            .get_json(&format!("/v1/view/exchange/market/{}", symbol_upper))
            .await
    }

    pub async fn get_orderbook(
        &self,
        symbol: &str,
        include_splines: bool,
    ) -> Result<OrderbookView, PhoenixHttpError> {
        #[derive(serde::Serialize)]
        struct Query {
            include_splines: bool,
        }

        let symbol_upper = symbol.to_ascii_uppercase();
        self.http
            .get_json_with_query(
                &format!("/v1/view/orderbook/{}", symbol_upper),
                &Query { include_splines },
            )
            .await
    }

    pub async fn get_next_market_calendar_transition(
        &self,
        symbol: &str,
    ) -> Result<NextMarketCalendarTransition, PhoenixHttpError> {
        let symbol_upper = symbol.to_ascii_uppercase();
        self.http
            .get_json(&format!(
                "/v1/market/{}/next-market-calendar-transition",
                symbol_upper
            ))
            .await
    }

    pub async fn get_next_commodity_market_transition(
        &self,
    ) -> Result<NextCommodityMarketTransition, PhoenixHttpError> {
        self.http
            .get_json("/v1/market/next-commodity-market-transition")
            .await
    }

    pub async fn get_market_calendar(
        &self,
        symbol: &str,
    ) -> Result<MarketCalendarResponse, PhoenixHttpError> {
        let symbol_upper = symbol.to_ascii_uppercase();
        self.http
            .get_json(&format!("/v1/market/{}/market-calendar", symbol_upper))
            .await
    }

    pub async fn get_commodity_market_calendar(
        &self,
    ) -> Result<CommodityMarketCalendarResponse, PhoenixHttpError> {
        self.http.get_json("/v1/market/commodity-calendar").await
    }

    pub async fn get_mark_price(
        &self,
        symbol: &str,
    ) -> Result<MarkPriceResponse, PhoenixHttpError> {
        let symbol_upper = symbol.to_ascii_uppercase();
        self.http
            .get_json(&format!("/v1/market/{}/mark-price", symbol_upper))
            .await
    }
}
