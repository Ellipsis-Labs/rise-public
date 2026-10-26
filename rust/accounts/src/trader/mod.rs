//! Borrowed views for Phoenix trader accounts.

use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::{
    SequenceNumberU8, SignedBaseLots, SignedQuoteLots, SignedQuoteLotsI56, SignedQuoteLotsI56Error,
    SignedQuoteLotsPerBaseLot, TraderPosition as MathTraderPosition,
};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

pub mod capabilities;
pub mod preferences;

pub use capabilities::{
    ALL_TRADER_CAPABILITY_KINDS, CapabilityAccess, REQUIRED_TRADER_CAPABILITIES,
    TRADER_CAPABILITY_CAN_DEPOSIT, TRADER_CAPABILITY_CAN_PLACE_LIMIT,
    TRADER_CAPABILITY_CAN_PLACE_MARKET, TRADER_CAPABILITY_CAN_RISK_INCREASE,
    TRADER_CAPABILITY_CAN_WITHDRAW, TRADER_CAPABILITY_HOT, TRADER_READY_CAPABILITIES,
    TraderCapabilities, TraderCapabilityFlags, TraderCapabilityFlagsError, TraderCapabilityKind,
    is_trader_cold, is_trader_frozen, is_trader_hot, is_trader_ready, is_trader_reduce_only,
};
pub use preferences::{
    ALL_TRADER_PREFERENCE_KINDS, TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP, TraderPreferenceFlags,
    TraderPreferenceFlagsError, TraderPreferenceKind, TraderPreferences,
};

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, borrow_pod, borrow_prefix, borrow_slice,
    read_prefix, read_u64, verify_discriminant,
};
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const TRADER_ACCOUNT: &str = "Trader";
const TRADER_HEADER_LEN: usize = core::mem::size_of::<TraderHeader>();
const TRADER_POSITION_MAP_PREFIX_LEN: usize = 16;
const TRADER_POSITION_ENTRY_LEN: usize = 40;
const TRADER_POSITION_ENTRIES_START: usize = TRADER_HEADER_LEN + TRADER_POSITION_MAP_PREFIX_LEN;
const POSITION_ENTRY_ASSET_INDEX_BOUND: u32 = 0xFF00_0000;
/// Upper 16 bits of every SPL spot collateral asset index.
const SPOT_ASSET_INDEX_PREFIX: u32 = 0xFFFE_0000;
const ASSET_INDEX_BAND_MASK: u32 = 0xFFFF_0000;

pub use phoenix_rise_math::NATIVE_SOL_ASSET_INDEX;

const_assert_eq!(core::mem::size_of::<TraderState>(), 16);
const_assert_eq!(core::mem::size_of::<TraderHeader>(), 224);

/// Fixed trader account state stored in the trader header.
///
/// Use [`TraderState::capability_flags`] and the convenience predicates when
/// deciding whether a trader can place orders, move collateral, or only reduce
/// risk.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct TraderState {
    pub quote_lot_collateral: SignedQuoteLots,
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

/// Fixed zero-copy trader account header.
///
/// This is the first 224 bytes of a trader account. Borrow it directly with
/// [`TraderHeader::try_from_account_bytes`] when only header fields are needed,
/// or use [`Trader::try_from_account_bytes`] to borrow the header together with
/// the dynamic position map.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct TraderHeader {
    pub discriminant: u64,
    pub sequence_number: SequenceNumber,
    pub key: [u8; 32],
    pub authority: [u8; 32],
    pub trader_state: TraderState,
    _padding0: [u8; 4],
    pub withdraw_queue_node: u32,
    pub max_positions: u32,
    pub trader_preference_bits: u32,
    pub position_authority: [u8; 32],
    pub num_markets_with_splines: u16,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,
    pub funding_key: [u8; 32],
    _padding1: [u8; 4],
    pub last_deposit_slot: u64,
    pub conditional_order_bits: [u8; 24],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct TraderPositionMapHeader {
    len: u64,
    capacity: u64,
}

const_assert_eq!(core::mem::size_of::<TraderPositionMapHeader>(), 16);

impl TraderPositionMapHeader {
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
}

fn position_map_prefix(data: &[u8]) -> Result<&[u8], PhoenixAccountDecodeError> {
    data.get(TRADER_HEADER_LEN..TRADER_POSITION_ENTRIES_START)
        .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
            account: TRADER_ACCOUNT,
            expected: TRADER_POSITION_ENTRIES_START,
            actual: data.len(),
        })
}

fn validate_position_map_header(
    position_map_header: &TraderPositionMapHeader,
) -> Result<(), PhoenixAccountDecodeError> {
    if position_map_header.len > position_map_header.capacity {
        return Err(PhoenixAccountDecodeError::InvalidData {
            account: TRADER_ACCOUNT,
            reason: "position map length exceeds capacity",
        });
    }
    Ok(())
}

