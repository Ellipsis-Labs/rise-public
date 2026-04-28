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
        self.http.get_json_with_query("/candles", &params).await
    }
}
