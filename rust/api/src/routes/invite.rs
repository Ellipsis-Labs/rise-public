use phoenix_rise_accounts::PhoenixAccountDecodeError;
use phoenix_rise_accounts::trader::TraderHeader;
use phoenix_rise_accounts::trader::capabilities::TRADER_CAPABILITY_CAN_DEPOSIT;
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::client_error::Error as RpcClientError;
use thiserror::Error;

use crate::http_client::HttpClientInner;
use crate::http_error::PhoenixHttpError;

const REFERRAL_ACTIVATION_AUTH_MESSAGE: &str = concat!(
    "Referral activation requires an authenticated user session for the authority wallet. ",
    "Build the Rise client with auth enabled and sign in as that wallet owner before calling ",
    "/v1/referral/activate."
);

pub struct InviteClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl InviteClient<'_> {
    pub async fn activate_invite(
        &self,
        authority: &Pubkey,
        code: &str,
    ) -> Result<String, PhoenixHttpError> {
        let response: ActivateInviteResponse = self
            .http
            .post_json(
                "/v1/invite/activate",
                &ActivateInviteRequest {
                    authority: authority.to_string(),
                    code,
                },
            )
            .await?;
        Ok(response.trader_pda)
    }

    pub async fn activate_referral(
        &self,
        authority: &Pubkey,
        referral_code: &str,
    ) -> Result<String, PhoenixHttpError> {
        let response: ActivateInviteResponse = self
            .http
            .post_json(
                "/v1/referral/activate",
                &ActivateReferralRequest {
                    authority: authority.to_string(),
                    referral_code,
                },
            )
            .await
            .map_err(with_referral_activation_auth_context)?;
        Ok(response.trader_pda)
    }

    pub async fn get_referral_activation_permission(
        &self,
    ) -> Result<ReferralActivationPermissionResponse, PhoenixHttpError> {
        self.http
            .get_json("/v1/referral/activation-permission")
            .await
    }

    pub async fn activate_referral_tx(
        &self,
        request: &ActivateReferralTxRequest,
    ) -> Result<ActivateReferralTxResponse, PhoenixHttpError> {
        self.http
            .post_json("/v1/referral/activate-tx", request)
            .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferralActivationTraderStatus {
    Missing,
    Registered,
    Activated,
}

impl ReferralActivationTraderStatus {
    pub const fn should_include_register_trader(self) -> bool {
        matches!(self, Self::Missing)
    }
}

#[derive(Debug, Error)]
pub enum ReferralActivationTraderStatusError {
    #[error("failed to fetch trader account {trader}: {source}")]
    Fetch {
        trader: Pubkey,
        source: RpcClientError,
    },

    #[error("failed to decode trader account {trader}: {source}")]
    Decode {
        trader: Pubkey,
        source: PhoenixAccountDecodeError,
    },
}

pub const fn has_referral_activation_capabilities(flags: u32) -> bool {
    flags & TRADER_CAPABILITY_CAN_DEPOSIT == TRADER_CAPABILITY_CAN_DEPOSIT
}

pub fn referral_activation_trader_status(flags: Option<u32>) -> ReferralActivationTraderStatus {
    match flags {
        None => ReferralActivationTraderStatus::Missing,
        Some(flags) if has_referral_activation_capabilities(flags) => {
            ReferralActivationTraderStatus::Activated
        }
        Some(_) => ReferralActivationTraderStatus::Registered,
    }
}

pub async fn fetch_referral_activation_trader_status(
    rpc: &RpcClient,
    trader: &Pubkey,
) -> Result<ReferralActivationTraderStatus, ReferralActivationTraderStatusError> {
    let Some(account) = rpc
        .get_account_with_commitment(trader, rpc.commitment())
        .await
        .map_err(|source| ReferralActivationTraderStatusError::Fetch {
            trader: *trader,
            source,
        })?
        .value
    else {
        return Ok(ReferralActivationTraderStatus::Missing);
    };

    let trader_header = TraderHeader::try_from_account_bytes(&account.data).map_err(|source| {
        ReferralActivationTraderStatusError::Decode {
            trader: *trader,
            source,
        }
    })?;
    Ok(referral_activation_trader_status(Some(
        trader_header.flags(),
    )))
}

fn with_referral_activation_auth_context(error: PhoenixHttpError) -> PhoenixHttpError {
    match error {
        PhoenixHttpError::Authentication {
            status,
            message,
            error_code,
        } if referral_activation_auth_context_applies(status, error_code.as_deref()) => {
            PhoenixHttpError::Authentication {
                status,
                message: format!("{REFERRAL_ACTIVATION_AUTH_MESSAGE} {message}"),
                error_code,
            }
        }
        other => other,
    }
}

fn referral_activation_auth_context_applies(status: Option<u16>, error_code: Option<&str>) -> bool {
    status == Some(401)
        || matches!(
            error_code,
            Some(
                "missing_access_token"
                    | "invalid_access_token"
                    | "access_token_expired"
                    | "session_missing"
                    | "no_auth_session"
                    | "user_only"
            )
        )
}

#[derive(Serialize)]
struct ActivateInviteRequest<'a> {
    authority: String,
    code: &'a str,
}

#[derive(Serialize)]
struct ActivateReferralRequest<'a> {
    authority: String,
    referral_code: &'a str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActivateReferralTxRequest {
    pub referral_code: String,
    pub trader_authority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_subaccount_index: Option<u8>,
    pub recent_blockhash: String,
    pub transaction: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActivateReferralTxResponse {
    #[serde(default)]
    pub signature: Option<String>,
    pub trader_pda: String,
    pub referral_code: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReferralActivationPermissionResponse {
    pub trader_onboarder: String,
    pub risk_authority: String,
    pub permission_account: String,
}

#[derive(Deserialize)]
struct ActivateInviteResponse {
    trader_pda: String,
}
