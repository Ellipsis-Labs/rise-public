use solana_pubkey::Pubkey;

use super::internal::Reader;
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "TokenAccount";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenAccountState {
    Uninitialized = 0,
    Initialized   = 1,
    Frozen        = 2,
}

impl TokenAccountState {
    fn from_u8(value: u8) -> Result<Self, AccountDeserializeError> {
        match value {
            0 => Ok(Self::Uninitialized),
            1 => Ok(Self::Initialized),
            2 => Ok(Self::Frozen),
            _ => Err(AccountDeserializeError::invalid_data(
                ACCOUNT,
                format!("invalid token account state {value}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate_option: u32,
    pub delegate: Pubkey,
    pub state: TokenAccountState,
    pub is_native_option: u32,
    pub is_native: u64,
    pub delegated_amount: u64,
    pub close_authority_option: u32,
    pub close_authority: Pubkey,
}

impl AccountDeserialize for TokenAccount {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        let mut reader = Reader::new(ACCOUNT, data);
        Ok(Self {
            mint: reader.read_pubkey()?,
            owner: reader.read_pubkey()?,
            amount: reader.read_u64()?,
            delegate_option: reader.read_u32()?,
            delegate: reader.read_pubkey()?,
            state: TokenAccountState::from_u8(reader.read_u8()?)?,
            is_native_option: reader.read_u32()?,
            is_native: reader.read_u64()?,
            delegated_amount: reader.read_u64()?,
            close_authority_option: reader.read_u32()?,
            close_authority: reader.read_pubkey()?,
        })
    }
}

impl TokenAccount {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
