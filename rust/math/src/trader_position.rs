//! TraderPosition type for margin calculations
//!
//! This module provides the TraderPosition struct which represents a trader's
//! position in a perp market, tracking base lots, quote lots, and funding
//! snapshots.

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};

use crate::quantities::{
    BaseLots, QuoteLotsPerBaseLot, SequenceNumberU8, SignedBaseLots, SignedQuoteLots,
    SignedQuoteLotsI56, SignedQuoteLotsPerBaseLot,
};

/// Represents a trader's position in a perp market
///
/// A position tracks:
/// - Base lot position (positive = long, negative = short)
/// - Virtual quote lot position (average entry cost)
/// - Cumulative funding snapshot (for funding rate calculations)
/// - Position sequence number (for tracking position flips)
/// - Accumulated funding for active position
#[repr(C)]
#[derive(
    Pod, Zeroable, Debug, Default, Copy, Clone, PartialEq, BorshDeserialize, BorshSerialize, Eq,
)]
pub struct TraderPosition {
    pub base_lot_position: SignedBaseLots,
    pub virtual_quote_lot_position: SignedQuoteLots,
    /// The cumulative funding snapshot for the position.
    pub cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
    pub position_sequence_number: SequenceNumberU8,
    pub accumulated_funding_for_active_position: SignedQuoteLotsI56,
}

impl TraderPosition {
    /// Create a new empty position
    pub fn new() -> Self {
        Self {
            base_lot_position: SignedBaseLots::ZERO,
            virtual_quote_lot_position: SignedQuoteLots::ZERO,
            cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot::ZERO,
            position_sequence_number: SequenceNumberU8::default(),
            accumulated_funding_for_active_position: SignedQuoteLotsI56::default(),
        }
    }

    /// Get the effective entry price for this position
    ///
    /// Returns None if there is no position
    pub fn effective_entry_price(&self) -> Option<QuoteLotsPerBaseLot> {
        if self.base_lot_position == SignedBaseLots::ZERO {
            None
        } else {
            self.virtual_quote_lot_position
                .abs_as_unsigned()
                .checked_div_by_base_lots(self.base_lot_position.abs_as_unsigned())
        }
    }

    /// Check if the position is long (positive base lots)
    pub fn is_long(&self) -> bool {
        self.base_lot_position > SignedBaseLots::ZERO
    }

    /// Check if the position is short (negative base lots)
    pub fn is_short(&self) -> bool {
        self.base_lot_position < SignedBaseLots::ZERO
    }

    /// Check if there is no position
    pub fn is_neutral(&self) -> bool {
        self.base_lot_position == SignedBaseLots::ZERO
    }

    /// Get the absolute size of the position in base lots
    pub fn abs_size(&self) -> BaseLots {
        self.base_lot_position.abs_as_unsigned()
    }

    pub fn unrealized_pnl(&self, mark_price_quote_lots_per_base_lot: u64) -> SignedQuoteLots {
        calculate_unrealized_pnl(
            self.base_lot_position,
            self.virtual_quote_lot_position,
            mark_price_quote_lots_per_base_lot,
        )
    }
}

/// Calculate unrealized PnL for a position.
///
/// `mark_price_quote_lots_per_base_lot` is already scaled into quote lots per
/// base lot, i.e. `price_in_ticks * tick_size_in_quote_lots_per_base_lot`.
pub fn calculate_unrealized_pnl(
    base_lots: SignedBaseLots,
    virtual_quote_lots: SignedQuoteLots,
    mark_price_quote_lots_per_base_lot: u64,
) -> SignedQuoteLots {
    if base_lots == SignedBaseLots::ZERO {
        return SignedQuoteLots::ZERO;
    }

    let position_value =
        (base_lots.as_inner() as i128).saturating_mul(mark_price_quote_lots_per_base_lot as i128);
    let total = position_value.saturating_add(virtual_quote_lots.as_inner() as i128);
    SignedQuoteLots::new(total.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrealized_pnl_handles_long_short_and_flat_positions() {
        let long_pnl =
            calculate_unrealized_pnl(SignedBaseLots::new(10), SignedQuoteLots::new(-800), 100);
        assert_eq!(long_pnl, SignedQuoteLots::new(200));

        let short_pnl =
            calculate_unrealized_pnl(SignedBaseLots::new(-10), SignedQuoteLots::new(1_200), 100);
        assert_eq!(short_pnl, SignedQuoteLots::new(200));

        let flat_pnl =
            calculate_unrealized_pnl(SignedBaseLots::ZERO, SignedQuoteLots::new(50), 100);
        assert_eq!(flat_pnl, SignedQuoteLots::ZERO);
    }

    #[test]
    fn trader_position_unrealized_pnl_delegates_to_helper() {
        let position = TraderPosition {
            base_lot_position: SignedBaseLots::new(5),
            virtual_quote_lot_position: SignedQuoteLots::new(-450),
            ..TraderPosition::new()
        };

        assert_eq!(position.unrealized_pnl(100), SignedQuoteLots::new(50));
    }
}
