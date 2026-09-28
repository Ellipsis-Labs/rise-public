//! Flight: Set Onboarder Signer instruction construction.
//!
//! Mirrors the TS SDK's `buildSetOnboarderSignerIx`
//! (`ts/src/flight/core/ixBuilders/SetOnboarderSigner`).

use solana_pubkey::Pubkey;

use crate::FlightInstruction;
use crate::constants::PHOENIX_PROGRAM_ID;
use crate::error::PhoenixIxError;
use crate::flight::constants::{
    FLIGHT_PROGRAM_ID, get_flight_builder_state_address, get_flight_global_state_address,
};
use crate::types::{AccountMeta, Instruction};

/// Parameters for setting a builder's onboarder signer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetOnboarderSignerParams {
    /// The builder's authority (readonly signer)
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    builder_authority: Pubkey,
    /// New onboarder signer.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    signer: Pubkey,
}

impl SetOnboarderSignerParams {
    pub fn builder() -> SetOnboarderSignerParamsBuilder {
        SetOnboarderSignerParamsBuilder::new()
    }

    pub fn builder_authority(&self) -> Pubkey {
        self.builder_authority
    }

    pub fn signer(&self) -> Pubkey {
        self.signer
    }
}

#[derive(Default)]
pub struct SetOnboarderSignerParamsBuilder {
    builder_authority: Option<Pubkey>,
    signer: Option<Pubkey>,
}

impl SetOnboarderSignerParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder_authority(mut self, builder_authority: Pubkey) -> Self {
        self.builder_authority = Some(builder_authority);
        self
    }

    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn build(self) -> Result<SetOnboarderSignerParams, PhoenixIxError> {
        Ok(SetOnboarderSignerParams {
            builder_authority: self
                .builder_authority
                .ok_or(PhoenixIxError::MissingField("builder_authority"))?,
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
        })
    }
}

/// Create a Flight `set_onboarder_signer` instruction.
pub fn create_set_onboarder_signer_ix(
    params: SetOnboarderSignerParams,
) -> Result<Instruction, PhoenixIxError> {
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(&FlightInstruction::SetOnboarderSigner.discriminant());
    data.extend_from_slice(&params.signer().to_bytes());

    let accounts = vec![
        AccountMeta::readonly(get_flight_global_state_address()?),
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly_signer(params.builder_authority()),
        AccountMeta::writable(get_flight_builder_state_address(
            &params.builder_authority(),
        )?),
    ];

    Ok(Instruction {
        program_id: FLIGHT_PROGRAM_ID,
        accounts,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_layout() {
        let authority = Pubkey::new_unique();
        let params = SetOnboarderSignerParams::builder()
            .builder_authority(authority)
            .signer(Pubkey::new_unique())
            .build()
            .unwrap();
        let ix = create_set_onboarder_signer_ix(params).unwrap();
        let actual: Vec<_> = ix
            .accounts
            .iter()
            .map(|a| (a.pubkey, a.is_writable, a.is_signer))
            .collect();
        assert_eq!(
            actual,
            [
                (get_flight_global_state_address().unwrap(), false, false),
                (*PHOENIX_PROGRAM_ID, false, false),
                (authority, false, true),
                (
                    get_flight_builder_state_address(&authority).unwrap(),
                    true,
                    false
                ),
            ]
        );
    }
}
