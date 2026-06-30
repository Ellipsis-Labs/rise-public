use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::BaseLots;

use super::types::{NodePointer, OptionalNonZeroU32, TraderPositionId};

/// Flags for a resting order.
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderFlags {
    flags: u8,
}

impl OrderFlags {
    const IS_CONDITIONAL_ORDER_BIT: u8 = 1 << 5;
    const IS_STOP_LOSS_BIT: u8 = 1 << 6;
    const IS_STOP_LOSS_DIRECTION_BIT: u8 = 1 << 4;
    const REDUCE_ONLY_BIT: u8 = 1 << 7;

    #[inline(always)]
    pub const fn from_bits(flags: u8) -> Self {
        Self { flags }
    }

    #[inline(always)]
    pub const fn new() -> Self {
        Self { flags: 0 }
    }

    #[inline(always)]
    pub const fn as_u8(&self) -> u8 {
        self.flags
    }

    #[inline(always)]
    pub const fn is_reduce_only(&self) -> bool {
        self.flags & Self::REDUCE_ONLY_BIT != 0
    }

    #[inline(always)]
    pub const fn is_stop_loss(&self) -> bool {
        self.flags & Self::IS_STOP_LOSS_BIT != 0
    }

    #[inline(always)]
    pub const fn is_conditional_order(&self) -> bool {
        self.flags & Self::IS_CONDITIONAL_ORDER_BIT != 0
    }

    #[inline(always)]
    pub const fn is_stop_loss_direction(&self) -> bool {
        self.flags & Self::IS_STOP_LOSS_DIRECTION_BIT != 0
    }
}

impl core::fmt::Debug for OrderFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OrderFlags")
            .field("raw", &self.as_u8())
            .field("reduce_only", &self.is_reduce_only())
            .field("is_stop_loss", &self.is_stop_loss())
            .field("is_conditional_order", &self.is_conditional_order())
            .field("is_stop_loss_direction", &self.is_stop_loss_direction())
            .finish()
    }
}

/// Optional conditional-order index stored on conditional resting orders.
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionalConditionalOrderIndex {
    value: u8,
}

impl OptionalConditionalOrderIndex {
    pub const NONE: Self = Self { value: 0 };

    #[inline(always)]
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    #[inline(always)]
    pub const fn is_none(&self) -> bool {
        self.value == 0
    }

    #[inline(always)]
    pub const fn is_some(&self) -> bool {
        self.value != 0
    }

    #[inline(always)]
    pub const fn as_u8_checked(&self) -> Option<u8> {
        if self.value == 0 {
            None
        } else {
            Some(self.value)
        }
    }

    #[inline(always)]
    pub const fn raw(&self) -> u8 {
        self.value
    }
}

impl core::fmt::Debug for OptionalConditionalOrderIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.as_u8_checked() {
            Some(value) => write!(f, "Some({value})"),
            None => write!(f, "None"),
        }
    }
}

/// A resting order in the FIFO orderbook.
#[repr(C)]
#[derive(Default, Copy, Clone, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FIFORestingOrder {
    trader_position_id: TraderPositionId,
    initial_trade_size: BaseLots,
    num_base_lots_remaining: BaseLots,
    order_flags: OrderFlags,
    optional_conditional_order_index: OptionalConditionalOrderIndex,
    _padding: [u8; 2],
    expiration_offset: OptionalNonZeroU32,
    initial_slot: u64,
    prev: NodePointer,
    next: NodePointer,
}

impl FIFORestingOrder {
    #[inline(always)]
    pub const fn trader_position_id(&self) -> TraderPositionId {
        self.trader_position_id
    }

    #[inline(always)]
    pub const fn initial_trade_size(&self) -> BaseLots {
        self.initial_trade_size
    }

    #[inline(always)]
    pub const fn num_base_lots_remaining(&self) -> BaseLots {
        self.num_base_lots_remaining
    }

    #[inline(always)]
    pub const fn order_flags(&self) -> OrderFlags {
        self.order_flags
    }

    #[inline(always)]
    pub const fn optional_conditional_order_index(&self) -> OptionalConditionalOrderIndex {
        self.optional_conditional_order_index
    }

    #[inline(always)]
    pub const fn is_reduce_only(&self) -> bool {
        self.order_flags.is_reduce_only()
    }

    #[inline(always)]
    pub const fn is_stop_loss(&self) -> bool {
        self.order_flags.is_stop_loss()
    }

    #[inline(always)]
    pub const fn is_conditional_order(&self) -> bool {
        self.order_flags.is_conditional_order()
    }

    #[inline(always)]
    pub const fn is_stop_loss_direction(&self) -> bool {
        self.order_flags.is_stop_loss_direction()
    }

    #[inline(always)]
    pub const fn initial_slot(&self) -> u64 {
        self.initial_slot
    }

    #[inline(always)]
    pub fn last_valid_slot(&self) -> Option<u64> {
        self.expiration_offset
            .map(|offset| self.initial_slot.saturating_add(offset.get() as u64))
    }

    #[inline(always)]
    pub fn is_expired(&self, current_slot: u64) -> bool {
        self.last_valid_slot()
            .map(|last_valid| current_slot > last_valid)
            .unwrap_or(false)
    }

    #[inline(always)]
    pub const fn prev(&self) -> NodePointer {
        self.prev
    }

    #[inline(always)]
    pub const fn next(&self) -> NodePointer {
        self.next
    }
}

impl core::fmt::Debug for FIFORestingOrder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FIFORestingOrder")
            .field("trader_position_id", &self.trader_position_id())
            .field("initial_trade_size", &self.initial_trade_size())
            .field("num_base_lots_remaining", &self.num_base_lots_remaining())
            .field("order_flags", &self.order_flags())
            .field(
                "optional_conditional_order_index",
                &self.optional_conditional_order_index(),
            )
            .field("initial_slot", &self.initial_slot())
            .field("last_valid_slot", &self.last_valid_slot())
            .finish()
    }
}

const_assert_eq!(core::mem::size_of::<OrderFlags>(), 1);
const_assert_eq!(core::mem::size_of::<OptionalConditionalOrderIndex>(), 1);
const_assert_eq!(core::mem::size_of::<FIFORestingOrder>(), 48);
