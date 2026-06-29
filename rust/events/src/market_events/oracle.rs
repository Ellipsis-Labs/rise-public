use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Oracle events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PricesUpdatedEvent {
    /// The oracle signer who authorized the update.
    /// Only applicable if exchange spot/perp price were updated by an oracle.
    pub oracle_signer: Option<Pubkey>,
    pub asset_symbol: Symbol,
    pub asset_id: u32,
    // TODO: make all of these required
    pub new_best_bid: Option<Ticks>,
    pub new_best_ask: Option<Ticks>,
    pub new_last_trade: Option<Ticks>,
    pub new_exchange_spot_price: Option<Ticks>,
    pub new_exchange_perp_price: Option<Ticks>,
    pub new_mid_spot_diff_ema_ticks: Option<SignedTicks>,
    pub new_mark_price: Ticks,
    pub cumulative_funding_rate: Option<SignedQuoteLotsPerBaseLot>,
    pub settled_contribution: Option<SignedQuoteLotsPerBaseLot>,
    pub interval_accumulator: Option<SignedQuoteLotsPerBaseLotUpcasted>,
    pub asset_sequence_number: u64,
    pub prev_asset_sequence_number_slot: u64,
}
