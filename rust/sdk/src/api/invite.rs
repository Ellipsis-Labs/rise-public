use serde::Serialize;
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::phoenix_rise_types::PhoenixHttpError;

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
                "/v1/invite/activate-with-referral",
                &ActivateInviteWithReferralRequest {
                    authority: authority.to_string(),
                    referral_code,
                },
            )
            .await?;
        Ok(response.trader_pda)
    }
}

#[derive(Serialize)]
struct ActivateInviteRequest<'a> {
    authority: String,
    code: &'a str,
}

#[derive(Serialize)]
struct ActivateInviteWithReferralRequest<'a> {
    authority: String,
    referral_code: &'a str,
}

#[derive(serde::Deserialize)]
struct ActivateInviteResponse {
    trader_pda: String,
}
