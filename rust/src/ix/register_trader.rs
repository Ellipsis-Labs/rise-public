//! Register trader instruction construction.
//!
//! This module provides instruction building for registering a new trader
//! account on the Phoenix protocol.

use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    register_trader_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for registering a new trader account.
#[derive(Debug, Clone)]
pub struct RegisterTraderParams {
    /// The payer for account creation (writable signer).
    payer: Pubkey,
    /// The trader authority (readonly).
    trader: Pubkey,
    /// The trader PDA account to be created (writable).
    trader_account: Pubkey,
    /// Maximum number of positions the account can hold.
    /// Cross margin: 128, Isolated margin: 1.
    max_positions: u64,
    /// The PDA index for trader account derivation (0-255).
    trader_pda_index: u8,
    /// The subaccount index.
    /// 0 for cross-margin, 1-100 for isolated margin.
    subaccount_index: u8,
}

impl RegisterTraderParams {
    /// Start building with the builder pattern.
    pub fn builder() -> RegisterTraderParamsBuilder {
        RegisterTraderParamsBuilder::new()
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn max_positions(&self) -> u64 {
        self.max_positions
    }

    pub fn trader_pda_index(&self) -> u8 {
        self.trader_pda_index
    }

    pub fn subaccount_index(&self) -> u8 {
        self.subaccount_index
    }
}

/// Builder for `RegisterTraderParams`.
#[derive(Default)]
pub struct RegisterTraderParamsBuilder {
    payer: Option<Pubkey>,
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    max_positions: Option<u64>,
    trader_pda_index: Option<u8>,
    subaccount_index: Option<u8>,
}

impl RegisterTraderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn max_positions(mut self, max_positions: u64) -> Self {
        self.max_positions = Some(max_positions);
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

    pub fn build(self) -> Result<RegisterTraderParams, PhoenixIxError> {
        Ok(RegisterTraderParams {
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            max_positions: self
                .max_positions
                .ok_or(PhoenixIxError::MissingField("max_positions"))?,
            trader_pda_index: self
                .trader_pda_index
                .ok_or(PhoenixIxError::MissingField("trader_pda_index"))?,
            subaccount_index: self
                .subaccount_index
                .ok_or(PhoenixIxError::MissingField("subaccount_index"))?,
        })
    }
}

/// Create a register trader instruction.
///
/// Registers a new trader account on the Phoenix protocol. The payer
/// covers rent for account creation.
///
/// # Arguments
///
/// * `params` - The register trader parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
pub fn create_register_trader_ix(
    params: RegisterTraderParams,
) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    let data = encode_register_trader(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate(params: &RegisterTraderParams) -> Result<(), PhoenixIxError> {
    if params.max_positions() == 0 || params.max_positions() > 128 {
        return Err(PhoenixIxError::MissingField(
            "max_positions must be between 1 and 128",
        ));
    }
    if params.subaccount_index() > 100 {
        return Err(PhoenixIxError::InvalidSubaccountIndex);
    }
    Ok(())
}

fn encode_register_trader(params: &RegisterTraderParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(18);

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&register_trader_discriminant());

    // max_positions (8 bytes, little-endian u64)
    data.extend_from_slice(&params.max_positions().to_le_bytes());

    // trader_pda_index (1 byte)
    data.push(params.trader_pda_index());

    // subaccount_index (1 byte)
    data.push(params.subaccount_index());

    data
}

fn build_accounts(params: &RegisterTraderParams) -> Vec<AccountMeta> {
    vec![
        // LogAccountGroupAccounts (2 accounts)
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        // RegisterTraderInstructionGroupAccounts
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::writable_signer(params.payer()),
        AccountMeta::readonly(params.trader()),
        AccountMeta::writable(params.trader_account()),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_cross_margin_params() -> RegisterTraderParams {
        RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .max_positions(128)
            .trader_pda_index(0)
            .subaccount_index(0)
            .build()
            .unwrap()
    }

    fn build_isolated_margin_params() -> RegisterTraderParams {
        RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .max_positions(1)
            .trader_pda_index(0)
            .subaccount_index(1)
            .build()
            .unwrap()
    }

    #[test]
    fn test_create_register_trader_ix_cross_margin() {
        let params = build_cross_margin_params();
        let ix = create_register_trader_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 7);
        assert_eq!(&ix.data[..8], &register_trader_discriminant());
    }

    #[test]
    fn test_create_register_trader_ix_isolated_margin() {
        let params = build_isolated_margin_params();
        let ix = create_register_trader_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 7);
        assert_eq!(&ix.data[..8], &register_trader_discriminant());
    }