fn borrow_position_map_header(
    data: &[u8],
) -> Result<&TraderPositionMapHeader, PhoenixAccountDecodeError> {
    let prefix = position_map_prefix(data)?;
    let position_map_header = borrow_pod::<TraderPositionMapHeader>(TRADER_ACCOUNT, prefix)?;
    validate_position_map_header(position_map_header)?;
    Ok(position_map_header)
}

fn position_entries_end_for_capacity(capacity: u64) -> Result<usize, PhoenixAccountDecodeError> {
    let capacity =
        usize::try_from(capacity).map_err(|_| PhoenixAccountDecodeError::InvalidData {
            account: TRADER_ACCOUNT,
            reason: "position map capacity does not fit usize",
        })?;
    let entries_len = capacity.checked_mul(TRADER_POSITION_ENTRY_LEN).ok_or(
        PhoenixAccountDecodeError::InvalidData {
            account: TRADER_ACCOUNT,
            reason: "position map capacity byte length overflow",
        },
    )?;
    TRADER_POSITION_ENTRIES_START
        .checked_add(entries_len)
        .ok_or(PhoenixAccountDecodeError::InvalidData {
            account: TRADER_ACCOUNT,
            reason: "position entries end offset overflow",
        })
}

fn position_entries_prefix(data: &[u8], capacity: u64) -> Result<&[u8], PhoenixAccountDecodeError> {
    let entries_end = position_entries_end_for_capacity(capacity)?;
    data.get(TRADER_POSITION_ENTRIES_START..entries_end).ok_or(
        PhoenixAccountDecodeError::AccountTooSmall {
            account: TRADER_ACCOUNT,
            expected: entries_end,
            actual: data.len(),
        },
    )
}

#[inline(always)]
const fn position_entry_asset_index(raw_asset_id: u64) -> u32 {
    raw_asset_id as u32
}

#[inline(always)]
const fn is_position_entry_asset_id(raw_asset_id: u64) -> bool {
    position_entry_asset_index(raw_asset_id) < POSITION_ENTRY_ASSET_INDEX_BOUND
}

#[inline(always)]
const fn is_spot_collateral_entry_asset_id(raw_asset_id: u64) -> bool {
    let asset_index = position_entry_asset_index(raw_asset_id);
    asset_index == NATIVE_SOL_ASSET_INDEX
        || asset_index & ASSET_INDEX_BAND_MASK == SPOT_ASSET_INDEX_PREFIX
}

impl TraderHeader {
    /// Borrow the fixed trader header from raw account data.
    ///
    /// This verifies the trader account discriminant and borrows the POD header
    /// directly from `data`. Misaligned input returns
    /// [`PhoenixAccountDecodeError::InvalidLayout`] instead of panicking. Use
    /// [`Trader::try_from_account_bytes`] for the full header plus position-map
    /// account view.
    pub fn try_from_account_bytes(data: &[u8]) -> Result<&Self, PhoenixAccountDecodeError> {
        verify_discriminant(TRADER_ACCOUNT, data, PhoenixAccount::Trader.discriminant())?;
        borrow_prefix::<Self>(TRADER_ACCOUNT, data)
    }

    pub fn try_borrow_from_account_bytes(data: &[u8]) -> Result<&Self, PhoenixAccountDecodeError> {
        Self::try_from_account_bytes(data)
    }

    /// Copy the fixed trader header from raw account data without requiring
    /// the input slice to satisfy Rust alignment for the POD layout.
    pub fn try_read_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        verify_discriminant(TRADER_ACCOUNT, data, PhoenixAccount::Trader.discriminant())?;
        read_prefix::<Self>(TRADER_ACCOUNT, data)
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn key(&self) -> &[u8; 32] {
        &self.key
    }

    #[inline(always)]
    pub const fn authority(&self) -> &[u8; 32] {
        &self.authority
    }

    #[inline(always)]
    pub const fn state(&self) -> &TraderState {
        &self.trader_state
    }

    #[inline(always)]
    pub const fn flags(&self) -> u32 {
        self.trader_state.flags
    }

    #[inline(always)]
    pub const fn capability_flags(&self) -> TraderCapabilityFlags {
        self.trader_state.capability_flags()
    }

    #[inline(always)]
    pub const fn capabilities(&self) -> TraderCapabilities {
        self.trader_state.capabilities()
    }

    #[inline(always)]
    pub const fn withdraw_queue_node(&self) -> Option<u32> {
        if self.withdraw_queue_node == 0 {
            None
        } else {
            Some(self.withdraw_queue_node)
        }
    }

    #[inline(always)]
    pub const fn max_positions(&self) -> u32 {
        self.max_positions
    }

    #[inline(always)]
    pub const fn trader_preference_bits(&self) -> u32 {
        self.trader_preference_bits
    }

    #[inline(always)]
    pub const fn trader_preference_flags(&self) -> TraderPreferenceFlags {
        TraderPreferenceFlags::new(self.trader_preference_bits)
    }

    #[inline(always)]
    pub const fn preferences(&self) -> TraderPreferences {
        self.trader_preference_flags().preferences()
    }

