//! Borrowed views for Phoenix conditional order accounts.

use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::{BaseLots, Ticks};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_pod, read_prefix, require_len,
    verify_discriminant,
};
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const CONDITIONAL_ORDER_ACCOUNT: &str = "ConditionalOrderHeader";
const CONDITIONAL_ORDER_HEADER_LEN: usize = core::mem::size_of::<ConditionalOrderHeader>();
const CONDITIONAL_ORDER_LEN: usize = core::mem::size_of::<ConditionalOrder>();

const_assert_eq!(core::mem::size_of::<ConditionalOrderHeader>(), 96);
const_assert_eq!(core::mem::size_of::<OptionalFifoOrderId>(), 16);
const_assert_eq!(core::mem::size_of::<TriggerOrderFlags>(), 1);
const_assert_eq!(core::mem::size_of::<TriggerOrder>(), 24);
const_assert_eq!(core::mem::size_of::<ConditionalOrderFlags>(), 1);
const_assert_eq!(core::mem::size_of::<ConditionalOrder>(), 112);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ConditionalOrderHeader {
    discriminant: u64,
    trader_key: [u8; 32],
    funding_key: [u8; 32],
    sequence_number: SequenceNumber,
    len: u8,
    capacity: u8,
    _padding: [u8; 6],
}

