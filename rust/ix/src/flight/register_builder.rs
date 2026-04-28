//! Flight: Register Builder instruction construction.
//!
//! Mirrors the TS SDK's `buildRegisterBuilderIx`
//! (`ts/src/flight/core/ixBuilders/RegisterBuilder`).

use solana_pubkey::Pubkey;

use crate::ix::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use crate::ix::error::PhoenixIxError;
use crate::ix::flight::constants::{
    FLIGHT_PROGRAM_ID, flight_register_builder_discriminant, get_flight_builder_state_address,
    get_flight_global_state_address,
};
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for registering a new builder account on the Flight program.
#[derive(Debug, Clone)]
pub struct RegisterBuilderParams {
    /// The builder's authority (readonly signer)
    trader_authority: Pubkey,
    /// The builder's Phoenix trader subaccount PDA (writable)
    trader_account: Pubkey,
    /// Phoenix trader PDA index for `trader_account` derivation (0-255)
    trader_pda_index: u8,
    /// Phoenix subaccount index for `trader_account` derivation. `0` for
    /// cross-margin, `1-100` for isolated margin
    subaccount_index: u8,
    /// Fee charged by this builder in basis points.
    fee_bps: u64,
}

impl RegisterBuilderParams {
    /// Start building with the builder pattern.
    pub fn builder() -> RegisterBuilderParamsBuilder {
        RegisterBuilderParamsBuilder::new()
    }

    pub fn trader_authority(&self) -> Pubkey {
        self.trader_authority
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn trader_pda_index(&self) -> u8 {
        self.trader_pda_index
    }

    pub fn subaccount_index(&self) -> u8 {
        self.subaccount_index
    }

    pub fn fee_bps(&self) -> u64 {
        self.fee_bps
    }
}

/// Builder for `RegisterBuilderParams`.
///
/// `trader_pda_index` and `subaccount_index` default to 0 when not set,
/// matching the TS SDK's defaults.
#[derive(Default)]
pub struct RegisterBuilderParamsBuilder {
    trader_authority: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    trader_pda_index: Option<u8>,
    subaccount_index: Option<u8>,
    fee_bps: Option<u64>,
}

impl RegisterBuilderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_authority(mut self, trader_authority: Pubkey) -> Self {
        self.trader_authority = Some(trader_authority);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn trader_pda_index(mut self, trader_pda_index: u8) -> Self {
        self.trader_pda_index = Some(trader_pda_index);
        self
    }

    pub fn subaccount_index(mut self, subaccount_index: u8) -> Self {
        self.subaccount_index = Some(subaccount_index);
        self
    }

    pub fn fee_bps(mut self, fee_bps: u64) -> Self {
        self.fee_bps = Some(fee_bps);
        self
    }

    pub fn build(self) -> Result<RegisterBuilderParams, PhoenixIxError> {
        Ok(RegisterBuilderParams {
            trader_authority: self
                .trader_authority
                .ok_or(PhoenixIxError::MissingField("trader_authority"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            trader_pda_index: self.trader_pda_index.unwrap_or(0),
            subaccount_index: self.subaccount_index.unwrap_or(0),
            fee_bps: self
                .fee_bps
                .ok_or(PhoenixIxError::MissingField("fee_bps"))?,
        })
    }
}

/// Create a Flight `register_builder` instruction.
pub fn create_register_builder_ix(
    params: RegisterBuilderParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_register_builder(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: FLIGHT_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_register_builder(params: &RegisterBuilderParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(18);
    data.extend_from_slice(&flight_register_builder_discriminant());
    data.push(params.trader_pda_index());
    data.push(params.subaccount_index());
    data.extend_from_slice(&params.fee_bps().to_le_bytes());
    data
}

fn build_accounts(params: &RegisterBuilderParams) -> Vec<AccountMeta> {
    vec![
        AccountMeta::readonly(get_flight_global_state_address()),
        AccountMeta::readonly(PHOENIX_PROGRAM_ID),
        AccountMeta::writable_signer(params.trader_authority()),
        AccountMeta::writable(params.trader_account()),
        AccountMeta::writable(get_flight_builder_state_address(&params.trader_authority())),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_params() -> RegisterBuilderParams {
        RegisterBuilderParams::builder()
            .trader_authority(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .fee_bps(250)
            .build()
            .unwrap()
    }

    #[test]
    fn test_defaults() {
        let params = build_params();
        assert_eq!(params.trader_pda_index(), 0);
        assert_eq!(params.subaccount_index(), 0);
        assert_eq!(params.fee_bps(), 250);
    }

    #[test]
    fn test_missing_authority() {
        let result = RegisterBuilderParams::builder().fee_bps(1).build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("trader_authority"))
        ));
    }

    #[test]
    fn test_missing_fee_bps() {
        let result = RegisterBuilderParams::builder()
            .trader_authority(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("fee_bps"))
        ));
    }

    #[test]
    fn test_data_encoding() {
        let authority = Pubkey::new_unique();
        let params = RegisterBuilderParams::builder()
            .trader_authority(authority)
            .trader_account(Pubkey::new_unique())
            .trader_pda_index(3)
            .subaccount_index(7)
            .fee_bps(1000)
            .build()
            .unwrap();

        let ix = create_register_builder_ix(params).unwrap();

        assert_eq!(ix.program_id, FLIGHT_PROGRAM_ID);
        assert_eq!(ix.data.len(), 18);
        assert_eq!(&ix.data[..8], &flight_register_builder_discriminant());
        assert_eq!(ix.data[8], 3);
        assert_eq!(ix.data[9], 7);
        assert_eq!(&ix.data[10..18], &1000u64.to_le_bytes());
    }

    #[test]
    fn test_account_layout() {
        let authority = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let params = RegisterBuilderParams::builder()
            .trader_authority(authority)
            .trader_account(trader_account)
            .fee_bps(1)
            .build()
            .unwrap();

        let ix = create_register_builder_ix(params).unwrap();
        assert_eq!(ix.accounts.len(), 6);

        assert_eq!(ix.accounts[0].pubkey, get_flight_global_state_address());
        assert!(!ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable);

        assert_eq!(ix.accounts[1].pubkey, PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[1].is_writable);

        assert_eq!(ix.accounts[2].pubkey, authority);
        assert!(ix.accounts[2].is_signer);
        assert!(ix.accounts[2].is_writable);

        assert_eq!(ix.accounts[3].pubkey, trader_account);
        assert!(ix.accounts[3].is_writable);
        assert!(!ix.accounts[3].is_signer);

        assert_eq!(
            ix.accounts[4].pubkey,
            get_flight_builder_state_address(&authority)
        );
        assert!(ix.accounts[4].is_writable);

        assert_eq!(ix.accounts[5].pubkey, SYSTEM_PROGRAM_ID);
        assert!(!ix.accounts[5].is_writable);
    }
}
