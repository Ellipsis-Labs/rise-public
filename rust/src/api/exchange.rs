use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::{
    ExchangeKeysView, ExchangeResponse, ExchangeSnapshotView, ExchangeStatusView, PhoenixHttpError,
};

pub struct ExchangeClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl ExchangeClient<'_> {
    pub async fn get_exchange(&self) -> Result<ExchangeResponse, PhoenixHttpError> {
        self.http.get_json("/v1/view/exchange").await
    }

    pub async fn get_keys(&self) -> Result<ExchangeKeysView, PhoenixHttpError> {
        self.http.get_json("/v1/view/exchange/keys").await
    }

    pub async fn get_snapshot(&self) -> Result<ExchangeSnapshotView, PhoenixHttpError> {
        self.http.get_json("/v1/exchange/snapshot").await
    }

    pub async fn get_status(&self) -> Result<ExchangeStatusView, PhoenixHttpError> {
        self.http.get_json("/exchange/status").await
    }
}
