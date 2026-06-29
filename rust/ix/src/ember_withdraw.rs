//! Ember withdraw instruction construction.
//!
//! This module provides instruction building for withdrawing Phoenix tokens
//! and receiving USDC via the Ember program.

use solana_pubkey::Pubkey;

use crate::EmberInstruction;
use crate::constants::{
    EMBER_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID, get_ember_state_address, get_ember_vault_address,
};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction};

/// Parameters for withdrawing Phoenix tokens and receiving USDC via Ember.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmberWithdrawParams {
    /// The trader's authority (wallet) - must sign.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    /// USDC mint (input - what user receives).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    usdc_mint: Pubkey,
    /// Phoenix token mint (output - what user gives).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    canonical_mint: Pubkey,
    /// Trader's USDC token account (destination).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_usdc_account: Pubkey,
    /// Trader's Phoenix token account (source).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_phoenix_account: Pubkey,
    /// Amount of Phoenix tokens to withdraw (None = full withdrawal).
    amount: Option<u64>,
}

impl EmberWithdrawParams {
    /// Start building with the builder pattern.
    pub fn builder() -> EmberWithdrawParamsBuilder {
        EmberWithdrawParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn usdc_mint(&self) -> Pubkey {
        self.usdc_mint
    }

    pub fn canonical_mint(&self) -> Pubkey {
        self.canonical_mint
    }

    pub fn trader_usdc_account(&self) -> Pubkey {
        self.trader_usdc_account
    }

    pub fn trader_phoenix_account(&self) -> Pubkey {
        self.trader_phoenix_account
    }

    pub fn amount(&self) -> Option<u64> {
        self.amount
    }
}

/// Builder for `EmberWithdrawParams`.
#[derive(Default)]
pub struct EmberWithdrawParamsBuilder {
    trader: Option<Pubkey>,
    usdc_mint: Option<Pubkey>,
    canonical_mint: Option<Pubkey>,
    trader_usdc_account: Option<Pubkey>,
    trader_phoenix_account: Option<Pubkey>,
    amount: Option<Option<u64>>,
}

impl EmberWithdrawParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
        self
    }

    pub fn usdc_mint(mut self, usdc_mint: Pubkey) -> Self {
        self.usdc_mint = Some(usdc_mint);
        self
    }

    pub fn canonical_mint(mut self, canonical_mint: Pubkey) -> Self {
        self.canonical_mint = Some(canonical_mint);
        self
    }

    pub fn trader_usdc_account(mut self, trader_usdc_account: Pubkey) -> Self {
        self.trader_usdc_account = Some(trader_usdc_account);
        self
    }

    pub fn trader_phoenix_account(mut self, trader_phoenix_account: Pubkey) -> Self {
        self.trader_phoenix_account = Some(trader_phoenix_account);
        self
    }

    /// Set the withdrawal amount. Pass `Some(amount)` for a specific amount,
    /// or `None` for a full withdrawal.
    pub fn amount(mut self, amount: Option<u64>) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn build(self) -> Result<EmberWithdrawParams, PhoenixIxError> {
        let amount = self.amount.ok_or(PhoenixIxError::MissingField("amount"))?;

        // Validate amount if specified
        if let Some(amt) = amount {
            if amt == 0 {
                return Err(PhoenixIxError::InvalidWithdrawAmount);
            }
        }

        Ok(EmberWithdrawParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            usdc_mint: self
                .usdc_mint
                .ok_or(PhoenixIxError::MissingField("usdc_mint"))?,
            canonical_mint: self
                .canonical_mint
                .ok_or(PhoenixIxError::MissingField("canonical_mint"))?,
            trader_usdc_account: self
                .trader_usdc_account
                .ok_or(PhoenixIxError::MissingField("trader_usdc_account"))?,
            trader_phoenix_account: self
                .trader_phoenix_account
                .ok_or(PhoenixIxError::MissingField("trader_phoenix_account"))?,
            amount,
        })
    }
}

