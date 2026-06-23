//! Direction and stop loss order types
//!
//! This module provides the Direction enum for price comparison directions
//! and StopLossOrderKind for stop loss order types.

use std::fmt::Display;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Bid,
    Ask,
}
