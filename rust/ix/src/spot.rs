//! SPL spot collateral instructions.
//!
//! SPL tokens can be posted as collateral alongside the canonical quote token
//! and native SOL. Each configured spot asset is identified by its mint, and
//! its tokens are custodied in the associated token account of the trader's
//! wallet PDA (seeds `["wallet", trader_account]`) for the asset's mint. The
//! *accounted* balance is tracked in the trader position map; tokens sitting in
//! the custody ATA beyond it ("excess") carry no margin value.
//!
//! The builders here derive the wallet PDA and its custody ATAs internally
//! from `trader_account` and the asset's mint, so callers never assemble
//! those addresses by hand.
//!
//! Spot user and liquidation instructions are exposed here:
//!
//! - [`create_sync_spot_ix`] reconciles the accounted balance against the
//!   custody ATA's actual token balance. Permissionless.
//! - [`create_withdraw_spot_ix`] moves tokens out to any token account of the
//!   asset's mint.
//! - [`create_transfer_spot_ix`] and
//!   [`create_transfer_spot_from_child_to_parent_ix`] move collateral between
//!   trader accounts.
//! - [`create_swap_spot_with_usdc_ix`], [`create_swap_spot_with_sol_ix`], and
//!   [`create_swap_spot_with_spot_ix`] swap between a spot asset and quote
//!   collateral, native SOL collateral, or another spot asset through an
//!   external venue.
//! - [`create_liquidate_spot_ix`] lets the risk authority or a delegated
//!   liquidator seize spot collateral.
//!
//! Admin instructions (add, configure) are deliberately not exposed, matching
//! the native SOL module.

use solana_pubkey::Pubkey;

use crate::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    get_associated_token_address, get_global_vault_address, get_native_sol_authority_address,
    get_trader_wallet_address,
};
use crate::error::PhoenixIxError;
use crate::native_sol::{
    PackedVenueInstructions, SwapDirection, SwapSlippage, encode_packed_instructions,
    encode_swap_data, log_accounts, require_index_accounts,
};
use crate::types::{AccountMeta, Instruction, push_trader_index_accounts};

////////////////////////////////////////////////////////////////////////////////
// SyncSpot
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_sync_spot_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SyncSpotParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    mint: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
}