impl ConditionalOrderHeader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(
            CONDITIONAL_ORDER_ACCOUNT,
            data,
            CONDITIONAL_ORDER_HEADER_LEN,
        )?;
        verify_discriminant(
            CONDITIONAL_ORDER_ACCOUNT,
            data,
            PhoenixAccount::ConditionalOrderHeader.discriminant(),
        )?;
        read_prefix(CONDITIONAL_ORDER_ACCOUNT, data)
    }

    #[inline(always)]
    pub const fn trader_key(&self) -> [u8; 32] {
        self.trader_key
    }

    #[inline(always)]
    pub const fn funding_key(&self) -> [u8; 32] {
        self.funding_key
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn len(&self) -> u8 {
        self.len
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u8 {
        self.capacity
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub const fn can_fit_raw_index(&self, index: u8) -> bool {
        index != 0 && self.capacity > index
    }

    #[inline(always)]
    pub const fn required_account_size(capacity: u8) -> usize {
        CONDITIONAL_ORDER_HEADER_LEN + (CONDITIONAL_ORDER_LEN * capacity as usize)
    }

    #[inline(always)]
    pub fn validate_trader(&self, trader_key: &[u8; 32]) -> bool {
        self.trader_key() == *trader_key
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OptionalFifoOrderId {
    price_in_ticks: Ticks,
    order_sequence_number: u64,
}

impl OptionalFifoOrderId {
    #[inline(always)]
    pub const fn price_in_ticks(&self) -> Ticks {
        self.price_in_ticks
    }

    #[inline(always)]
    pub const fn order_sequence_number(&self) -> u64 {
        self.order_sequence_number
    }

    #[inline(always)]
    pub const fn is_none(&self) -> bool {
        self.price_in_ticks.as_inner() == 0 && self.order_sequence_number == 0
    }

    #[inline(always)]
    pub const fn is_some(&self) -> bool {
        !self.is_none()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct TriggerOrderFlags {
    flags: u8,
}

impl TriggerOrderFlags {
    #[inline(always)]
    pub const fn bits(&self) -> u8 {
        self.flags
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        self.flags & 1 != 0
    }

    #[inline(always)]
    pub const fn is_greater_than_trigger(&self) -> bool {
        self.flags & (1 << 1) != 0
    }

    #[inline(always)]
    pub const fn is_bid(&self) -> bool {
        self.flags & (1 << 2) != 0
    }

    #[inline(always)]
    pub const fn is_limit_order(&self) -> bool {
        self.flags & (1 << 3) != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TriggerOrder {
    trigger_price: Ticks,
    execution_price: Ticks,
    position_sequence_number: u8,
    flags: TriggerOrderFlags,
    _padding: [u8; 6],
}

impl TriggerOrder {
    #[inline(always)]
    pub const fn trigger_price(&self) -> Ticks {
        self.trigger_price
    }

    #[inline(always)]
    pub const fn execution_price(&self) -> Ticks {
        self.execution_price
    }

    #[inline(always)]
    pub const fn position_sequence_number(&self) -> u8 {
        self.position_sequence_number
    }

    #[inline(always)]
    pub const fn flags(&self) -> TriggerOrderFlags {
        self.flags
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        self.flags.is_active()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct ConditionalOrderFlags {
    flags: u8,
}

impl ConditionalOrderFlags {
    #[inline(always)]
    pub const fn bits(&self) -> u8 {
        self.flags
    }

    #[inline(always)]
    pub const fn uses_percent(&self) -> bool {
        self.flags & 1 != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ConditionalOrder {
    sequence_number: u64,
    order_id: OptionalFifoOrderId,
    max_size: BaseLots,
    fillable_size: BaseLots,
    filled_size: BaseLots,
    slot: u64,
    asset_id: u32,
    flags: ConditionalOrderFlags,
    percent: u8,
    _padding: [u8; 2],
    greater_trigger_order: TriggerOrder,
    less_trigger_order: TriggerOrder,
}

impl ConditionalOrder {
    #[inline(always)]
    pub const fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn order_id(&self) -> OptionalFifoOrderId {
        self.order_id
    }

    #[inline(always)]
    pub const fn max_size(&self) -> BaseLots {
        self.max_size
    }

    #[inline(always)]
    pub const fn fillable_size(&self) -> BaseLots {
        self.fillable_size
    }

    #[inline(always)]
    pub const fn filled_size(&self) -> BaseLots {
        self.filled_size
    }

    #[inline(always)]
    pub const fn slot(&self) -> u64 {
        self.slot
    }

    #[inline(always)]
    pub const fn asset_id(&self) -> u32 {
        self.asset_id
    }

    #[inline(always)]
    pub const fn flags(&self) -> ConditionalOrderFlags {
        self.flags
    }

    #[inline(always)]
    pub const fn percent(&self) -> u8 {
        self.percent
    }

    #[inline(always)]
    pub const fn greater_trigger_order(&self) -> TriggerOrder {
        self.greater_trigger_order
    }

    #[inline(always)]
    pub const fn less_trigger_order(&self) -> TriggerOrder {
        self.less_trigger_order
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        self.greater_trigger_order.is_active() || self.less_trigger_order.is_active()
    }
}

/// View over a conditional-order collection account.
#[derive(Clone, Copy, Debug)]
pub struct ConditionalOrders<'a> {
    header: ConditionalOrderHeader,
    orders: &'a [u8],
}

impl<'a> ConditionalOrders<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        let header = ConditionalOrderHeader::try_from_account_bytes(data)?;
        if header.len() > header.capacity() {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: CONDITIONAL_ORDER_ACCOUNT,
                reason: "conditional order length exceeds capacity",
            });
        }

        let orders_start = CONDITIONAL_ORDER_HEADER_LEN;
        let orders_len = (header.capacity() as usize)
            .checked_mul(CONDITIONAL_ORDER_LEN)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: CONDITIONAL_ORDER_ACCOUNT,
                reason: "conditional order capacity overflow",
            })?;
        let orders_end =
            orders_start
                .checked_add(orders_len)
                .ok_or(PhoenixAccountDecodeError::InvalidData {
                    account: CONDITIONAL_ORDER_ACCOUNT,
                    reason: "conditional order end offset overflow",
                })?;
        let orders_bytes = data.get(orders_start..orders_end).ok_or(
            PhoenixAccountDecodeError::AccountTooSmall {
                account: CONDITIONAL_ORDER_ACCOUNT,
                expected: orders_end,
                actual: data.len(),
            },
        )?;
        Ok(Self {
            header,
            orders: orders_bytes,
        })
    }

    #[inline(always)]
    pub const fn header(&self) -> &ConditionalOrderHeader {
        &self.header
    }

    #[inline(always)]
    pub fn orders(&self) -> ConditionalOrderIter<'a> {
        self.iter()
    }

    #[inline(always)]
    pub const fn order_capacity(&self) -> usize {
        self.orders.len() / CONDITIONAL_ORDER_LEN
    }

    #[inline(always)]
    pub fn iter(&self) -> ConditionalOrderIter<'a> {
        ConditionalOrderIter {
            remaining: self.orders,
        }
    }
}

pub struct ConditionalOrderIter<'a> {
    remaining: &'a [u8],
}

impl Iterator for ConditionalOrderIter<'_> {
    type Item = Result<ConditionalOrder, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let order = self.remaining.get(..CONDITIONAL_ORDER_LEN)?;
        self.remaining = &self.remaining[CONDITIONAL_ORDER_LEN..];
        Some(read_pod(CONDITIONAL_ORDER_ACCOUNT, order))
    }
}

