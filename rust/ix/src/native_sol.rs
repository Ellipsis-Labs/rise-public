//! Native SOL spot collateral instructions.
//!
//! Native SOL can be posted as collateral alongside the canonical quote token.
//! Its lamports live in the trader account itself, above the rent floor, and
//! the *accounted* portion is tracked as a header-extension entry in the trader
//! position map. Lamports beyond the accounted balance ("excess") carry no
//! margin value and can be swept out freely.
//!
//! Native SOL user and liquidation instructions are exposed here:
//!
//! - [`create_sync_native_ix`] reconciles the accounted balance against the
//!   account's actual lamports. Permissionless.
//! - [`create_withdraw_native_sol_ix`] moves lamports out.
//! - [`create_transfer_native_sol_ix`] and
//!   [`create_transfer_native_sol_from_child_to_parent_ix`] move collateral
//!   between trader accounts.
//! - [`create_swap_native_ix`] swaps between native SOL and quote collateral
//!   through an external venue.
//! - [`create_liquidate_native_sol_ix`] lets the risk authority or a delegated
//!   liquidator seize native SOL collateral.
//!
//! Admin instructions (configure, activate) are deliberately not exposed.
//!
//! Two custom program errors are specific to these instructions:
//!
//! - **7100 `FeatureDisabled`** — the exchange has native SOL collateral turned
//!   off, or the asset is not active.
//! - **7101 `PositionAuthoritySwapDisabled`** — a `SwapNative` was signed by a
//!   position authority while either opt-out gate is set. See
//!   [`create_swap_native_ix`].

use solana_pubkey::Pubkey;

use crate::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID, get_global_vault_address, get_native_sol_authority_address,
};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction, push_trader_index_accounts};

/// The packed venue-instruction encoding addresses accounts with 6 bits, so a
/// swap or liquidation can reference at most this many distinct accounts.
pub const MAX_PACKED_EXTERNAL_ACCOUNTS: usize = 64;

////////////////////////////////////////////////////////////////////////////////
// SyncNative
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_sync_native_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SyncNativeParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
}

impl SyncNativeParams {
    pub fn builder() -> SyncNativeParamsBuilder {
        SyncNativeParamsBuilder::default()
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }
}

#[derive(Debug, Clone, Default)]
pub struct SyncNativeParamsBuilder {
    trader_account: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl SyncNativeParamsBuilder {
    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
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

    pub fn build(self) -> Result<SyncNativeParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(SyncNativeParams {
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

/// Reconcile a trader's accounted native SOL collateral against the lamports
/// actually in the account.
///
/// Permissionless — anyone may crank it. An upward reconciliation is clamped to
/// the per-trader and exchange-wide caps; whatever the clamp leaves over stays
/// in the account as uncounted excess, recoverable by a later sync once
/// headroom frees up.
pub fn create_sync_native_ix(params: SyncNativeParams) -> Result<Instruction, PhoenixIxError> {
    let mut accounts = log_accounts();
    // The global configuration is writable: syncing moves the exchange-wide
    // collateral tally.
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.trader_account()));
    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data: crate::PhoenixInstruction::SyncNative
            .discriminant()
            .to_vec(),
    })
}

////////////////////////////////////////////////////////////////////////////////
// WithdrawNativeSol
////////////////////////////////////////////////////////////////////////////////

/// What a [`create_withdraw_native_sol_ix`] call takes out.
///
/// Accounted collateral is margin checked and consumes its quote-notional value
/// from the exchange-wide withdrawal throttle. Uncounted excess does neither,
/// because it was never protocol collateral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WithdrawNativeSolAction {
    /// Sweep every uncounted excess lamport, leaving accounted collateral and
    /// the rent floor untouched. Margin-free.
    AllExcess,
    /// Withdraw `amount` lamports, spending uncounted excess first and only
    /// then accounted collateral.
    WithExcess { amount: u64 },
    /// Withdraw `amount` lamports of accounted collateral, leaving any excess
    /// in place.
    WithoutExcess { amount: u64 },
}

impl WithdrawNativeSolAction {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::AllExcess => out.push(0),
            Self::WithExcess { amount } => {
                out.push(1);
                out.extend_from_slice(&amount.to_le_bytes());
            }
            Self::WithoutExcess { amount } => {
                out.push(2);
                out.extend_from_slice(&amount.to_le_bytes());
            }
        }
    }

    fn amount(&self) -> Option<u64> {
        match self {
            Self::AllExcess => None,
            Self::WithExcess { amount } | Self::WithoutExcess { amount } => Some(*amount),
        }
    }
}

/// Parameters for [`create_withdraw_native_sol_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawNativeSolParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    destination: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    withdraw_queue: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    action: WithdrawNativeSolAction,
}

impl WithdrawNativeSolParams {
    pub fn builder() -> WithdrawNativeSolParamsBuilder {
        WithdrawNativeSolParamsBuilder::default()
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

    pub fn destination(&self) -> Pubkey {
        self.destination
    }

    pub fn withdraw_queue(&self) -> Pubkey {
        self.withdraw_queue
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }

    pub fn action(&self) -> WithdrawNativeSolAction {
        self.action
    }
}

#[derive(Debug, Clone, Default)]
pub struct WithdrawNativeSolParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    destination: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    action: Option<WithdrawNativeSolAction>,
}

impl WithdrawNativeSolParamsBuilder {
    /// The trader's wallet authority, which must sign. A position authority
    /// cannot sign this instruction.
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

