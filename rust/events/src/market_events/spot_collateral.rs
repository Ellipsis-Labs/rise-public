use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

/// The configured cap that bound a spot-collateral credit. Carried as
/// `Option<SpotCollateralCap>` (`None` when the credit applied in full). Keep
/// variant order in sync with the on-chain enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpotCollateralCap {
    /// The per-trader balance cap.
    PerTrader,
    /// The exchange-wide balance cap.
    Global,
}

/// Which flow produced a spot-collateral event, so consumers need not infer it
/// from the surrounding events. Keep variant order in sync with the on-chain
/// enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpotCollateralFlow {
    /// Upward or downward `SyncNative` reconciliation.
    Sync,
    /// The spot leg of a `SwapNative` (buy credit or sell debit).
    Swap,
    /// The credit or debit leg of a trader-to-trader transfer.
    Transfer,
    /// A `WithdrawNativeSol`, including the privileged self-CPI legs of a swap
    /// sell and a liquidation seizure. Tell those apart at read time via
    /// `is_self_cpi` and any co-emitted [`SpotCollateralLiquidatedEvent`].
    Withdraw,
}

/// Per-asset spot collateral configuration, carried by
/// `AdminParameterUpdateKind::SpotCollateralConfig`.
///
/// This is the Borsh wire mirror of the on-chain account struct, padding
/// included: it must serialize to exactly 320 bytes or every enclosing event
/// decodes misaligned. Balances are native units, discounts and slippage are
/// basis points, and the two buffers are quote lots.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpotCollateralMetadata {
    /// Mint of the collateral asset; all-zero for native SOL.
    pub mint_address: Pubkey,
    pub decimals: u32,
    /// Perp asset whose index price values this collateral. Meaningful only
    /// when the `has_perp_asset` flag is set.
    pub perp_asset_index: u32,
    pub max_per_trader_balance: u64,
    pub max_global_balance: u64,
    pub curr_global_balance: u64,
    pub min_margin_discount: u32,
    pub max_margin_discount: u32,
    pub max_liquidation_discount: u32,
    pub min_liquidation_slippage: u32,
    pub max_liquidation_size: u64,
    pub post_liquidation_buffer: QuoteLots,
    pub quote_lot_collateral_shortfall_buffer: QuoteLots,
    /// Bit 0 `is_active`, bit 1 `has_perp_asset`, bit 2
    /// `disable_position_authority_swap`.
    pub flags: u8,
    pub _padding_flags: [u8; 7],
    pub padding: [u64; 26],
}

impl SpotCollateralMetadata {
    /// Whether the asset is engaged in margin. The exchange feature bit must
    /// also be set for the asset to actually count.
    pub const fn is_active(&self) -> bool {
        self.flags & (1 << 0) != 0
    }

    pub const fn has_perp_asset(&self) -> bool {
        self.flags & (1 << 1) != 0
    }

    pub const fn position_authority_swap_disabled(&self) -> bool {
        self.flags & (1 << 2) != 0
    }
}

/// Spot collateral credited to a trader: an upward `SyncNative`
/// reconciliation, the SOL leg of a `SwapNative` buy, or the credit leg of a
/// transfer.
///
/// All spot collateral amounts are in the asset's native units (lamports for
/// native SOL). `asset_index` is the raw asset-index key of the asset's entry
/// in the trader position map, NOT the perp market asset id used for pricing.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpotCollateralDepositedEvent {
    pub asset_index: u32,
    pub trader: Pubkey,
    /// The signer for swap and transfer credits; the trader's wallet
    /// authority for permissionless sync credits (which have no signer).
    pub authority: Pubkey,
    /// Amount credited, in native units. For a sync this is the credited
    /// delta after any protocol-cap clamp.
    pub amount: u64,
    /// The delta the trader tried to credit before any cap clamp (lamports
    /// physically moved in). `requested_amount - amount` is the excess that
    /// stayed uncounted in the trader account.
    pub requested_amount: u64,
    /// Which cap bound the credit, or `None` when it applied in full.
    pub binding_cap: Option<SpotCollateralCap>,
    /// The flow that produced this credit.
    pub flow: SpotCollateralFlow,
    pub new_collateral_balance: u64,
    pub post_global_collateral: u64,
    pub trader_sequence_number: u64,
    pub prev_sequence_number_slot: u64,
}

/// Spot collateral debited from a trader: `WithdrawNativeSol`, the SOL leg of
/// a `SwapNative` sell, a downward `SyncNative` reconciliation, the debit leg
/// of a transfer, or a liquidation seizure (where `authority` is the native
/// SOL authority PDA and `destination` is the liquidator).
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpotCollateralWithdrawnEvent {
    pub asset_index: u32,
    pub trader: Pubkey,
    pub authority: Pubkey,
    /// Where the debited funds went: the withdrawal destination, the signer
    /// for swap sells, the liquidator for seizures, the destination trader
    /// account for transfers, or the trader account itself for downward syncs
    /// (the lamports never left; they were absorbed by rent).
    pub destination: Pubkey,
    /// Accounted collateral debited, in native units.
    pub amount: u64,
    /// Uncounted lamports additionally swept out beyond `amount`, nonzero only
    /// when `withdraw_excess` was requested and the account held uncounted
    /// lamports. Physical lamports moved equal `amount + excess`.
    pub excess: u64,
    /// Whether the withdrawal requested an excess sweep; `excess` may still be
    /// 0. Always false for swap, sync, transfer, and liquidation flows.
    pub withdraw_excess: bool,
    /// Whether the debit was a program-initiated self-CPI (swap sell or
    /// liquidation seizure) rather than a direct user withdrawal.
    pub is_self_cpi: bool,
    /// The flow that produced this debit.
    pub flow: SpotCollateralFlow,
    pub new_collateral_balance: u64,
    pub post_global_collateral: u64,
    pub trader_sequence_number: u64,
    pub prev_sequence_number_slot: u64,
}

/// Spot collateral seized in a liquidation (`LiquidateNativeSol`). Summary
/// event linking the seizure ([`SpotCollateralWithdrawnEvent`]) and the quote
/// credit (`TraderFundsDeposited`); carries no balance change itself and no
/// sequence number.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpotCollateralLiquidatedEvent {
    /// Liquidator wallet (the signer), mirroring `LiquidationEvent`.
    pub liquidator: Pubkey,
    /// Liquidated trader account, mirroring `LiquidationEvent`.
    pub liquidated_trader: Pubkey,
    pub asset_index: u32,
    /// Spot collateral seized, in the asset's native units.
    pub liquidation_size: u64,
    /// Portion of the liquidatee's collateral that sat above the per-trader
    /// cap and was therefore additionally seizable, in native units.
    pub over_cap_excess: u64,
    /// Quote collateral deposited by the liquidator and credited to the
    /// liquidated trader.
    pub quote_lots_deposited: QuoteLots,
    /// Oracle value of the seized collateral.
    pub oracle_notional: QuoteLots,
    /// Discount applied to the oracle value for the minimum deposit.
    pub liquidation_discount: BasisPoints,
}
