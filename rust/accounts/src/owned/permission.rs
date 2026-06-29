use serde::Serialize;
use solana_pubkey::Pubkey;

use super::{AccountDeserialize, AccountDeserializeError};
use crate::permission as borrowed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Permission {
    pub permission_authority: Pubkey,
    pub delegated_key: Pubkey,
    pub permission: u64,
    pub expires_at_timestamp: i64,
    pub allowed_signer_actions: i64,
}

impl AccountDeserialize for Permission {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        Ok(borrowed::PermissionAccount::try_from_account_bytes(data)?.into())
    }
}

impl From<&borrowed::PermissionAccount> for Permission {
    fn from(value: &borrowed::PermissionAccount) -> Self {
        Self {
            permission_authority: Pubkey::new_from_array(value.authority()),
            delegated_key: Pubkey::new_from_array(value.user()),
            permission: value.permission_bits(),
            expires_at_timestamp: value.expires_at_timestamp(),
            allowed_signer_actions: value.num_signer_actions_remaining(),
        }
    }
}

impl From<borrowed::PermissionAccount> for Permission {
    fn from(value: borrowed::PermissionAccount) -> Self {
        Self::from(&value)
    }
}

impl Permission {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }

    pub fn num_signer_actions_remaining(&self) -> i64 {
        self.allowed_signer_actions
    }
}