impl SyncSpotParams {
    pub fn builder() -> SyncSpotParamsBuilder {
        SyncSpotParamsBuilder::default()
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn mint(&self) -> Pubkey {
        self.mint
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

#[derive(Debug, Clone, Default)]
pub struct SyncSpotParamsBuilder {
    trader_account: Option<Pubkey>,
    mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl SyncSpotParamsBuilder {
    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    /// The spot asset's mint, which selects the asset.
    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
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

    pub fn build(self) -> Result<SyncSpotParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(SyncSpotParams {
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            mint: self.mint.ok_or(PhoenixIxError::MissingField("mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

/// Reconcile a trader's accounted spot collateral against the token balance
/// actually in the custody ATA.
///
/// Permissionless — anyone may crank it. An upward reconciliation is clamped
/// to the per-trader and exchange-wide caps; whatever the clamp leaves over
/// stays in the custody ATA as uncounted excess, recoverable by a later sync
/// once headroom frees up. Depositing is a plain SPL transfer into the custody
/// ATA followed by this instruction.
pub fn create_sync_spot_ix(params: SyncSpotParams) -> Result<Instruction, PhoenixIxError> {
    let wallet = get_trader_wallet_address(&params.trader_account)?;
    let wallet_ata = get_associated_token_address(&wallet, &params.mint)?;

    let data = crate::PhoenixInstruction::SyncSpot.discriminant().to_vec();

    let mut accounts = log_accounts();
    // Unlike `SyncNative`, the global configuration is readonly: the spot
    // metadata and exchange-wide tally live in the perp asset map, which is
    // writable instead.
    accounts.push(AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));
    accounts.push(AccountMeta::readonly(params.mint()));
    accounts.push(AccountMeta::writable(params.trader_account()));
    accounts.push(AccountMeta::readonly(wallet_ata));
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
// WithdrawSpot
////////////////////////////////////////////////////////////////////////////////

/// What a [`create_withdraw_spot_ix`] call takes out; amounts are in the
/// asset's base units. Same encoding as the native SOL withdraw.
pub type WithdrawSpotAction = crate::native_sol::WithdrawNativeSolAction;

/// Parameters for [`create_withdraw_spot_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawSpotParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    mint: Pubkey,
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
    action: WithdrawSpotAction,
}

impl WithdrawSpotParams {
    pub fn builder() -> WithdrawSpotParamsBuilder {
        WithdrawSpotParamsBuilder::default()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn mint(&self) -> Pubkey {
        self.mint
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

    pub fn action(&self) -> WithdrawSpotAction {
        self.action
    }
}

#[derive(Debug, Clone, Default)]
pub struct WithdrawSpotParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    destination: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    action: Option<WithdrawSpotAction>,
}

impl WithdrawSpotParamsBuilder {
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

    /// The spot asset's mint, which selects the asset.
    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    /// Where the tokens go. Must be a token account of the asset's mint, and
    /// must not be the custody ATA itself.
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

    pub fn action(mut self, action: WithdrawSpotAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn build(self) -> Result<WithdrawSpotParams, PhoenixIxError> {
        let action = self.action.ok_or(PhoenixIxError::MissingField("action"))?;
        if action.amount() == Some(0) {
            return Err(PhoenixIxError::InvalidWithdrawAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(WithdrawSpotParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            mint: self.mint.ok_or(PhoenixIxError::MissingField("mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            destination: self
                .destination
                .ok_or(PhoenixIxError::MissingField("destination"))?,
            withdraw_queue: self
                .withdraw_queue
                .ok_or(PhoenixIxError::MissingField("withdraw_queue"))?,
            global_trader_index,
            active_trader_buffer,
            action,
        })
    }
}

/// Withdraw SPL spot collateral to a token account of the asset's mint.
///
/// Like a native SOL withdrawal this charges no fee, ignores the deposit
/// cooldown, and is never enqueued: if the exchange-wide throttle cannot
/// absorb the withdrawal's quote-notional value right now, the instruction
/// fails rather than queueing. Excess-only withdrawals bypass the throttle
/// entirely.
pub fn create_withdraw_spot_ix(params: WithdrawSpotParams) -> Result<Instruction, PhoenixIxError> {
    let wallet = get_trader_wallet_address(&params.trader_account)?;
    let wallet_ata = get_associated_token_address(&wallet, &params.mint)?;
    if params.destination() == wallet_ata {
        return Err(PhoenixIxError::InvalidWithdrawDestination);
    }

    let mut data = crate::PhoenixInstruction::WithdrawSpot
        .discriminant()
        .to_vec();
    params.action().encode(&mut data);

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    accounts.push(AccountMeta::writable(params.trader_account()));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));
    accounts.push(AccountMeta::readonly(params.mint()));
    accounts.push(AccountMeta::readonly(wallet));
    accounts.push(AccountMeta::writable(wallet_ata));
    accounts.push(AccountMeta::writable(params.destination()));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    // The withdraw queue precedes the index arenas here, matching the native
    // SOL withdrawal.
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
// TransferSpot
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_transfer_spot_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransferSpotParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    src_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    dst_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    mint: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_option"))]
    permission_account: Option<Pubkey>,
    amount: u64,
}

impl TransferSpotParams {
    pub fn builder() -> TransferSpotParamsBuilder {
        TransferSpotParamsBuilder::default()
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

    pub fn mint(&self) -> Pubkey {
        self.mint
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

    /// Amount in the token's **base units**, not quote units.
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransferSpotParamsBuilder {
    trader: Option<Pubkey>,
    src_trader_account: Option<Pubkey>,
    dst_trader_account: Option<Pubkey>,
    mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    permission_account: Option<Pubkey>,
    amount: Option<u64>,
}

impl TransferSpotParamsBuilder {
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

    /// The spot asset's mint, which selects the asset.
    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
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

    /// Amount in the token's **base units**, not quote units.
    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn build(self) -> Result<TransferSpotParams, PhoenixIxError> {
        let amount = self.amount.ok_or(PhoenixIxError::MissingField("amount"))?;
        if amount == 0 {
            return Err(PhoenixIxError::InvalidTransferAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(TransferSpotParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            src_trader_account: self
                .src_trader_account
                .ok_or(PhoenixIxError::MissingField("src_trader_account"))?,
            dst_trader_account: self
                .dst_trader_account
                .ok_or(PhoenixIxError::MissingField("dst_trader_account"))?,
            mint: self.mint.ok_or(PhoenixIxError::MissingField("mint"))?,
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

/// Move SPL spot collateral between two of a trader's accounts, moving the
/// backing tokens between the custody ATAs.
///
/// The source debit is margin checked. The destination custody ATA must
/// already exist (anyone may create it idempotently). The exchange-wide tally
/// is unchanged, since the collateral never leaves the protocol.
///
/// The trailing permission account enables the position-authority path,
/// exactly as for a quote collateral transfer.
pub fn create_transfer_spot_ix(params: TransferSpotParams) -> Result<Instruction, PhoenixIxError> {
    let src_wallet = get_trader_wallet_address(&params.src_trader_account)?;
    let dst_wallet = get_trader_wallet_address(&params.dst_trader_account)?;
    let src_wallet_ata = get_associated_token_address(&src_wallet, &params.mint)?;
    let dst_wallet_ata = get_associated_token_address(&dst_wallet, &params.mint)?;

    let mut data = crate::PhoenixInstruction::TransferSpot
        .discriminant()
        .to_vec();
    data.extend_from_slice(&params.amount().to_le_bytes());

    let mut accounts = log_accounts();
    // The mint and token-movement accounts precede the regular collateral
    // transfer group, which is reused verbatim for permissioning.
    accounts.push(AccountMeta::readonly(params.mint()));
    accounts.push(AccountMeta::readonly(src_wallet));
    accounts.push(AccountMeta::writable(src_wallet_ata));
    accounts.push(AccountMeta::writable(dst_wallet_ata));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
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
// TransferSpotFromChildToParent
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_transfer_spot_from_child_to_parent_ix`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransferSpotFromChildToParentParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    child_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    parent_trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    mint: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    trader_signs: bool,
}

impl TransferSpotFromChildToParentParams {
    pub fn builder() -> TransferSpotFromChildToParentParamsBuilder {
        TransferSpotFromChildToParentParamsBuilder::default()
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

    pub fn mint(&self) -> Pubkey {
        self.mint
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
pub struct TransferSpotFromChildToParentParamsBuilder {
    trader: Option<Pubkey>,
    child_trader_account: Option<Pubkey>,
    parent_trader_account: Option<Pubkey>,
    mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    trader_signs: bool,
}

impl Default for TransferSpotFromChildToParentParamsBuilder {
    fn default() -> Self {
        Self {
            trader: None,
            child_trader_account: None,
            parent_trader_account: None,
            mint: None,
            perp_asset_map: None,
            global_trader_index: None,
            active_trader_buffer: None,
            trader_signs: true,
        }
    }
}

impl TransferSpotFromChildToParentParamsBuilder {
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

    /// The spot asset's mint, which selects the asset.
    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
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
    /// `disable_collateral_sweep` preference, so a crank that does not hold
    /// the wallet key should set this to `false`.
    pub fn trader_signs(mut self, trader_signs: bool) -> Self {
        self.trader_signs = trader_signs;
        self
    }

    pub fn build(self) -> Result<TransferSpotFromChildToParentParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(TransferSpotFromChildToParentParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            child_trader_account: self
                .child_trader_account
                .ok_or(PhoenixIxError::MissingField("child_trader_account"))?,
            parent_trader_account: self
                .parent_trader_account
                .ok_or(PhoenixIxError::MissingField("parent_trader_account"))?,
            mint: self.mint.ok_or(PhoenixIxError::MissingField("mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            global_trader_index,
            active_trader_buffer,
            trader_signs: self.trader_signs,
        })
    }
}

/// Sweep a flat isolated child account's balance of one spot collateral asset
/// into its parent, moving the backing tokens between the custody ATAs. The
/// parent's custody ATA must already exist. One asset per call.
///
/// This is a silent no-op — not an error — when the child still has splines,
/// open orders, a position, or a negative quote balance, or when it holds none
/// of the asset.
pub fn create_transfer_spot_from_child_to_parent_ix(
    params: TransferSpotFromChildToParentParams,
) -> Result<Instruction, PhoenixIxError> {
    let child_wallet = get_trader_wallet_address(&params.child_trader_account)?;
    let parent_wallet = get_trader_wallet_address(&params.parent_trader_account)?;
    let child_wallet_ata = get_associated_token_address(&child_wallet, &params.mint)?;
    let parent_wallet_ata = get_associated_token_address(&parent_wallet, &params.mint)?;

    let data = crate::PhoenixInstruction::TransferSpotFromChildToParent
        .discriminant()
        .to_vec();

    let mut accounts = log_accounts();
    // The mint and token-movement accounts precede the regular collateral
    // child-to-parent group, which is reused verbatim for permissioning.
    accounts.push(AccountMeta::readonly(params.mint()));
    accounts.push(AccountMeta::readonly(child_wallet));
    accounts.push(AccountMeta::writable(child_wallet_ata));
    accounts.push(AccountMeta::writable(parent_wallet_ata));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
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
        data,
    })
}

////////////////////////////////////////////////////////////////////////////////
// SwapSpotWithUsdc
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_swap_spot_with_usdc_ix`].
#[derive(Debug, Clone)]
pub struct SwapSpotWithUsdcParams {
    signer: Pubkey,
    trader_account: Pubkey,
    quote_mint: Pubkey,
    spot_mint: Pubkey,
    perp_asset_map: Pubkey,
    signer_quote_token_account: Pubkey,
    signer_spot_token_account: Pubkey,
    withdraw_queue: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    direction: SwapDirection,
    amount_in: u64,
    slippage: SwapSlippage,
    venue: PackedVenueInstructions,
}

impl SwapSpotWithUsdcParams {
    pub fn builder() -> SwapSpotWithUsdcParamsBuilder {
        SwapSpotWithUsdcParamsBuilder::default()
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
pub struct SwapSpotWithUsdcParamsBuilder {
    signer: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    quote_mint: Option<Pubkey>,
    spot_mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    signer_quote_token_account: Option<Pubkey>,
    signer_spot_token_account: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    direction: Option<SwapDirection>,
    amount_in: Option<u64>,
    slippage: Option<SwapSlippage>,
    venue: Option<PackedVenueInstructions>,
}

impl SwapSpotWithUsdcParamsBuilder {
    /// The swap signer: either the trader's wallet or its position authority.
    /// Both legs of the swap transit this key's token accounts.
    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    /// The exchange's canonical quote mint.
    pub fn quote_mint(mut self, quote_mint: Pubkey) -> Self {
        self.quote_mint = Some(quote_mint);
        self
    }

    /// The spot asset's mint, which selects the asset.
    pub fn spot_mint(mut self, spot_mint: Pubkey) -> Self {
        self.spot_mint = Some(spot_mint);
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

    pub fn signer_spot_token_account(mut self, signer_spot_token_account: Pubkey) -> Self {
        self.signer_spot_token_account = Some(signer_spot_token_account);
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

    /// `Sell`: spot in, quote collateral out. `Buy`: quote collateral in,
    /// spot out.
    pub fn direction(mut self, direction: SwapDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Amount of the *input* asset: base units of the spot token for a sell,
    /// quote lots for a buy.
    pub fn amount_in(mut self, amount_in: u64) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    /// Minimum acceptable amount of the **output** asset — quote lots for a
    /// [`SwapDirection::Sell`], base units of the spot token for a
    /// [`SwapDirection::Buy`].
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

    pub fn build(self) -> Result<SwapSpotWithUsdcParams, PhoenixIxError> {
        let amount_in = self
            .amount_in
            .ok_or(PhoenixIxError::MissingField("amount_in"))?;
        if amount_in == 0 {
            return Err(PhoenixIxError::InvalidSwapAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(SwapSpotWithUsdcParams {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            quote_mint: self
                .quote_mint
                .ok_or(PhoenixIxError::MissingField("quote_mint"))?,
            spot_mint: self
                .spot_mint
                .ok_or(PhoenixIxError::MissingField("spot_mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            signer_quote_token_account: self
                .signer_quote_token_account
                .ok_or(PhoenixIxError::MissingField("signer_quote_token_account"))?,
            signer_spot_token_account: self
                .signer_spot_token_account
                .ok_or(PhoenixIxError::MissingField("signer_spot_token_account"))?,
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

/// Swap between an SPL spot collateral asset and quote collateral through an
/// external venue.
///
/// The program withdraws the input leg to the signer, runs the caller-supplied
/// venue instructions, deposits the output leg, and then checks that the
/// trader's margin state did not worsen. The exchange-wide withdrawal throttle
/// is charged only the swap's *value loss* (slippage plus venue fees), not the
/// full amount moved.
///
/// [`SwapSpotWithUsdcParamsBuilder::min_amount_out`] is the **only** price
/// protection this instruction has — there is no oracle floor. The signer may
/// be the trader's position authority; the same opt-out gates apply as for
/// `SwapNative` (error **7101 `PositionAuthoritySwapDisabled`**).
pub fn create_swap_spot_with_usdc_ix(
    params: SwapSpotWithUsdcParams,
) -> Result<Instruction, PhoenixIxError> {
    let wallet = get_trader_wallet_address(&params.trader_account)?;
    let spot_wallet_ata = get_associated_token_address(&wallet, &params.spot_mint)?;

    let data = encode_swap_data(
        crate::PhoenixInstruction::SwapSpotWithUsdc,
        params.direction,
        params.amount_in,
        params.slippage,
        &params.venue,
    );

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    accounts.push(AccountMeta::readonly(params.spot_mint));
    accounts.push(AccountMeta::writable(get_global_vault_address(
        &params.quote_mint,
    )?));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(wallet));
    accounts.push(AccountMeta::writable_signer(params.signer));
    accounts.push(AccountMeta::writable(params.signer_quote_token_account));
    accounts.push(AccountMeta::writable(params.signer_spot_token_account));
    accounts.push(AccountMeta::writable(params.trader_account));
    accounts.push(AccountMeta::writable(params.withdraw_queue));
    accounts.push(AccountMeta::writable(spot_wallet_ata));
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

////////////////////////////////////////////////////////////////////////////////
// SwapSpotWithSol
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_swap_spot_with_sol_ix`].
#[derive(Debug, Clone)]
pub struct SwapSpotWithSolParams {
    signer: Pubkey,
    trader_account: Pubkey,
    spot_mint: Pubkey,
    perp_asset_map: Pubkey,
    signer_spot_token_account: Pubkey,
    withdraw_queue: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    direction: SwapDirection,
    amount_in: u64,
    slippage: SwapSlippage,
    venue: PackedVenueInstructions,
}

impl SwapSpotWithSolParams {
    pub fn builder() -> SwapSpotWithSolParamsBuilder {
        SwapSpotWithSolParamsBuilder::default()
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
pub struct SwapSpotWithSolParamsBuilder {
    signer: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    spot_mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    signer_spot_token_account: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    direction: Option<SwapDirection>,
    amount_in: Option<u64>,
    slippage: Option<SwapSlippage>,
    venue: Option<PackedVenueInstructions>,
}

impl SwapSpotWithSolParamsBuilder {
    /// The swap signer: either the trader's wallet or its position authority.
    /// Both legs of the swap transit this key's accounts, including the
    /// lamports leg.
    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    /// The spot asset's mint, which selects the asset.
    pub fn spot_mint(mut self, spot_mint: Pubkey) -> Self {
        self.spot_mint = Some(spot_mint);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn signer_spot_token_account(mut self, signer_spot_token_account: Pubkey) -> Self {
        self.signer_spot_token_account = Some(signer_spot_token_account);
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

    /// `Sell`: spot in, native SOL out. `Buy`: native SOL in, spot out.
    pub fn direction(mut self, direction: SwapDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Amount of the *input* asset: base units of the spot token for a sell,
    /// lamports for a buy.
    pub fn amount_in(mut self, amount_in: u64) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    /// Minimum acceptable amount of the **output** asset — lamports for a
    /// [`SwapDirection::Sell`], base units of the spot token for a
    /// [`SwapDirection::Buy`].
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

    pub fn build(self) -> Result<SwapSpotWithSolParams, PhoenixIxError> {
        let amount_in = self
            .amount_in
            .ok_or(PhoenixIxError::MissingField("amount_in"))?;
        if amount_in == 0 {
            return Err(PhoenixIxError::InvalidSwapAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(SwapSpotWithSolParams {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            spot_mint: self
                .spot_mint
                .ok_or(PhoenixIxError::MissingField("spot_mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            signer_spot_token_account: self
                .signer_spot_token_account
                .ok_or(PhoenixIxError::MissingField("signer_spot_token_account"))?,
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

/// Swap between an SPL spot collateral asset and native SOL collateral
/// through an external venue.
///
/// Same shape and caveats as [`create_swap_spot_with_usdc_ix`], with lamports
/// as the non-spot leg: the signer's own account carries the lamports in both
/// directions.
pub fn create_swap_spot_with_sol_ix(
    params: SwapSpotWithSolParams,
) -> Result<Instruction, PhoenixIxError> {
    let wallet = get_trader_wallet_address(&params.trader_account)?;
    let spot_wallet_ata = get_associated_token_address(&wallet, &params.spot_mint)?;

    let data = encode_swap_data(
        crate::PhoenixInstruction::SwapSpotWithSol,
        params.direction,
        params.amount_in,
        params.slippage,
        &params.venue,
    );

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    accounts.push(AccountMeta::readonly(params.spot_mint));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(get_native_sol_authority_address()?));
    accounts.push(AccountMeta::readonly(wallet));
    accounts.push(AccountMeta::writable_signer(params.signer));
    accounts.push(AccountMeta::writable(params.signer_spot_token_account));
    accounts.push(AccountMeta::writable(params.trader_account));
    accounts.push(AccountMeta::writable(params.withdraw_queue));
    accounts.push(AccountMeta::writable(spot_wallet_ata));
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

////////////////////////////////////////////////////////////////////////////////
// SwapSpotWithSpot
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_swap_spot_with_spot_ix`].
#[derive(Debug, Clone)]
pub struct SwapSpotWithSpotParams {
    signer: Pubkey,
    trader_account: Pubkey,
    src_mint: Pubkey,
    dst_mint: Pubkey,
    perp_asset_map: Pubkey,
    signer_src_token_account: Pubkey,
    signer_dst_token_account: Pubkey,
    withdraw_queue: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    amount_in: u64,
    slippage: SwapSlippage,
    venue: PackedVenueInstructions,
}

impl SwapSpotWithSpotParams {
    pub fn builder() -> SwapSpotWithSpotParamsBuilder {
        SwapSpotWithSpotParamsBuilder::default()
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
pub struct SwapSpotWithSpotParamsBuilder {
    signer: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    src_mint: Option<Pubkey>,
    dst_mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    signer_src_token_account: Option<Pubkey>,
    signer_dst_token_account: Option<Pubkey>,
    withdraw_queue: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    amount_in: Option<u64>,
    slippage: Option<SwapSlippage>,
    venue: Option<PackedVenueInstructions>,
}

impl SwapSpotWithSpotParamsBuilder {
    /// The swap signer: either the trader's wallet or its position authority.
    /// The withdrawn input leg transits this key's source token account.
    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    /// The input asset's mint, which selects the source asset.
    pub fn src_mint(mut self, src_mint: Pubkey) -> Self {
        self.src_mint = Some(src_mint);
        self
    }

    /// The output asset's mint, which selects the destination asset.
    pub fn dst_mint(mut self, dst_mint: Pubkey) -> Self {
        self.dst_mint = Some(dst_mint);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    /// The signer's token account for the source mint; the external venue
    /// spends the withdrawn tokens from here.
    pub fn signer_src_token_account(mut self, signer_src_token_account: Pubkey) -> Self {
        self.signer_src_token_account = Some(signer_src_token_account);
        self
    }

    /// The signer's token account for the destination mint; the external
    /// venue pays the swap output here, and the balance increase is moved into
    /// the destination custody ATA and credited.
    pub fn signer_dst_token_account(mut self, signer_dst_token_account: Pubkey) -> Self {
        self.signer_dst_token_account = Some(signer_dst_token_account);
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

    /// Amount withdrawn from the trader and swapped externally, in the input
    /// token's base units.
    pub fn amount_in(mut self, amount_in: u64) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    /// Minimum acceptable amount of the output asset, in the output token's
    /// base units.
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

    pub fn build(self) -> Result<SwapSpotWithSpotParams, PhoenixIxError> {
        let amount_in = self
            .amount_in
            .ok_or(PhoenixIxError::MissingField("amount_in"))?;
        if amount_in == 0 {
            return Err(PhoenixIxError::InvalidSwapAmount);
        }

        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;

        Ok(SwapSpotWithSpotParams {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            src_mint: self
                .src_mint
                .ok_or(PhoenixIxError::MissingField("src_mint"))?,
            dst_mint: self
                .dst_mint
                .ok_or(PhoenixIxError::MissingField("dst_mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            signer_src_token_account: self
                .signer_src_token_account
                .ok_or(PhoenixIxError::MissingField("signer_src_token_account"))?,
            signer_dst_token_account: self
                .signer_dst_token_account
                .ok_or(PhoenixIxError::MissingField("signer_dst_token_account"))?,
            withdraw_queue: self
                .withdraw_queue
                .ok_or(PhoenixIxError::MissingField("withdraw_queue"))?,
            global_trader_index,
            active_trader_buffer,
            amount_in,
            // Deliberately not defaulted: on-chain a zero disables the check.
            slippage: self
                .slippage
                .ok_or(PhoenixIxError::MissingField("min_amount_out"))?,
            venue: self.venue.ok_or(PhoenixIxError::MissingField("venue"))?,
        })
    }
}

/// Swap one SPL spot collateral asset for another through an external venue.
///
/// The venue must pay the output into `signer_dst_token_account`; its balance
/// increase is moved into the destination custody ATA and credited. Same
/// slippage and position-authority caveats as
/// [`create_swap_spot_with_usdc_ix`].
pub fn create_swap_spot_with_spot_ix(
    params: SwapSpotWithSpotParams,
) -> Result<Instruction, PhoenixIxError> {
    let wallet = get_trader_wallet_address(&params.trader_account)?;
    let src_wallet_ata = get_associated_token_address(&wallet, &params.src_mint)?;
    let dst_wallet_ata = get_associated_token_address(&wallet, &params.dst_mint)?;

    let mut data = crate::PhoenixInstruction::SwapSpotWithSpot
        .discriminant()
        .to_vec();
    data.extend_from_slice(&params.amount_in.to_le_bytes());
    data.extend_from_slice(&params.slippage.as_min_amount_out().to_le_bytes());
    encode_packed_instructions(&mut data, params.venue.instructions());

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    accounts.push(AccountMeta::readonly(params.src_mint));
    accounts.push(AccountMeta::readonly(params.dst_mint));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(wallet));
    accounts.push(AccountMeta::writable_signer(params.signer));
    accounts.push(AccountMeta::writable(params.trader_account));
    accounts.push(AccountMeta::writable(params.withdraw_queue));
    accounts.push(AccountMeta::writable(src_wallet_ata));
    accounts.push(AccountMeta::writable(params.signer_src_token_account));
    accounts.push(AccountMeta::writable(params.signer_dst_token_account));
    accounts.push(AccountMeta::writable(dst_wallet_ata));
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

////////////////////////////////////////////////////////////////////////////////
// LiquidateSpot
////////////////////////////////////////////////////////////////////////////////

/// Parameters for [`create_liquidate_spot_ix`].
#[derive(Debug, Clone)]
pub struct LiquidateSpotParams {
    signer: Pubkey,
    permission_account: Option<Pubkey>,
    liquidatee_account: Pubkey,
    quote_mint: Pubkey,
    spot_mint: Pubkey,
    perp_asset_map: Pubkey,
    signer_quote_token_account: Pubkey,
    signer_spot_token_account: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    max_spot_amount: u64,
    extra_trader_accounts: Vec<Pubkey>,
    venue: PackedVenueInstructions,
}

impl LiquidateSpotParams {
    pub fn builder() -> LiquidateSpotParamsBuilder {
        LiquidateSpotParamsBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LiquidateSpotParamsBuilder {
    signer: Option<Pubkey>,
    permission_account: Option<Pubkey>,
    liquidatee_account: Option<Pubkey>,
    quote_mint: Option<Pubkey>,
    spot_mint: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    signer_quote_token_account: Option<Pubkey>,
    signer_spot_token_account: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    max_spot_amount: Option<u64>,
    extra_trader_accounts: Vec<Pubkey>,
    venue: Option<PackedVenueInstructions>,
}

impl LiquidateSpotParamsBuilder {
    pub fn signer(mut self, signer: Pubkey) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Delegated liquidation permission PDA. Omit for the direct
    /// risk-authority path, where the signer occupies both account slots.
    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn liquidatee_account(mut self, liquidatee_account: Pubkey) -> Self {
        self.liquidatee_account = Some(liquidatee_account);
        self
    }

    /// The exchange's canonical quote mint, for the global vault.
    pub fn quote_mint(mut self, quote_mint: Pubkey) -> Self {
        self.quote_mint = Some(quote_mint);
        self
    }

    /// The seized asset's mint, which selects the asset.
    pub fn spot_mint(mut self, spot_mint: Pubkey) -> Self {
        self.spot_mint = Some(spot_mint);
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

    /// The signer's token account for the seized asset's mint; receives the
    /// seized tokens so the venue instructions can spend them.
    pub fn signer_spot_token_account(mut self, account: Pubkey) -> Self {
        self.signer_spot_token_account = Some(account);
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

    /// Maximum spot collateral seized, in the token's base units.
    pub fn max_spot_amount(mut self, amount: u64) -> Self {
        self.max_spot_amount = Some(amount);
        self
    }

    /// Additional traders checked for the exchange-wide shortfall condition.
    pub fn extra_trader_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.extra_trader_accounts = accounts;
        self
    }

    pub fn venue(mut self, venue: PackedVenueInstructions) -> Self {
        self.venue = Some(venue);
        self
    }

    pub fn build(self) -> Result<LiquidateSpotParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) =
            require_index_accounts(self.global_trader_index, self.active_trader_buffer)?;
        Ok(LiquidateSpotParams {
            signer: self.signer.ok_or(PhoenixIxError::MissingField("signer"))?,
            permission_account: self.permission_account,
            liquidatee_account: self
                .liquidatee_account
                .ok_or(PhoenixIxError::MissingField("liquidatee_account"))?,
            quote_mint: self
                .quote_mint
                .ok_or(PhoenixIxError::MissingField("quote_mint"))?,
            spot_mint: self
                .spot_mint
                .ok_or(PhoenixIxError::MissingField("spot_mint"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            signer_quote_token_account: self
                .signer_quote_token_account
                .ok_or(PhoenixIxError::MissingField("signer_quote_token_account"))?,
            signer_spot_token_account: self
                .signer_spot_token_account
                .ok_or(PhoenixIxError::MissingField("signer_spot_token_account"))?,
            global_trader_index,
            active_trader_buffer,
            max_spot_amount: self
                .max_spot_amount
                .ok_or(PhoenixIxError::MissingField("max_spot_amount"))?,
            extra_trader_accounts: self.extra_trader_accounts,
            venue: self.venue.ok_or(PhoenixIxError::MissingField("venue"))?,
        })
    }
}

/// Seize a liquidatee's SPL spot collateral at the discounted index price.
/// The signer must be the risk authority or hold the supplied delegated
/// liquidation permission.
pub fn create_liquidate_spot_ix(
    params: LiquidateSpotParams,
) -> Result<Instruction, PhoenixIxError> {
    let wallet = get_trader_wallet_address(&params.liquidatee_account)?;
    let wallet_ata = get_associated_token_address(&wallet, &params.spot_mint)?;

    let mut data = crate::PhoenixInstruction::LiquidateSpot
        .discriminant()
        .to_vec();
    data.extend_from_slice(&params.max_spot_amount.to_le_bytes());
    data.extend_from_slice(
        &u64::try_from(params.extra_trader_accounts.len())
            .map_err(|_| PhoenixIxError::InstructionDataTooLarge)?
            .to_le_bytes(),
    );
    encode_packed_instructions(&mut data, params.venue.instructions());

    let mut accounts = log_accounts();
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    accounts.push(AccountMeta::readonly(params.spot_mint));
    accounts.push(AccountMeta::writable(get_global_vault_address(
        &params.quote_mint,
    )?));
    accounts.push(AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(wallet));
    accounts.push(AccountMeta::writable_signer(params.signer));
    accounts.push(AccountMeta::writable(
        params.permission_account.unwrap_or(params.signer),
    ));
    accounts.push(AccountMeta::writable(params.signer_quote_token_account));
    accounts.push(AccountMeta::writable(params.signer_spot_token_account));
    accounts.push(AccountMeta::writable(params.liquidatee_account));
    accounts.push(AccountMeta::writable(wallet_ata));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_sol::pack_venue_instructions;

    fn key(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn index_accounts() -> (Vec<Pubkey>, Vec<Pubkey>) {
        (vec![key(20), key(21)], vec![key(30), key(31)])
    }

    fn wallet_for(trader_account: Pubkey) -> Pubkey {
        get_trader_wallet_address(&trader_account).unwrap()
    }

    fn ata_for(owner: Pubkey, mint: Pubkey) -> Pubkey {
        get_associated_token_address(&owner, &mint).unwrap()
    }

    fn venue() -> PackedVenueInstructions {
        pack_venue_instructions(
            &key(9),
            &[Instruction {
                program_id: key(40),
                accounts: vec![AccountMeta::writable(key(41))],
                data: vec![1, 2, 3],
            }],
        )
        .unwrap()
    }

    fn withdraw_spot_params(
        action: WithdrawSpotAction,
    ) -> Result<WithdrawSpotParams, PhoenixIxError> {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        WithdrawSpotParams::builder()
            .trader(key(1))
            .trader_account(key(2))
            .mint(key(6))
            .perp_asset_map(key(3))
            .destination(key(4))
            .withdraw_queue(key(5))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .action(action)
            .build()
    }

    #[test]
    fn withdraw_spot_rejects_a_zero_amount_but_allows_an_excess_sweep() {
        assert!(matches!(
            withdraw_spot_params(WithdrawSpotAction::WithoutExcess { amount: 0 }),
            Err(PhoenixIxError::InvalidWithdrawAmount)
        ));
        assert!(withdraw_spot_params(WithdrawSpotAction::AllExcess).is_ok());
    }

    #[test]
    fn withdraw_spot_rejects_the_custody_ata_as_a_destination() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let custody_ata = ata_for(wallet_for(key(2)), key(6));
        let params = WithdrawSpotParams::builder()
            .trader(key(1))
            .trader_account(key(2))
            .mint(key(6))
            .perp_asset_map(key(3))
            .destination(custody_ata)
            .withdraw_queue(key(5))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .action(WithdrawSpotAction::AllExcess)
            .build()
            .unwrap();

        assert!(matches!(
            create_withdraw_spot_ix(params),
            Err(PhoenixIxError::InvalidWithdrawDestination)
        ));
    }

    #[test]
    fn transfer_spot_appends_an_optional_permission_account() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let base = TransferSpotParams::builder()
            .trader(key(1))
            .src_trader_account(key(2))
            .dst_trader_account(key(3))
            .mint(key(6))
            .perp_asset_map(key(4))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .amount(11);

        let without = create_transfer_spot_ix(base.clone().build().unwrap()).unwrap();
        let with =
            create_transfer_spot_ix(base.permission_account(key(50)).build().unwrap()).unwrap();

        assert_eq!(with.accounts.len(), without.accounts.len() + 1);
        let trailing = with.accounts.last().unwrap();
        assert_eq!(trailing.pubkey, key(50));
        assert!(trailing.is_writable);
    }

    #[test]
    fn transfer_spot_rejects_a_zero_amount() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let result = TransferSpotParams::builder()
            .trader(key(1))
            .src_trader_account(key(2))
            .dst_trader_account(key(3))
            .mint(key(6))
            .perp_asset_map(key(4))
            .global_trader_index(global_trader_index)
            .active_trader_buffer(active_trader_buffer)
            .amount(0)
            .build();

        assert!(matches!(result, Err(PhoenixIxError::InvalidTransferAmount)));
    }

    #[test]
    fn swap_builders_reject_a_zero_amount_and_require_slippage() {
        let (global_trader_index, active_trader_buffer) = index_accounts();
        let base = || {
            SwapSpotWithUsdcParams::builder()
                .signer(key(9))
                .trader_account(key(2))
                .quote_mint(key(5))
                .spot_mint(key(6))
                .perp_asset_map(key(3))
                .signer_quote_token_account(key(10))
                .signer_spot_token_account(key(11))
                .withdraw_queue(key(12))
                .global_trader_index(global_trader_index.clone())
                .active_trader_buffer(active_trader_buffer.clone())
                .direction(SwapDirection::Sell)
                .venue(venue())
        };

        assert!(matches!(
            base().amount_in(0).min_amount_out(1).build(),
            Err(PhoenixIxError::InvalidSwapAmount)
        ));
        assert!(matches!(
            base().amount_in(1).build(),
            Err(PhoenixIxError::MissingField("min_amount_out"))
        ));
    }
}
