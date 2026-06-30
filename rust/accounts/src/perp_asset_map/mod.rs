//! Borrowed views for Phoenix perp asset map accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

use self::metadata::PerpAssetMetadataLayout;
use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_pod, read_prefix, require_len,
    verify_discriminant,
};
use super::discriminants::PhoenixAccount;

mod metadata;
mod price;
mod symbol;

pub use metadata::{
    AssetFlags, FundingAccumulator, LeverageTier, OpenInterestParams, PerpAssetMetadata,
    PerpAssetMetadataEntry, RiskParams, StaticMarketParams, TransferFeeTier,
};
pub use price::{
    BookPriceComponent, MarkPrice, OracleData, OracleParameters, PerpPriceComponent,
    PriceComponent, SpotPriceComponent, TicksAtSlot,
};
pub use symbol::AssetSymbol;

pub(super) const PERP_ASSET_MAP_ACCOUNT: &str = "PerpAssetMap";
const MAX_NUMBER_OF_PERP_ASSETS: usize = 1024;
pub(super) const MAX_ORACLES: usize = 5;

const PERP_ASSET_MAP_HEADER_LEN: usize = core::mem::size_of::<PerpAssetMapHeader>();
const PERP_ASSET_MAP_ENTRY_LEN: usize = core::mem::size_of::<PerpAssetMapEntry>();
const PERP_ASSET_MAP_LEN: usize =
    PERP_ASSET_MAP_HEADER_LEN + MAX_NUMBER_OF_PERP_ASSETS * PERP_ASSET_MAP_ENTRY_LEN;

const_assert_eq!(core::mem::size_of::<StableIndexedShortMapHeader>(), 16);
const_assert_eq!(core::mem::size_of::<PerpAssetMapHeader>(), 48);
const_assert_eq!(core::mem::size_of::<PerpAssetMapEntry>(), 1584);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
struct StableIndexedShortMapHeader {
    slots_used: u32,
    tombstones: u32,
    capacity: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
struct PerpAssetMapHeader {
    discriminant: u64,
    sequence_number: SequenceNumber,
    num_assets: u16,
    _padding0: [u8; 6],
    metadata_header: StableIndexedShortMapHeader,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
struct PerpAssetMapEntry {
    symbol: [u8; 16],
    metadata: PerpAssetMetadataLayout,
}

/// Borrowed view over a Phoenix PerpAssetMap account.
///
/// This is the main account view for market metadata. It exposes active and
/// tombstoned assets, mark price inputs, funding accumulators, risk parameters,
/// open-interest caps, and static market params without allocating owned
/// metadata for every slot.
#[derive(Clone, Copy, Debug)]
pub struct PerpAssetMap<'a> {
    data: &'a [u8],
    sequence_number: SequenceNumber,
    num_assets: u16,
    slots_used: u32,
    tombstones: u32,
    capacity: u64,
}

impl<'a> PerpAssetMap<'a> {
    /// Decode a PerpAssetMap from raw account data.
    ///
    /// The returned view borrows `data`, verifies the account discriminant and
    /// fixed account size, and reads layout fields with unaligned-safe copies.
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(PERP_ASSET_MAP_ACCOUNT, data, PERP_ASSET_MAP_LEN)?;
        verify_discriminant(
            PERP_ASSET_MAP_ACCOUNT,
            data,
            PhoenixAccount::PerpAssetMap.discriminant(),
        )?;
        let value = read_prefix::<PerpAssetMapHeader>(PERP_ASSET_MAP_ACCOUNT, data)?;
        let slots_used = value.metadata_header.slots_used;
        let tombstones = value.metadata_header.tombstones;
        let capacity = value.metadata_header.capacity;
        if slots_used as usize > MAX_NUMBER_OF_PERP_ASSETS {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: PERP_ASSET_MAP_ACCOUNT,
                reason: "metadata slots_used exceeds capacity",
            });
        }
        if tombstones > slots_used {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: PERP_ASSET_MAP_ACCOUNT,
                reason: "metadata tombstones exceeds slots_used",
            });
        }
        Ok(Self {
            data,
            sequence_number: value.sequence_number,
            num_assets: value.num_assets,
            slots_used,
            tombstones,
            capacity,
        })
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn num_assets(&self) -> u16 {
        self.num_assets
    }

    #[inline(always)]
    pub const fn slots_used(&self) -> u32 {
        self.slots_used
    }

    #[inline(always)]
    pub const fn tombstones(&self) -> u32 {
        self.tombstones
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Iterate active metadata slots in account order.
    ///
    /// Tombstoned slots are skipped. Use [`PerpAssetMetadata::map_index`] on
    /// returned entries when callers need stable map indexes.
    #[inline(always)]
    pub fn iter(&self) -> PerpAssetMapIter<'a> {
        let entries_start = PERP_ASSET_MAP_HEADER_LEN;
        let entries_len = self.slots_used as usize * PERP_ASSET_MAP_ENTRY_LEN;
        PerpAssetMapIter {
            remaining: &self.data[entries_start..entries_start + entries_len],
        }
    }

    /// Find the first metadata entry whose symbol bytes match `symbol`.
    ///
    /// Tombstoned entries are skipped.
    pub fn find_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<PerpAssetMetadataEntry>, PhoenixAccountDecodeError> {
        for entry in self.iter() {
            let entry = entry?;
            if entry.symbol.matches(symbol) {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }
}

pub struct PerpAssetMapIter<'a> {
    remaining: &'a [u8],
}

impl<'a> Iterator for PerpAssetMapIter<'a> {
    type Item = Result<PerpAssetMetadataEntry, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.remaining.is_empty() {
            if self.remaining.len() < PERP_ASSET_MAP_ENTRY_LEN {
                let actual = self.remaining.len();
                self.remaining = &[];
                return Some(Err(PhoenixAccountDecodeError::AccountTooSmall {
                    account: PERP_ASSET_MAP_ACCOUNT,
                    expected: PERP_ASSET_MAP_ENTRY_LEN,
                    actual,
                }));
            }

            let (entry_bytes, remaining) = self.remaining.split_at(PERP_ASSET_MAP_ENTRY_LEN);
            self.remaining = remaining;

            let value = match read_pod::<PerpAssetMapEntry>(PERP_ASSET_MAP_ACCOUNT, entry_bytes) {
                Ok(value) => value,
                Err(_) => {
                    return Some(Err(PhoenixAccountDecodeError::InvalidLayout {
                        account: PERP_ASSET_MAP_ACCOUNT,
                    }));
                }
            };
            let symbol = match AssetSymbol::try_from_bytes(value.symbol) {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            let metadata = PerpAssetMetadata::new(value.metadata);
            if metadata.is_active() {
                return Some(Ok(PerpAssetMetadataEntry { symbol, metadata }));
            }
        }
        None
    }
}

#[cfg(feature = "serde")]
struct PerpAssetMapEntries<'a>(&'a PerpAssetMap<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for PerpAssetMapEntries<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.num_assets() as usize))?;
        for entry in self.0.iter() {
            let entry = entry.map_err(serde::ser::Error::custom)?;
            seq.serialize_element(&entry)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PerpAssetMap<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PerpAssetMap", 7)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("num_assets", &self.num_assets())?;
        state.serialize_field("slots_used", &self.slots_used())?;
        state.serialize_field("tombstones", &self.tombstones())?;
        state.serialize_field("capacity", &self.capacity())?;
        state.serialize_field("entries", &PerpAssetMapEntries(self))?;
        state.serialize_field("active_entries", &self.num_assets())?;
        state.end()
    }
}