    #[test]
    fn test_register_trader_data_encoding() {
        let params = RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .max_positions(128)
            .trader_pda_index(0)
            .subaccount_index(5)
            .build()
            .unwrap();

        let ix = create_register_trader_ix(params).unwrap();

        // Total data: 8 (discriminant) + 8 (max_positions) + 1 (pda_index) + 1
        // (subaccount_index) = 18
        assert_eq!(ix.data.len(), 18);

        // max_positions = 128 as u64 LE
        assert_eq!(&ix.data[8..16], &128u64.to_le_bytes());

        // trader_pda_index = 0
        assert_eq!(ix.data[16], 0);

        // subaccount_index = 5
        assert_eq!(ix.data[17], 5);
    }

    #[test]
    fn test_register_trader_account_order() {
        let payer = Pubkey::new_unique();
        let trader = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();

        let params = RegisterTraderParams::builder()
            .payer(payer)
            .trader(trader)
            .trader_account(trader_account)
            .max_positions(128)
            .trader_pda_index(0)
            .subaccount_index(0)
            .build()
            .unwrap();

        let ix = create_register_trader_ix(params).unwrap();

        // Account 0: *PHOENIX_PROGRAM_ID (readonly)
        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable);

        // Account 1: *PHOENIX_LOG_AUTHORITY (readonly)
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert!(!ix.accounts[1].is_signer);
        assert!(!ix.accounts[1].is_writable);

        // Account 2: *PHOENIX_GLOBAL_CONFIGURATION (readonly)
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert!(!ix.accounts[2].is_signer);
        assert!(!ix.accounts[2].is_writable);

        // Account 3: payer (writable signer)
        assert_eq!(ix.accounts[3].pubkey, payer);
        assert!(ix.accounts[3].is_signer);
        assert!(ix.accounts[3].is_writable);

        // Account 4: trader (readonly)
        assert_eq!(ix.accounts[4].pubkey, trader);
        assert!(!ix.accounts[4].is_signer);
        assert!(!ix.accounts[4].is_writable);

        // Account 5: trader_account (writable)
        assert_eq!(ix.accounts[5].pubkey, trader_account);
        assert!(!ix.accounts[5].is_signer);
        assert!(ix.accounts[5].is_writable);

        // Account 6: SYSTEM_PROGRAM_ID (readonly)
        assert_eq!(ix.accounts[6].pubkey, SYSTEM_PROGRAM_ID);
        assert!(!ix.accounts[6].is_signer);
        assert!(!ix.accounts[6].is_writable);
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .build();

        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }

    #[test]
    fn test_invalid_subaccount_index() {
        let params = RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .max_positions(1)
            .trader_pda_index(0)
            .subaccount_index(101)
            .build()
            .unwrap();

        let result = create_register_trader_ix(params);
        assert!(matches!(
            result,
            Err(PhoenixIxError::InvalidSubaccountIndex)
        ));
    }

    #[test]
    fn test_invalid_max_positions_zero() {
        let params = RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .max_positions(0)
            .trader_pda_index(0)
            .subaccount_index(0)
            .build()
            .unwrap();

        let result = create_register_trader_ix(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_max_positions_too_large() {
        let params = RegisterTraderParams::builder()
            .payer(Pubkey::new_unique())
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .max_positions(129)
            .trader_pda_index(0)
            .subaccount_index(0)
            .build()
            .unwrap();

        let result = create_register_trader_ix(params);
        assert!(result.is_err());
    }
}
