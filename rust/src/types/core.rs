//! Core primitive types for Phoenix API.
//!
//! These types are fundamental building blocks used across the SDK.

use serde::{Deserialize, Serialize};

/// A decimal type representing a fixed-precision number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decimal {
    pub value: i64,
    pub decimals: i8,
    pub ui: String,
}

impl Decimal {
    pub const ZERO: Decimal = Decimal {
        value: 0,
        decimals: 0,
        ui: String::new(),
    };

    pub fn from_i64_with_decimals(value: i64, decimals: i8) -> Self {
        let scale = 10f64.powi(decimals as i32);
        let ui = format!("{:.*}", decimals as usize, value as f64 / scale);
        Self {
            value,
            decimals,
            ui,
        }
    }
}

/// A price for an asset.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub price: f64,
    pub slot: u64,
}

/// Order side (Bid or Ask).
pub use crate::phoenix_rise_math::Side;

/// Generic paginated response wrapper with bidirectional cursor support.
///
/// The cursor system supports both forward (newer) and backward (older)
/// pagination:
/// - `prev_cursor`: Use this cursor to poll for new items (items newer than the
///   current result set)
/// - `next_cursor`: Use this cursor to load more items (items older than the
///   current result set)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    /// The data payload (array of items).
    pub data: T,
    /// Opaque cursor for fetching newer items (for polling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Opaque cursor for fetching the next page of older results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Whether there are more results available after this page.
    pub has_more: bool,
}
