//! TraderPosition type for margin calculations
//!
//! This module provides the TraderPosition struct which represents a trader's
//! position in a perp market, tracking base lots, quote lots, and funding
//! snapshots.

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};

use crate::math::quantities::{
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
}
