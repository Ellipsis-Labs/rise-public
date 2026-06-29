//! Borrowed views for Phoenix trader accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

pub mod capabilities;

pub use capabilities::{
    ALL_TRADER_CAPABILITY_KINDS, CapabilityAccess, REQUIRED_TRADER_CAPABILITIES,
    TRADER_CAPABILITY_CAN_DEPOSIT, TRADER_CAPABILITY_CAN_PLACE_LIMIT,
    TRADER_CAPABILITY_CAN_PLACE_MARKET, TRADER_CAPABILITY_CAN_RISK_INCREASE,
    TRADER_CAPABILITY_CAN_WITHDRAW, TRADER_CAPABILITY_HOT, TRADER_READY_CAPABILITIES,
    TraderCapabilities, TraderCapabilityFlags, TraderCapabilityFlagsError, TraderCapabilityKind,
    is_trader_cold, is_trader_frozen, is_trader_hot, is_trader_ready, is_trader_reduce_only,
};

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_i56, read_i64, read_prefix, read_u64,
    verify_discriminant,
};
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const TRADER_ACCOUNT: &str = "Trader";
const TRADER_HEADER_LEN: usize = core::mem::size_of::<TraderHeaderRaw>();
const TRADER_POSITION_MAP_PREFIX_LEN: usize = 16;
const TRADER_POSITION_ENTRY_LEN: usize = 40;

const_assert_eq!(core::mem::size_of::<TraderState>(), 16);
const_assert_eq!(core::mem::size_of::<TraderHeaderRaw>(), 224);

/// Fixed trader account state stored in the trader header.
///
/// Use [`TraderState::capability_flags`] and the convenience predicates when
/// deciding whether a trader can place orders, move collateral, or only reduce
/// risk.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct TraderState {
    pub quote_lot_collateral: i64,
    pub flags: u32,
    _padding: [u8; 1],
    pub global_position_sequence_number: u8,
    pub maker_fee_override_multiplier: i8,
    pub taker_fee_override_multiplier: i8,
}

impl TraderState {
    #[inline(always)]
    pub const fn capability_flags(&self) -> TraderCapabilityFlags {
        TraderCapabilityFlags::new(self.flags)
    }

    #[inline(always)]
    pub const fn capabilities(&self) -> TraderCapabilities {
        self.capability_flags().capabilities()
    }

    #[inline(always)]
    pub const fn is_ready(&self) -> bool {
        self.capability_flags().is_ready()
    }

    #[inline(always)]
    pub const fn is_hot(&self) -> bool {
        self.capability_flags().is_hot()
    }

    #[inline(always)]
    pub const fn is_cold(&self) -> bool {
        self.capability_flags().is_cold()
    }

    #[inline(always)]
    pub const fn is_reduce_only(&self) -> bool {
        self.capability_flags().is_reduce_only()
    }