    /// Where the lamports go. Must be a system-owned account, and must be
    /// neither the trader account nor the native SOL authority PDA.
    pub fn destination(mut self, destination: Pubkey) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn withdraw_queue(mut self, withdraw_queue: Pubkey) -> Self {
        self.withdraw_queue = Some(withdraw_queue);
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

    pub fn action(mut self, action: WithdrawNativeSolAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn build(self) -> Result<WithdrawNativeSolParams, PhoenixIxError> {
        let action = self.action.ok_or(PhoenixIxError::MissingField("action"))?;
        if action.amount() == Some(0) {
            return Err(PhoenixIxError::InvalidWithdrawAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let destination = self
            .destination
            .ok_or(PhoenixIxError::MissingField("destination"))?;
        if destination == trader_account {
            return Err(PhoenixIxError::InvalidWithdrawDestination);
        }

        Ok(WithdrawNativeSolParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            destination,
            withdraw_queue: self
                .withdraw_queue
                .ok_or(PhoenixIxError::MissingField("withdraw_queue"))?,
            global_trader_index,
            active_trader_buffer,
            action,
        })
    }
}

/// Withdraw native SOL spot collateral.
///
/// Unlike a quote withdrawal this charges no fee, ignores the deposit cooldown,
/// and is never enqueued: if the exchange-wide throttle cannot absorb the
/// withdrawal's quote-notional value right now, the instruction fails rather
/// than queueing. Excess-only withdrawals bypass the throttle entirely.
pub fn create_withdraw_native_sol_ix(
    params: WithdrawNativeSolParams,
) -> Result<Instruction, PhoenixIxError> {
    let mut data = crate::PhoenixInstruction::WithdrawNativeSol
        .discriminant()
        .to_vec();
    params.action().encode(&mut data);

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    accounts.push(AccountMeta::writable(params.trader_account()));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));
    accounts.push(AccountMeta::writable(params.destination()));
    // The withdraw queue precedes the index arenas here, unlike a quote
    // withdrawal where it trails them.
    accounts.push(AccountMeta::writable(params.withdraw_queue()));
    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

////////////////////////////////////////////////////////////////////////////////
// TransferNativeSol
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_transfer_native_sol_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransferNativeSolParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    src_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    dst_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_option"))]
    permission_account: Option<Pubkey>,
    lamports: u64,
}

impl TransferNativeSolParams {
    pub fn builder() -> TransferNativeSolParamsBuilder {
        TransferNativeSolParamsBuilder::default()
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

    /// Amount in **lamports**, not quote units.
    pub fn lamports(&self) -> u64 {
        self.lamports
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransferNativeSolParamsBuilder {
    trader: Option<Pubkey>,
    src_trader_account: Option<Pubkey>,
    dst_trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    permission_account: Option<Pubkey>,
    lamports: Option<u64>,
}

impl TransferNativeSolParamsBuilder {
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

    /// Amount in **lamports**, not quote units.
    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Some(lamports);
        self
    }

    pub fn build(self) -> Result<TransferNativeSolParams, PhoenixIxError> {
        let lamports = self
            .lamports
            .ok_or(PhoenixIxError::MissingField("lamports"))?;
        if lamports == 0 {
            return Err(PhoenixIxError::InvalidTransferAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(TransferNativeSolParams {
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
            lamports,
        })
    }
}

/// Move native SOL spot collateral between two of a trader's accounts.
///
/// The source debit is margin checked. The destination is **rejected**, not
/// clamped, if the credit would push it past the per-trader cap. The
/// exchange-wide tally is unchanged, since the collateral never leaves the
/// protocol.
///
/// This shares its account layout exactly with a quote collateral transfer.
pub fn create_transfer_native_sol_ix(
    params: TransferNativeSolParams,
) -> Result<Instruction, PhoenixIxError> {
    let mut data = crate::PhoenixInstruction::TransferNativeSol
        .discriminant()
        .to_vec();
    data.extend_from_slice(&params.lamports().to_le_bytes());

    let mut accounts = log_accounts();
    // Both the global configuration and the perp asset map are readonly for a
    // transfer, unlike a withdrawal where both are writable.
    accounts.push(AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    accounts.push(AccountMeta::writable(params.src_trader_account()));
    accounts.push(AccountMeta::writable(params.dst_trader_account()));
    accounts.push(AccountMeta::readonly(params.perp_asset_map()));
    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );
    if let Some(permission_account) = params.permission_account() {
        accounts.push(AccountMeta::writable(permission_account));
    }

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

////////////////////////////////////////////////////////////////////////////////
// TransferNativeSolFromChildToParent
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_transfer_native_sol_from_child_to_parent_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransferNativeSolFromChildToParentParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    child_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    parent_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    trader_signs: bool,
}

impl TransferNativeSolFromChildToParentParams {
    pub fn builder() -> TransferNativeSolFromChildToParentParamsBuilder {
        TransferNativeSolFromChildToParentParamsBuilder::default()
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

    pub fn trader_signs(&self) -> bool {
        self.trader_signs
    }
}

#[derive(Debug, Clone)]
pub struct TransferNativeSolFromChildToParentParamsBuilder {
    trader: Option<Pubkey>,
    child_trader_account: Option<Pubkey>,
    parent_trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    trader_signs: bool,
}

impl Default for TransferNativeSolFromChildToParentParamsBuilder {
    fn default() -> Self {
        Self {
            trader: None,
            child_trader_account: None,
            parent_trader_account: None,
            perp_asset_map: None,
            global_trader_index: None,
            active_trader_buffer: None,
            trader_signs: true,
        }
    }
}

impl TransferNativeSolFromChildToParentParamsBuilder {
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

    /// Whether the trader wallet signs. Defaults to `true`, matching the quote
    /// collateral sweep builder.
    ///
    /// The sweep is permissionless unless the child opted out with the
    /// `disable_collateral_sweep` preference, so a crank that does not hold the
    /// wallet key should set this to `false`.
    pub fn trader_signs(mut self, trader_signs: bool) -> Self {
        self.trader_signs = trader_signs;
        self
    }

    pub fn build(self) -> Result<TransferNativeSolFromChildToParentParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(TransferNativeSolFromChildToParentParams {
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
            trader_signs: self.trader_signs,
        })
    }
}

/// Sweep a flat isolated child account's native SOL spot collateral into its
/// parent.
///
/// This is a silent no-op — not an error — when the child still has splines,
/// open orders, a position, or a negative quote balance, or when it holds no
/// native SOL.
pub fn create_transfer_native_sol_from_child_to_parent_ix(
    params: TransferNativeSolFromChildToParentParams,
) -> Result<Instruction, PhoenixIxError> {
    let mut accounts = log_accounts();
    accounts.push(AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(if params.trader_signs() {
        AccountMeta::readonly_signer(params.trader())
    } else {
        AccountMeta::readonly(params.trader())
    });
    accounts.push(AccountMeta::writable(params.child_trader_account()));
    accounts.push(AccountMeta::writable(params.parent_trader_account()));
    accounts.push(AccountMeta::readonly(params.perp_asset_map()));
    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data: crate::PhoenixInstruction::TransferNativeSolFromChildToParent
            .discriminant()
            .to_vec(),
    })
}

////////////////////////////////////////////////////////////////////////////////
// SwapNative
////////////////////////////////////////////////////////////////////////////////

/// Which way a [`create_swap_native_ix`] call trades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SwapDirection {
    /// Native SOL in, quote collateral out.
    Sell,
    /// Quote collateral in, native SOL out.
    Buy,
}

impl SwapDirection {
    fn as_u8(self) -> u8 {
        match self {
            Self::Sell => 0,
            Self::Buy => 1,
        }
    }

    /// The asset `min_amount_out` is denominated in, for rendering to users.
    pub fn output_asset(self) -> &'static str {
        match self {
            Self::Sell => "quote lots",
            Self::Buy => "lamports",
        }
    }
}

/// An account reference inside a packed venue instruction: an index into the
/// deduplicated account list plus its signer/writable bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PackedAccountMeta(u8);

impl PackedAccountMeta {
    const INDEX_SHIFT: u8 = 2;
    const SIGNER_BIT: u8 = 0b10;
    const WRITABLE_BIT: u8 = 0b01;

