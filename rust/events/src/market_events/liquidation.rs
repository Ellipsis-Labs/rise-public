use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Liquidation events
////////////////////////////////////////////////////////////////////////////////////////////////

/// A position was liquidated via market order
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LiquidationEvent {
    pub liquidator: Pubkey,
    pub liquidated_trader: Pubkey,
    pub asset_id: u32,
    pub liquidation_size: BaseLots,
    pub mark_price: Ticks,
    pub base_lots_filled: BaseLots,
    pub quote_lots_filled: QuoteLots,
    pub position_closed: bool,
}

////////////////////////////////////////////////////////////////////////////////////////////////
// Liquidation transfer events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LiquidationTransferSummaryEvent {
    pub liquidatee: Pubkey,
    pub liquidator: Pubkey,
    pub total_transfers: u32,
    pub liquidatee_collateral_change: SignedQuoteLots,
    pub liquidator_collateral_change: SignedQuoteLots,
    pub haircut_collected: QuoteLots,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LiquidationTransferEvent {
    pub liquidatee: Pubkey,
    pub liquidator: Pubkey,
    pub asset_id: u64,
    pub base_lots_transferred: SignedBaseLots,
    pub virtual_quote_lots_transferred: SignedQuoteLots,
    pub haircut_rate: u16,
    pub liquidatee_collateral_change: SignedQuoteLots,
    pub liquidator_collateral_change: SignedQuoteLots,
}

////////////////////////////////////////////////////////////////////////////////////////////////
// Close Match Position events (ADL)
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseMatchedPositionsEvent {
    pub caller: Pubkey,
    pub closed_short: Pubkey,
    pub closed_long: Pubkey,
    pub in_profit_account: Pubkey,
    pub asset_id: u64,
    pub base_lots_closed: SignedBaseLots,
    pub at_loss_close_value: SignedQuoteLots,
    pub in_profit_close_value: SignedQuoteLots,
    pub at_loss_collateral_change: SignedQuoteLots,
    pub in_profit_collateral_change: SignedQuoteLots,
}