    #[inline(always)]
    pub const fn is_frozen(&self) -> bool {
        self.capability_flags().is_frozen()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct TraderHeaderRaw {
    discriminant: u64,
    sequence_number: SequenceNumber,
    key: [u8; 32],
    authority: [u8; 32],
    trader_state: TraderState,
    _padding0: [u8; 4],
    withdraw_queue_node: u32,
    max_positions: u32,
    trader_preference_bits: u32,
    position_authority: [u8; 32],
    num_markets_with_splines: u16,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    funding_key: [u8; 32],
    _padding1: [u8; 4],
    last_deposit_slot: u64,
    conditional_order_bits: [u8; 24],
}

/// View over the fixed-size Phoenix trader header plus the position map prefix.
#[derive(Clone, Copy, Debug)]
pub struct TraderHeader {
    raw: TraderHeaderRaw,
    position_count: u64,
    position_capacity: u64,
}

impl TraderHeader {
    /// Decode the fixed trader header and position-map prefix from raw account
    /// data.
    ///
    /// This verifies the trader account discriminant and checks that the
    /// dynamic position map length is internally consistent. Use
    /// [`TraderPositions::try_from_account_bytes`] to iterate the positions
    /// themselves.
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        verify_discriminant(TRADER_ACCOUNT, data, PhoenixAccount::Trader.discriminant())?;
        let raw = read_prefix::<TraderHeaderRaw>(TRADER_ACCOUNT, data)?;
        let position_prefix_start = TRADER_HEADER_LEN;
        let position_prefix_end = position_prefix_start
            .checked_add(TRADER_POSITION_MAP_PREFIX_LEN)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position map prefix offset overflow",
            })?;
        let prefix = data.get(position_prefix_start..position_prefix_end).ok_or(
            PhoenixAccountDecodeError::AccountTooSmall {
                account: TRADER_ACCOUNT,
                expected: position_prefix_end,
                actual: data.len(),
            },
        )?;
        let position_count = read_u64(prefix, 0);
        let position_capacity = read_u64(prefix, 8);
        if position_count > position_capacity {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position map length exceeds capacity",
            });
        }
        Ok(Self {
            raw,
            position_count,
            position_capacity,
        })
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.raw.sequence_number
    }

    #[inline(always)]
    pub const fn key(&self) -> [u8; 32] {
        self.raw.key
    }

    #[inline(always)]
    pub const fn authority(&self) -> [u8; 32] {
        self.raw.authority
    }

    #[inline(always)]
    pub const fn state(&self) -> TraderState {
        self.raw.trader_state
    }

    #[inline(always)]
    pub const fn flags(&self) -> u32 {
        self.raw.trader_state.flags
    }

    #[inline(always)]
    pub const fn capability_flags(&self) -> TraderCapabilityFlags {
        self.raw.trader_state.capability_flags()
    }

    #[inline(always)]
    pub const fn capabilities(&self) -> TraderCapabilities {
        self.raw.trader_state.capabilities()
    }

    #[inline(always)]
    pub const fn withdraw_queue_node(&self) -> Option<u32> {
        if self.raw.withdraw_queue_node == 0 {
            None
        } else {
            Some(self.raw.withdraw_queue_node)
        }
    }

    #[inline(always)]
    pub const fn max_positions(&self) -> u32 {
        self.raw.max_positions
    }

    #[inline(always)]
    pub const fn trader_preference_bits(&self) -> u32 {
        self.raw.trader_preference_bits
    }

    #[inline(always)]
    pub const fn position_authority(&self) -> [u8; 32] {
        self.raw.position_authority
    }

    #[inline(always)]
    pub const fn num_markets_with_splines(&self) -> u16 {
        self.raw.num_markets_with_splines
    }

    #[inline(always)]
    pub const fn trader_pda_index(&self) -> u8 {
        self.raw.trader_pda_index
    }

    #[inline(always)]
    pub const fn trader_subaccount_index(&self) -> u8 {
        self.raw.trader_subaccount_index
    }

    #[inline(always)]
    pub const fn funding_key(&self) -> [u8; 32] {
        self.raw.funding_key
    }

    #[inline(always)]
    pub const fn last_deposit_slot(&self) -> u64 {
        self.raw.last_deposit_slot
    }

    #[inline(always)]
    pub const fn conditional_order_bits(&self) -> &[u8; 24] {
        &self.raw.conditional_order_bits
    }

    #[inline(always)]
    pub const fn position_count(&self) -> u64 {
        self.position_count
    }

    #[inline(always)]
    pub const fn position_capacity(&self) -> u64 {
        self.position_capacity
    }

    #[inline(always)]
    pub const fn has_position_capacity(&self) -> bool {
        self.position_count < self.position_capacity
            && self.position_count < self.raw.max_positions as u64
    }

    #[inline(always)]
    pub const fn is_ready(&self) -> bool {
        is_trader_ready(self.flags())
    }

    #[inline(always)]
    pub const fn is_hot(&self) -> bool {
        is_trader_hot(self.flags())
    }

    #[inline(always)]
    pub const fn is_cold(&self) -> bool {
        is_trader_cold(self.flags())
    }

    #[inline(always)]
    pub const fn is_reduce_only(&self) -> bool {
        is_trader_reduce_only(self.flags())
    }

    #[inline(always)]
    pub const fn is_frozen(&self) -> bool {
        is_trader_frozen(self.flags())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderPosition {
    pub base_lot_position: i64,
    pub virtual_quote_lot_position: i64,
    pub cumulative_funding_snapshot: i64,
    pub position_sequence_number: u8,
    pub accumulated_funding_for_active_position: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderPositionEntry {
    pub asset_id: u64,
    pub position: TraderPosition,
}

/// Borrowed view over a trader's dynamic position map.
#[derive(Clone, Copy, Debug)]
pub struct TraderPositions<'a> {
    entries: &'a [u8],
    len: u64,
    capacity: u64,
}

impl<'a> TraderPositions<'a> {
    /// Decode a borrowed view over a trader account's dynamic position map.
    ///
    /// The view borrows `data`; each iterator item is copied out of the account
    /// bytes so callers do not rely on the input slice being aligned to the
    /// on-chain struct layout.
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        let header = TraderHeader::try_from_account_bytes(data)?;
        let entries_start = TRADER_HEADER_LEN
            .checked_add(TRADER_POSITION_MAP_PREFIX_LEN)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position entries offset overflow",
            })?;
        let entries_len = (header.position_count as usize)
            .checked_mul(TRADER_POSITION_ENTRY_LEN)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position map length overflow",
            })?;
        let entries_end = entries_start.checked_add(entries_len).ok_or(
            PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position entries end offset overflow",
            },
        )?;
        let entries = data.get(entries_start..entries_end).ok_or(
            PhoenixAccountDecodeError::AccountTooSmall {
                account: TRADER_ACCOUNT,
                expected: entries_end,
                actual: data.len(),
            },
        )?;
        Ok(Self {
            entries,
            len: header.position_count,
            capacity: header.position_capacity,
        })
    }

    #[inline(always)]
    pub const fn len(&self) -> u64 {
        self.len
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u64 {
        self.capacity
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate copied position entries in account order.
    #[inline(always)]
    pub fn iter(&self) -> TraderPositionIter<'a> {
        TraderPositionIter {
            remaining: self.entries,
        }
    }
}

