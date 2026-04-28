use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{
    MAX_NUMBER_OF_PERP_ASSETS, PerpAssetMetadata, Reader, SequenceNumber, ShortEntries,
    read_perp_asset_metadata, read_sequence_number, verify_discriminant,
};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "PerpAssetMap";
const PERP_ASSET_METADATA_BYTES: usize = 1568;
const PERP_ASSET_ENTRY_BYTES: usize = 16 + PERP_ASSET_METADATA_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerpAssetMap {
    pub sequence_number: SequenceNumber,
    pub num_assets: u16,
    pub metadata: ShortEntries<String, PerpAssetMetadata>,
}

impl AccountDeserialize for PerpAssetMap {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.perp_asset_map)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let sequence_number = read_sequence_number(&mut reader)?;
        let num_assets = reader.read_u16()?;
        reader.skip(6)?;
        let metadata = read_metadata_entries(&mut reader)?;
        Ok(Self {
            sequence_number,
            num_assets,
            metadata,
        })
    }
}

impl PerpAssetMap {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &PerpAssetMetadata)> {
        self.metadata
            .entries
            .iter()
            .map(|(symbol, metadata)| (symbol, metadata))
    }

    pub fn get_by_symbol(&self, symbol: &str) -> Option<&PerpAssetMetadata> {
        self.metadata
            .entries
            .iter()
            .find_map(|(entry_symbol, metadata)| (entry_symbol == symbol).then_some(metadata))
    }
}

fn read_metadata_entries(
    reader: &mut Reader<'_>,
) -> Result<ShortEntries<String, PerpAssetMetadata>, AccountDeserializeError> {
    let slots_used = reader.read_u32()? as usize;
    let tombstones = reader.read_u32()? as usize;
    let capacity = reader.read_u64()?;
    if slots_used > MAX_NUMBER_OF_PERP_ASSETS {
        return Err(AccountDeserializeError::invalid_data(
            ACCOUNT,
            format!("slots_used {slots_used} exceeds max {MAX_NUMBER_OF_PERP_ASSETS}"),
        ));
    }
    let expected_remaining = MAX_NUMBER_OF_PERP_ASSETS
        .checked_mul(PERP_ASSET_ENTRY_BYTES)
        .ok_or_else(|| AccountDeserializeError::invalid_data(ACCOUNT, "asset map size overflow"))?;
    if reader.remaining() < expected_remaining {
        return Err(AccountDeserializeError::too_short(
            ACCOUNT,
            reader.offset() + expected_remaining,
            reader.offset() + reader.remaining(),
        ));
    }

    let mut entries = Vec::with_capacity(slots_used.saturating_sub(tombstones));
    for _ in 0..slots_used {
        let symbol = reader.read_symbol()?;
        let (metadata, is_active) = read_perp_asset_metadata(reader)?;
        if is_active {
            entries.push((symbol, metadata));
        }
    }
    let unused_slots = MAX_NUMBER_OF_PERP_ASSETS - slots_used;
    reader.skip(unused_slots * PERP_ASSET_ENTRY_BYTES)?;
    Ok(ShortEntries {
        len: entries.len() as u64,
        capacity,
        entries,
    })
}
