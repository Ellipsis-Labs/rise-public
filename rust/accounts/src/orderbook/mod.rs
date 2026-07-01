//! Borrowed views for Phoenix orderbook accounts.
//!
//! The orderbook account stores FIFO limit orders only. Prices returned by this
//! module do not include spline liquidity and may not represent the full
//! liquidity available when Phoenix matches orders. Use Hawkeye's BBO view
//! function when you need a market BBO that includes spline uncrossing logic.

mod fifo_order_id;
mod header;
mod resting_order;
mod side;
mod sokoban;
mod types;

pub use fifo_order_id::FIFOOrderId;
pub use header::{CircBuf, OrderFill, OrderbookHeader, TradeEvent};
pub use resting_order::{FIFORestingOrder, OptionalConditionalOrderIndex, OrderFlags};
pub use side::{OrderbookEntryRef, OrderbookSide, OrderbookSideIter};
pub use types::{NodePointer, OptionalNonZeroU32, Side, TraderPositionId};

use self::sokoban::{StaticOrderedListMap, StaticOrderedListMapPod};
use crate::common::{PhoenixAccountDecodeError, borrow_prefix, require_len, verify_discriminant};
use crate::discriminants::PhoenixAccount;

/// Capacity of one side of the FIFO orderbook.
pub const ORDERBOOK_CAPACITY: usize = 8192;
/// Capacity of the recent-trade circular buffer.
pub const TRADE_LIST_CAPACITY: usize = 64;
/// Size of the fixed orderbook header prefix.
pub const ORDERBOOK_HEADER_LEN: usize = core::mem::size_of::<OrderbookHeader>();
/// Size of one side's static sokoban list map.
pub const ORDERBOOK_SIDE_LEN: usize = core::mem::size_of::<OrderbookSidePod>();
/// Minimum byte length of a complete orderbook account.
pub const ORDERBOOK_ACCOUNT_LEN: usize = ORDERBOOK_HEADER_LEN + 2 * ORDERBOOK_SIDE_LEN;

pub(crate) const ORDERBOOK_ACCOUNT: &str = "Orderbook";

pub(crate) type OrderbookSidePod =
    StaticOrderedListMapPod<FIFOOrderId, FIFORestingOrder, ORDERBOOK_CAPACITY>;
pub(crate) type OrderbookSideMap<'a> =
    StaticOrderedListMap<'a, FIFOOrderId, FIFORestingOrder, ORDERBOOK_CAPACITY>;

/// Full borrowed orderbook view.
///
/// The best bid/ask and side iterators are raw FIFO orderbook views. They do
/// not include spline liquidity and may not represent the full liquidity used
/// when Phoenix matches orders. Use Hawkeye's BBO view function when you need
/// BBO semantics that include spline uncrossing logic.
pub struct Orderbook<'a> {
    pub header: &'a OrderbookHeader,
    pub bids_tree: OrderbookSide<'a>,
    pub asks_tree: OrderbookSide<'a>,
}

impl<'a> Orderbook<'a> {
    /// Borrows a zero-copy orderbook view from raw account bytes.
    ///
    /// The input must be aligned for the orderbook layout. Short, misaligned,
    /// or internally inconsistent buffers return [`PhoenixAccountDecodeError`]
    /// instead of panicking.
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(ORDERBOOK_ACCOUNT, data, ORDERBOOK_ACCOUNT_LEN)?;
        verify_discriminant(
            ORDERBOOK_ACCOUNT,
            data,
            PhoenixAccount::Orderbook.discriminant(),
        )?;

