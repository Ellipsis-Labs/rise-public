//! Limit order margin state for margin calculations
//!
//! This module provides the LimitOrderMarginState struct which aggregates
//! limit order information needed for margin calculations.

use crate::direction::Side;
use crate::quantities::{BaseLots, SignedBaseLots, Ticks};

/// A resting order's margin-relevant fields, the entry type for
/// [`LimitOrderMarginState::from_orders`].
#[derive(Copy, Clone, Debug)]
pub struct LimitOrderSummary {
    pub side: Side,
    pub price_in_ticks: Ticks,
    pub base_lots_remaining: BaseLots,
    pub reduce_only: bool,
}

/// Aggregated state of limit orders for margin calculations
///
/// Tracks the number of ask/bid orders, the resting price extremes
/// (`lowest_ask` / `highest_bid`), and the total reduce-only and
/// non-reduce-only base lots on each side. The prices and reduce-only totals
/// feed the adverse-fill-loss reserve computed in [`crate::margin_calc`].
///
/// Construction is deliberately restricted to the canonical
/// [`LimitOrderMarginState::from_orders`] aggregation (and
/// [`LimitOrderMarginState::empty`]), which enforces the same invariants as
/// the on-chain program: zero-sentinel best prices and reduce-only totals
/// capped at the reducible position.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct LimitOrderMarginState {
    pub num_ask_orders: u32,
    pub num_bid_orders: u32,
    pub lowest_ask: Ticks,
    pub highest_bid: Ticks,
    pub total_non_reduce_only_ask_base_lots: BaseLots,
    pub total_reduce_only_ask_base_lots: BaseLots,
    pub total_non_reduce_only_bid_base_lots: BaseLots,
    pub total_reduce_only_bid_base_lots: BaseLots,
    /// Private marker so the struct cannot be built with a literal outside
    /// this module; use one of the canonical constructors.
    _canonical: (),
}

impl LimitOrderMarginState {
    /// Aggregate a trader's resting orders, applying the same invariants as
    /// the on-chain construction from the tracked trader position state.
    pub fn from_orders(
        orders: impl IntoIterator<Item = LimitOrderSummary>,
        base_lot_position: SignedBaseLots,
    ) -> Self {
        let mut state = Self::empty();

        for LimitOrderSummary {
            side,
            price_in_ticks,
            base_lots_remaining,
            reduce_only,
        } in orders
        {
            match side {
                Side::Ask => {
                    state.num_ask_orders += 1;
                    if state.lowest_ask.is_zero() || price_in_ticks < state.lowest_ask {
                        state.lowest_ask = price_in_ticks;
                    }
                    if reduce_only {
                        state.total_reduce_only_ask_base_lots += base_lots_remaining;
                    } else {
                        state.total_non_reduce_only_ask_base_lots += base_lots_remaining;
                    }
                }
                Side::Bid => {
                    state.num_bid_orders += 1;
                    if price_in_ticks > state.highest_bid {
                        state.highest_bid = price_in_ticks;
                    }
                    if reduce_only {
                        state.total_reduce_only_bid_base_lots += base_lots_remaining;
                    } else {
                        state.total_non_reduce_only_bid_base_lots += base_lots_remaining;
                    }
                }
            }
        }

        state.cap_reduce_only_lots_to_position(base_lot_position)
    }

    /// Create an empty limit order margin state
    pub const fn empty() -> Self {
        Self {
            num_ask_orders: 0,
            num_bid_orders: 0,
            lowest_ask: Ticks::ZERO,
            highest_bid: Ticks::ZERO,
            total_non_reduce_only_ask_base_lots: BaseLots::ZERO,
            total_reduce_only_ask_base_lots: BaseLots::ZERO,
            total_non_reduce_only_bid_base_lots: BaseLots::ZERO,
            total_reduce_only_bid_base_lots: BaseLots::ZERO,
            _canonical: (),
        }
    }

    /// Total ask base lots resting on the book (reduce-only + non-reduce-only).
    pub fn total_ask_base_lots(&self) -> BaseLots {
        self.total_reduce_only_ask_base_lots + self.total_non_reduce_only_ask_base_lots
    }

    /// Total bid base lots resting on the book (reduce-only + non-reduce-only).
    pub fn total_bid_base_lots(&self) -> BaseLots {
        self.total_reduce_only_bid_base_lots + self.total_non_reduce_only_bid_base_lots
    }

    /// Lowest resting ask price, or `None` when there are no ask orders.
    pub fn lowest_ask(&self) -> Option<Ticks> {
        if self.lowest_ask.is_zero() {
            None
        } else {
            Some(self.lowest_ask)
        }
    }

    /// Highest resting bid price, or `None` when there are no bid orders.
    pub fn highest_bid(&self) -> Option<Ticks> {
        if self.highest_bid.is_zero() {
            None
        } else {
            Some(self.highest_bid)
        }
    }

    /// Cap the reduce-only totals at the lots that can actually reduce the
    /// given position, mirroring the on-chain construction: a reduce-only ask
    /// can only reduce a long position, and a reduce-only bid can only reduce
    /// a short position; anything beyond the position can never fill (it
    /// would be an increase), so it reserves no margin.
    fn cap_reduce_only_lots_to_position(mut self, base_lot_position: SignedBaseLots) -> Self {
        let reducible_by_asks = if base_lot_position.is_non_negative() {
            base_lot_position.abs_as_unsigned()
        } else {
            BaseLots::ZERO
        };
        let reducible_by_bids = if base_lot_position.is_non_negative() {
            BaseLots::ZERO
        } else {
            base_lot_position.abs_as_unsigned()
        };
        self.total_reduce_only_ask_base_lots =
            self.total_reduce_only_ask_base_lots.min(reducible_by_asks);
        self.total_reduce_only_bid_base_lots =
            self.total_reduce_only_bid_base_lots.min(reducible_by_bids);
        self
    }
}
