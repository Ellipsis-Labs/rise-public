//! Direction and stop loss order types
//!
//! This module provides the Direction enum for price comparison directions
//! and StopLossOrderKind for stop loss order types.

use std::fmt::Display;

use borsh::{BorshDeserialize, BorshSerialize};

/// Direction for price comparisons (used in stop loss orders)
#[derive(Clone, Copy, BorshDeserialize, BorshSerialize, Debug, Eq, PartialEq, Hash)]
pub enum Direction {
    /// Greater than comparison
    GreaterThan,
    /// Less than comparison
    LessThan,
}

impl Direction {
    /// Get the opposite direction
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::GreaterThan => Direction::LessThan,
            Direction::LessThan => Direction::GreaterThan,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::GreaterThan => write!(f, "gt"),
            Direction::LessThan => write!(f, "lt"),
        }
    }
}

/// Stop loss order execution kind
#[derive(Clone, Copy, BorshDeserialize, BorshSerialize, Debug, Eq, PartialEq, Hash)]
pub enum StopLossOrderKind {
    /// Immediate-or-cancel order
    IOC,
    /// Limit order
    Limit,
}

impl Display for StopLossOrderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopLossOrderKind::IOC => write!(f, "ioc"),
            StopLossOrderKind::Limit => write!(f, "limit"),
        }
    }
}

/// Order side (Bid or Ask).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum Side {
    #[cfg_attr(feature = "serde", serde(alias = "Bid"))]
    Bid = 0,
    #[cfg_attr(feature = "serde", serde(alias = "Ask"))]
    Ask = 1,
}

impl Side {
    /// Returns the API wire string for this side (`"buy"` or `"sell"`).
    pub const fn to_api_string(self) -> &'static str {
        match self {
            Self::Bid => "buy",
            Self::Ask => "sell",
        }
    }

    /// Determine side from a FIFO order sequence number.
    #[inline(always)]
    pub const fn from_order_sequence_number(seq: u64) -> Self {
        if seq & (1 << 63) != 0 {
            Self::Bid
        } else {
            Self::Ask
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bid => write!(f, "bid"),
            Self::Ask => write!(f, "ask"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Side;

    #[test]
    fn side_borsh_discriminants_match_instruction_encoding() {
        assert_eq!(borsh::to_vec(&Side::Bid).unwrap(), [0]);
        assert_eq!(borsh::to_vec(&Side::Ask).unwrap(), [1]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn side_serializes_lowercase_for_api_wire_types() {
        assert_eq!(serde_json::to_string(&Side::Bid).unwrap(), "\"bid\"");
        assert_eq!(serde_json::to_string(&Side::Ask).unwrap(), "\"ask\"");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn side_accepts_legacy_pascal_case_strings() {
        assert_eq!(serde_json::from_str::<Side>("\"Bid\"").unwrap(), Side::Bid);
        assert_eq!(serde_json::from_str::<Side>("\"Ask\"").unwrap(), Side::Ask);
    }
}
