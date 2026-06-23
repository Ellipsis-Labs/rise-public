//! Deposit funds instruction construction.
//!
//! This module provides instruction building for depositing Phoenix tokens
//! into the Phoenix protocol.

use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID,
    deposit_funds_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for depositing Phoenix tokens into the protocol.
#[derive(Debug, Clone)]
pub struct DepositFundsParams {
    /// The trader's authority (wallet) - must sign.
    trader: Pubkey,
    /// The trader's PDA account.
    trader_account: Pubkey,
    /// The canonical mint (Phoenix token mint).
    canonical_mint: Pubkey,
    /// The global vault (Phoenix protocol vault for the mint).
    global_vault: Pubkey,
    /// The trader's token account (ATA for Phoenix tokens).
    trader_token_account: Pubkey,
    /// Global trader index addresses (header + arenas).
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
    active_trader_buffer: Vec<Pubkey>,
    /// Amount to deposit in token base units.
    amount: u64,
    /// Permission account (optional, if the trader authority is delegated by
    /// the actual authority)
    permission_account: Option<Pubkey>,
}

impl DepositFundsParams {
    /// Start building with the builder pattern.
    pub fn builder() -> DepositFundsParamsBuilder {
        DepositFundsParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn canonical_mint(&self) -> Pubkey {
        self.canonical_mint
    }

    pub fn global_vault(&self) -> Pubkey {
        self.global_vault
    }

    pub fn trader_token_account(&self) -> Pubkey {
        self.trader_token_account
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn permission_account(&self) -> Option<Pubkey> {
        self.permission_account
    }
}

/// Builder for `DepositFundsParams`.
#[derive(Default)]
pub struct DepositFundsParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    canonical_mint: Option<Pubkey>,
    global_vault: Option<Pubkey>,
    trader_token_account: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    amount: Option<u64>,
    permission_account: Option<Pubkey>,
}

impl DepositFundsParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn canonical_mint(mut self, canonical_mint: Pubkey) -> Self {
        self.canonical_mint = Some(canonical_mint);
        self
    }

    pub fn global_vault(mut self, global_vault: Pubkey) -> Self {
        self.global_vault = Some(global_vault);
        self
    }

    pub fn trader_token_account(mut self, trader_token_account: Pubkey) -> Self {
        self.trader_token_account = Some(trader_token_account);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn build(self) -> Result<DepositFundsParams, PhoenixIxError> {
        let amount = self.amount.ok_or(PhoenixIxError::MissingField("amount"))?;
        if amount == 0 {
            return Err(PhoenixIxError::InvalidDepositAmount);
        }

        let global_trader_index = self
            .global_trader_index
            .ok_or(PhoenixIxError::MissingField("global_trader_index"))?;
        if global_trader_index.is_empty() {
            return Err(PhoenixIxError::EmptyGlobalTraderIndex);
        }

        let active_trader_buffer = self
            .active_trader_buffer
            .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?;
        if active_trader_buffer.is_empty() {
            return Err(PhoenixIxError::EmptyActiveTraderBuffer);
        }

        Ok(DepositFundsParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            canonical_mint: self
                .canonical_mint
                .ok_or(PhoenixIxError::MissingField("canonical_mint"))?,
            global_vault: self
                .global_vault
                .ok_or(PhoenixIxError::MissingField("global_vault"))?,
            trader_token_account: self
                .trader_token_account
                .ok_or(PhoenixIxError::MissingField("trader_token_account"))?,
            global_trader_index,
            active_trader_buffer,
            amount,
            permission_account: self.permission_account,
        })
    }
}