    #[inline(always)]
    pub const fn disable_collateral_sweep(&self) -> bool {
        self.trader_preference_flags().disable_collateral_sweep()
    }

    /// Whether this trader opted out of `SwapNative` signed by its position
    /// authority. The trader wallet itself is never gated.
    #[inline(always)]
    pub const fn disable_position_authority_swap(&self) -> bool {
        self.trader_preference_flags()
            .disable_position_authority_swap()
    }

    #[inline(always)]
    pub const fn position_authority(&self) -> &[u8; 32] {
        &self.position_authority
    }

    #[inline(always)]
    pub const fn num_markets_with_splines(&self) -> u16 {
        self.num_markets_with_splines
    }

    #[inline(always)]
    pub const fn trader_pda_index(&self) -> u8 {
        self.trader_pda_index
    }

    #[inline(always)]
    pub const fn trader_subaccount_index(&self) -> u8 {
        self.trader_subaccount_index
    }

    #[inline(always)]
    pub const fn funding_key(&self) -> &[u8; 32] {
        &self.funding_key
    }

    #[inline(always)]
    pub const fn last_deposit_slot(&self) -> u64 {
        self.last_deposit_slot
    }

    #[inline(always)]
    pub const fn conditional_order_bits(&self) -> &[u8; 24] {
        &self.conditional_order_bits
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
    pub base_lot_position: SignedBaseLots,
    pub virtual_quote_lot_position: SignedQuoteLots,
    pub cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
    pub position_sequence_number: u8,
    pub accumulated_funding_for_active_position: SignedQuoteLots,
}

impl TraderPosition {
    pub fn try_to_math_position(&self) -> Result<MathTraderPosition, SignedQuoteLotsI56Error> {
        Ok(MathTraderPosition {
            base_lot_position: self.base_lot_position,
            virtual_quote_lot_position: self.virtual_quote_lot_position,
            cumulative_funding_snapshot: self.cumulative_funding_snapshot,
            position_sequence_number: SequenceNumberU8::from(self.position_sequence_number),
            accumulated_funding_for_active_position: SignedQuoteLotsI56::try_from(
                self.accumulated_funding_for_active_position,
            )?,
        })
    }
}

impl TryFrom<TraderPosition> for MathTraderPosition {
    type Error = SignedQuoteLotsI56Error;

    fn try_from(value: TraderPosition) -> Result<Self, Self::Error> {
        value.try_to_math_position()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderPositionEntry {
    pub asset_id: u64,
    pub position: TraderPosition,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct TraderPositionRaw {
    base_lot_position: SignedBaseLots,
    virtual_quote_lot_position: SignedQuoteLots,
    cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
    position_sequence_number: u8,
    accumulated_funding_for_active_position: SignedQuoteLotsI56,
}

const_assert_eq!(core::mem::size_of::<TraderPositionRaw>(), 32);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct TraderPositionEntryRaw {
    asset_id: u64,
    position: TraderPositionRaw,
}

const_assert_eq!(core::mem::size_of::<TraderPositionEntryRaw>(), 40);

/// One spot collateral balance from the trader position map.
///
/// Native SOL and SPL spot assets share this entry layout; they differ only in
/// the asset-index band of the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderSpotCollateral {
    /// Raw asset-index key: [`NATIVE_SOL_ASSET_INDEX`] for native SOL,
    /// otherwise in the `0xFFFE_xxxx` spot band. Not a perp asset id.
    pub asset_index: u32,
    /// Accounted balance in the asset's native units (lamports for SOL).
    pub balance: u64,
    /// Trader-specific per-trader cap. It only applies when it raises the cap
    /// above the asset's `max_per_trader_balance`.
    pub max_per_trader_balance_override: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct TraderPositionRef<'a> {
    raw: &'a TraderPositionRaw,
}

impl TraderPositionRef<'_> {
    #[inline(always)]
    pub const fn base_lot_position(&self) -> SignedBaseLots {
        self.raw.base_lot_position
    }

    #[inline(always)]
    pub const fn virtual_quote_lot_position(&self) -> SignedQuoteLots {
        self.raw.virtual_quote_lot_position
    }

    #[inline(always)]
    pub const fn cumulative_funding_snapshot(&self) -> SignedQuoteLotsPerBaseLot {
        self.raw.cumulative_funding_snapshot
    }

    #[inline(always)]
    pub const fn position_sequence_number(&self) -> u8 {
        self.raw.position_sequence_number
    }

    #[inline(always)]
    pub fn accumulated_funding_for_active_position(&self) -> SignedQuoteLots {
        self.raw
            .accumulated_funding_for_active_position
            .to_signed_quote_lots()
    }

    #[inline(always)]
    pub fn to_position(&self) -> TraderPosition {
        TraderPosition {
            base_lot_position: self.base_lot_position(),
            virtual_quote_lot_position: self.virtual_quote_lot_position(),
            cumulative_funding_snapshot: self.cumulative_funding_snapshot(),
            position_sequence_number: self.position_sequence_number(),
            accumulated_funding_for_active_position: self.accumulated_funding_for_active_position(),
        }
    }

    #[inline(always)]
    pub fn to_math_position(&self) -> MathTraderPosition {
        MathTraderPosition {
            base_lot_position: self.base_lot_position(),
            virtual_quote_lot_position: self.virtual_quote_lot_position(),
            cumulative_funding_snapshot: self.cumulative_funding_snapshot(),
            position_sequence_number: SequenceNumberU8::from(self.position_sequence_number()),
            accumulated_funding_for_active_position: self
                .raw
                .accumulated_funding_for_active_position,
        }
    }
}

impl From<TraderPositionRef<'_>> for TraderPosition {
    fn from(value: TraderPositionRef<'_>) -> Self {
        value.to_position()
    }
}

impl From<TraderPositionRef<'_>> for MathTraderPosition {
    fn from(value: TraderPositionRef<'_>) -> Self {
        value.to_math_position()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TraderPositionEntryRef<'a> {
    raw: &'a TraderPositionEntryRaw,
}

impl<'a> TraderPositionEntryRef<'a> {
    #[inline(always)]
    pub const fn raw_asset_id(&self) -> u64 {
        self.raw.asset_id
    }

