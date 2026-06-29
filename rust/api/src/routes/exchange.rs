use phoenix_rise_types::prelude::{
    ExchangeKeysView, ExchangeResponse, ExchangeSnapshotView, ExchangeStatusView,
};
use serde::{Deserialize, Serialize};

use crate::http_client::HttpClientInner;
use crate::http_error::PhoenixHttpError;

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

    pub async fn build_register_ixs(
        &self,
        request: &BuildRegisterIxsRequest,
    ) -> Result<BuildRegisterIxsResponse, PhoenixHttpError> {
        self.http
            .post_json("/v1/exchange/build-register-ixs", request)
            .await
    }

    pub async fn send_register_ixs(
        &self,
        request: &SendRegisterIxsRequest,
    ) -> Result<SendRegisterIxsResponse, PhoenixHttpError> {
        self.http
            .post_json("/v1/exchange/send-register-ixs", request)
            .await
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiInstructionResponse {
    pub data: Vec<u8>,
    pub keys: Vec<ApiAccountMeta>,
    pub program_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildRegisterIxsRequest {
    pub trader_authority: String,
    pub tx_fee_payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_positions: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildRegisterIxsResponse {
    pub instructions: Vec<ApiInstructionResponse>,
    pub trader_pda: String,
    pub trader_onboarder: String,
    pub tx_fee_payer: String,
    pub max_positions: u32,
    pub include_register_trader: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendRegisterIxsRequest {
    pub transaction: String,
    pub trader_authority: String,
    pub tx_fee_payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_positions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_subaccount_index: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendRegisterIxsResponse {
    pub signature: String,
    pub trader_pda: String,
    pub trader_onboarder: String,
    pub tx_fee_payer: String,
    pub max_positions: u32,
    pub include_register_trader: bool,
}
