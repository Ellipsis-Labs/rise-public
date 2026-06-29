//! Withdraw funds instruction construction.
//!
//! This module provides instruction building for withdrawing Phoenix tokens
//! from the Phoenix protocol.

use solana_pubkey::Pubkey;

use crate::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID,
};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction, push_trader_index_accounts};

/// Parameters for withdrawing Phoenix tokens from the protocol.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawFundsParams {
    /// The trader's authority (wallet) - must sign.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    /// The trader's PDA account.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    /// The perp asset map account.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    /// The global vault (Phoenix protocol vault for the mint).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    global_vault: Pubkey,
    /// The trader's token account (ATA for Phoenix tokens) - destination.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_token_account: Pubkey,
    /// Global trader index addresses (header + arenas).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    /// The withdraw queue account.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    withdraw_queue: Pubkey,
    /// Amount to withdraw in token base units.
    amount: u64,
}

impl WithdrawFundsParams {
    /// Start building with the builder pattern.
    pub fn builder() -> WithdrawFundsParamsBuilder {
        WithdrawFundsParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
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

    pub fn withdraw_queue(&self) -> Pubkey {
        self.withdraw_queue
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

/// Builder for `WithdrawFundsParams`.
#[derive(Default)]
pub struct WithdrawFundsParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_vault: Option<Pubkey>,
    trader_token_account: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    withdraw_queue: Option<Pubkey>,
    amount: Option<u64>,
}

impl WithdrawFundsParamsBuilder {
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

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
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

    pub fn withdraw_queue(mut self, withdraw_queue: Pubkey) -> Self {
        self.withdraw_queue = Some(withdraw_queue);
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn build(self) -> Result<WithdrawFundsParams, PhoenixIxError> {
        let amount = self.amount.ok_or(PhoenixIxError::MissingField("amount"))?;
        if amount == 0 {
            return Err(PhoenixIxError::InvalidWithdrawAmount);
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

        Ok(WithdrawFundsParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_vault: self
                .global_vault
                .ok_or(PhoenixIxError::MissingField("global_vault"))?,
            trader_token_account: self
                .trader_token_account
                .ok_or(PhoenixIxError::MissingField("trader_token_account"))?,
            global_trader_index,
            active_trader_buffer,
            withdraw_queue: self
                .withdraw_queue
                .ok_or(PhoenixIxError::MissingField("withdraw_queue"))?,
            amount,
        })
    }
}

/// Create a withdraw funds instruction.
///
/// This instruction withdraws Phoenix tokens from the Phoenix protocol
/// to the trader's token account.
///
/// # Arguments
///
/// * `params` - The withdraw funds parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
///
/// # Errors
///
/// Returns an error if required parameters are missing, amount is zero,
/// or trader index arrays are empty.
pub fn create_withdraw_funds_ix(
    params: WithdrawFundsParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_withdraw_funds(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_withdraw_funds(params: &WithdrawFundsParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(16);

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&crate::PhoenixInstruction::WithdrawFunds.discriminant());

    // Amount (8 bytes, little-endian u64)
    data.extend_from_slice(&params.amount().to_le_bytes());

    data
}

fn build_accounts(params: &WithdrawFundsParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // 1. phoenix_program (readonly) - Log accounts
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    // 2. PHOENIX_LOG_AUTHORITY (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));
    // 3. global_configuration_account (writable)
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    // 4. trader_wallet (signer, readonly)
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    // 5. trader_account (writable) - Trader PDA
    accounts.push(AccountMeta::writable(params.trader_account()));
    // 6. perp_asset_map (writable)
    accounts.push(AccountMeta::writable(params.perp_asset_map()));
    // 7. global_vault (writable)
    accounts.push(AccountMeta::writable(params.global_vault()));
    // 8. destination_token_account (writable) - Owner's Phoenix token ATA
    accounts.push(AccountMeta::writable(params.trader_token_account()));
    // 9. token_program (readonly)
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));

    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    // M+1. withdraw_queue (writable)
    accounts.push(AccountMeta::writable(params.withdraw_queue()));

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_withdraw_funds_ix() {
        let params = WithdrawFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .withdraw_queue(Pubkey::new_unique())
            .amount(100_000_000)
            .build()
            .unwrap();

        let ix = create_withdraw_funds_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 9 base accounts + 1 global_trader_index + 1 active_trader_buffer + 1
        // withdraw_queue = 12
        assert_eq!(ix.accounts.len(), 12);

        // Verify data encoding
        assert_eq!(
            &ix.data[..8],
            &crate::PhoenixInstruction::WithdrawFunds.discriminant()
        );
        let amount_bytes: [u8; 8] = ix.data[8..16].try_into().unwrap();
        assert_eq!(u64::from_le_bytes(amount_bytes), 100_000_000);
    }

    #[test]
    fn test_withdraw_funds_zero_amount() {
        let result = WithdrawFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .withdraw_queue(Pubkey::new_unique())
            .amount(0)
            .build();

        assert!(matches!(result, Err(PhoenixIxError::InvalidWithdrawAmount)));
    }

    #[test]
    fn test_withdraw_funds_empty_global_trader_index() {
        let result = WithdrawFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .withdraw_queue(Pubkey::new_unique())
            .amount(100)
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_withdraw_funds_empty_active_trader_buffer() {
        let result = WithdrawFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![])
            .withdraw_queue(Pubkey::new_unique())
            .amount(100)
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyActiveTraderBuffer)
        ));
    }

    #[test]
    fn test_withdraw_funds_account_order() {
        let trader = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let perp_asset_map = Pubkey::new_unique();
        let global_vault = Pubkey::new_unique();
        let trader_token_account = Pubkey::new_unique();
        let gti = Pubkey::new_unique();
        let atb = Pubkey::new_unique();
        let withdraw_queue = Pubkey::new_unique();

        let params = WithdrawFundsParams::builder()
            .trader(trader)
            .trader_account(trader_account)
            .perp_asset_map(perp_asset_map)
            .global_vault(global_vault)
            .trader_token_account(trader_token_account)
            .global_trader_index(vec![gti])
            .active_trader_buffer(vec![atb])
            .withdraw_queue(withdraw_queue)
            .amount(1)
            .build()
            .unwrap();

        let ix = create_withdraw_funds_ix(params).unwrap();

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

        assert_eq!(ix.accounts[4].pubkey, trader_account);
        assert!(ix.accounts[4].is_writable);

        assert_eq!(ix.accounts[5].pubkey, perp_asset_map);
        assert!(ix.accounts[5].is_writable);

        assert_eq!(ix.accounts[6].pubkey, global_vault);
        assert!(ix.accounts[6].is_writable);

        assert_eq!(ix.accounts[7].pubkey, trader_token_account);
        assert!(ix.accounts[7].is_writable);

        assert_eq!(ix.accounts[8].pubkey, SPL_TOKEN_PROGRAM_ID);
        assert!(!ix.accounts[8].is_writable);

        assert_eq!(ix.accounts[9].pubkey, gti);
        assert!(ix.accounts[9].is_writable);

        assert_eq!(ix.accounts[10].pubkey, atb);
        assert!(ix.accounts[10].is_writable);

        assert_eq!(ix.accounts[11].pubkey, withdraw_queue);
        assert!(ix.accounts[11].is_writable);
    }

    #[test]
    fn test_withdraw_funds_multiple_index_accounts() {
        let gti1 = Pubkey::new_unique();
        let gti2 = Pubkey::new_unique();
        let atb1 = Pubkey::new_unique();
        let atb2 = Pubkey::new_unique();

        let params = WithdrawFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![gti1, gti2])
            .active_trader_buffer(vec![atb1, atb2])
            .withdraw_queue(Pubkey::new_unique())
            .amount(1)
            .build()
            .unwrap();

        let ix = create_withdraw_funds_ix(params).unwrap();

        // 9 base accounts + 2 gti + 2 atb + 1 withdraw_queue = 14
        assert_eq!(ix.accounts.len(), 14);

        // Verify gti accounts
        assert_eq!(ix.accounts[9].pubkey, gti1);
        assert_eq!(ix.accounts[10].pubkey, gti2);

        // Verify atb accounts
        assert_eq!(ix.accounts[11].pubkey, atb1);
        assert_eq!(ix.accounts[12].pubkey, atb2);

        // Verify withdraw_queue is last
        assert!(ix.accounts[13].is_writable);
    }
}
