use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Spline events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineRegisteredEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub market: Pubkey,
    pub symbol: Symbol,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineActivatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
    pub mid_price: u64,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineDeactivatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
}

/// Per-side position-size limits for a spline, in base lots.
#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PositionSizeLimits {
    pub long: u32,
    pub short: u32,
}

impl PositionSizeLimits {
    pub fn symmetric(size: u32) -> Self {
        Self {
            long: size,
            short: size,
        }
    }
}

/// Describes whether a position-size limit is active on a spline.
/// `Disabled` means no cap; `Limit(limits)` caps the position per-side
/// (`Limit(PositionSizeLimits { long: 0, short: 0 })` is reduce-only mode).
#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum PositionSizeLimit {
    Disabled,
    Limit(PositionSizeLimits),
}

impl PositionSizeLimit {
    /// Build a `PositionSizeLimit` from the per-side `Option<BaseLots>` stored
    /// on a `Spline`. If both sides are `Some`, produces `Limit`; otherwise
    /// `Disabled`.
    /// Build a `PositionSizeLimit` from the per-side `Option<BaseLots>` stored
    /// on a `Spline`. Both sides must agree: either both `Some` (→ `Limit`)
    /// or both `None` (→ `Disabled`). A mismatch is a fatal invariant
    /// violation.
    pub fn from_spline_state(
        max_long: Option<BaseLots>,
        max_short: Option<BaseLots>,
    ) -> Result<Self, PositionSizeLimitError> {
        match (max_long, max_short) {
            (Some(long), Some(short)) => Ok(PositionSizeLimit::Limit(PositionSizeLimits {
                long: long.as_inner() as u32,
                short: short.as_inner() as u32,
            })),
            (None, None) => Ok(PositionSizeLimit::Disabled),
            _ => Err(PositionSizeLimitError::MismatchedLongShortLimits),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PositionSizeLimitError {
    #[error("max_position_size long/short must both be Some or both be None")]
    MismatchedLongShortLimits,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplinePositionLimitsConfigUpdatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
    /// `None` = not updated. `Some(limit)` = updated to the given limit.
    pub max_position_size: Option<PositionSizeLimit>,
    /// Previous position size limits before this update.
    pub prev_max_position_size: PositionSizeLimit,
    /// `None` = not updated. `Some(v)` = set leverage decrease to `v` bps.
    pub leverage_decrease_in_bps: Option<u32>,
    /// Previous leverage decrease before this update.
    pub prev_leverage_decrease_in_bps: u32,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplinePriceUpdatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
    pub price_in_ticks: u64,
    pub user_update_slot: u64,
    pub refresh_regions: bool,
}

#[deprecated(note = "This event is no longer emitted and should not be used by indexers")]
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineParametersUpdatedEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
    pub mid_price: u64,
    pub bid_regions: [TickRegion; 10],
    pub ask_regions: [TickRegion; 10],
}

/// Spline price update event with anti-reordering protection.
/// Includes `user_sequence_number` and `client_order_id` fields.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplinePriceUpdatedWithOrderingEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
    pub price_in_ticks: u64,
    pub user_update_slot: u64,
    pub refresh_regions: bool,
    /// User-provided sequence number for anti-reordering protection.
    pub user_sequence_number: u64,
    /// Client order id for tracking across exchanges.
    pub client_order_id: [u8; 16],
}

/// Spline parameter update event with anti-reordering protection.
/// Includes `user_sequence_number` and `client_order_id` fields.
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineParametersUpdatedWithOrderingEvent {
    pub trader: Pubkey,
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub authority: Pubkey,
    pub market: Pubkey,
    pub symbol: Symbol,
    pub mid_price: u64,
    pub bid_regions: [TickRegion; 10],
    pub ask_regions: [TickRegion; 10],
    /// User-provided sequence number for anti-reordering protection.
    pub user_sequence_number: u64,
    /// Client order id for tracking across exchanges.
    pub client_order_id: [u8; 16],
}