    fn new(index: u8, is_signer: bool, is_writable: bool) -> Self {
        let mut state = index << Self::INDEX_SHIFT;
        if is_signer {
            state |= Self::SIGNER_BIT;
        }
        if is_writable {
            state |= Self::WRITABLE_BIT;
        }
        Self(state)
    }

    pub fn state(self) -> u8 {
        self.0
    }

    pub fn index(self) -> u8 {
        self.0 >> Self::INDEX_SHIFT
    }

    pub fn is_signer(self) -> bool {
        self.0 & Self::SIGNER_BIT != 0
    }

    pub fn is_writable(self) -> bool {
        self.0 & Self::WRITABLE_BIT != 0
    }
}

/// One venue instruction, with its accounts addressed by index rather than by
/// pubkey.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PackedInstruction {
    pub program_id_index: u8,
    pub data: Vec<u8>,
    pub account_metas: Vec<PackedAccountMeta>,
}

/// Venue instructions packed for [`create_swap_native_ix`] or
/// [`create_liquidate_native_sol_ix`], together with the deduplicated account
/// list they index into.
///
/// The two halves must stay together — an index list is meaningless against a
/// different account list — so [`pack_venue_instructions`] returns them as one
/// value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedVenueInstructions {
    instructions: Vec<PackedInstruction>,
    accounts: Vec<AccountMeta>,
}

impl PackedVenueInstructions {
    pub fn instructions(&self) -> &[PackedInstruction] {
        &self.instructions
    }

    pub fn accounts(&self) -> &[AccountMeta] {
        &self.accounts
    }
}

/// Pack venue instructions for a swap.
///
/// Accounts are deduplicated by pubkey, unioning their writable and signer
/// bits, and each instruction's metas become indices into that list.
///
/// Only `signer` — the swap's signer — may be marked as a signer by a venue
/// instruction: the program has no other key to sign with.
pub fn pack_venue_instructions(
    signer: &Pubkey,
    instructions: &[Instruction],
) -> Result<PackedVenueInstructions, PhoenixIxError> {
    let mut accounts: Vec<AccountMeta> = Vec::new();
    let mut packed = Vec::with_capacity(instructions.len());

    let index_of = |accounts: &mut Vec<AccountMeta>,
                    pubkey: Pubkey,
                    is_signer: bool,
                    is_writable: bool|
     -> Result<u8, PhoenixIxError> {
        if let Some(position) = accounts.iter().position(|meta| meta.pubkey == pubkey) {
            accounts[position].is_signer |= is_signer;
            accounts[position].is_writable |= is_writable;
            return u8::try_from(position).map_err(|_| PhoenixIxError::TooManyVenueAccounts);
        }
        if accounts.len() >= MAX_PACKED_EXTERNAL_ACCOUNTS {
            return Err(PhoenixIxError::TooManyVenueAccounts);
        }
        accounts.push(AccountMeta {
            pubkey,
            is_signer,
            is_writable,
        });
        u8::try_from(accounts.len() - 1).map_err(|_| PhoenixIxError::TooManyVenueAccounts)
    };

    for instruction in instructions {
        let program_id_index = index_of(&mut accounts, instruction.program_id, false, false)?;

        let mut account_metas = Vec::with_capacity(instruction.accounts.len());
        for meta in &instruction.accounts {
            if meta.is_signer && meta.pubkey != *signer {
                return Err(PhoenixIxError::UnexpectedVenueSigner);
            }
            let index = index_of(&mut accounts, meta.pubkey, meta.is_signer, meta.is_writable)?;
            account_metas.push(PackedAccountMeta::new(
                index,
                meta.is_signer,
                meta.is_writable,
            ));
        }

        packed.push(PackedInstruction {
            program_id_index,
            data: instruction.data.clone(),
            account_metas,
        });
    }

    Ok(PackedVenueInstructions {
        instructions: packed,
        accounts,
    })
}

