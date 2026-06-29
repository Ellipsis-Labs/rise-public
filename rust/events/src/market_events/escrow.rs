use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

// Escrow events
////////////////////////////////////////////////////////////////////////////////////////////////

/// Escrow account was created for a trader.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscrowAccountCreatedEvent {
    pub authority: Pubkey,
    pub capacity: u64,
}

/// Escrow request was created.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscrowRequestCreatedEvent {
    pub receiver_authority: Pubkey,
    pub sender_authority: Pubkey,
    pub sequence_number: u64,
    pub sender_pda_index: u8,
    pub sender_subaccount_index: u8,
    pub receiver_pda_index: u8,
    pub receiver_subaccount_index: u8,
    pub expiration_offset: u32, // expiration_offset from initial_slot (0 = no expiration)
    pub initial_slot: u64,
    pub actions: [EscrowAction; 4],
}

/// Escrow request was accepted and actions were executed.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscrowRequestAcceptedEvent {
    pub receiver_authority: Pubkey,
    pub sequence_number: u64,
}

/// Reason why an escrow request was cancelled.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EscrowRequestCancelReason {
    /// Request expired (e.g. removed when receiver tried to accept after
    /// last_valid_slot).
    Expiration,
    /// Sender cancelled the request.
    CancelledBySender,
    /// Receiver cancelled the request.
    CancelledByReceiver,
}

/// Escrow request was cancelled.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscrowRequestCancelledEvent {
    pub receiver_authority: Pubkey,
    pub sequence_number: u64,
    pub reason: EscrowRequestCancelReason,
}
