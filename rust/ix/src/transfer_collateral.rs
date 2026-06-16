//! Transfer collateral instruction construction.
//!
//! This module provides instruction building for transferring collateral
//! between subaccounts (e.g., cross-margin to isolated margin).

use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    transfer_collateral_child_to_parent_discriminant, transfer_collateral_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for transferring collateral between subaccounts.
#[derive(Debug, Clone)]
pub struct TransferCollateralParams {
    /// The trader's authority (wallet) — must sign.
    trader: Pubkey,
    /// The source trader PDA account (writable).
    src_trader_account: Pubkey,
    /// The destination trader PDA account (writable).
    dst_trader_account: Pubkey,
    /// The perp asset map account (readonly).
    perp_asset_map: Pubkey,
    /// Global trader index addresses (header + arenas).
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
    active_trader_buffer: Vec<Pubkey>,
    /// Optional permission account authorizing `trader` as a secondary position
    /// authority.
    permission_account: Option<Pubkey>,
    /// Amount to transfer in token base units.
    amount: u64,
}

impl TransferCollateralParams {
    /// Start building with the builder pattern.
    pub fn builder() -> TransferCollateralParamsBuilder {
        TransferCollateralParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn src_trader_account(&self) -> Pubkey {
        self.src_trader_account
    }

    pub fn dst_trader_account(&self) -> Pubkey {
        self.dst_trader_account
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }

    pub fn permission_account(&self) -> Option<Pubkey> {
        self.permission_account
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

/// Builder for `TransferCollateralParams`.
#[derive(Default)]
pub struct TransferCollateralParamsBuilder {
    trader: Option<Pubkey>,
    src_trader_account: Option<Pubkey>,
    dst_trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    permission_account: Option<Pubkey>,
    amount: Option<u64>,
}

impl TransferCollateralParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
        self
    }

    pub fn src_trader_account(mut self, src_trader_account: Pubkey) -> Self {
        self.src_trader_account = Some(src_trader_account);
        self
    }

    pub fn dst_trader_account(mut self, dst_trader_account: Pubkey) -> Self {
        self.dst_trader_account = Some(dst_trader_account);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
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

    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn build(self) -> Result<TransferCollateralParams, PhoenixIxError> {
        let amount = self.amount.ok_or(PhoenixIxError::MissingField("amount"))?;
        if amount == 0 {
            return Err(PhoenixIxError::InvalidTransferAmount);
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

        Ok(TransferCollateralParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            src_trader_account: self
                .src_trader_account
                .ok_or(PhoenixIxError::MissingField("src_trader_account"))?,
            dst_trader_account: self
                .dst_trader_account
                .ok_or(PhoenixIxError::MissingField("dst_trader_account"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_trader_index,
            active_trader_buffer,
            permission_account: self.permission_account,
            amount,
        })
    }
}

/// Create a transfer collateral instruction.
///
/// Transfers collateral between two subaccounts (e.g., from cross-margin
/// subaccount 0 to an isolated margin subaccount).
///
/// # Arguments
///
/// * `params` - The transfer collateral parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
pub fn create_transfer_collateral_ix(
    params: TransferCollateralParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_transfer_collateral(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_transfer_collateral(params: &TransferCollateralParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(16);

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&transfer_collateral_discriminant());

    // Amount (8 bytes, little-endian u64)
    data.extend_from_slice(&params.amount().to_le_bytes());

    data
}

fn build_accounts(params: &TransferCollateralParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // 1. phoenix_program (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    // 2. PHOENIX_LOG_AUTHORITY (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));
    // 3. global_configuration (readonly — differs from deposit which is writable)
    accounts.push(AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION));
    // 4. trader (readonly signer)
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    // 5. src_trader_account (writable)
    accounts.push(AccountMeta::writable(params.src_trader_account()));
    // 6. dst_trader_account (writable)
    accounts.push(AccountMeta::writable(params.dst_trader_account()));
    // 7. perp_asset_map (readonly)
    accounts.push(AccountMeta::readonly(params.perp_asset_map()));

    // 8-N. global_trader_index addresses (all writable)
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

/// Parameters for transferring all collateral from a child subaccount back to
/// the parent (subaccount 0).
#[derive(Debug, Clone)]
pub struct TransferCollateralChildToParentParams {
    /// The trader's authority (wallet) — must sign.
    trader: Pubkey,
    /// The child trader PDA account (writable).
    child_trader_account: Pubkey,
    /// The parent trader PDA account (writable).
    parent_trader_account: Pubkey,
    /// The perp asset map account (readonly).
    perp_asset_map: Pubkey,
    /// Global trader index addresses (header + arenas).
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
    active_trader_buffer: Vec<Pubkey>,
}

impl TransferCollateralChildToParentParams {
    pub fn builder() -> TransferCollateralChildToParentParamsBuilder {
        TransferCollateralChildToParentParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn child_trader_account(&self) -> Pubkey {
        self.child_trader_account
    }

    pub fn parent_trader_account(&self) -> Pubkey {
        self.parent_trader_account
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }
}

/// Builder for `TransferCollateralChildToParentParams`.
#[derive(Default)]
pub struct TransferCollateralChildToParentParamsBuilder {
    trader: Option<Pubkey>,
    child_trader_account: Option<Pubkey>,
    parent_trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl TransferCollateralChildToParentParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
        self
    }

    pub fn child_trader_account(mut self, child_trader_account: Pubkey) -> Self {
        self.child_trader_account = Some(child_trader_account);
        self
    }

    pub fn parent_trader_account(mut self, parent_trader_account: Pubkey) -> Self {
        self.parent_trader_account = Some(parent_trader_account);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
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

    pub fn build(self) -> Result<TransferCollateralChildToParentParams, PhoenixIxError> {
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

        Ok(TransferCollateralChildToParentParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            child_trader_account: self
                .child_trader_account
                .ok_or(PhoenixIxError::MissingField("child_trader_account"))?,
            parent_trader_account: self
                .parent_trader_account
                .ok_or(PhoenixIxError::MissingField("parent_trader_account"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

/// Create a transfer collateral child-to-parent instruction.
///
/// Transfers **all** collateral from a child subaccount back to the parent
/// (subaccount 0). No-ops if the child has open positions, open orders, or
/// zero collateral.
pub fn create_transfer_collateral_child_to_parent_ix(
    params: TransferCollateralChildToParentParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = transfer_collateral_child_to_parent_discriminant().to_vec();
    let accounts = build_child_to_parent_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn build_child_to_parent_accounts(
    params: &TransferCollateralChildToParentParams,
) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // 1. phoenix_program (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    // 2. PHOENIX_LOG_AUTHORITY (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));
    // 3. global_configuration (readonly)
    accounts.push(AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION));
    // 4. trader (readonly signer)
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    // 5. child_trader_account (writable)
    accounts.push(AccountMeta::writable(params.child_trader_account()));
    // 6. parent_trader_account (writable)
    accounts.push(AccountMeta::writable(params.parent_trader_account()));
    // 7. perp_asset_map (readonly)
    accounts.push(AccountMeta::readonly(params.perp_asset_map()));

    // 8-N. global_trader_index addresses (all writable)
    for addr in params.global_trader_index() {
        accounts.push(AccountMeta::writable(*addr));
    }

    // N+1-M. active_trader_buffer addresses (all writable)
    for addr in params.active_trader_buffer() {
        accounts.push(AccountMeta::writable(*addr));
    }

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transfer_collateral_ix() {
        let params = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(100_000_000)
            .build()
            .unwrap();

        let ix = create_transfer_collateral_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 7 base accounts + 1 global_trader_index + 1 active_trader_buffer = 9
        assert_eq!(ix.accounts.len(), 9);

        // Verify data encoding
        assert_eq!(&ix.data[..8], &transfer_collateral_discriminant());
        let amount_bytes: [u8; 8] = ix.data[8..16].try_into().unwrap();
        assert_eq!(u64::from_le_bytes(amount_bytes), 100_000_000);
    }

    #[test]
    fn test_transfer_collateral_zero_amount() {
        let result = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(0)
            .build();

        assert!(matches!(result, Err(PhoenixIxError::InvalidTransferAmount)));
    }

    #[test]
    fn test_transfer_collateral_empty_global_trader_index() {
        let result = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
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
    fn test_transfer_collateral_empty_active_trader_buffer() {
        let result = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
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
    fn test_transfer_collateral_account_order() {
        let trader = Pubkey::new_unique();
        let src = Pubkey::new_unique();
        let dst = Pubkey::new_unique();
        let pam = Pubkey::new_unique();
        let gti = Pubkey::new_unique();
        let atb = Pubkey::new_unique();

        let params = TransferCollateralParams::builder()
            .trader(trader)
            .src_trader_account(src)
            .dst_trader_account(dst)
            .perp_asset_map(pam)
            .global_trader_index(vec![gti])
            .active_trader_buffer(vec![atb])
            .amount(1)
            .build()
            .unwrap();

        let ix = create_transfer_collateral_ix(params).unwrap();

        // Account 0: *PHOENIX_PROGRAM_ID (readonly)
        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[0].is_writable);

        // Account 1: *PHOENIX_LOG_AUTHORITY (readonly)
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert!(!ix.accounts[1].is_writable);

        // Account 2: *PHOENIX_GLOBAL_CONFIGURATION (readonly — not writable)
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert!(!ix.accounts[2].is_writable);

        // Account 3: trader (readonly signer)
        assert_eq!(ix.accounts[3].pubkey, trader);
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);

        // Account 4: src_trader_account (writable)
        assert_eq!(ix.accounts[4].pubkey, src);
        assert!(ix.accounts[4].is_writable);

        // Account 5: dst_trader_account (writable)
        assert_eq!(ix.accounts[5].pubkey, dst);
        assert!(ix.accounts[5].is_writable);

        // Account 6: perp_asset_map (readonly)
        assert_eq!(ix.accounts[6].pubkey, pam);
        assert!(!ix.accounts[6].is_writable);

        // Account 7: gti (writable)
        assert_eq!(ix.accounts[7].pubkey, gti);
        assert!(ix.accounts[7].is_writable);

        // Account 8: atb (writable)
        assert_eq!(ix.accounts[8].pubkey, atb);
        assert!(ix.accounts[8].is_writable);
    }

    #[test]
    fn test_transfer_collateral_optional_permission_account() {
        let permission_account = Pubkey::new_unique();
        let params = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .permission_account(permission_account)
            .amount(1)
            .build()
            .unwrap();

        let ix = create_transfer_collateral_ix(params).unwrap();

        assert_eq!(ix.accounts.len(), 10);
        assert_eq!(ix.accounts[9].pubkey, permission_account);
        assert!(ix.accounts[9].is_writable);
        assert!(!ix.accounts[9].is_signer);
    }

    #[test]
    fn test_transfer_collateral_multiple_index_accounts() {
        let gti1 = Pubkey::new_unique();
        let gti2 = Pubkey::new_unique();
        let atb1 = Pubkey::new_unique();
        let atb2 = Pubkey::new_unique();

        let params = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![gti1, gti2])
            .active_trader_buffer(vec![atb1, atb2])
            .amount(1)
            .build()
            .unwrap();

        let ix = create_transfer_collateral_ix(params).unwrap();

        // 7 base accounts + 2 gti + 2 atb = 11
        assert_eq!(ix.accounts.len(), 11);

        // Verify gti accounts
        assert_eq!(ix.accounts[7].pubkey, gti1);
        assert_eq!(ix.accounts[8].pubkey, gti2);

        // Verify atb accounts
        assert_eq!(ix.accounts[9].pubkey, atb1);
        assert_eq!(ix.accounts[10].pubkey, atb2);
    }

    #[test]
    fn test_create_transfer_collateral_child_to_parent_ix() {
        let params = TransferCollateralChildToParentParams::builder()
            .trader(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();

        let ix = create_transfer_collateral_child_to_parent_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 7 base accounts + 1 global_trader_index + 1 active_trader_buffer = 9
        assert_eq!(ix.accounts.len(), 9);

        // Data is discriminant only (8 bytes, no payload)
        assert_eq!(ix.data.len(), 8);
        assert_eq!(
            &ix.data[..8],
            &transfer_collateral_child_to_parent_discriminant()
        );
    }

    #[test]
    fn test_child_to_parent_account_order() {
        let trader = Pubkey::new_unique();
        let child = Pubkey::new_unique();
        let parent = Pubkey::new_unique();
        let pam = Pubkey::new_unique();
        let gti = Pubkey::new_unique();
        let atb = Pubkey::new_unique();

        let params = TransferCollateralChildToParentParams::builder()
            .trader(trader)
            .child_trader_account(child)
            .parent_trader_account(parent)
            .perp_asset_map(pam)
            .global_trader_index(vec![gti])
            .active_trader_buffer(vec![atb])
            .build()
            .unwrap();

        let ix = create_transfer_collateral_child_to_parent_ix(params).unwrap();

        // Account 0: *PHOENIX_PROGRAM_ID (readonly)
        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[0].is_writable);

        // Account 1: *PHOENIX_LOG_AUTHORITY (readonly)
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert!(!ix.accounts[1].is_writable);

        // Account 2: *PHOENIX_GLOBAL_CONFIGURATION (readonly)
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert!(!ix.accounts[2].is_writable);

        // Account 3: trader (readonly signer)
        assert_eq!(ix.accounts[3].pubkey, trader);
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);

        // Account 4: child_trader_account (writable)
        assert_eq!(ix.accounts[4].pubkey, child);
        assert!(ix.accounts[4].is_writable);

        // Account 5: parent_trader_account (writable)
        assert_eq!(ix.accounts[5].pubkey, parent);
        assert!(ix.accounts[5].is_writable);

        // Account 6: perp_asset_map (readonly)
        assert_eq!(ix.accounts[6].pubkey, pam);
        assert!(!ix.accounts[6].is_writable);

        // Account 7: gti (writable)
        assert_eq!(ix.accounts[7].pubkey, gti);
        assert!(ix.accounts[7].is_writable);

        // Account 8: atb (writable)
        assert_eq!(ix.accounts[8].pubkey, atb);
        assert!(ix.accounts[8].is_writable);
    }

    #[test]
    fn test_child_to_parent_empty_global_trader_index() {
        let result = TransferCollateralChildToParentParams::builder()
            .trader(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_child_to_parent_empty_active_trader_buffer() {
        let result = TransferCollateralChildToParentParams::builder()
            .trader(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![])
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyActiveTraderBuffer)
        ));
    }
}
