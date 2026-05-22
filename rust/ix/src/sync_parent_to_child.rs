//! Sync parent-to-child instruction construction.
//!
//! This module provides instruction building for syncing a parent trader
//! account's state to a child (isolated) subaccount, including global trader
//! index updates.

use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    sync_parent_to_child_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// Parameters for syncing parent state to a child subaccount.
#[derive(Debug, Clone)]
pub struct SyncParentToChildParams {
    /// The trader wallet authority (readonly signer).
    trader_wallet: Pubkey,
    /// The parent trader account (readonly) - source of state.
    parent_trader_account: Pubkey,
    /// The child trader account (writable) - destination.
    child_trader_account: Pubkey,
    /// Global trader index addresses (header + arenas).
    global_trader_index: Vec<Pubkey>,
}

impl SyncParentToChildParams {
    pub fn builder() -> SyncParentToChildParamsBuilder {
        SyncParentToChildParamsBuilder::new()
    }

    pub fn trader_wallet(&self) -> Pubkey {
        self.trader_wallet
    }

    pub fn parent_trader_account(&self) -> Pubkey {
        self.parent_trader_account
    }

    pub fn child_trader_account(&self) -> Pubkey {
        self.child_trader_account
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }
}

/// Builder for `SyncParentToChildParams`.
#[derive(Default)]
pub struct SyncParentToChildParamsBuilder {
    trader_wallet: Option<Pubkey>,
    parent_trader_account: Option<Pubkey>,
    child_trader_account: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
}

impl SyncParentToChildParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn parent_trader_account(mut self, parent_trader_account: Pubkey) -> Self {
        self.parent_trader_account = Some(parent_trader_account);
        self
    }

    pub fn child_trader_account(mut self, child_trader_account: Pubkey) -> Self {
        self.child_trader_account = Some(child_trader_account);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn build(self) -> Result<SyncParentToChildParams, PhoenixIxError> {
        let parent_trader_account = self
            .parent_trader_account
            .ok_or(PhoenixIxError::MissingField("parent_trader_account"))?;
        let child_trader_account = self
            .child_trader_account
            .ok_or(PhoenixIxError::MissingField("child_trader_account"))?;

        if parent_trader_account == child_trader_account {
            return Err(PhoenixIxError::MissingField(
                "parent and child trader accounts must be different",
            ));
        }

        let global_trader_index = self
            .global_trader_index
            .ok_or(PhoenixIxError::MissingField("global_trader_index"))?;
        if global_trader_index.is_empty() {
            return Err(PhoenixIxError::EmptyGlobalTraderIndex);
        }

        Ok(SyncParentToChildParams {
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            parent_trader_account,
            child_trader_account,
            global_trader_index,
        })
    }
}

/// Create a sync parent-to-child instruction.
///
/// Syncs a parent trader account's state to a child (isolated) subaccount.
/// The trader wallet must be the authority for both accounts.
pub fn create_sync_parent_to_child_ix(
    params: SyncParentToChildParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = sync_parent_to_child_discriminant().to_vec();
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn build_accounts(params: &SyncParentToChildParams) -> Vec<AccountMeta> {
    let mut accounts = vec![
        // LogAccountGroupAccounts (2 accounts)
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        // SyncParentToChildInstructionGroupAccounts
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::readonly_signer(params.trader_wallet()),
        AccountMeta::readonly(params.parent_trader_account()),
        AccountMeta::writable(params.child_trader_account()),
    ];

    // global_trader_index addresses (all writable)
    for addr in params.global_trader_index() {
        accounts.push(AccountMeta::writable(*addr));
    }

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_params() -> SyncParentToChildParams {
        SyncParentToChildParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .build()
            .unwrap()
    }

    #[test]
    fn test_create_sync_parent_to_child_ix() {
        let params = build_params();
        let ix = create_sync_parent_to_child_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 6 base accounts + 1 global_trader_index = 7
        assert_eq!(ix.accounts.len(), 7);
        assert_eq!(&ix.data[..8], &sync_parent_to_child_discriminant());
    }

    #[test]
    fn test_data_is_discriminant_only() {
        let params = build_params();
        let ix = create_sync_parent_to_child_ix(params).unwrap();

        assert_eq!(ix.data.len(), 8);
    }

    #[test]
    fn test_account_order() {
        let trader_wallet = Pubkey::new_unique();
        let parent = Pubkey::new_unique();
        let child = Pubkey::new_unique();
        let gti = Pubkey::new_unique();

        let params = SyncParentToChildParams::builder()
            .trader_wallet(trader_wallet)
            .parent_trader_account(parent)
            .child_trader_account(child)
            .global_trader_index(vec![gti])
            .build()
            .unwrap();

        let ix = create_sync_parent_to_child_ix(params).unwrap();

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

        // Account 3: trader_wallet (readonly signer)
        assert_eq!(ix.accounts[3].pubkey, trader_wallet);
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);

        // Account 4: parent_trader_account (readonly)
        assert_eq!(ix.accounts[4].pubkey, parent);
        assert!(!ix.accounts[4].is_signer);
        assert!(!ix.accounts[4].is_writable);

        // Account 5: child_trader_account (writable)
        assert_eq!(ix.accounts[5].pubkey, child);
        assert!(!ix.accounts[5].is_signer);
        assert!(ix.accounts[5].is_writable);

        // Account 6: global_trader_index (writable)
        assert_eq!(ix.accounts[6].pubkey, gti);
        assert!(ix.accounts[6].is_writable);
    }

    #[test]
    fn test_multiple_global_trader_index() {
        let gti1 = Pubkey::new_unique();
        let gti2 = Pubkey::new_unique();

        let params = SyncParentToChildParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .global_trader_index(vec![gti1, gti2])
            .build()
            .unwrap();

        let ix = create_sync_parent_to_child_ix(params).unwrap();

        // 6 base + 2 gti = 8
        assert_eq!(ix.accounts.len(), 8);
        assert_eq!(ix.accounts[6].pubkey, gti1);
        assert_eq!(ix.accounts[7].pubkey, gti2);
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = SyncParentToChildParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .build();

        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }

    #[test]
    fn test_same_parent_and_child_rejected() {
        let account = Pubkey::new_unique();
        let result = SyncParentToChildParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .parent_trader_account(account)
            .child_trader_account(account)
            .global_trader_index(vec![Pubkey::new_unique()])
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_global_trader_index() {
        let result = SyncParentToChildParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .global_trader_index(vec![])
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }
}
