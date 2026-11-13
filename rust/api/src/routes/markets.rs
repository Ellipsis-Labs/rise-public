use phoenix_rise_types::prelude::{
    CommodityMarketCalendarResponse, ExchangeMarketConfig, MarkPriceResponse,
    MarketCalendarResponse, MarketView, NextCommodityMarketTransition,
    NextMarketCalendarTransition, OrderbookView, PriceHistoryQuery, PriceHistoryResponse,
};

use crate::http_client::HttpClientInner;
use crate::http_error::PhoenixHttpError;

pub struct MarketsClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl MarketsClient<'_> {
    pub async fn get_markets(&self) -> Result<Vec<ExchangeMarketConfig>, PhoenixHttpError> {
        self.http.get_json("/v1/view/exchange/markets").await
    }

    pub async fn get_market(&self, symbol: &str) -> Result<ExchangeMarketConfig, PhoenixHttpError> {
        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json(&format!("/v1/view/exchange/market/{}", symbol))
            .await
    }

    /// Fetches the detailed market view (`GET /v1/view/market/{symbol}`).
    pub async fn get_market_view(&self, symbol: &str) -> Result<MarketView, PhoenixHttpError> {
        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json(&format!("/v1/view/market/{}", symbol))
            .await
    }

    /// Fetches mark-price history (`GET /v1/prices/{symbol}/history`).
    pub async fn get_price_history(
        &self,
        symbol: &str,
        query: PriceHistoryQuery,
    ) -> Result<PriceHistoryResponse, PhoenixHttpError> {
        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json_with_query(&format!("/v1/prices/{}/history", symbol), &query)
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

        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json_with_query(
                &format!("/v1/view/orderbook/{}", symbol),
                &Query { include_splines },
            )
            .await
    }

    pub async fn get_next_market_calendar_transition(
        &self,
        symbol: &str,
    ) -> Result<NextMarketCalendarTransition, PhoenixHttpError> {
        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json(&format!(
                "/v1/market/{}/next-market-calendar-transition",
                symbol
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
        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json(&format!("/v1/market/{}/market-calendar", symbol))
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
        let symbol = self.http.canonical_market_symbol(symbol).await;
        self.http
            .get_json(&format!("/v1/market/{}/mark-price", symbol))
            .await
    }
}