    #[inline(always)]
    pub const fn asset_id(&self) -> u64 {
        position_entry_asset_index(self.raw.asset_id) as u64
    }

    #[inline(always)]
    pub const fn is_position(&self) -> bool {
        is_position_entry_asset_id(self.raw.asset_id)
    }

    #[inline(always)]
    pub const fn position(&self) -> TraderPositionRef<'a> {
        TraderPositionRef {
            raw: &self.raw.position,
        }
    }

    #[inline(always)]
    pub fn to_entry(&self) -> TraderPositionEntry {
        TraderPositionEntry {
            asset_id: self.asset_id(),
            position: self.position().to_position(),
        }
    }
}

impl From<TraderPositionEntryRef<'_>> for TraderPositionEntry {
    fn from(value: TraderPositionEntryRef<'_>) -> Self {
        value.to_entry()
    }
}

/// Zero-copy view over a full trader account.
///
/// This is the preferred trader account API. It borrows the fixed header,
/// position-map metadata, and the full allocated position-entry array directly
/// from the account data.
#[derive(Clone, Copy, Debug)]
pub struct Trader<'a> {
    pub header: &'a TraderHeader,
    pub position_map_header: &'a TraderPositionMapHeader,
    position_entries: &'a [TraderPositionEntryRaw],
    position_count: usize,
}

impl<'a> Trader<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        let header = TraderHeader::try_from_account_bytes(data)?;
        let position_map_header = borrow_position_map_header(data)?;
        let position_count = usize::try_from(position_map_header.len()).map_err(|_| {
            PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position map length does not fit usize",
            }
        })?;
        let entries = position_entries_prefix(data, position_map_header.capacity())?;
        let position_entries = borrow_slice::<TraderPositionEntryRaw>(TRADER_ACCOUNT, entries)?;
        Ok(Self {
            header,
            position_map_header,
            position_entries,
            position_count,
        })
    }

    /// Borrow the full trader account from raw account data without copying.
    ///
    /// This is an explicit alias for [`Trader::try_from_account_bytes`].
    pub fn try_borrow_from_account_bytes(
        data: &'a [u8],
    ) -> Result<Self, PhoenixAccountDecodeError> {
        Self::try_from_account_bytes(data)
    }

    #[inline(always)]
    pub const fn header(&self) -> &'a TraderHeader {
        self.header
    }

    #[inline(always)]
    pub const fn max_positions(&self) -> u32 {
        self.header.max_positions()
    }

    #[inline(always)]
    pub const fn trader_preference_bits(&self) -> u32 {
        self.header.trader_preference_bits()
    }

    #[inline(always)]
    pub const fn trader_preference_flags(&self) -> TraderPreferenceFlags {
        self.header.trader_preference_flags()
    }

    #[inline(always)]
    pub const fn preferences(&self) -> TraderPreferences {
        self.header.preferences()
    }

    #[inline(always)]
    pub const fn disable_collateral_sweep(&self) -> bool {
        self.header.disable_collateral_sweep()
    }

    /// See [`TraderHeader::disable_position_authority_swap`].
    #[inline(always)]
    pub const fn disable_position_authority_swap(&self) -> bool {
        self.header.disable_position_authority_swap()
    }

    #[inline(always)]
    pub const fn position_map_header(&self) -> &'a TraderPositionMapHeader {
        self.position_map_header
    }

    #[inline(always)]
    pub fn len(&self) -> u64 {
        self.position_entries().count() as u64
    }

    #[inline(always)]
    pub const fn raw_len(&self) -> u64 {
        self.position_map_header.len()
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u64 {
        self.position_map_header.capacity()
    }

    #[inline(always)]
    pub const fn has_position_capacity(&self) -> bool {
        self.raw_len() < self.capacity() && self.raw_len() < self.header.max_positions as u64
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.position_entries().next().is_none()
    }

    #[inline(always)]
    fn active_position_entries_raw(&self) -> &'a [TraderPositionEntryRaw] {
        &self.position_entries[..self.position_count]
    }

    /// Iterate active position entries in account order.
    #[inline(always)]
    pub fn position_entries(&self) -> impl Iterator<Item = TraderPositionEntryRef<'a>> + '_ {
        self.active_position_entries_raw()
            .iter()
            .map(|raw| TraderPositionEntryRef { raw })
            .filter(|entry| entry.is_position())
    }

    /// Iterate all allocated position entry slots, including unused slots.
    #[inline(always)]
    pub fn allocated_position_entries(
        &self,
    ) -> impl ExactSizeIterator<Item = TraderPositionEntryRef<'a>> + '_ {
        self.position_entries
            .iter()
            .map(|raw| TraderPositionEntryRef { raw })
    }

    /// Iterate active positions as `(asset_id, position)` pairs.
    #[inline(always)]
    pub fn positions(&self) -> impl Iterator<Item = (u64, TraderPositionRef<'a>)> + '_ {
        self.position_entries()
            .map(|entry| (entry.asset_id(), entry.position()))
    }

    /// Spot collateral entries (native SOL and SPL spot assets) in account
    /// order.
    ///
    /// These are stored in the position map but are not positions, so they
    /// never appear in [`Self::positions`].
    pub fn spot_collaterals(&self) -> impl Iterator<Item = TraderSpotCollateral> + '_ {
        self.active_position_entries_raw()
            .iter()
            .filter_map(|raw| read_spot_collateral_entry(bytemuck::bytes_of(raw)))
    }

    /// Native SOL spot collateral, in lamports. Zero when the trader holds
    /// none.
    ///
    /// This is the *accounted* balance. Lamports sitting in the account above
    /// it (rent plus any excess a capped credit could not take) are not
    /// collateral and carry no margin value.
    #[deprecated(
        since = "0.7.0",
        note = "use `spot_collaterals` and pick the `NATIVE_SOL_ASSET_INDEX` entry"
    )]
    pub fn native_sol_collateral(&self) -> u64 {
        self.spot_collaterals()
            .find(|spot| spot.asset_index == NATIVE_SOL_ASSET_INDEX)
            .map_or(0, |spot| spot.balance)
    }
}

