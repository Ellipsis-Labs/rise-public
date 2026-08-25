use phoenix_rise_types::prelude::{
    ApiCandle, CandlesQueryParams, CandlesV2QueryParams, CandlesV2Response,
};

use crate::http_client::HttpClientInner;
use crate::http_error::PhoenixHttpError;

const DEFAULT_CANDLES_V2_LIMIT: i64 = 1_000;

fn normalize_candles_v2_query(params: &mut CandlesV2QueryParams) {
    if let CandlesV2QueryParams::Initial(params) = params {
        params.limit.get_or_insert(DEFAULT_CANDLES_V2_LIMIT);
    }
}

pub struct CandlesClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl CandlesClient<'_> {
    /// Fetch canonical candles v2 with explicit range or cursor pagination.
    pub async fn get_candles_v2<Q>(&self, params: Q) -> Result<CandlesV2Response, PhoenixHttpError>
    where
        Q: Into<CandlesV2QueryParams>,
    {
        let mut params = params.into();
        normalize_candles_v2_query(&mut params);
        let symbol = params.symbol().to_ascii_uppercase();
        self.http
            .get_json_with_query(&format!("/v1/candles_v2/{symbol}"), &params)
            .await
    }

    /// Fetch finalized candles using the legacy request and response shape.
    ///
    /// This remains on the legacy endpoint to preserve exchange-only defaults
    /// and server-relative finalized windows.
    pub async fn get_candles(
        &self,
        params: CandlesQueryParams,
    ) -> Result<Vec<ApiCandle>, PhoenixHttpError> {
        let symbol = params.symbol.to_ascii_uppercase();
        self.http
            .get_json_with_query(&format!("/v1/candles/{symbol}"), &params)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candles_v2_initial_query_defaults_to_one_thousand_bars() {
        let mut initial = phoenix_rise_types::prelude::CandlesV2InitialQueryParams::new(
            "SOL",
            phoenix_rise_types::prelude::Timeframe::Minute1,
            0,
            60_000,
        )
        .into();

        normalize_candles_v2_query(&mut initial);

        let CandlesV2QueryParams::Initial(initial) = initial else {
            panic!("expected initial query");
        };
        assert_eq!(initial.limit, Some(DEFAULT_CANDLES_V2_LIMIT));

        let mut explicit = phoenix_rise_types::prelude::CandlesV2InitialQueryParams::new(
            "SOL",
            phoenix_rise_types::prelude::Timeframe::Minute1,
            0,
            60_000,
        )
        .with_limit(10_000)
        .into();
        normalize_candles_v2_query(&mut explicit);
        let CandlesV2QueryParams::Initial(explicit) = explicit else {
            panic!("expected initial query");
        };
        assert_eq!(explicit.limit, Some(10_000));

        let mut cursor =
            phoenix_rise_types::prelude::CandlesV2CursorQueryParams::new("SOL", "opaque-cursor")
                .into();
        normalize_candles_v2_query(&mut cursor);
        assert!(matches!(cursor, CandlesV2QueryParams::Cursor(_)));
    }
}