pub struct TraderPositionIter<'a> {
    remaining: &'a [u8],
}

impl Iterator for TraderPositionIter<'_> {
    type Item = TraderPositionEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.remaining.get(..TRADER_POSITION_ENTRY_LEN)?;
        self.remaining = &self.remaining[TRADER_POSITION_ENTRY_LEN..];
        Some(read_trader_position_entry(entry))
    }
}

fn read_trader_position_entry(data: &[u8]) -> TraderPositionEntry {
    TraderPositionEntry {
        asset_id: read_u64(data, 0),
        position: TraderPosition {
            base_lot_position: read_i64(data, 8),
            virtual_quote_lot_position: read_i64(data, 16),
            cumulative_funding_snapshot: read_i64(data, 24),
            position_sequence_number: data[32],
            accumulated_funding_for_active_position: read_i56(data, 33),
        },
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TraderState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TraderState", 12)?;
        state.serialize_field("quote_lot_collateral", &self.quote_lot_collateral)?;
        state.serialize_field("flags", &self.flags)?;
        state.serialize_field("capability_flags", &self.capability_flags())?;
        state.serialize_field("capabilities", &self.capabilities())?;
        state.serialize_field(
            "global_position_sequence_number",
            &self.global_position_sequence_number,
        )?;
        state.serialize_field(
            "maker_fee_override_multiplier",
            &self.maker_fee_override_multiplier,
        )?;
        state.serialize_field(
            "taker_fee_override_multiplier",
            &self.taker_fee_override_multiplier,
        )?;
        state.serialize_field("is_ready", &self.is_ready())?;
        state.serialize_field("is_hot", &self.is_hot())?;
        state.serialize_field("is_cold", &self.is_cold())?;
        state.serialize_field("is_reduce_only", &self.is_reduce_only())?;
        state.serialize_field("is_frozen", &self.is_frozen())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TraderHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TraderHeader", 25)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("key", &pubkey_string(&self.key()))?;
        state.serialize_field("authority", &pubkey_string(&self.authority()))?;
        state.serialize_field("state", &self.state())?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("capability_flags", &self.capability_flags())?;
        state.serialize_field("capabilities", &self.capabilities())?;
        state.serialize_field("withdraw_queue_node", &self.withdraw_queue_node())?;
        state.serialize_field("max_positions", &self.max_positions())?;
        state.serialize_field("trader_preference_bits", &self.trader_preference_bits())?;
        state.serialize_field(
            "position_authority",
            &pubkey_string(&self.position_authority()),
        )?;
        state.serialize_field("num_markets_with_splines", &self.num_markets_with_splines())?;
        state.serialize_field("trader_pda_index", &self.trader_pda_index())?;
        state.serialize_field("trader_subaccount_index", &self.trader_subaccount_index())?;
        state.serialize_field("funding_key", &pubkey_string(&self.funding_key()))?;
        state.serialize_field("last_deposit_slot", &self.last_deposit_slot())?;
        state.serialize_field("conditional_order_bits", self.conditional_order_bits())?;
        state.serialize_field("position_count", &self.position_count())?;
        state.serialize_field("position_capacity", &self.position_capacity())?;
        state.serialize_field("has_position_capacity", &self.has_position_capacity())?;
        state.serialize_field("is_ready", &self.is_ready())?;
        state.serialize_field("is_hot", &self.is_hot())?;
        state.serialize_field("is_cold", &self.is_cold())?;
        state.serialize_field("is_reduce_only", &self.is_reduce_only())?;
        state.serialize_field("is_frozen", &self.is_frozen())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
struct TraderPositionEntries<'a>(&'a TraderPositions<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for TraderPositionEntries<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let len = usize::try_from(self.0.len()).ok();
        let mut seq = serializer.serialize_seq(len)?;
        for entry in self.0.iter() {
            seq.serialize_element(&entry)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TraderPositions<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TraderPositions", 4)?;
        state.serialize_field("len", &self.len())?;
        state.serialize_field("capacity", &self.capacity())?;
        state.serialize_field("is_empty", &self.is_empty())?;
        state.serialize_field("entries", &TraderPositionEntries(self))?;
        state.end()
    }
}