/// Borrowed view over a trader's dynamic position map.
#[deprecated(
    since = "0.7.0",
    note = "use `Trader`; `Trader::position_entries` yields the same entries"
)]
#[derive(Clone, Copy, Debug)]
pub struct TraderPositions<'a>(Trader<'a>);

#[allow(deprecated)]
impl<'a> TraderPositions<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        Trader::try_from_account_bytes(data).map(Self)
    }

    #[inline(always)]
    pub fn len(&self) -> u64 {
        self.0.len()
    }

    #[inline(always)]
    pub const fn raw_len(&self) -> u64 {
        self.0.raw_len()
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u64 {
        self.0.capacity()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate copied position entries in account order.
    #[inline(always)]
    pub fn iter(&self) -> TraderPositionIter<'a> {
        TraderPositionIter {
            remaining: self.0.active_position_entries_raw(),
        }
    }

    pub fn native_sol_collateral(&self) -> u64 {
        self.0.native_sol_collateral()
    }
}

#[deprecated(since = "0.7.0", note = "use `Trader::position_entries`")]
pub struct TraderPositionIter<'a> {
    remaining: &'a [TraderPositionEntryRaw],
}

#[allow(deprecated)]
impl Iterator for TraderPositionIter<'_> {
    type Item = TraderPositionEntry;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((raw, rest)) = self.remaining.split_first() {
            self.remaining = rest;
            let entry = TraderPositionEntryRef { raw };
            if entry.is_position() {
                return Some(entry.to_entry());
            }
        }
        None
    }
}

