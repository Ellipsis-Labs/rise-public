//! Trader position-authority delegation instruction construction.
//!
//! `DelegateTrader` updates a trader account's position authority. Passing the
//! trader wallet as the new position authority resets delegation.

use solana_pubkey::Pubkey;

use crate::constants::{PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction};

/// Parameters for updating a trader account's position authority.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DelegateTraderParams {
    /// Trader wallet authority. Must sign the transaction.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_wallet: Pubkey,
    /// Trader account whose position authority will be updated.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    /// New position authority. Use `trader_wallet` to reset delegation.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    new_position_authority: Pubkey,
}

impl DelegateTraderParams {
    /// Start building with the builder pattern.
    pub fn builder() -> DelegateTraderParamsBuilder {
        DelegateTraderParamsBuilder::new()
    }

    pub fn trader_wallet(&self) -> Pubkey {
        self.trader_wallet
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn new_position_authority(&self) -> Pubkey {
        self.new_position_authority
    }
}

/// Builder for [`DelegateTraderParams`].
#[derive(Default)]
pub struct DelegateTraderParamsBuilder {
    trader_wallet: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    new_position_authority: Option<Pubkey>,
}

impl DelegateTraderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn new_position_authority(mut self, new_position_authority: Pubkey) -> Self {
        self.new_position_authority = Some(new_position_authority);
        self
    }

    pub fn build(self) -> Result<DelegateTraderParams, PhoenixIxError> {
        Ok(DelegateTraderParams {
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            new_position_authority: self
                .new_position_authority
                .ok_or(PhoenixIxError::MissingField("new_position_authority"))?,
        })
    }
}

/// Create a Phoenix `delegate_trader` instruction.
///
/// The trader wallet authority, not the existing position authority, must sign.
/// To reset a delegated position authority, set `new_position_authority` to the
/// trader wallet.
pub fn create_delegate_trader_ix(
    params: DelegateTraderParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = crate::PhoenixInstruction::DelegateTrader
        .discriminant()
        .to_vec();
    let accounts = vec![
        // LogAccountGroupAccounts
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        // DelegateTraderInstructionGroupAccounts
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::readonly_signer(params.trader_wallet()),
        AccountMeta::writable(params.trader_account()),
        AccountMeta::readonly(params.new_position_authority()),
    ];

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_delegate_trader_instruction() {
        let trader_wallet = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let new_position_authority = Pubkey::new_unique();
        let params = DelegateTraderParams::builder()
            .trader_wallet(trader_wallet)
            .trader_account(trader_account)
            .new_position_authority(new_position_authority)
            .build()
            .unwrap();

        let ix = create_delegate_trader_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(
            ix.data,
            crate::PhoenixInstruction::DelegateTrader
                .discriminant()
                .to_vec()
        );
        assert_eq!(ix.accounts.len(), 6);
        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable);
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert_eq!(ix.accounts[3].pubkey, trader_wallet);
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);
        assert_eq!(ix.accounts[4].pubkey, trader_account);
        assert!(!ix.accounts[4].is_signer);
        assert!(ix.accounts[4].is_writable);
        assert_eq!(ix.accounts[5].pubkey, new_position_authority);
        assert!(!ix.accounts[5].is_signer);
        assert!(!ix.accounts[5].is_writable);
    }

    #[test]
    fn supports_reset_to_trader_wallet() {
        let trader_wallet = Pubkey::new_unique();
        let params = DelegateTraderParams::builder()
            .trader_wallet(trader_wallet)
            .trader_account(Pubkey::new_unique())
            .new_position_authority(trader_wallet)
            .build()
            .unwrap();

        let ix = create_delegate_trader_ix(params).unwrap();

        assert_eq!(ix.accounts[3].pubkey, trader_wallet);
        assert_eq!(ix.accounts[5].pubkey, trader_wallet);
    }
}
