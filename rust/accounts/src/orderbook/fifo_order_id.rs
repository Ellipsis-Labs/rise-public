use core::cmp::Ordering;

use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::Ticks;

use super::types::Side;

/// FIFO order identifier.
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FIFOOrderId {
    pub price_in_ticks: Ticks,
    pub order_sequence_number: u64,
}

impl FIFOOrderId {
    #[inline(always)]
    pub fn new(price_in_ticks: impl Into<Ticks>, order_sequence_number: u64) -> Self {
        Self {
            price_in_ticks: price_in_ticks.into(),
            order_sequence_number,
        }
    }

    #[inline(always)]
    pub const fn side(&self) -> Side {
        Side::from_order_sequence_number(self.order_sequence_number)
    }

    #[inline(always)]
    pub const fn raw_sequence_number(&self) -> u64 {
        match self.side() {
            Side::Bid => !self.order_sequence_number,
            Side::Ask => self.order_sequence_number,
        }
    }
}

impl core::fmt::Debug for FIFOOrderId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FIFOOrderId")
            .field("price_in_ticks", &self.price_in_ticks)
            .field("order_sequence_number", &self.order_sequence_number)
            .field("raw_sequence_number", &self.raw_sequence_number())
            .field("side", &self.side())
            .finish()
    }
}

impl PartialOrd for FIFOOrderId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FIFOOrderId {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_side = self.side();
        let other_side = other.side();
        if self_side != other_side {
            return match self_side {
                Side::Ask => Ordering::Less,
                Side::Bid => Ordering::Greater,
            };
        }

        let (tick_cmp, seq_cmp) = match self_side {
            Side::Bid => (
                other.price_in_ticks.cmp(&self.price_in_ticks),
                other.order_sequence_number.cmp(&self.order_sequence_number),
            ),
            Side::Ask => (
                self.price_in_ticks.cmp(&other.price_in_ticks),
                self.order_sequence_number.cmp(&other.order_sequence_number),
            ),
        };
        if tick_cmp == Ordering::Equal {
            seq_cmp
        } else {
            tick_cmp
        }
    }
}

const_assert_eq!(core::mem::size_of::<FIFOOrderId>(), 16);