/// Create a deposit funds instruction.
///
/// This instruction deposits Phoenix tokens from the trader's token account
/// into the Phoenix protocol.
///
/// # Arguments
///
/// * `params` - The deposit funds parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
///
/// # Errors
///
/// Returns an error if required parameters are missing, amount is zero,
/// or trader index arrays are empty.
pub fn create_deposit_funds_ix(params: DepositFundsParams) -> Result<Instruction, PhoenixIxError> {
    let data = encode_deposit_funds(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_deposit_funds(params: &DepositFundsParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(16);

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&deposit_funds_discriminant());

    // Amount (8 bytes, little-endian u64)
    data.extend_from_slice(&params.amount().to_le_bytes());

    data
}

fn build_accounts(params: &DepositFundsParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // 1. phoenix_program (readonly) - Log accounts
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    // 2. PHOENIX_LOG_AUTHORITY (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));
    // 3. global_configuration_account (writable)
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    // 4. trader_wallet (signer, readonly)
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    // 5. trader_token_account (writable) - Owner's Phoenix token ATA
    accounts.push(AccountMeta::writable(params.trader_token_account()));
    // 6. trader_account (writable) - Trader PDA
    accounts.push(AccountMeta::writable(params.trader_account()));
    // 7. global_vault (writable)
    accounts.push(AccountMeta::writable(params.global_vault()));
    // 8. token_program (readonly)
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));

    // 9-N. global_trader_index addresses (all writable)
    for addr in params.global_trader_index() {
        accounts.push(AccountMeta::writable(*addr));
    }

    // N+1-M. active_trader_buffer addresses (all writable)
    for addr in params.active_trader_buffer() {
        accounts.push(AccountMeta::writable(*addr));
    }

    if let Some(permission_account) = params.permission_account() {
        accounts.push(AccountMeta::writable(permission_account));
    }

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_deposit_funds_ix() {
        let params = DepositFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(100_000_000)
            .build()
            .unwrap();

        let ix = create_deposit_funds_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 8 base accounts + 1 global_trader_index + 1 active_trader_buffer = 10
        assert_eq!(ix.accounts.len(), 10);

        // Verify data encoding
        assert_eq!(&ix.data[..8], &deposit_funds_discriminant());
        let amount_bytes: [u8; 8] = ix.data[8..16].try_into().unwrap();
        assert_eq!(u64::from_le_bytes(amount_bytes), 100_000_000);
    }

    #[test]
    fn test_deposit_funds_zero_amount() {
        let result = DepositFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(0)
            .build();

        assert!(matches!(result, Err(PhoenixIxError::InvalidDepositAmount)));
    }

    #[test]
    fn test_deposit_funds_empty_global_trader_index() {
        let result = DepositFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(100)
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_deposit_funds_empty_active_trader_buffer() {
        let result = DepositFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![])
            .amount(100)
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyActiveTraderBuffer)
        ));
    }

    #[test]
    fn test_deposit_funds_account_order() {
        let trader = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let canonical_mint = Pubkey::new_unique();
        let global_vault = Pubkey::new_unique();
        let trader_token_account = Pubkey::new_unique();
        let gti = Pubkey::new_unique();
        let atb = Pubkey::new_unique();

        let params = DepositFundsParams::builder()
            .trader(trader)
            .trader_account(trader_account)
            .canonical_mint(canonical_mint)
            .global_vault(global_vault)
            .trader_token_account(trader_token_account)
            .global_trader_index(vec![gti])
            .active_trader_buffer(vec![atb])
            .amount(1)
            .build()
            .unwrap();

        let ix = create_deposit_funds_ix(params).unwrap();

        // Verify account order
        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[0].is_writable);

        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert!(!ix.accounts[1].is_writable);

        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert!(ix.accounts[2].is_writable);

        assert_eq!(ix.accounts[3].pubkey, trader);
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);

        assert_eq!(ix.accounts[4].pubkey, trader_token_account);
        assert!(ix.accounts[4].is_writable);

        assert_eq!(ix.accounts[5].pubkey, trader_account);
        assert!(ix.accounts[5].is_writable);

        assert_eq!(ix.accounts[6].pubkey, global_vault);
        assert!(ix.accounts[6].is_writable);

        assert_eq!(ix.accounts[7].pubkey, SPL_TOKEN_PROGRAM_ID);
        assert!(!ix.accounts[7].is_writable);

        assert_eq!(ix.accounts[8].pubkey, gti);
        assert!(ix.accounts[8].is_writable);

        assert_eq!(ix.accounts[9].pubkey, atb);
        assert!(ix.accounts[9].is_writable);
    }

    #[test]
    fn test_deposit_funds_multiple_index_accounts() {
        let gti1 = Pubkey::new_unique();
        let gti2 = Pubkey::new_unique();
        let atb1 = Pubkey::new_unique();
        let atb2 = Pubkey::new_unique();

        let params = DepositFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![gti1, gti2])
            .active_trader_buffer(vec![atb1, atb2])
            .amount(1)
            .build()
            .unwrap();

        let ix = create_deposit_funds_ix(params).unwrap();

        // 8 base accounts + 2 gti + 2 atb = 12
        assert_eq!(ix.accounts.len(), 12);

        // Verify gti accounts
        assert_eq!(ix.accounts[8].pubkey, gti1);
        assert_eq!(ix.accounts[9].pubkey, gti2);

        // Verify atb accounts
        assert_eq!(ix.accounts[10].pubkey, atb1);
        assert_eq!(ix.accounts[11].pubkey, atb2);
    }
}
