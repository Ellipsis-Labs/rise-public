//! SPL Token approve instruction construction.
//!
//! This module provides instruction building for approving a delegate
//! to spend tokens from a token account.

use solana_pubkey::Pubkey;

use crate::ix::constants::SPL_TOKEN_PROGRAM_ID;
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for SPL Token approve instruction.
#[derive(Debug, Clone)]
pub struct SplApproveParams {
    /// The source token account to approve spending from.
    source: Pubkey,
    /// The delegate account that will be allowed to spend tokens.
    delegate: Pubkey,
    /// The owner of the source token account (must sign).
    owner: Pubkey,
    /// Amount of tokens to approve for spending.
    amount: u64,
}

impl SplApproveParams {
    /// Start building with the builder pattern.
    pub fn builder() -> SplApproveParamsBuilder {
        SplApproveParamsBuilder::new()
    }

    pub fn source(&self) -> Pubkey {
        self.source
    }

    pub fn delegate(&self) -> Pubkey {
        self.delegate
    }

    pub fn owner(&self) -> Pubkey {
        self.owner
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

/// Builder for `SplApproveParams`.
#[derive(Default)]
pub struct SplApproveParamsBuilder {
    source: Option<Pubkey>,
    delegate: Option<Pubkey>,
    owner: Option<Pubkey>,
    amount: Option<u64>,
}

impl SplApproveParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn source(mut self, source: Pubkey) -> Self {
        self.source = Some(source);
        self
    }

    pub fn delegate(mut self, delegate: Pubkey) -> Self {
        self.delegate = Some(delegate);
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn build(self) -> Result<SplApproveParams, PhoenixIxError> {
        Ok(SplApproveParams {
            source: self.source.ok_or(PhoenixIxError::MissingField("source"))?,
            delegate: self
                .delegate
                .ok_or(PhoenixIxError::MissingField("delegate"))?,
            owner: self.owner.ok_or(PhoenixIxError::MissingField("owner"))?,
            amount: self.amount.ok_or(PhoenixIxError::MissingField("amount"))?,
        })
    }
}

/// Create an SPL Token approve instruction.
///
/// This instruction approves a delegate to spend tokens from a token account.
///
/// # Arguments
///
/// * `params` - The approve parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
///
/// # Errors
///
/// Returns an error if required parameters are missing.
pub fn create_spl_approve_ix(params: SplApproveParams) -> Result<Instruction, PhoenixIxError> {
    let data = encode_spl_approve(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_spl_approve(params: &SplApproveParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(9);

    // SPL Token Approve instruction discriminant
    data.push(4);

    // Amount (8 bytes, little-endian u64)
    data.extend_from_slice(&params.amount().to_le_bytes());

    data
}

fn build_accounts(params: &SplApproveParams) -> Vec<AccountMeta> {
    vec![
        // 1. source_token_account (writable)
        AccountMeta::writable(params.source()),
        // 2. delegate (readonly)
        AccountMeta::readonly(params.delegate()),
        // 3. owner (signer, readonly)
        AccountMeta::readonly_signer(params.owner()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_spl_approve_ix() {
        let params = SplApproveParams::builder()
            .source(Pubkey::new_unique())
            .delegate(Pubkey::new_unique())
            .owner(Pubkey::new_unique())
            .amount(100_000_000)
            .build()
            .unwrap();

        let ix = create_spl_approve_ix(params).unwrap();

        assert_eq!(ix.program_id, SPL_TOKEN_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 3);

        // Verify data encoding
        assert_eq!(ix.data[0], 4); // SPL Token Approve discriminant
        let amount_bytes: [u8; 8] = ix.data[1..9].try_into().unwrap();
        assert_eq!(u64::from_le_bytes(amount_bytes), 100_000_000);
    }

    #[test]
    fn test_spl_approve_missing_source() {
        let result = SplApproveParams::builder()
            .delegate(Pubkey::new_unique())
            .owner(Pubkey::new_unique())
            .amount(100)
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("source"))
        ));
    }

    #[test]
    fn test_spl_approve_account_order() {
        let source = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let params = SplApproveParams::builder()
            .source(source)
            .delegate(delegate)
            .owner(owner)
            .amount(1)
            .build()
            .unwrap();

        let ix = create_spl_approve_ix(params).unwrap();

        // Verify account order and properties
        assert_eq!(ix.accounts[0].pubkey, source);
        assert!(ix.accounts[0].is_writable);
        assert!(!ix.accounts[0].is_signer);

        assert_eq!(ix.accounts[1].pubkey, delegate);
        assert!(!ix.accounts[1].is_writable);
        assert!(!ix.accounts[1].is_signer);

        assert_eq!(ix.accounts[2].pubkey, owner);
        assert!(!ix.accounts[2].is_writable);
        assert!(ix.accounts[2].is_signer);
    }
}
