use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{ApiCandle, CandlesQueryParams, PhoenixHttpError};

pub struct CandlesClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl CandlesClient<'_> {
    pub async fn get_candles(
        &self,
        params: CandlesQueryParams,
    ) -> Result<Vec<ApiCandle>, PhoenixHttpError> {
        let symbol = params.symbol.to_ascii_uppercase();
        self.http
            .get_json_with_query(&format!("/v1/candles/{}", symbol), &params)
            .await
    }
}
