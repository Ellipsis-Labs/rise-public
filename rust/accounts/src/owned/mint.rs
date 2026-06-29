use serde::Serialize;
use solana_pubkey::Pubkey;

use super::internal::Reader;
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "Mint";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mint {
    pub mint_authority_option: u32,
    pub mint_authority: Pubkey,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority_option: u32,
    pub freeze_authority: Pubkey,
}

impl AccountDeserialize for Mint {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        let mut reader = Reader::new(ACCOUNT, data);
        Ok(Self {
            mint_authority_option: reader.read_u32()?,
            mint_authority: reader.read_pubkey()?,
            supply: reader.read_u64()?,
            decimals: reader.read_u8()?,
            is_initialized: reader.read_u8()? != 0,
            freeze_authority_option: reader.read_u32()?,
            freeze_authority: reader.read_pubkey()?,
        })
    }
}

impl Mint {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
