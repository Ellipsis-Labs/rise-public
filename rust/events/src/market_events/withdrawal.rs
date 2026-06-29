use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

/// Event emitted when a withdrawal request transitions between states
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawStateTransitionEvent {
    /// The trader requesting the withdrawal
    pub trader: Pubkey,
    /// The amount being withdrawn
    pub amount: QuoteLots,
    /// The state before the transition (as u8)
    pub from_state: u8,
    /// The state after the transition (as u8)
    pub to_state: u8,
    /// The reason for the transition (as u8)
    pub reason: u8,
    /// Number of state transitions this request has gone through
    pub transition_count: u16,
    /// Queue node index if applicable
    pub node_index: NodePointer,
}
