use phoenix_rise_math::{BaseLots, Ticks};

use super::fifo_order_id::FIFOOrderId;
use super::resting_order::FIFORestingOrder;
use super::sokoban::StaticOrderedListMapIterator;
use super::{ORDERBOOK_CAPACITY, OrderbookSideMap, Side};

/// A zero-copy view over one orderbook side.
pub struct OrderbookSide<'a> {
    pub(crate) side: Side,
    pub(crate) tree: OrderbookSideMap<'a>,
}

impl<'a> OrderbookSide<'a> {
    #[inline(always)]
    pub const fn side(&self) -> Side {
        self.side
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Returns the best FIFO order for this side.
    ///
    /// This is the top of the raw orderbook side only. It does not include
    /// spline liquidity or Hawkeye spline uncrossing logic.
    pub fn best(&self) -> Option<OrderbookEntryRef<'_>> {
        self.tree
            .get_by_index(self.tree.get_head_index())
            .map(|(order_id, order)| OrderbookEntryRef {
                side: self.side,
                order_id,
                order,
            })
    }

    /// Iterates over this side from best price/time priority to worst.
    pub fn iter(&self) -> OrderbookSideIter<'_> {
        OrderbookSideIter {
            side: self.side,
            inner: self.tree.iter(),
        }
    }
}

/// Borrowed orderbook entry.
#[derive(Clone, Copy, Debug)]
pub struct OrderbookEntryRef<'a> {
    side: Side,
    order_id: &'a FIFOOrderId,
    order: &'a FIFORestingOrder,
}

impl<'a> OrderbookEntryRef<'a> {
    #[inline(always)]
    pub const fn side(&self) -> Side {
        self.side
    }

    #[inline(always)]
    pub const fn order_id(&self) -> &'a FIFOOrderId {
        self.order_id
    }

    #[inline(always)]
    pub const fn order(&self) -> &'a FIFORestingOrder {
        self.order
    }

    #[inline(always)]
    pub const fn price_in_ticks(&self) -> Ticks {
        self.order_id.price_in_ticks
    }

    #[inline(always)]
    pub const fn order_sequence_number(&self) -> u64 {
        self.order_id.order_sequence_number
    }

    #[inline(always)]
    pub const fn raw_sequence_number(&self) -> u64 {
        self.order_id.raw_sequence_number()
    }

    #[inline(always)]
    pub const fn num_base_lots_remaining(&self) -> BaseLots {
        self.order.num_base_lots_remaining()
    }
}

/// Iterator over one side of the orderbook.
pub struct OrderbookSideIter<'a> {
    side: Side,
    inner: StaticOrderedListMapIterator<'a, FIFOOrderId, FIFORestingOrder, ORDERBOOK_CAPACITY>,
}

impl<'a> Iterator for OrderbookSideIter<'a> {
    type Item = OrderbookEntryRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(order_id, order)| OrderbookEntryRef {
                side: self.side,
                order_id,
                order,
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for OrderbookSideIter<'_> {}
