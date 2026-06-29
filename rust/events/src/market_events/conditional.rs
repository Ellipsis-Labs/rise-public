use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Stop loss events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopLossPlacedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,

    // Stop loss parameters
    pub asset_id: u64,
    pub trigger_price: Ticks,
    pub execution_price: Ticks,
    pub trade_size: BaseLots,
    pub trade_side: Side,
    pub execution_direction: Direction,
    pub position_sequence_number: u8,
    pub place_slot: u64,
    pub funding_key: Pubkey,
    pub order_kind: StopLossOrderKind,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopLossCancelledEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub execution_direction: Direction,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopLossExecutedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub execution_direction: Direction,
    pub order_sequence_number: u64,
}

////////////////////////////////////////////////////////////////////////////////////////////////
// Conditional order trigger events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TriggerOrderPlacedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub conditional_order_index: u8,
    // Trigger parameters
    pub trigger_price: Ticks,
    pub execution_price: Ticks,
    pub trade_side: Side,
    pub trigger_direction: Direction,
    pub order_kind: StopLossOrderKind,
    // Sizing
    pub max_size: BaseLots,
    pub fillable_size: BaseLots,
    pub filled_size: BaseLots,
    pub use_percent: bool,
    pub percent: u8,
    pub position_sequence_number: u8,
    /// Attached parent FIFO order id (None if position-based)
    pub attached_order_id: OptionalFIFOOrderId,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TriggerOrderCancelledEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub conditional_order_index: u8,
    pub trigger_direction: Direction,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TriggerOrderExecutedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub conditional_order_index: u8,
    pub trigger_direction: Direction,
    pub order_sequence_number: u64,
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConditionalOrderPingStateSnapshot(pub u8);

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConditionalOrderPingSnapshot {
    pub state: ConditionalOrderPingStateSnapshot,
    pub attached_order_id: OptionalFIFOOrderId,
    pub max_size: BaseLots,
    pub fillable_size: BaseLots,
    pub filled_size: BaseLots,
    pub co_position_sequence_number: Option<u8>,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PingInvalidatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub conditional_order_index: u8,
    pub pre: ConditionalOrderPingSnapshot,
    pub post: ConditionalOrderPingSnapshot,
    pub current_position_sequence_number: u8,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PingActivatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_id: u64,
    pub conditional_order_index: u8,
    pub pre: ConditionalOrderPingSnapshot,
    pub post: ConditionalOrderPingSnapshot,
    pub book_order_remaining_base_lots: Option<BaseLots>,
}
