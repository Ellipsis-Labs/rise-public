//! Core primitive types for Phoenix API.
//!
//! These types are fundamental building blocks used across the SDK.

pub use phoenix_rise_math::Side;
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
        let sign = if value < 0 { "-" } else { "" };
        let digits = value.unsigned_abs().to_string();
        let places = decimals.unsigned_abs() as usize;
        let ui = if decimals <= 0 {
            let zeros = if value == 0 { 0 } else { places };
            format!("{sign}{digits}{}", "0".repeat(zeros))
        } else {
            let padded = format!("{digits:0>width$}", width = places + 1);
            let (whole, fraction) = padded.split_at(padded.len() - places);
            format!("{sign}{whole}.{fraction}")
        };
        Self {
            value,
            decimals,
            ui,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Decimal;

    #[test]
    fn from_i64_with_decimals_formats_ui() {
        let ui = |value, decimals| Decimal::from_i64_with_decimals(value, decimals).ui;
        assert_eq!(ui(0, 6), "0.000000");
        assert_eq!(ui(1_234_567, 6), "1.234567");
        assert_eq!(ui(-5, 1), "-0.5");
        assert_eq!(ui(-1, 0), "-1");
        assert_eq!(ui(12, -2), "1200");
        assert_eq!(ui(0, -2), "0");
        assert_eq!(ui(i64::MAX, 6), "9223372036854.775807");
        assert_eq!(ui(i64::MIN, 6), "-9223372036854.775808");
    }
}

/// A price for an asset.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub price: f64,
    pub slot: u64,
}

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