/// Value layout: `balance: u64`, `max_per_trader_balance_override: u64` (zero
/// means none), 16 bytes of padding.
fn read_spot_collateral_entry(data: &[u8]) -> Option<TraderSpotCollateral> {
    let raw_asset_id = read_u64(data, 0);
    if !is_spot_collateral_entry_asset_id(raw_asset_id) {
        return None;
    }
    let max_per_trader_balance_override = read_u64(data, 16);
    Some(TraderSpotCollateral {
        asset_index: position_entry_asset_index(raw_asset_id),
        balance: read_u64(data, 8),
        max_per_trader_balance_override: (max_per_trader_balance_override != 0)
            .then_some(max_per_trader_balance_override),
    })
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
        let mut state = serializer.serialize_struct("TraderHeader", 26)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("key", &pubkey_string(self.key()))?;
        state.serialize_field("authority", &pubkey_string(self.authority()))?;
        state.serialize_field("state", &self.state())?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("capability_flags", &self.capability_flags())?;
        state.serialize_field("capabilities", &self.capabilities())?;
        state.serialize_field("withdraw_queue_node", &self.withdraw_queue_node())?;
        state.serialize_field("max_positions", &self.max_positions())?;
        state.serialize_field("trader_preference_bits", &self.trader_preference_bits())?;
        state.serialize_field("trader_preference_flags", &self.trader_preference_flags())?;
        state.serialize_field("preferences", &self.preferences())?;
        state.serialize_field("disable_collateral_sweep", &self.disable_collateral_sweep())?;
        state.serialize_field(
            "disable_position_authority_swap",
            &self.disable_position_authority_swap(),
        )?;
        state.serialize_field(
            "position_authority",
            &pubkey_string(self.position_authority()),
        )?;
        state.serialize_field("num_markets_with_splines", &self.num_markets_with_splines())?;
        state.serialize_field("trader_pda_index", &self.trader_pda_index())?;
        state.serialize_field("trader_subaccount_index", &self.trader_subaccount_index())?;
        state.serialize_field("funding_key", &pubkey_string(self.funding_key()))?;
        state.serialize_field("last_deposit_slot", &self.last_deposit_slot())?;
        state.serialize_field("conditional_order_bits", self.conditional_order_bits())?;
        state.serialize_field("is_ready", &self.is_ready())?;
        state.serialize_field("is_hot", &self.is_hot())?;
        state.serialize_field("is_cold", &self.is_cold())?;
        state.serialize_field("is_reduce_only", &self.is_reduce_only())?;
        state.serialize_field("is_frozen", &self.is_frozen())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
struct TraderPositionEntryRefs<'a>(&'a Trader<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for TraderPositionEntryRefs<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let len = usize::try_from(self.0.len()).ok();
        let mut seq = serializer.serialize_seq(len)?;
        for entry in self.0.position_entries() {
            seq.serialize_element(&entry.to_entry())?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Trader<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Trader", 6)?;
        state.serialize_field("header", self.header())?;
        state.serialize_field("position_count", &self.len())?;
        state.serialize_field("raw_position_count", &self.raw_len())?;
        state.serialize_field("position_capacity", &self.capacity())?;
        state.serialize_field("has_position_capacity", &self.has_position_capacity())?;
        state.serialize_field("positions", &TraderPositionEntryRefs(self))?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhoenixAccount;

    const TRADER_HEADER_WITH_PREFIX_LEN: usize = TRADER_HEADER_LEN + TRADER_POSITION_MAP_PREFIX_LEN;

    fn aligned_trader_header_data() -> [u64; TRADER_HEADER_WITH_PREFIX_LEN / 8] {
        let mut words = [0u64; TRADER_HEADER_WITH_PREFIX_LEN / 8];
        let data = bytemuck::cast_slice_mut::<u64, u8>(&mut words);
        data[..8].copy_from_slice(&PhoenixAccount::Trader.discriminant());
        data[24..56].copy_from_slice(&[7; 32]);
        data[56..88].copy_from_slice(&[9; 32]);
        data[112..116].copy_from_slice(&8_u32.to_le_bytes());
        data[116..120].copy_from_slice(&TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP.to_le_bytes());
        data[120..152].copy_from_slice(&[11; 32]);
        data[154] = 3;
        data[155] = 4;
        data[156..188].copy_from_slice(&[13; 32]);
        data[192..200].copy_from_slice(&123_u64.to_le_bytes());
        data[TRADER_HEADER_LEN..TRADER_HEADER_LEN + 8].copy_from_slice(&2_u64.to_le_bytes());
        data[TRADER_HEADER_LEN + 8..TRADER_HEADER_LEN + 16].copy_from_slice(&8_u64.to_le_bytes());
        words
    }

    fn aligned_trader_account_data(len: u64, capacity: u64) -> Vec<u64> {
        let capacity_usize = usize::try_from(capacity).unwrap();
        let total_len = TRADER_POSITION_ENTRIES_START + capacity_usize * TRADER_POSITION_ENTRY_LEN;
        let mut words = vec![0u64; total_len.div_ceil(8)];
        let data = bytemuck::cast_slice_mut::<u64, u8>(&mut words);
        data[..8].copy_from_slice(&PhoenixAccount::Trader.discriminant());
        data[24..56].copy_from_slice(&[7; 32]);
        data[56..88].copy_from_slice(&[9; 32]);
        data[112..116].copy_from_slice(&(capacity as u32).to_le_bytes());
        data[116..120].copy_from_slice(&TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP.to_le_bytes());
        data[120..152].copy_from_slice(&[11; 32]);
        data[154] = 3;
        data[155] = 4;
        data[156..188].copy_from_slice(&[13; 32]);
        data[192..200].copy_from_slice(&123_u64.to_le_bytes());
        data[TRADER_HEADER_LEN..TRADER_HEADER_LEN + 8].copy_from_slice(&len.to_le_bytes());
        data[TRADER_HEADER_LEN + 8..TRADER_HEADER_LEN + 16]
            .copy_from_slice(&capacity.to_le_bytes());

        if capacity_usize > 0 {
            write_position_entry(data, 0, 42, -5, 10, 11, 9, 123);
        }
        if capacity_usize > 1 {
            write_position_entry(data, 1, 43, 0, 0, 0, 0, 0);
        }

        words
    }

    fn write_position_entry(
        data: &mut [u8],
        index: usize,
        asset_id: u64,
        base_lot_position: i64,
        virtual_quote_lot_position: i64,
        cumulative_funding_snapshot: i64,
        position_sequence_number: u8,
        accumulated_funding_for_active_position: i64,
    ) {
        let start = TRADER_POSITION_ENTRIES_START + index * TRADER_POSITION_ENTRY_LEN;
        data[start..start + 8].copy_from_slice(&asset_id.to_le_bytes());
        data[start + 8..start + 16].copy_from_slice(&base_lot_position.to_le_bytes());
        data[start + 16..start + 24].copy_from_slice(&virtual_quote_lot_position.to_le_bytes());
        data[start + 24..start + 32].copy_from_slice(&cumulative_funding_snapshot.to_le_bytes());
        data[start + 32] = position_sequence_number;
        data[start + 33..start + 40]
            .copy_from_slice(&accumulated_funding_for_active_position.to_le_bytes()[..7]);
    }

    #[test]
    fn trader_header_borrows_aligned_account_bytes() {
        let words = aligned_trader_header_data();
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let header = TraderHeader::try_from_account_bytes(data).unwrap();

        assert_eq!(header.key(), &[7; 32]);
        assert_eq!(header.authority(), &[9; 32]);
        assert_eq!(header.max_positions(), 8);
        assert!(header.disable_collateral_sweep());
        assert_eq!(header.position_authority(), &[11; 32]);
        assert_eq!(header.trader_pda_index(), 3);
        assert_eq!(header.trader_subaccount_index(), 4);
        assert_eq!(header.funding_key(), &[13; 32]);
        assert_eq!(header.last_deposit_slot(), 123);
    }

    #[test]
    fn trader_header_rejects_short_input() {
        let words = aligned_trader_header_data();
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let err = TraderHeader::try_from_account_bytes(&data[..TRADER_HEADER_LEN - 1]).unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::AccountTooSmall {
                account: TRADER_ACCOUNT,
                expected: TRADER_HEADER_LEN,
                actual: TRADER_HEADER_LEN - 1,
            }
        );
    }

    #[test]
    fn trader_header_rejects_misaligned_input_without_panicking() {
        let mut words = [0u64; (TRADER_HEADER_WITH_PREFIX_LEN / 8) + 1];
        let buffer = bytemuck::cast_slice_mut::<u64, u8>(&mut words);
        let data = &mut buffer[1..1 + TRADER_HEADER_WITH_PREFIX_LEN];
        data[..8].copy_from_slice(&PhoenixAccount::Trader.discriminant());

        let err = TraderHeader::try_from_account_bytes(data).unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::InvalidLayout {
                account: TRADER_ACCOUNT,
            }
        );
    }

    #[test]
    fn trader_borrows_header_and_allocated_position_entries() {
        let words = aligned_trader_account_data(1, 2);
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let trader = Trader::try_from_account_bytes(data).unwrap();
        let borrowed_trader = Trader::try_borrow_from_account_bytes(data).unwrap();

        assert_eq!(trader.header().authority(), &[9; 32]);
        assert_eq!(borrowed_trader.header().authority(), &[9; 32]);
        assert_eq!(trader.position_map_header().len(), 1);
        assert_eq!(trader.raw_len(), 1);
        assert_eq!(trader.len(), 1);
        assert_eq!(trader.capacity(), 2);
        assert!(trader.has_position_capacity());
        assert_eq!(trader.position_entries().count(), 1);
        assert_eq!(trader.allocated_position_entries().len(), 2);
        assert_eq!(
            trader.trader_preference_bits(),
            TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP
        );
        assert!(trader.disable_collateral_sweep());
        assert!(
            trader
                .preferences()
                .is_enabled(TraderPreferenceKind::DisableCollateralSweep)
        );

        let mut positions = trader.positions();
        let (asset_id, position) = positions.next().unwrap();
        assert_eq!(asset_id, 42);
        assert_eq!(position.base_lot_position(), SignedBaseLots::new(-5));
        assert_eq!(
            position.virtual_quote_lot_position(),
            SignedQuoteLots::new(10)
        );
        assert_eq!(
            position.cumulative_funding_snapshot(),
            SignedQuoteLotsPerBaseLot::new(11)
        );
        assert_eq!(position.position_sequence_number(), 9);
        assert_eq!(
            position.accumulated_funding_for_active_position(),
            SignedQuoteLots::new(123)
        );
        let math_position = position.to_math_position();
        assert_eq!(math_position.base_lot_position, SignedBaseLots::new(-5));
        assert_eq!(
            math_position.virtual_quote_lot_position,
            SignedQuoteLots::new(10)
        );
        assert_eq!(math_position.position_sequence_number.as_u8(), 9);
        assert_eq!(
            math_position
                .accumulated_funding_for_active_position
                .to_signed_quote_lots(),
            SignedQuoteLots::new(123)
        );
        assert!(positions.next().is_none());
    }

    #[test]
    fn trader_position_iterators_filter_non_position_entries_and_preserve_cold_positions() {
        let mut words = aligned_trader_account_data(3, 3);
        let data = bytemuck::cast_slice_mut::<u64, u8>(&mut words);

        write_position_entry(data, 0, 42, -5, 10, 11, 9, 123);
        write_position_entry(data, 1, (1_u64 << 32) | 44, 6, -12, 13, 10, -321);
        write_position_entry(data, 2, 0xFFFF_0000, 99, 99, 99, 99, 99);

        let data = bytemuck::cast_slice::<u64, u8>(&words);
        let trader = Trader::try_from_account_bytes(data).unwrap();
        assert_eq!(trader.raw_len(), 3);
        assert_eq!(trader.len(), 2);
        let ref_entries = trader.position_entries().collect::<Vec<_>>();
        assert_eq!(ref_entries.len(), 2);
        assert_eq!(ref_entries[0].asset_id(), 42);
        assert_eq!(ref_entries[1].raw_asset_id(), (1_u64 << 32) | 44);
        assert_eq!(ref_entries[1].asset_id(), 44);
        assert_eq!(
            trader
                .positions()
                .map(|(asset_id, _)| asset_id)
                .collect::<Vec<_>>(),
            vec![42, 44]
        );
    }

    #[test]
    fn spot_collaterals_read_native_sol_and_spl_entries_without_leaking_positions() {
        let mut words = aligned_trader_account_data(4, 4);
        let data = bytemuck::cast_slice_mut::<u64, u8>(&mut words);

        // Spot collateral values reuse the position slot:
        // `write_position_entry` writes the balance as
        // `base_lot_position` and the per-trader cap override as
        // `virtual_quote_lot_position`.
        write_position_entry(data, 0, 42, -5, 10, 11, 9, 123);
        write_position_entry(
            data,
            1,
            NATIVE_SOL_ASSET_INDEX as u64,
            1_234_567_890,
            5_000_000_000,
            0,
            0,
            0,
        );
        write_position_entry(data, 2, 0xFFFE_4001, 777, 0, 0, 0, 0);
        write_position_entry(data, 3, 43, 6, -12, 13, 10, -321);

        let data = bytemuck::cast_slice::<u64, u8>(&words);
        let expected = vec![
            TraderSpotCollateral {
                asset_index: NATIVE_SOL_ASSET_INDEX,
                balance: 1_234_567_890,
                max_per_trader_balance_override: Some(5_000_000_000),
            },
            TraderSpotCollateral {
                asset_index: 0xFFFE_4001,
                balance: 777,
                max_per_trader_balance_override: None,
            },
        ];

        let trader = Trader::try_from_account_bytes(data).unwrap();
        assert_eq!(trader.spot_collaterals().collect::<Vec<_>>(), expected);
        #[allow(deprecated)]
        let legacy_positions = TraderPositions::try_from_account_bytes(data)
            .unwrap()
            .iter()
            .map(|entry| entry.asset_id)
            .collect::<Vec<_>>();
        assert_eq!(legacy_positions, vec![42, 43]);
        assert_eq!(
            trader
                .positions()
                .map(|(asset_id, _)| asset_id)
                .collect::<Vec<_>>(),
            vec![42, 43]
        );
    }

    #[test]
    fn trader_rejects_missing_allocated_position_capacity() {
        let words = aligned_trader_account_data(1, 2);
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let err = Trader::try_from_account_bytes(&data[..TRADER_POSITION_ENTRIES_START + 40])
            .unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::AccountTooSmall {
                account: TRADER_ACCOUNT,
                expected: TRADER_POSITION_ENTRIES_START + 80,
                actual: TRADER_POSITION_ENTRIES_START + 40,
            }
        );
    }

    #[test]
    fn trader_rejects_misaligned_input_without_panicking() {
        let words = aligned_trader_account_data(1, 2);
        let data = bytemuck::cast_slice::<u64, u8>(&words);
        let mut buffer = vec![0u8; data.len() + 1];
        buffer[1..].copy_from_slice(data);

        let err = Trader::try_from_account_bytes(&buffer[1..]).unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::InvalidLayout {
                account: TRADER_ACCOUNT,
            }
        );
    }

    #[test]
    fn trader_rejects_position_len_above_capacity() {
        let words = aligned_trader_account_data(2, 1);
        let data = bytemuck::cast_slice::<u64, u8>(&words);

        let err = Trader::try_from_account_bytes(data).unwrap_err();

        assert_eq!(
            err,
            PhoenixAccountDecodeError::InvalidData {
                account: TRADER_ACCOUNT,
                reason: "position map length exceeds capacity",
            }
        );
    }
}
