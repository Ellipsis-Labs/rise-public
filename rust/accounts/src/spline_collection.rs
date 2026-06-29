//! Borrowed views for Phoenix spline collection accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_pod, read_prefix, verify_discriminant,
};
use super::discriminants::PhoenixAccount;
use super::perp_asset_map::AssetSymbol;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const SPLINE_COLLECTION_ACCOUNT: &str = "SplineCollection";
const SPLINE_COLLECTION_HEADER_LEN: usize = core::mem::size_of::<SplineCollectionHeaderLayout>();
const SPLINE_LEN: usize = core::mem::size_of::<Spline>();

pub const SPLINE_REGION_CAPACITY: usize = 10;
pub const GTC_LIFESPAN: u64 = u64::MAX;
pub const MIN_SLOT: u64 = 0;

const_assert_eq!(core::mem::size_of::<SplineCollectionHeaderLayout>(), 112);
const_assert_eq!(core::mem::size_of::<TickRegion>(), 48);
const_assert_eq!(core::mem::size_of::<Spline>(), 1336);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SplineCollectionHeaderLayout {
    discriminant: u64,
    market: [u8; 32],
    asset_symbol: [u8; 16],
    sequence_number: SequenceNumber,
    num_splines: u32,
    num_active: u32,
    _padding: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplineCollectionHeader {
    market: [u8; 32],
    asset_symbol: AssetSymbol,
    sequence_number: SequenceNumber,
    num_splines: u32,
    num_active: u32,
}

impl SplineCollectionHeader {
    fn try_from_layout(
        value: SplineCollectionHeaderLayout,
    ) -> Result<Self, PhoenixAccountDecodeError> {
        Ok(Self {
            market: value.market,
            asset_symbol: AssetSymbol::try_from_bytes_for_account(
                value.asset_symbol,
                SPLINE_COLLECTION_ACCOUNT,
            )?,
            sequence_number: value.sequence_number,
            num_splines: value.num_splines,
            num_active: value.num_active,
        })
    }

    #[inline(always)]
    pub const fn market(&self) -> [u8; 32] {
        self.market
    }

    #[inline(always)]
    pub const fn asset_symbol(&self) -> AssetSymbol {
        self.asset_symbol
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn num_splines(&self) -> u32 {
        self.num_splines
    }

    #[inline(always)]
    pub const fn num_active(&self) -> u32 {
        self.num_active
    }

    #[inline(always)]
    pub const fn has_no_active_splines(&self) -> bool {
        self.num_active == 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TickRegion {
    start_offset: u64,
    end_offset: u64,
    density: u32,
    top_level_hidden_take_size: u32,
    total_size: u64,
    filled_size: u32,
    hidden_filled_size: u32,
    lifespan: u64,
}

impl TickRegion {
    #[inline(always)]
    pub const fn start_offset(&self) -> u64 {
        self.start_offset
    }

    #[inline(always)]
    pub const fn end_offset(&self) -> u64 {
        self.end_offset
    }

    #[inline(always)]
    pub const fn density(&self) -> u32 {
        self.density
    }

    #[inline(always)]
    pub const fn top_level_hidden_take_size(&self) -> u32 {
        self.top_level_hidden_take_size
    }

    #[inline(always)]
    pub const fn total_size(&self) -> u64 {
        self.total_size
    }

    #[inline(always)]
    pub const fn filled_size(&self) -> u32 {
        self.filled_size
    }

    #[inline(always)]
    pub const fn hidden_filled_size(&self) -> u32 {
        self.hidden_filled_size
    }

    #[inline(always)]
    pub const fn lifespan(&self) -> u64 {
        self.lifespan
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.density == 0
    }

    #[inline(always)]
    pub const fn unfilled_size(&self) -> u64 {
        self.total_size.saturating_sub(self.filled_size as u64)
    }

    #[inline(always)]
    pub const fn is_active(&self, current_slot: u64, last_updated_slot: u64) -> bool {
        self.lifespan.saturating_add(last_updated_slot) >= current_slot
            && (self.filled_size as u64) < self.total_size
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Spline {
    trader: [u8; 32],
    is_active: u8,
    _padding: [u8; 7],
    mid_price: u64,
    sequence_number: SequenceNumber,
    user_update_slot: u64,
    bid_offset: u64,
    ask_offset: u64,
    bid_filled_amount: u64,
    ask_filled_amount: u64,
    bid_num_regions: u64,
    ask_num_regions: u64,
    bid_regions: [TickRegion; SPLINE_REGION_CAPACITY],
    ask_regions: [TickRegion; SPLINE_REGION_CAPACITY],
    user_price_sequence_number: u64,
    user_parameter_sequence_number: u64,
    flags: u32,
    leverage_decrease_in_bps: u32,
    max_position_size_long: u32,
    max_position_size_short: u32,
    _padding2: [u64; 28],
}

impl Spline {
    const MAX_POSITION_SIZE_ACTIVE: u32 = 1 << 0;

    #[inline(always)]
    pub const fn trader(&self) -> [u8; 32] {
        self.trader
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        self.is_active != 0
    }

    #[inline(always)]
    pub const fn is_enabled(&self) -> bool {
        self.is_active() && self.mid_price != 0
    }

    #[inline(always)]
    pub const fn mid_price(&self) -> u64 {
        self.mid_price
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn user_update_slot(&self) -> u64 {
        self.user_update_slot
    }

    #[inline(always)]
    pub const fn bid_offset(&self) -> u64 {
        self.bid_offset
    }

    #[inline(always)]
    pub const fn ask_offset(&self) -> u64 {
        self.ask_offset
    }

    #[inline(always)]
    pub const fn bid_filled_amount(&self) -> u64 {
        self.bid_filled_amount
    }

    #[inline(always)]
    pub const fn ask_filled_amount(&self) -> u64 {
        self.ask_filled_amount
    }

    #[inline(always)]
    pub const fn bid_num_regions(&self) -> u64 {
        self.bid_num_regions
    }

    #[inline(always)]
    pub const fn ask_num_regions(&self) -> u64 {
        self.ask_num_regions
    }

    #[inline(always)]
    pub const fn bid_regions(&self) -> &[TickRegion; SPLINE_REGION_CAPACITY] {
        &self.bid_regions
    }

    #[inline(always)]
    pub const fn ask_regions(&self) -> &[TickRegion; SPLINE_REGION_CAPACITY] {
        &self.ask_regions
    }

    #[inline(always)]
    pub fn active_bid_regions(&self) -> &[TickRegion] {
        active_regions(&self.bid_regions, self.bid_offset, self.bid_num_regions)
    }

    #[inline(always)]
    pub fn active_ask_regions(&self) -> &[TickRegion] {
        active_regions(&self.ask_regions, self.ask_offset, self.ask_num_regions)
    }

    #[inline(always)]
    pub const fn user_price_sequence_number(&self) -> u64 {
        self.user_price_sequence_number
    }

    #[inline(always)]
    pub const fn user_parameter_sequence_number(&self) -> u64 {
        self.user_parameter_sequence_number
    }

    #[inline(always)]
    pub const fn flags(&self) -> u32 {
        self.flags
    }

    #[inline(always)]
    pub const fn leverage_decrease_in_bps(&self) -> u32 {
        self.leverage_decrease_in_bps
    }

    #[inline(always)]
    pub const fn max_position_size_long(&self) -> u32 {
        self.max_position_size_long
    }

    #[inline(always)]
    pub const fn max_position_size_short(&self) -> u32 {
        self.max_position_size_short
    }

    #[inline(always)]
    pub const fn has_max_position_size(&self) -> bool {
        self.flags & Self::MAX_POSITION_SIZE_ACTIVE != 0
    }
}

/// View over a spline collection account.
#[derive(Clone, Copy, Debug)]
pub struct SplineCollection<'a> {
    header: SplineCollectionHeader,
    splines: &'a [u8],
}

impl<'a> SplineCollection<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        verify_discriminant(
            SPLINE_COLLECTION_ACCOUNT,
            data,
            PhoenixAccount::SplineCollection.discriminant(),
        )?;
        let header_layout: SplineCollectionHeaderLayout =
            read_prefix(SPLINE_COLLECTION_ACCOUNT, data)?;
        let header = SplineCollectionHeader::try_from_layout(header_layout)?;

        if header.num_active() > header.num_splines() {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: SPLINE_COLLECTION_ACCOUNT,
                reason: "active spline count exceeds spline count",
            });
        }

        let splines_len = (header.num_splines() as usize)
            .checked_mul(SPLINE_LEN)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: SPLINE_COLLECTION_ACCOUNT,
                reason: "spline count overflow",
            })?;
        let splines_end = SPLINE_COLLECTION_HEADER_LEN
            .checked_add(splines_len)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: SPLINE_COLLECTION_ACCOUNT,
                reason: "spline end offset overflow",
            })?;
        let splines = data.get(SPLINE_COLLECTION_HEADER_LEN..splines_end).ok_or(
            PhoenixAccountDecodeError::AccountTooSmall {
                account: SPLINE_COLLECTION_ACCOUNT,
                expected: splines_end,
                actual: data.len(),
            },
        )?;

        Ok(Self { header, splines })
    }

    #[inline(always)]
    pub const fn header(&self) -> &SplineCollectionHeader {
        &self.header
    }

    #[inline(always)]
    pub const fn market(&self) -> [u8; 32] {
        self.header.market()
    }

    #[inline(always)]
    pub const fn asset_symbol(&self) -> AssetSymbol {
        self.header.asset_symbol()
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.header.sequence_number()
    }

    #[inline(always)]
    pub const fn num_splines(&self) -> u32 {
        self.header.num_splines()
    }

    #[inline(always)]
    pub const fn num_active(&self) -> u32 {
        self.header.num_active()
    }

    #[inline(always)]
    pub const fn spline_capacity(&self) -> usize {
        self.splines.len() / SPLINE_LEN
    }

    #[inline(always)]
    pub fn splines(&self) -> SplineIter<'a> {
        self.iter()
    }

    #[inline(always)]
    pub fn iter(&self) -> SplineIter<'a> {
        SplineIter {
            remaining: self.splines,
        }
    }
}

