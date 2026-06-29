use serde::Serialize;

use super::internal::{PerpAssetMetadata, SequenceNumber, ShortEntries};
use super::{AccountDeserialize, AccountDeserializeError};
use crate::perp_asset_map as borrowed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerpAssetMapOwned {
    pub sequence_number: SequenceNumber,
    pub num_assets: u16,
    pub metadata: ShortEntries<String, PerpAssetMetadata>,
}

#[deprecated(
    since = "0.1.13",
    note = "use PerpAssetMapOwned for the allocating owned view or \
            phoenix_rise::accounts::perp_asset_map::PerpAssetMap for the borrowed view"
)]
pub type PerpAssetMap = PerpAssetMapOwned;

impl AccountDeserialize for PerpAssetMapOwned {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        borrowed::PerpAssetMap::try_from_account_bytes(data)?.try_into()
    }
}

impl TryFrom<borrowed::PerpAssetMap<'_>> for PerpAssetMapOwned {
    type Error = AccountDeserializeError;

    fn try_from(value: borrowed::PerpAssetMap<'_>) -> Result<Self, Self::Error> {
        let mut entries = Vec::with_capacity(value.num_assets() as usize);
        for entry in value.iter() {
            let entry = entry?;
            entries.push((entry.symbol.as_str().to_owned(), entry.metadata.into()));
        }
        Ok(Self {
            sequence_number: value.sequence_number().into(),
            num_assets: value.num_assets(),
            metadata: ShortEntries {
                len: entries.len() as u64,
                capacity: value.capacity(),
                entries,
            },
        })
    }
}

impl PerpAssetMapOwned {
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
