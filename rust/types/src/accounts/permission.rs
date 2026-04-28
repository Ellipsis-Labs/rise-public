use solana_pubkey::Pubkey;

use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{Reader, verify_discriminant};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "Permission";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub permission_authority: Pubkey,
    pub delegated_key: Pubkey,
    pub permission: u64,
    pub expires_at_timestamp: i64,
    pub allowed_signer_actions: i64,
}

impl AccountDeserialize for Permission {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.permission_account)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let permission_authority = reader.read_pubkey()?;
        let delegated_key = reader.read_pubkey()?;
        reader.skip(8)?;
        let permission = reader.read_u64()?;
        let expires_at_timestamp = reader.read_i64()?;
        let allowed_signer_actions = reader.read_i64()?;
        reader.skip(64)?;
        Ok(Self {
            permission_authority,
            delegated_key,
            permission,
            expires_at_timestamp,
            allowed_signer_actions,
        })
    }
}

impl Permission {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