////////////////////////////////////////////////////////////////////////////////
// LiquidateNativeSol
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_liquidate_native_sol_ix`].
#[derive(Debug, Clone)]
pub struct LiquidateNativeSolParams {
    signer: Pubkey,
    permission_account: Option<Pubkey>,
    liquidatee_account: Pubkey,
    mint: Pubkey,
    perp_asset_map: Pubkey,
    signer_quote_token_account: Pubkey,
    withdraw_queue: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    max_native_sol_amount: u64,
    extra_trader_accounts: Vec<Pubkey>,
    venue: PackedVenueInstructions,
}

impl LiquidateNativeSolParams {
    pub fn builder() -> LiquidateNativeSolParamsBuilder {
        LiquidateNativeSolParamsBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LiquidateNativeSolParamsBuilder {
    signer: Option<Pubkey>,
    permission_account: Option<Pubkey>,
    liquidatee_account: Option<Pubkey>,
    mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    signer_quote_token_account: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    max_native_sol_amount: Option<u64>,
    extra_trader_accounts: Vec<Pubkey>,
    venue: Option<PackedVenueInstructions>,
}

impl LiquidateNativeSolParamsBuilder {
    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Delegated liquidation permission PDA. Omit for the direct risk-authority
    /// path, where the signer occupies both account slots.
    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn liquidatee_account(mut self, liquidatee_account: Pubkey) -> Self {
        self.liquidatee_account = Some(liquidatee_account);
        self
    }

    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn signer_quote_token_account(mut self, account: Pubkey) -> Self {
        self.signer_quote_token_account = Some(account);
        self
    }

    pub fn withdraw_queue(mut self, withdraw_queue: Pubkey) -> Self {
        self.withdraw_queue = Some(withdraw_queue);
        self
    }

    pub fn global_trader_index(mut self, accounts: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(accounts);
        self
    }

    pub fn active_trader_buffer(mut self, accounts: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(accounts);
        self
    }

    pub fn max_native_sol_amount(mut self, amount: u64) -> Self {
        self.max_native_sol_amount = Some(amount);
        self
    }

    pub fn extra_trader_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.extra_trader_accounts = accounts;
        self
    }

    pub fn venue(mut self, venue: PackedVenueInstructions) -> Self {
        self.venue = Some(venue);
        self
    }

    pub fn build(self) -> Result<LiquidateNativeSolParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;
        Ok(LiquidateNativeSolParams {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            permission_account: self.permission_account,
            liquidatee_account: self
                .liquidatee_account
                .ok_or(PhoenixIxError::MissingField("liquidatee_account"))?,
            mint: self.mint.ok_or(PhoenixIxError::MissingField("mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            signer_quote_token_account: self
                .signer_quote_token_account
                .ok_or(PhoenixIxError::MissingField("signer_quote_token_account"))?,
            withdraw_queue: self
                .withdraw_queue
                .ok_or(PhoenixIxError::MissingField("withdraw_queue"))?,
            global_trader_index,
            active_trader_buffer,
            max_native_sol_amount: self
                .max_native_sol_amount
                .ok_or(PhoenixIxError::MissingField("max_native_sol_amount"))?,
            extra_trader_accounts: self.extra_trader_accounts,
            venue: self.venue.ok_or(PhoenixIxError::MissingField("venue"))?,
        })
    }
}

/// Seize a liquidatee's native SOL collateral at the discounted index price.
/// The signer must be the risk authority or hold the supplied delegated
/// liquidation permission.
pub fn create_liquidate_native_sol_ix(
    params: LiquidateNativeSolParams,
) -> Result<Instruction, PhoenixIxError> {
    let mut data = crate::PhoenixInstruction::LiquidateNativeSol
        .discriminant()
        .to_vec();
    data.extend_from_slice(&params.max_native_sol_amount.to_le_bytes());
    data.extend_from_slice(
        &u64::try_from(params.extra_trader_accounts.len())
            .map_err(|_| PhoenixIxError::InstructionDataTooLarge)?
            .to_le_bytes(),
    );
    encode_packed_instructions(&mut data, params.venue.instructions());

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    accounts.push(AccountMeta::writable(get_global_vault_address(
        &params.mint,
    )?));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(get_native_sol_authority_address()?));
    accounts.push(AccountMeta::writable_signer(params.signer));
    accounts.push(AccountMeta::writable(
        params.permission_account.unwrap_or(params.signer),
    ));
    accounts.push(AccountMeta::writable(params.signer_quote_token_account));
    accounts.push(AccountMeta::writable(params.liquidatee_account));
    accounts.push(AccountMeta::writable(params.withdraw_queue));
    push_trader_index_accounts(
        &mut accounts,
        &params.global_trader_index,
        &params.active_trader_buffer,
    );
    accounts.extend(
        params
            .extra_trader_accounts
            .into_iter()
            .map(AccountMeta::writable),
    );
    accounts.extend_from_slice(params.venue.accounts());

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Slippage protection for a swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SwapSlippage {
    /// Fail the swap unless the venue returns at least this much of the output
    /// asset. See [`SwapNativeParamsBuilder::min_amount_out`] for the units.
    MinAmountOut(u64),
    /// No slippage protection at all: the swap executes at whatever price the
    /// venue returns.
    ///
    /// Only use this when some other mechanism bounds the price. The program
    /// applies no oracle floor to swaps.
    Unprotected,
}

impl SwapSlippage {
    fn as_min_amount_out(self) -> u64 {
        match self {
            Self::MinAmountOut(min_amount_out) => min_amount_out,
            Self::Unprotected => 0,
        }
    }
}

/// Parameters for [`create_swap_native_ix`].
#[derive(Debug, Clone)]
pub struct SwapNativeParams {
    signer: Pubkey,
    trader_account: Pubkey,
    mint: Pubkey,
    perp_asset_map: Pubkey,
    signer_quote_token_account: Pubkey,
    withdraw_queue: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    direction: SwapDirection,
    amount_in: u64,
    slippage: SwapSlippage,
    venue: PackedVenueInstructions,
}

impl SwapNativeParams {
    pub fn builder() -> SwapNativeParamsBuilder {
        SwapNativeParamsBuilder::default()
    }

    pub fn direction(&self) -> SwapDirection {
        self.direction
    }

    pub fn amount_in(&self) -> u64 {
        self.amount_in
    }

    pub fn slippage(&self) -> SwapSlippage {
        self.slippage
    }

    pub fn venue(&self) -> &PackedVenueInstructions {
        &self.venue
    }
}

#[derive(Debug, Clone, Default)]
pub struct SwapNativeParamsBuilder {
    signer: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    signer_quote_token_account: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    direction: Option<SwapDirection>,
    amount_in: Option<u64>,
    slippage: Option<SwapSlippage>,
    venue: Option<PackedVenueInstructions>,
}

impl SwapNativeParamsBuilder {
    /// The swap signer: either the trader's wallet or its position authority.
    /// Both legs of the swap transit this key's accounts.
    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    /// The exchange's canonical quote mint.
    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn signer_quote_token_account(mut self, signer_quote_token_account: Pubkey) -> Self {
        self.signer_quote_token_account = Some(signer_quote_token_account);
        self
    }

    pub fn withdraw_queue(mut self, withdraw_queue: Pubkey) -> Self {
        self.withdraw_queue = Some(withdraw_queue);
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

    pub fn direction(mut self, direction: SwapDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Amount of the *input* asset: lamports for a sell, quote lots for a buy.
    pub fn amount_in(mut self, amount_in: u64) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    /// Minimum acceptable amount of the **output** asset — quote lots for a
    /// [`SwapDirection::Sell`], lamports for a [`SwapDirection::Buy`].
    ///
    /// Getting the unit wrong either disables protection or makes every swap
    /// fail, so this is required rather than defaulted.
    pub fn min_amount_out(mut self, min_amount_out: u64) -> Self {
        self.slippage = Some(SwapSlippage::MinAmountOut(min_amount_out));
        self
    }

    /// Disable slippage protection entirely.
    ///
    /// The program applies no oracle price floor to swaps, so an unprotected
    /// swap can execute at any price the venue returns. Prefer
    /// [`Self::min_amount_out`].
    pub fn without_slippage_protection(mut self) -> Self {
        self.slippage = Some(SwapSlippage::Unprotected);
        self
    }

    pub fn venue(mut self, venue: PackedVenueInstructions) -> Self {
        self.venue = Some(venue);
        self
    }

    pub fn build(self) -> Result<SwapNativeParams, PhoenixIxError> {
        let amount_in = self
            .amount_in
            .ok_or(PhoenixIxError::MissingField("amount_in"))?;
        if amount_in == 0 {
            return Err(PhoenixIxError::InvalidSwapAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(SwapNativeParams {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            mint: self.mint.ok_or(PhoenixIxError::MissingField("mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            signer_quote_token_account: self
                .signer_quote_token_account
                .ok_or(PhoenixIxError::MissingField("signer_quote_token_account"))?,
            withdraw_queue: self
                .withdraw_queue
                .ok_or(PhoenixIxError::MissingField("withdraw_queue"))?,
            global_trader_index,
            active_trader_buffer,
            direction: self
                .direction
                .ok_or(PhoenixIxError::MissingField("direction"))?,
            amount_in,
            // Deliberately not defaulted: on-chain a zero disables the check.
            slippage: self
                .slippage
                .ok_or(PhoenixIxError::MissingField("min_amount_out"))?,
            venue: self.venue.ok_or(PhoenixIxError::MissingField("venue"))?,
        })
    }
}

/// Swap between native SOL spot collateral and quote collateral through an
/// external venue.
///
/// The program withdraws the input leg to the signer, runs the caller-supplied
/// venue instructions, deposits the output leg, and then checks that the
/// trader's margin state did not worsen. The exchange-wide withdrawal throttle
/// is charged only the swap's *value loss* (slippage plus venue fees), not the
/// full amount moved.
///
/// # Slippage
///
/// [`SwapNativeParamsBuilder::min_amount_out`] is the **only** price protection
/// this instruction has — there is no oracle floor. Its unit is the output
/// asset: quote lots for a sell, lamports for a buy.
///
/// # Delegated authorities
///
/// The signer may be the trader's *position authority* rather than its wallet,
/// and the venue instructions are entirely caller-supplied. A position
/// authority you do not control can therefore route a delegator's collateral
/// through a venue of its choosing, bounded only by `min_amount_out`. Two
/// independent opt-outs gate that path, and either one makes such a swap fail
/// with error **7101 `PositionAuthoritySwapDisabled`**:
///
/// - the exchange-wide `disable_position_authority_swap` flag on the spot
///   collateral configuration; and
/// - the trader's own `disable_position_authority_swap` preference bit.
///
/// Swaps signed by the trader's wallet are never gated. Use
/// `phoenix_rise_accounts` to check both flags before building the
/// instruction.
///
/// A [`SwapDirection::Buy`] additionally honors the deposit cooldown and is
/// rejected while the trader has a queued withdrawal.
pub fn create_swap_native_ix(params: SwapNativeParams) -> Result<Instruction, PhoenixIxError> {
    let mut data = crate::PhoenixInstruction::SwapNative
        .discriminant()
        .to_vec();
    data.push(params.direction.as_u8());
    data.extend_from_slice(&params.amount_in.to_le_bytes());
    data.extend_from_slice(&params.slippage.as_min_amount_out().to_le_bytes());
    encode_packed_instructions(&mut data, params.venue.instructions());

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    accounts.push(AccountMeta::writable(get_global_vault_address(
        &params.mint,
    )?));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(get_native_sol_authority_address()?));
    accounts.push(AccountMeta::writable_signer(params.signer));
    accounts.push(AccountMeta::writable(params.signer_quote_token_account));
    accounts.push(AccountMeta::writable(params.trader_account));
    accounts.push(AccountMeta::writable(params.withdraw_queue));
    push_trader_index_accounts(
        &mut accounts,
        &params.global_trader_index,
        &params.active_trader_buffer,
    );
    accounts.extend_from_slice(params.venue.accounts());

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_packed_instructions(data: &mut Vec<u8>, instructions: &[PackedInstruction]) {
    data.extend_from_slice(&(instructions.len() as u32).to_le_bytes());
    for instruction in instructions {
        data.push(instruction.program_id_index);
        data.extend_from_slice(&(instruction.data.len() as u32).to_le_bytes());
        data.extend_from_slice(&instruction.data);
        data.extend_from_slice(&(instruction.account_metas.len() as u32).to_le_bytes());
        for meta in &instruction.account_metas {
            data.push(meta.state());
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Shared helpers
////////////////////////////////////////////////////////////////////////////////

fn log_accounts() -> Vec<AccountMeta> {
    vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
    ]
}

fn require_index_accounts(
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
) -> Result<(Vec<Pubkey>, Vec<Pubkey>), PhoenixIxError> {
    let global_trader_index =
        global_trader_index.ok_or(PhoenixIxError::MissingField("global_trader_index"))?;
    if global_trader_index.is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }

    let active_trader_buffer =
        active_trader_buffer.ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?;
    if active_trader_buffer.is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }

    Ok((global_trader_index, active_trader_buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer_collateral::{TransferCollateralParams, create_transfer_collateral_ix};

    fn key(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn index_accounts() -> (Vec<Pubkey>, Vec<Pubkey>) {
        (vec![key(20), key(21)], vec![key(30), key(31)])
    }

    fn sync_native_ix() -> Instruction {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        create_sync_native_ix(
            SyncNativeParams::builder()
                .trader_account(key(2))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .build()
                .unwrap(),
        )
        .unwrap()
    }

    fn withdraw_native_sol_ix(action: WithdrawNativeSolAction) -> Instruction {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        create_withdraw_native_sol_ix(
            WithdrawNativeSolParams::builder()
                .trader(key(1))
                .trader_account(key(2))
                .perp_asset_map(key(3))
                .destination(key(4))
                .withdraw_queue(key(5))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .action(action)
                .build()
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn sync_native_takes_no_data_and_writes_the_global_configuration() {
        let ix = sync_native_ix();

        assert_eq!(
            ix.data,
            crate::PhoenixInstruction::SyncNative
                .discriminant()
                .to_vec()
        );
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert!(ix.accounts[2].is_writable);
        assert_eq!(ix.accounts[3].pubkey, key(2));
        assert!(ix.accounts[3].is_writable);
        // Permissionless: nothing signs.
        assert!(ix.accounts.iter().all(|meta| !meta.is_signer));
    }

    /// The withdraw queue sits before the index arenas here, unlike the quote
    /// withdrawal builder where it trails them.
    #[test]
    fn withdraw_native_sol_places_the_withdraw_queue_before_the_arenas() {
        let ix = withdraw_native_sol_ix(WithdrawNativeSolAction::WithoutExcess { amount: 7 });

        let pubkeys: Vec<Pubkey> = ix.accounts.iter().map(|meta| meta.pubkey).collect();
        assert_eq!(
            pubkeys,
            vec![
                *PHOENIX_PROGRAM_ID,
                *PHOENIX_LOG_AUTHORITY,
                *PHOENIX_GLOBAL_CONFIGURATION,
                key(1),
                key(2),
                key(3),
                key(4),
                key(5),
                key(20),
                key(21),
                key(30),
                key(31),
            ]
        );
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);
        assert!(ix.accounts[7].is_writable);
    }

    #[test]
    fn withdraw_native_sol_encodes_each_action_variant() {
        let discriminant = crate::PhoenixInstruction::WithdrawNativeSol
            .discriminant()
            .to_vec();

        let all_excess = withdraw_native_sol_ix(WithdrawNativeSolAction::AllExcess);
        assert_eq!(all_excess.data, [discriminant.clone(), vec![0]].concat());

        let with_excess = withdraw_native_sol_ix(WithdrawNativeSolAction::WithExcess { amount: 7 });
        assert_eq!(
            with_excess.data,
            [discriminant.clone(), vec![1], 7u64.to_le_bytes().to_vec()].concat()
        );

        let without_excess =
            withdraw_native_sol_ix(WithdrawNativeSolAction::WithoutExcess { amount: 7 });
        assert_eq!(
            without_excess.data,
            [discriminant, vec![2], 7u64.to_le_bytes().to_vec()].concat()
        );
    }

    #[test]
    fn withdraw_native_sol_rejects_a_zero_amount_but_allows_an_excess_sweep() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let params = |action| {
            WithdrawNativeSolParams::builder()
                .trader(key(1))
                .trader_account(key(2))
                .perp_asset_map(key(3))
                .destination(key(4))
                .withdraw_queue(key(5))
                .global_trader_index(global_trader_index.clone())
                .active_trader_buffer(active_trader_buffer.clone())
                .action(action)
                .build()
        };

        assert!(matches!(
            params(WithdrawNativeSolAction::WithoutExcess { amount: 0 }),
            Err(PhoenixIxError::InvalidWithdrawAmount)
        ));
        assert!(params(WithdrawNativeSolAction::AllExcess).is_ok());
    }

    #[test]
    fn withdraw_native_sol_rejects_the_trader_account_as_a_destination() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let result = WithdrawNativeSolParams::builder()
            .trader(key(1))
            .trader_account(key(2))
            .perp_asset_map(key(3))
            .destination(key(2))
            .withdraw_queue(key(5))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .action(WithdrawNativeSolAction::WithoutExcess { amount: 7 })
            .build();

        assert!(matches!(
            result,
            Err(PhoenixIxError::InvalidWithdrawDestination)
        ));
    }

    /// `TransferNativeSol` reuses the quote transfer's account group verbatim.
    /// Locking that in catches an accidental divergence in either builder.
    #[test]
    fn transfer_native_sol_matches_the_quote_transfer_account_layout() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let native = create_transfer_native_sol_ix(
            TransferNativeSolParams::builder()
                .trader(key(1))
                .src_trader_account(key(2))
                .dst_trader_account(key(3))
                .perp_asset_map(key(4))
                .global_trader_index(global_trader_index.clone())
                .active_trader_buffer(active_trader_buffer.clone())
                .lamports(9)
                .build()
                .unwrap(),
        )
        .unwrap();
        let quote = create_transfer_collateral_ix(
            TransferCollateralParams::builder()
                .trader(key(1))
                .src_trader_account(key(2))
                .dst_trader_account(key(3))
                .perp_asset_map(key(4))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .amount(9)
                .build()
                .unwrap(),
        )
        .unwrap();

        assert_eq!(native.accounts, quote.accounts);
        // Same amount encoding, different discriminant.
        assert_eq!(native.data[8..], quote.data[8..]);
        assert_ne!(native.data[..8], quote.data[..8]);
    }

    #[test]
    fn transfer_native_sol_appends_an_optional_permission_account() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let ix = create_transfer_native_sol_ix(
            TransferNativeSolParams::builder()
                .trader(key(1))
                .src_trader_account(key(2))
                .dst_trader_account(key(3))
                .perp_asset_map(key(4))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .permission_account(key(9))
                .lamports(9)
                .build()
                .unwrap(),
        )
        .unwrap();

        let last = ix.accounts.last().unwrap();
        assert_eq!(last.pubkey, key(9));
        assert!(last.is_writable);
    }

    #[test]
    fn child_to_parent_sweep_can_drop_the_trader_signature_for_cranks() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let build = |trader_signs: bool| {
            create_transfer_native_sol_from_child_to_parent_ix(
                TransferNativeSolFromChildToParentParams::builder()
                    .trader(key(1))
                    .child_trader_account(key(2))
                    .parent_trader_account(key(3))
                    .perp_asset_map(key(4))
                    .global_trader_index(global_trader_index.clone())
                    .active_trader_buffer(active_trader_buffer.clone())
                    .trader_signs(trader_signs)
                    .build()
                    .unwrap(),
            )
            .unwrap()
        };

        assert!(build(true).accounts[3].is_signer);
        assert!(!build(false).accounts[3].is_signer);

        // Defaults to signing, matching the quote sweep builder.
        let default = create_transfer_native_sol_from_child_to_parent_ix(
            TransferNativeSolFromChildToParentParams::builder()
                .trader(key(1))
                .child_trader_account(key(2))
                .parent_trader_account(key(3))
                .perp_asset_map(key(4))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .build()
                .unwrap(),
        )
        .unwrap();
        assert!(default.accounts[3].is_signer);
    }

    #[test]
    fn pack_venue_instructions_deduplicates_accounts_and_unions_their_bits() {
        let signer = key(1);
        let venue_program = key(50);
        let shared = key(51);

        let packed = pack_venue_instructions(
            &signer,
            &[
                Instruction {
                    program_id: venue_program,
                    accounts: vec![
                        AccountMeta::readonly(shared),
                        AccountMeta::writable_signer(signer),
                    ],
                    data: vec![1, 2, 3],
                },
                Instruction {
                    program_id: venue_program,
                    accounts: vec![AccountMeta::writable(shared)],
                    data: vec![4],
                },
            ],
        )
        .unwrap();

        // venue_program, shared, signer — the program id is itself an account.
        assert_eq!(packed.accounts().len(), 3);
        assert_eq!(packed.accounts()[1].pubkey, shared);
        // Read-only in the first instruction, writable in the second.
        assert!(packed.accounts()[1].is_writable);
        assert!(packed.accounts()[2].is_signer);

        assert_eq!(packed.instructions()[0].program_id_index, 0);
        let first_metas = &packed.instructions()[0].account_metas;
        assert_eq!(first_metas[0].index(), 1);
        assert!(!first_metas[0].is_writable());
        assert_eq!(first_metas[1].index(), 2);
        assert!(first_metas[1].is_signer() && first_metas[1].is_writable());
    }

    #[test]
    fn pack_venue_instructions_rejects_a_foreign_signer() {
        let result = pack_venue_instructions(
            &key(1),
            &[Instruction {
                program_id: key(50),
                accounts: vec![AccountMeta::readonly_signer(key(99))],
                data: Vec::new(),
            }],
        );

        assert!(matches!(result, Err(PhoenixIxError::UnexpectedVenueSigner)));
    }

    #[test]
    fn pack_venue_instructions_rejects_more_accounts_than_the_encoding_addresses() {
        let accounts: Vec<AccountMeta> = (0..MAX_PACKED_EXTERNAL_ACCOUNTS)
            .map(|i| AccountMeta::readonly(Pubkey::new_from_array([i as u8; 32])))
            .collect();

        // The program id occupies one slot, so a full complement of distinct
        // accounts overflows.
        let result = pack_venue_instructions(
            &key(1),
            &[Instruction {
                program_id: key(200),
                accounts,
                data: Vec::new(),
            }],
        );

        assert!(matches!(result, Err(PhoenixIxError::TooManyVenueAccounts)));
    }

    #[test]
    fn liquidate_native_sol_encodes_and_orders_direct_authority_accounts() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let ix = create_liquidate_native_sol_ix(
            LiquidateNativeSolParams::builder()
                .signer(key(1))
                .liquidatee_account(key(2))
                .mint(key(3))
                .perp_asset_map(key(4))
                .signer_quote_token_account(key(5))
                .withdraw_queue(key(6))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .max_native_sol_amount(7)
                .extra_trader_accounts(vec![key(40)])
                .venue(pack_venue_instructions(&key(1), &[]).unwrap())
                .build()
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            &ix.data[..8],
            crate::PhoenixInstruction::LiquidateNativeSol.discriminant()
        );
        assert_eq!(&ix.data[8..16], &7u64.to_le_bytes());
        assert_eq!(&ix.data[16..24], &1u64.to_le_bytes());
        assert_eq!(&ix.data[24..28], &0u32.to_le_bytes());

        // Full account order, mirroring the on-chain
        // `LiquidateNativeSolInstructionGroup` behind the log prefix. A wrong
        // position here is a wrong account on a liquidation.
        assert_eq!(ix.accounts[0], AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
        assert_eq!(
            ix.accounts[1],
            AccountMeta::readonly(crate::constants::phoenix_log_authority())
        );
        assert_eq!(
            ix.accounts[2],
            AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION)
        );
        assert_eq!(ix.accounts[3], AccountMeta::writable(key(4)));
        assert_eq!(
            ix.accounts[4],
            AccountMeta::writable(get_global_vault_address(&key(3)).unwrap())
        );
        assert_eq!(ix.accounts[5], AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
        assert_eq!(
            ix.accounts[6],
            AccountMeta::readonly(get_native_sol_authority_address().unwrap())
        );
        assert_eq!(ix.accounts[7].pubkey, key(1));
        assert!(ix.accounts[7].is_signer && ix.accounts[7].is_writable);
        assert_eq!(ix.accounts[8].pubkey, key(1));
        assert!(ix.accounts[8].is_writable && !ix.accounts[8].is_signer);
        assert_eq!(ix.accounts[9].pubkey, key(5));
        assert_eq!(ix.accounts[10].pubkey, key(2));
        assert_eq!(ix.accounts[11].pubkey, key(6));
        // Trader index groups: GTI header+arena then ATB header+arena, all
        // writable, followed by writable shortfall traders.
        let (global_trader_index, active_trader_buffer) = index_accounts();
        assert_eq!(
            ix.accounts[12..16]
                .iter()
                .map(|meta| meta.pubkey)
                .collect::<Vec<_>>(),
            [global_trader_index, active_trader_buffer].concat()
        );
        assert!(ix.accounts[12..16].iter().all(|meta| meta.is_writable));
        assert_eq!(ix.accounts[16], AccountMeta::writable(key(40)));
        assert_eq!(ix.accounts.len(), 17, "no venue accounts in this case");
    }

    #[test]
    fn liquidate_native_sol_accepts_a_delegated_permission_account() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let ix = create_liquidate_native_sol_ix(
            LiquidateNativeSolParams::builder()
                .signer(key(1))
                .permission_account(key(9))
                .liquidatee_account(key(2))
                .mint(key(3))
                .perp_asset_map(key(4))
                .signer_quote_token_account(key(5))
                .withdraw_queue(key(6))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .max_native_sol_amount(7)
                .venue(pack_venue_instructions(&key(1), &[]).unwrap())
                .build()
                .unwrap(),
        )
        .unwrap();

        assert_eq!(ix.accounts[8], AccountMeta::writable(key(9)));
    }

    fn swap_params(direction: SwapDirection) -> Result<SwapNativeParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        SwapNativeParams::builder()
            .signer(key(1))
            .trader_account(key(2))
            .mint(key(3))
            .perp_asset_map(key(4))
            .signer_quote_token_account(key(5))
            .withdraw_queue(key(6))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .direction(direction)
            .amount_in(1_000)
            .min_amount_out(900)
            .venue(pack_venue_instructions(&key(1), &[]).unwrap())
            .build()
    }

    #[test]
    fn swap_native_requires_explicit_slippage_protection() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let without_slippage = SwapNativeParams::builder()
            .signer(key(1))
            .trader_account(key(2))
            .mint(key(3))
            .perp_asset_map(key(4))
            .signer_quote_token_account(key(5))
            .withdraw_queue(key(6))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .direction(SwapDirection::Sell)
            .amount_in(1_000)
            .venue(pack_venue_instructions(&key(1), &[]).unwrap())
            .build();

        assert!(matches!(
            without_slippage,
            Err(PhoenixIxError::MissingField("min_amount_out"))
        ));
    }

    #[test]
    fn swap_native_rejects_a_zero_input_amount() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let result = SwapNativeParams::builder()
            .signer(key(1))
            .trader_account(key(2))
            .mint(key(3))
            .perp_asset_map(key(4))
            .signer_quote_token_account(key(5))
            .withdraw_queue(key(6))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .direction(SwapDirection::Sell)
            .amount_in(0)
            .min_amount_out(1)
            .venue(pack_venue_instructions(&key(1), &[]).unwrap())
            .build();

        assert!(matches!(result, Err(PhoenixIxError::InvalidSwapAmount)));
    }

    #[test]
    fn swap_native_encodes_direction_amounts_and_packed_venue_instructions() {
        let venue = pack_venue_instructions(
            &key(1),
            &[Instruction {
                program_id: key(50),
                accounts: vec![AccountMeta::writable(key(51))],
                data: vec![9, 9],
            }],
        )
        .unwrap();

        let (global_trader_index, active_trader_buffer) = index_accounts();
        let ix = create_swap_native_ix(
            SwapNativeParams::builder()
                .signer(key(1))
                .trader_account(key(2))
                .mint(key(3))
                .perp_asset_map(key(4))
                .signer_quote_token_account(key(5))
                .withdraw_queue(key(6))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .direction(SwapDirection::Buy)
                .amount_in(1_000)
                .min_amount_out(900)
                .venue(venue)
                .build()
                .unwrap(),
        )
        .unwrap();

        let mut expected = crate::PhoenixInstruction::SwapNative
            .discriminant()
            .to_vec();
        expected.push(1); // Buy
        expected.extend_from_slice(&1_000u64.to_le_bytes());
        expected.extend_from_slice(&900u64.to_le_bytes());
        expected.extend_from_slice(&1u32.to_le_bytes()); // one packed instruction
        expected.push(0); // program id index
        expected.extend_from_slice(&2u32.to_le_bytes()); // data length
        expected.extend_from_slice(&[9, 9]);
        expected.extend_from_slice(&1u32.to_le_bytes()); // one account meta
        expected.push(PackedAccountMeta::new(1, false, true).state());
        assert_eq!(ix.data, expected);

        // Venue accounts trail the protocol accounts.
        let venue_accounts = &ix.accounts[ix.accounts.len() - 2..];
        assert_eq!(venue_accounts[0].pubkey, key(50));
        assert_eq!(venue_accounts[1].pubkey, key(51));
    }

    #[test]
    fn swap_native_marks_the_signer_writable_and_derives_the_native_sol_authority() {
        let ix = create_swap_native_ix(swap_params(SwapDirection::Sell).unwrap()).unwrap();

        assert_eq!(ix.accounts[5].pubkey, SPL_TOKEN_PROGRAM_ID);
        assert_eq!(ix.accounts[6].pubkey, SYSTEM_PROGRAM_ID);
        assert_eq!(
            ix.accounts[7].pubkey,
            get_native_sol_authority_address().unwrap()
        );
        assert!(!ix.accounts[7].is_writable);
        assert_eq!(ix.accounts[8].pubkey, key(1));
        assert!(ix.accounts[8].is_signer && ix.accounts[8].is_writable);
    }

    #[test]
    fn unprotected_swaps_encode_a_zero_min_amount_out() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let ix = create_swap_native_ix(
            SwapNativeParams::builder()
                .signer(key(1))
                .trader_account(key(2))
                .mint(key(3))
                .perp_asset_map(key(4))
                .signer_quote_token_account(key(5))
                .withdraw_queue(key(6))
                .global_trader_index(global_trader_index)
                .active_trader_buffer(active_trader_buffer)
                .direction(SwapDirection::Sell)
                .amount_in(1_000)
                .without_slippage_protection()
                .venue(pack_venue_instructions(&key(1), &[]).unwrap())
                .build()
                .unwrap(),
        )
        .unwrap();

        // discriminant (8) + direction (1) + amount_in (8) => min_amount_out.
        assert_eq!(&ix.data[17..25], &0u64.to_le_bytes());
    }

    #[test]
    fn swap_direction_output_asset_units_are_documented_per_direction() {
        assert_eq!(SwapDirection::Sell.output_asset(), "quote lots");
        assert_eq!(SwapDirection::Buy.output_asset(), "lamports");
    }
}