pub struct SplineIter<'a> {
    remaining: &'a [u8],
}

impl Iterator for SplineIter<'_> {
    type Item = Result<Spline, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let spline = self.remaining.get(..SPLINE_LEN)?;
        self.remaining = &self.remaining[SPLINE_LEN..];
        Some(read_pod(SPLINE_COLLECTION_ACCOUNT, spline))
    }
}

impl ExactSizeIterator for SplineIter<'_> {
    fn len(&self) -> usize {
        self.remaining.len() / SPLINE_LEN
    }
}

fn active_regions(
    regions: &[TickRegion; SPLINE_REGION_CAPACITY],
    offset: u64,
    num_regions: u64,
) -> &[TickRegion] {
    let start = usize::try_from(offset).unwrap_or(SPLINE_REGION_CAPACITY);
    let len = usize::try_from(num_regions).unwrap_or(SPLINE_REGION_CAPACITY);
    let end = start.saturating_add(len).min(SPLINE_REGION_CAPACITY);
    regions.get(start..end).unwrap_or(&[])
}

#[cfg(feature = "serde")]
impl serde::Serialize for SplineCollectionHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("SplineCollectionHeader", 6)?;
        state.serialize_field("market", &pubkey_string(&self.market()))?;
        state.serialize_field("asset_symbol", &self.asset_symbol())?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("num_splines", &self.num_splines())?;
        state.serialize_field("num_active", &self.num_active())?;
        state.serialize_field("has_no_active_splines", &self.has_no_active_splines())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Spline {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Spline", 21)?;
        state.serialize_field("trader", &pubkey_string(&self.trader()))?;
        state.serialize_field("is_active", &self.is_active())?;
        state.serialize_field("is_enabled", &self.is_enabled())?;
        state.serialize_field("mid_price", &self.mid_price())?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("user_update_slot", &self.user_update_slot())?;
        state.serialize_field("bid_offset", &self.bid_offset())?;
        state.serialize_field("ask_offset", &self.ask_offset())?;
        state.serialize_field("bid_filled_amount", &self.bid_filled_amount())?;
        state.serialize_field("ask_filled_amount", &self.ask_filled_amount())?;
        state.serialize_field("bid_num_regions", &self.bid_num_regions())?;
        state.serialize_field("ask_num_regions", &self.ask_num_regions())?;
        state.serialize_field("bid_regions", self.bid_regions())?;
        state.serialize_field("ask_regions", self.ask_regions())?;
        state.serialize_field(
            "user_price_sequence_number",
            &self.user_price_sequence_number(),
        )?;
        state.serialize_field(
            "user_parameter_sequence_number",
            &self.user_parameter_sequence_number(),
        )?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("leverage_decrease_in_bps", &self.leverage_decrease_in_bps())?;
        state.serialize_field("max_position_size_long", &self.max_position_size_long())?;
        state.serialize_field("max_position_size_short", &self.max_position_size_short())?;
        state.serialize_field("has_max_position_size", &self.has_max_position_size())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SplineCollection<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("SplineCollection", 2)?;
        state.serialize_field("header", self.header())?;
        state.serialize_field("splines", &SplineList(self))?;
        state.end()
    }
}

#[cfg(feature = "serde")]
struct SplineList<'a>(&'a SplineCollection<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for SplineList<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.spline_capacity()))?;
        for spline in self.0.iter() {
            seq.serialize_element(&spline.map_err(serde::ser::Error::custom)?)?;
        }
        seq.end()
    }
}
