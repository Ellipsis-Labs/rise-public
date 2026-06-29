use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Authority events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NameSuccessorEvent {
    pub authority: Pubkey,
    pub new_authority: Pubkey,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClaimAuthorityEvent {
    pub previous_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AuthorityChangedEvent {
    pub previous_authority: Pubkey,
    pub new_authority: Pubkey,
    pub authority_type: AuthorityType,
}