impl ExactSizeIterator for ConditionalOrderIter<'_> {
    fn len(&self) -> usize {
        self.remaining.len() / CONDITIONAL_ORDER_LEN
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ConditionalOrderHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ConditionalOrderHeader", 6)?;
        state.serialize_field("trader_key", &pubkey_string(&self.trader_key()))?;
        state.serialize_field("funding_key", &pubkey_string(&self.funding_key()))?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("len", &self.len())?;
        state.serialize_field("capacity", &self.capacity())?;
        state.serialize_field("is_empty", &self.is_empty())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for OptionalFifoOrderId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("OptionalFifoOrderId", 3)?;
        state.serialize_field("price_in_ticks", &self.price_in_ticks())?;
        state.serialize_field("order_sequence_number", &self.order_sequence_number())?;
        state.serialize_field("is_some", &self.is_some())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TriggerOrderFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TriggerOrderFlags", 5)?;
        state.serialize_field("bits", &self.bits())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.serialize_field("is_greater_than_trigger", &self.is_greater_than_trigger())?;
        state.serialize_field("is_bid", &self.is_bid())?;
        state.serialize_field("is_limit_order", &self.is_limit_order())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TriggerOrder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TriggerOrder", 5)?;
        state.serialize_field("trigger_price", &self.trigger_price())?;
        state.serialize_field("execution_price", &self.execution_price())?;
        state.serialize_field("position_sequence_number", &self.position_sequence_number())?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ConditionalOrderFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ConditionalOrderFlags", 2)?;
        state.serialize_field("bits", &self.bits())?;
        state.serialize_field("uses_percent", &self.uses_percent())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ConditionalOrder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ConditionalOrder", 12)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("order_id", &self.order_id())?;
        state.serialize_field("max_size", &self.max_size())?;
        state.serialize_field("fillable_size", &self.fillable_size())?;
        state.serialize_field("filled_size", &self.filled_size())?;
        state.serialize_field("slot", &self.slot())?;
        state.serialize_field("asset_id", &self.asset_id())?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("percent", &self.percent())?;
        state.serialize_field("greater_trigger_order", &self.greater_trigger_order())?;
        state.serialize_field("less_trigger_order", &self.less_trigger_order())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ConditionalOrders<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ConditionalOrders", 2)?;
        state.serialize_field("header", self.header())?;
        state.serialize_field("orders", &ConditionalOrderList(self))?;
        state.end()
    }
}

#[cfg(feature = "serde")]
struct ConditionalOrderList<'a>(&'a ConditionalOrders<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for ConditionalOrderList<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.order_capacity()))?;
        for order in self.0.iter() {
            seq.serialize_element(&order.map_err(serde::ser::Error::custom)?)?;
        }
        seq.end()
    }
}
