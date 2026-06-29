use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// PnL events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PnLEvent {
    pub trader: Pubkey,
    pub asset_id: u32,
    pub asset_symbol: Symbol,
    pub realized_pnl: SignedQuoteLots,
    pub funding_payment: SignedQuoteLots,
    pub base_lots_before: SignedBaseLots,
    pub base_lots_after: SignedBaseLots,
    pub virtual_quote_lots_before: SignedQuoteLots,
    pub virtual_quote_lots_after: SignedQuoteLots,
}

impl PnLEvent {
    pub fn is_noop(&self) -> bool {
        self.base_lots_before == self.base_lots_after
            && self.virtual_quote_lots_before == self.virtual_quote_lots_after
            && self.realized_pnl == SignedQuoteLots::ZERO
            && self.funding_payment == SignedQuoteLots::ZERO
    }
}
