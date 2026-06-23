use serde::Serialize;
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::PhoenixHttpError;

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

#[derive(serde::Deserialize)]
struct ActivateInviteResponse {
    trader_pda: String,
}