        let header = borrow_prefix::<OrderbookHeader>(ORDERBOOK_ACCOUNT, data)?;
        header.asset_symbol()?;
        if header.trade_list().len() > TRADE_LIST_CAPACITY {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: ORDERBOOK_ACCOUNT,
                reason: "trade list length exceeds capacity",
            });
        }

        let book_bytes = data
            .get(ORDERBOOK_HEADER_LEN..ORDERBOOK_ACCOUNT_LEN)
            .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
                account: ORDERBOOK_ACCOUNT,
                expected: ORDERBOOK_ACCOUNT_LEN,
                actual: data.len(),
            })?;
        let (bids_tree_bytes, asks_tree_bytes) = book_bytes.split_at(ORDERBOOK_SIDE_LEN);
        let bids_tree = OrderbookSide {
            side: Side::Bid,
            tree: OrderbookSideMap::try_load_from_buffer(ORDERBOOK_ACCOUNT, bids_tree_bytes)?,
        };
        let asks_tree = OrderbookSide {
            side: Side::Ask,
            tree: OrderbookSideMap::try_load_from_buffer(ORDERBOOK_ACCOUNT, asks_tree_bytes)?,
        };

        Ok(Self {
            header,
            bids_tree,
            asks_tree,
        })
    }

    /// Alias for [`Self::try_from_account_bytes`] emphasizing that the returned
    /// view borrows account data.
    pub fn try_borrow_from_account_bytes(
        data: &'a [u8],
    ) -> Result<Self, PhoenixAccountDecodeError> {
        Self::try_from_account_bytes(data)
    }

    #[inline(always)]
    pub const fn header(&self) -> &'a OrderbookHeader {
        self.header
    }

    #[inline(always)]
    pub const fn bids(&self) -> &OrderbookSide<'a> {
        &self.bids_tree
    }

    #[inline(always)]
    pub const fn asks(&self) -> &OrderbookSide<'a> {
        &self.asks_tree
    }

    #[inline(always)]
    pub fn has_no_orders(&self) -> bool {
        self.bids_tree.is_empty() && self.asks_tree.is_empty()
    }

    #[inline(always)]
    pub fn num_bids(&self) -> usize {
        self.bids_tree.len()
    }

    #[inline(always)]
    pub fn num_asks(&self) -> usize {
        self.asks_tree.len()
    }

    /// Returns the raw best bid from the FIFO orderbook.
    ///
    /// This does not include spline liquidity and may differ from Hawkeye BBO
    /// because matching can include spline uncrossing logic.
    #[inline(always)]
    pub fn best_bid(&self) -> Option<OrderbookEntryRef<'_>> {
        self.bids_tree.best()
    }

    /// Returns the raw best ask from the FIFO orderbook.
    ///
    /// This does not include spline liquidity and may differ from Hawkeye BBO
    /// because matching can include spline uncrossing logic.
    #[inline(always)]
    pub fn best_ask(&self) -> Option<OrderbookEntryRef<'_>> {
        self.asks_tree.best()
    }

    /// Iterates bids from best price/time priority to worst.
    #[inline(always)]
    pub fn bid_orders(&self) -> OrderbookSideIter<'_> {
        self.bids_tree.iter()
    }

    /// Iterates asks from best price/time priority to worst.
    #[inline(always)]
    pub fn ask_orders(&self) -> OrderbookSideIter<'_> {
        self.asks_tree.iter()
    }
}

#[cfg(test)]
mod tests {
    use phoenix_rise_math::Ticks;

    use super::*;

    #[test]
    fn orderbook_layout_sizes_match_program_core() {
        assert_eq!(ORDERBOOK_HEADER_LEN, 5504);
        assert_eq!(core::mem::size_of::<FIFORestingOrder>(), 48);
        assert_eq!(core::mem::size_of::<OrderbookSidePod>(), 589_856);
        assert_eq!(ORDERBOOK_ACCOUNT_LEN, 1_185_216);
    }

    #[test]
    fn rejects_short_buffers() {
        let data = [0u8; 7];
        assert!(matches!(
            Orderbook::try_from_account_bytes(&data),
            Err(PhoenixAccountDecodeError::AccountTooSmall { .. })
        ));
    }

    #[test]
    fn fifo_order_id_orders_best_prices_first() {
        let ask_100_seq_1 = FIFOOrderId::new(Ticks::new(100), 1);
        let ask_100_seq_2 = FIFOOrderId::new(Ticks::new(100), 2);
        let ask_101_seq_1 = FIFOOrderId::new(Ticks::new(101), 1);
        assert!(ask_100_seq_1 < ask_100_seq_2);
        assert!(ask_100_seq_1 < ask_101_seq_1);

        let bid_100_seq_1 = FIFOOrderId::new(Ticks::new(100), !1u64);
        let bid_100_seq_2 = FIFOOrderId::new(Ticks::new(100), !2u64);
        let bid_101_seq_1 = FIFOOrderId::new(Ticks::new(101), !1u64);
        assert!(bid_100_seq_1 < bid_100_seq_2);
        assert!(bid_101_seq_1 < bid_100_seq_1);
    }
}
