use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Trader events
////////////////////////////////////////////////////////////////////////////////////////////////

/// Trader was added to global index.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderRegisteredEvent {
    pub trader_sequence_number: u64,
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub max_positions: u32,
    pub trader_preference_bits: u32,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,
}

/// Trader delegated their position authority to another wallet.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderDelegatedEvent {
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub old_position_authority: Pubkey,
    pub new_position_authority: Pubkey,
}

/// Trader fee override multipliers were updated.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderFeesUpdatedEvent {
    /// The trader account whose fees were updated
    pub trader: Pubkey,
    /// The authority that signed the fee update
    pub authority: Pubkey,
    /// Previous maker fee override multiplier
    pub previous_maker_fee_override_multiplier: i8,
    /// New maker fee override multiplier
    pub new_maker_fee_override_multiplier: i8,
    /// Previous taker fee override multiplier
    pub previous_taker_fee_override_multiplier: i8,
    /// New taker fee override multiplier
    pub new_taker_fee_override_multiplier: i8,
    /// Whether the trader was found in GTI (hot) or trader account (cold)
    pub is_hot_trader: bool,
}

/// Trader capabilities were enabled by a delegated authority.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderCapabilitiesEnabledEvent {
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub previous_flags: TraderCapabilityFlags,
    pub new_flags: TraderCapabilityFlags,
    /// Global trader index at enablement time (0 if still cold).
    pub global_trader_index: u32,
}

/// Trader was moved to hot state.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderActivatedEvent {
    pub global_trader_index: u32,
    pub authority: Pubkey,
}

/// Trader was moved to cold state.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderDeactivatedEvent {
    pub prev_global_trader_index: u32,
    pub authority: Pubkey,
}

/// Trader deposited more collateral to their account.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderFundsDepositedEvent {
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub amount: QuoteLots,
    pub trader_flags: TraderCapabilityFlags,
    pub new_collateral_balance: SignedQuoteLots,
    pub trader_sequence_number: u64,
    pub prev_sequence_number_slot: u64,
}

/// Trader withdrew collateral from their account.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderFundsWithdrawnEvent {
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
    pub trader_sequence_number: u64,
    pub trader_prev_sequence_number_slot: u64,
    pub post_withdrawal_budget: u64,
    pub post_queue_size: u64,
    pub total_queued_amount: u64,
    pub withdraw_queue_sequence_number: u64,
    pub withdraw_queue_prev_sequence_number_slot: u64,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderFundsWithdrawnFeePaymentEvent {
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub fee: u64,
    pub trader_sequence_number: u64,
    pub trader_prev_sequence_number_slot: u64,
    pub withdraw_queue_sequence_number: u64,
    pub withdraw_queue_prev_sequence_number_slot: u64,
}
/// Trader cancelled a pending withdrawal request.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderWithdrawCancelledEvent {
    pub trader: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
    pub trader_sequence_number: u64,
    pub trader_prev_sequence_number_slot: u64,
    pub withdraw_queue_sequence_number: u64,
    pub withdraw_queue_prev_sequence_number_slot: u64,
}

/// Collateral was transferred between trader accounts.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderCollateralTransferredEvent {
    pub authority: Pubkey,
    pub amount: QuoteLots,
    pub src_trader: Pubkey,
    pub src_trader_sequence_number: u64,
    pub src_trader_prev_sequence_number_slot: u64,
    pub src_trader_new_collateral_balance: SignedQuoteLots,
    pub dst_trader: Pubkey,
    pub dst_trader_sequence_number: u64,
    pub dst_trader_prev_sequence_number_slot: u64,
    pub dst_trader_new_collateral_balance: SignedQuoteLots,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderPositionCreatedEvent {
    pub trader_sequence_number: Option<u64>,
    pub prev_sequence_number_slot: Option<u64>,
    pub global_trader_index: Option<u32>,
    pub asset_id: u32,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderFundingSettledEvent {
    pub trader: Pubkey,
    pub asset_symbol: Symbol,
    pub asset_id: u32,
    pub funding_payment: SignedQuoteLots,
    pub cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
    pub new_collateral_balance: SignedQuoteLots,
}
