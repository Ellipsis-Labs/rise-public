use solana_pubkey::Pubkey;

use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{
    Reader, SequenceNumber, Spline, read_sequence_number, read_spline, verify_discriminant,
};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "SplineCollection";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplineCollection {
    pub market: Pubkey,
    pub asset_symbol: String,
    pub sequence_number: SequenceNumber,
    pub num_splines: u32,
    pub num_active: u32,
    pub splines: Vec<Spline>,
}

impl AccountDeserialize for SplineCollection {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.spline_collection)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let market = reader.read_pubkey()?;
        let asset_symbol = reader.read_symbol()?;
        let sequence_number = read_sequence_number(&mut reader)?;
        let num_splines = reader.read_u32()?;
        let num_active = reader.read_u32()?;
        reader.skip(32)?;
        let mut splines = Vec::with_capacity(num_splines as usize);
        for _ in 0..num_splines {
            splines.push(read_spline(&mut reader)?);
        }
        Ok(Self {
            market,
            asset_symbol,
            sequence_number,
            num_splines,
            num_active,
            splines,
        })
    }
}

impl SplineCollection {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }

    pub fn active_splines(&self) -> impl Iterator<Item = &Spline> {
        self.splines.iter().filter(|spline| spline.is_active)
    }
}