/// Create an Ember withdraw instruction.
///
/// This instruction burns Phoenix tokens and releases USDC from the Ember vault
/// to the trader's account.
///
/// # Arguments
///
/// * `params` - The Ember withdraw parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
///
/// # Errors
///
/// Returns an error if required parameters are missing or amount is zero.
pub fn create_ember_withdraw_ix(
    params: EmberWithdrawParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_ember_withdraw(&params);
    let accounts = build_accounts(&params)?;

    Ok(Instruction {
        program_id: EMBER_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_ember_withdraw(params: &EmberWithdrawParams) -> Vec<u8> {
    // Capacity: 8 (discriminant) + 1 (Option tag) + 8 (amount if Some) = 17 max
    let mut data = Vec::with_capacity(17);

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&EmberInstruction::Withdraw.discriminant());

    // Amount as Borsh-encoded Option<u64>
    match params.amount() {
        Some(amount) => {
            data.push(1); // Some variant tag
            data.extend_from_slice(&amount.to_le_bytes());
        }
        None => {
            data.push(0); // None variant tag
        }
    }

    data
}

fn build_accounts(params: &EmberWithdrawParams) -> Result<Vec<AccountMeta>, PhoenixIxError> {
    Ok(vec![
        // 1. owner (signer, readonly)
        AccountMeta::readonly_signer(params.trader()),
        // 2. ember_state (readonly)
        AccountMeta::readonly(get_ember_state_address()?),
        // 3. input_mint (readonly) - USDC
        AccountMeta::readonly(params.usdc_mint()),
        // 4. output_mint (writable) - Phoenix token
        AccountMeta::writable(params.canonical_mint()),
        // 5. input_token_account (writable) - owner's USDC ATA
        AccountMeta::writable(params.trader_usdc_account()),
        // 6. output_token_account (writable) - owner's Phoenix token ATA
        AccountMeta::writable(params.trader_phoenix_account()),
        // 7. ember_vault (writable)
        AccountMeta::writable(get_ember_vault_address()?),
        // 8. spl_token (readonly)
        AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ember_withdraw_ix_with_amount() {
        let params = EmberWithdrawParams::builder()
            .trader(Pubkey::new_unique())
            .usdc_mint(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .trader_usdc_account(Pubkey::new_unique())
            .trader_phoenix_account(Pubkey::new_unique())
            .amount(Some(100_000_000))
            .build()
            .unwrap();

        let ix = create_ember_withdraw_ix(params).unwrap();

        assert_eq!(ix.program_id, EMBER_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 8);

        // Verify data encoding
        assert_eq!(&ix.data[..8], &EmberInstruction::Withdraw.discriminant());
        assert_eq!(ix.data[8], 1); // Some variant
        let amount_bytes: [u8; 8] = ix.data[9..17].try_into().unwrap();
        assert_eq!(u64::from_le_bytes(amount_bytes), 100_000_000);
    }

    #[test]
    fn test_create_ember_withdraw_ix_full_withdrawal() {
        let params = EmberWithdrawParams::builder()
            .trader(Pubkey::new_unique())
            .usdc_mint(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .trader_usdc_account(Pubkey::new_unique())
            .trader_phoenix_account(Pubkey::new_unique())
            .amount(None) // Full withdrawal
            .build()
            .unwrap();

        let ix = create_ember_withdraw_ix(params).unwrap();

        // Verify data encoding for full withdrawal
        assert_eq!(&ix.data[..8], &EmberInstruction::Withdraw.discriminant());
        assert_eq!(ix.data[8], 0); // None variant
        assert_eq!(ix.data.len(), 9); // 8 bytes discriminant + 1 byte None tag
    }

    #[test]
    fn test_ember_withdraw_missing_amount() {
        let result = EmberWithdrawParams::builder()
            .trader(Pubkey::new_unique())
            .usdc_mint(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .trader_usdc_account(Pubkey::new_unique())
            .trader_phoenix_account(Pubkey::new_unique())
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("amount"))
        ));
    }

    #[test]
    fn test_ember_withdraw_zero_amount() {
        let result = EmberWithdrawParams::builder()
            .trader(Pubkey::new_unique())
            .usdc_mint(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .trader_usdc_account(Pubkey::new_unique())
            .trader_phoenix_account(Pubkey::new_unique())
            .amount(Some(0))
            .build();

        assert!(matches!(result, Err(PhoenixIxError::InvalidWithdrawAmount)));
    }

    #[test]
    fn test_ember_withdraw_account_order() {
        let trader = Pubkey::new_unique();
        let usdc_mint = Pubkey::new_unique();
        let canonical_mint = Pubkey::new_unique();
        let trader_usdc = Pubkey::new_unique();
        let trader_phoenix = Pubkey::new_unique();

        let params = EmberWithdrawParams::builder()
            .trader(trader)
            .usdc_mint(usdc_mint)
            .canonical_mint(canonical_mint)
            .trader_usdc_account(trader_usdc)
            .trader_phoenix_account(trader_phoenix)
            .amount(Some(1))
            .build()
            .unwrap();

        let ix = create_ember_withdraw_ix(params).unwrap();

        // Verify account order and properties
        assert_eq!(ix.accounts[0].pubkey, trader);
        assert!(ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable);

        assert_eq!(ix.accounts[1].pubkey, get_ember_state_address().unwrap());
        assert!(!ix.accounts[1].is_signer);
        assert!(!ix.accounts[1].is_writable);

        assert_eq!(ix.accounts[2].pubkey, usdc_mint);
        assert!(!ix.accounts[2].is_writable);

        assert_eq!(ix.accounts[3].pubkey, canonical_mint);
        assert!(ix.accounts[3].is_writable);

        assert_eq!(ix.accounts[4].pubkey, trader_usdc);
        assert!(ix.accounts[4].is_writable);

        assert_eq!(ix.accounts[5].pubkey, trader_phoenix);
        assert!(ix.accounts[5].is_writable);

        assert_eq!(ix.accounts[6].pubkey, get_ember_vault_address().unwrap());
        assert!(ix.accounts[6].is_writable);

        assert_eq!(ix.accounts[7].pubkey, SPL_TOKEN_PROGRAM_ID);
        assert!(!ix.accounts[7].is_writable);
    }
}
