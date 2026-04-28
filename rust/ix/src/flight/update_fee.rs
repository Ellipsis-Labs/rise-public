//! Flight: Update Fee instruction construction.
//!
//! Mirrors the TS SDK's `buildUpdateFeeIx`
//! (`ts/src/flight/core/ixBuilders/UpdateFee`).

use solana_pubkey::Pubkey;

use crate::ix::constants::PHOENIX_PROGRAM_ID;
use crate::ix::error::PhoenixIxError;
use crate::ix::flight::constants::{
    FLIGHT_PROGRAM_ID, flight_update_fee_discriminant, get_flight_builder_state_address,
    get_flight_global_state_address,
};
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for updating a registered builder's fee on the Flight program.
#[derive(Debug, Clone)]
pub struct UpdateFeeParams {
    /// The builder's authority (readonly signer)
    trader_authority: Pubkey,
    /// New fee charged by this builder in basis points.
    fee_bps: u64,
}

impl UpdateFeeParams {
    pub fn builder() -> UpdateFeeParamsBuilder {
        UpdateFeeParamsBuilder::new()
    }

    pub fn trader_authority(&self) -> Pubkey {
        self.trader_authority
    }

    pub fn fee_bps(&self) -> u64 {
        self.fee_bps
    }
}

#[derive(Default)]
pub struct UpdateFeeParamsBuilder {
    trader_authority: Option<Pubkey>,
    fee_bps: Option<u64>,
}

impl UpdateFeeParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_authority(mut self, trader_authority: Pubkey) -> Self {
        self.trader_authority = Some(trader_authority);
        self
    }

    pub fn fee_bps(mut self, fee_bps: u64) -> Self {
        self.fee_bps = Some(fee_bps);
        self
    }

    pub fn build(self) -> Result<UpdateFeeParams, PhoenixIxError> {
        Ok(UpdateFeeParams {
            trader_authority: self
                .trader_authority
                .ok_or(PhoenixIxError::MissingField("trader_authority"))?,
            fee_bps: self
                .fee_bps
                .ok_or(PhoenixIxError::MissingField("fee_bps"))?,
        })
    }
}

/// Create a Flight `update_fee` instruction.
pub fn create_update_fee_ix(params: UpdateFeeParams) -> Result<Instruction, PhoenixIxError> {
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&flight_update_fee_discriminant());
    data.extend_from_slice(&params.fee_bps().to_le_bytes());

    let accounts = vec![
        AccountMeta::readonly(get_flight_global_state_address()),
        AccountMeta::readonly(PHOENIX_PROGRAM_ID),
        AccountMeta::readonly_signer(params.trader_authority()),
        AccountMeta::writable(get_flight_builder_state_address(&params.trader_authority())),
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
    fn test_missing_fields() {
        let result = UpdateFeeParams::builder().fee_bps(1).build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("trader_authority"))
        ));

        let result = UpdateFeeParams::builder()
            .trader_authority(Pubkey::new_unique())
            .build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("fee_bps"))
        ));
    }

    #[test]
    fn test_data_encoding() {
        let params = UpdateFeeParams::builder()
            .trader_authority(Pubkey::new_unique())
            .fee_bps(2500)
            .build()
            .unwrap();

        let ix = create_update_fee_ix(params).unwrap();

        assert_eq!(ix.program_id, FLIGHT_PROGRAM_ID);
        assert_eq!(ix.data.len(), 16);
        assert_eq!(&ix.data[..8], &flight_update_fee_discriminant());
        assert_eq!(&ix.data[8..16], &2500u64.to_le_bytes());
    }

    #[test]
    fn test_account_layout() {
        let authority = Pubkey::new_unique();
        let params = UpdateFeeParams::builder()
            .trader_authority(authority)
            .fee_bps(1)
            .build()
            .unwrap();

        let ix = create_update_fee_ix(params).unwrap();
        assert_eq!(ix.accounts.len(), 4);

        assert_eq!(ix.accounts[0].pubkey, get_flight_global_state_address());
        assert!(!ix.accounts[0].is_writable);

        assert_eq!(ix.accounts[1].pubkey, PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[1].is_writable);

        assert_eq!(ix.accounts[2].pubkey, authority);
        assert!(ix.accounts[2].is_signer);
        assert!(!ix.accounts[2].is_writable);

        assert_eq!(
            ix.accounts[3].pubkey,
            get_flight_builder_state_address(&authority)
        );
        assert!(ix.accounts[3].is_writable);
    }
}
