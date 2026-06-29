//! Limit order margin state for margin calculations
//!
//! This module provides the LimitOrderMarginState struct which aggregates
//! limit order information needed for margin calculations.

use crate::quantities::BaseLots;

/// Aggregated state of limit orders for margin calculations
///
/// Tracks the number of ask/bid orders and total non-reduce-only
/// base lots on each side.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct LimitOrderMarginState {
    pub num_ask_orders: u32,
    pub num_bid_orders: u32,
    pub total_non_reduce_only_ask_base_lots: BaseLots,
    pub total_non_reduce_only_bid_base_lots: BaseLots,
}

impl LimitOrderMarginState {
    /// Create a new limit order margin state
    pub fn new(
        num_ask_orders: u32,
        num_bid_orders: u32,
        total_non_reduce_only_ask_base_lots: BaseLots,
        total_non_reduce_only_bid_base_lots: BaseLots,
    ) -> Self {
        Self {
            num_ask_orders,
            num_bid_orders,
            total_non_reduce_only_ask_base_lots,
            total_non_reduce_only_bid_base_lots,
        }
    }

    /// Create an empty limit order margin state
    pub const fn empty() -> Self {
        Self {
            num_ask_orders: 0,
            num_bid_orders: 0,
            total_non_reduce_only_ask_base_lots: BaseLots::ZERO,
            total_non_reduce_only_bid_base_lots: BaseLots::ZERO,
        }
    }
}
