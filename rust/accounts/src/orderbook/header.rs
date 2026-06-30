use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::{
    QuoteLots, QuoteLotsPerBaseLotPerTick, SignedBaseLots, SignedQuoteLots, Ticks,
};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::{ORDERBOOK_ACCOUNT, TRADE_LIST_CAPACITY};
use crate::common::{PhoenixAccountDecodeError, SequenceNumber};
use crate::perp_asset_map::AssetSymbol;

/// A trade fill stored in the recent-trades ring buffer.
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderFill {
    pub price: Ticks,
    pub quantity: SignedBaseLots,
}

/// A recent trade event stored in the orderbook header.
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeEvent {
    pub slot: u64,
    pub timestamp: i64,
    pub trade_sequence_number: u64,
    pub prev_trade_sequence_number_slot: u64,
    pub maker_fill: OrderFill,
    pub maker: [u8; 32],
}

/// Circular buffer for recent trade events.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CircBuf {
    ptr: u32,
    len: u32,
    buf: [TradeEvent; TRADE_LIST_CAPACITY],
}

impl CircBuf {
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        TRADE_LIST_CAPACITY
    }

    pub fn get(&self, idx: usize) -> Option<&TradeEvent> {
        if idx >= self.len() {
            None
        } else {
            let real_idx = (self.ptr as usize + idx) % TRADE_LIST_CAPACITY;
            Some(&self.buf[real_idx])
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &TradeEvent> {
        (0..self.len()).filter_map(|i| self.get(i))
    }
}

impl core::fmt::Debug for CircBuf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CircBuf")
            .field("ptr", &self.ptr)
            .field("len", &self.len)
            .finish()
    }
}

/// Orderbook header.
///
/// This is a `repr(C)` zero-copy header view like the orderbook header in
/// `phoenix-eternal-types`. It is borrowed directly from the account bytes by
/// [`super::Orderbook::try_from_account_bytes`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct OrderbookHeader {
    pub discriminant: u64,
    pub market_status: u8,
    /// The number of decimals for the base lot.
    pub base_lots_decimals: i8,
    _padding0: [u8; 6],
    pub sequence_number: SequenceNumber,
    /// The ID of the asset being traded in the market.
    pub asset_id: u32,
    _asset_id_padding: [u8; 4],
    /// The symbol of the asset being traded in the market.
    pub asset_symbol: [u8; 16],
    /// Tick size in quote lots per base lot.
    pub tick_size_in_quote_lots_per_base_lot: QuoteLotsPerBaseLotPerTick,
    /// The sequence number of the next order.
    pub order_sequence_number: SequenceNumber,
    /// The sequence number of the next trade.
    pub trade_sequence_number: SequenceNumber,
    /// Taker fees in micros.
    pub default_taker_fee_micro: u32,
    /// Default maker fee in micros. Can be negative for rebates.
    pub default_maker_fee_micro: i32,
    /// Total maker fees collected, in signed quote lots.
    pub total_maker_quote_lot_fees: SignedQuoteLots,
    /// Total taker fees collected, in quote lots.
    pub total_taker_quote_lot_fees: QuoteLots,
    _padding1: [u64; 32],
    /// List of recent trades that have occurred in the market.
    pub trade_list: CircBuf,
}

impl OrderbookHeader {
    /// Get the market status.
    #[inline(always)]
    pub const fn market_status(&self) -> u8 {
        self.market_status
    }

    /// Get the base lots decimals.
    #[inline(always)]
    pub const fn base_lots_decimals(&self) -> i8 {
        self.base_lots_decimals
    }

    /// Get the sequence number.
    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    /// Get the asset ID.
    #[inline(always)]
    pub const fn asset_id(&self) -> u32 {
        self.asset_id
    }

    /// Get the asset symbol.
    #[inline(always)]
    pub fn asset_symbol(&self) -> Result<AssetSymbol, PhoenixAccountDecodeError> {
        AssetSymbol::try_from_bytes_for_account(self.asset_symbol, ORDERBOOK_ACCOUNT)
    }

    /// Get the tick size in quote lots per base lot.
    #[inline(always)]
    pub const fn tick_size_in_quote_lots_per_base_lot(&self) -> QuoteLotsPerBaseLotPerTick {
        self.tick_size_in_quote_lots_per_base_lot
    }

    /// Get the order sequence number.
    #[inline(always)]
    pub const fn order_sequence_number(&self) -> SequenceNumber {
        self.order_sequence_number
    }

    /// Get the trade sequence number.
    #[inline(always)]
    pub const fn trade_sequence_number(&self) -> SequenceNumber {
        self.trade_sequence_number
    }

    #[inline(always)]
    pub const fn default_taker_fee_micro(&self) -> u32 {
        self.default_taker_fee_micro
    }

    #[inline(always)]
    pub const fn default_maker_fee_micro(&self) -> i32 {
        self.default_maker_fee_micro
    }

    #[inline(always)]
    pub const fn total_maker_quote_lot_fees(&self) -> SignedQuoteLots {
        self.total_maker_quote_lot_fees
    }

    #[inline(always)]
    pub const fn total_taker_quote_lot_fees(&self) -> QuoteLots {
        self.total_taker_quote_lot_fees
    }

    #[inline(always)]
    pub const fn trade_list(&self) -> &CircBuf {
        &self.trade_list
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for OrderbookHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let asset_symbol = self.asset_symbol().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("OrderbookHeader", 12)?;
        state.serialize_field("market_status", &self.market_status())?;
        state.serialize_field("base_lots_decimals", &self.base_lots_decimals())?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("asset_id", &self.asset_id())?;
        state.serialize_field("asset_symbol", &asset_symbol)?;
        state.serialize_field(
            "tick_size_in_quote_lots_per_base_lot",
            &self.tick_size_in_quote_lots_per_base_lot(),
        )?;
        state.serialize_field("order_sequence_number", &self.order_sequence_number())?;
        state.serialize_field("trade_sequence_number", &self.trade_sequence_number())?;
        state.serialize_field("default_taker_fee_micro", &self.default_taker_fee_micro())?;
        state.serialize_field("default_maker_fee_micro", &self.default_maker_fee_micro())?;
        state.serialize_field(
            "total_maker_quote_lot_fees",
            &self.total_maker_quote_lot_fees(),
        )?;
        state.serialize_field(
            "total_taker_quote_lot_fees",
            &self.total_taker_quote_lot_fees(),
        )?;
        state.end()
    }
}

const_assert_eq!(core::mem::size_of::<OrderFill>(), 16);
const_assert_eq!(core::mem::size_of::<TradeEvent>(), 80);
const_assert_eq!(core::mem::size_of::<CircBuf>(), 5128);
const_assert_eq!(core::mem::size_of::<OrderbookHeader>(), 5504);
