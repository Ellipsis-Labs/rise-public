//! Safe big integers for JSON serialization.
//!
//! JavaScript numbers are IEEE-754 doubles, so only integers in the range
//! `[-(2^53 - 1), 2^53 - 1]` round-trip without losing bits. Anything outside
//! this window is either rounded or rejected by browsers.
//!
//! This module provides wrapper types that serialize as strings and can
//! deserialize from either strings or numbers, ensuring safe round-tripping
//! with JavaScript clients.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{AddAssign, Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};

/// Wrapper for signed 64-bit values that must be JSON-safe for consumers
/// written in JavaScript/TypeScript.
///
/// Serializes as a string and can deserialize from either a string or number.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsSafeI64(#[serde_as(as = "PickFirst<(DisplayFromStr, _)>")] i64);

impl From<i64> for JsSafeI64 {
    fn from(value: i64) -> Self {
        JsSafeI64(value)
    }
}

impl From<JsSafeI64> for i64 {
    fn from(value: JsSafeI64) -> Self {
        value.0
    }
}

impl JsSafeI64 {
    pub fn into_inner(self) -> i64 {
        self.0
    }

    pub fn as_inner(&self) -> i64 {
        self.0
    }
}

impl Deref for JsSafeI64 {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JsSafeI64 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AddAssign for JsSafeI64 {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl PartialEq<i64> for JsSafeI64 {
    fn eq(&self, other: &i64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<JsSafeI64> for i64 {
    fn eq(&self, other: &JsSafeI64) -> bool {
        *self == other.0
    }
}

impl PartialOrd<i64> for JsSafeI64 {
    fn partial_cmp(&self, other: &i64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<JsSafeI64> for i64 {
    fn partial_cmp(&self, other: &JsSafeI64) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl fmt::Display for JsSafeI64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Wrapper for unsigned 64-bit values that must be JSON-safe for consumers
/// written in JavaScript/TypeScript.
///
/// Serializes as a string and can deserialize from either a string or number.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsSafeU64(#[serde_as(as = "PickFirst<(DisplayFromStr, _)>")] u64);

impl From<u64> for JsSafeU64 {
    fn from(value: u64) -> Self {
        JsSafeU64(value)
    }
}

impl From<JsSafeU64> for u64 {
    fn from(value: JsSafeU64) -> Self {
        value.0
    }
}

impl JsSafeU64 {
    pub fn into_inner(self) -> u64 {
        self.0
    }

    pub fn as_inner(&self) -> u64 {
        self.0
    }
}

impl Deref for JsSafeU64 {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JsSafeU64 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AddAssign for JsSafeU64 {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl PartialEq<u64> for JsSafeU64 {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<JsSafeU64> for u64 {
    fn eq(&self, other: &JsSafeU64) -> bool {
        *self == other.0
    }
}

impl PartialOrd<u64> for JsSafeU64 {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<JsSafeU64> for u64 {
    fn partial_cmp(&self, other: &JsSafeU64) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl fmt::Display for JsSafeU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_safe_i64_from_string() {
        let json = r#""-9007199254740993""#;
        let value: JsSafeI64 = serde_json::from_str(json).unwrap();
        assert_eq!(value.into_inner(), -9007199254740993_i64);
    }

    #[test]
    fn test_js_safe_i64_from_number() {
        let json = "-12345";
        let value: JsSafeI64 = serde_json::from_str(json).unwrap();
        assert_eq!(value.into_inner(), -12345);
    }

    #[test]
    fn test_js_safe_i64_serializes_as_string() {
        let value = JsSafeI64::from(-9007199254740993_i64);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#""-9007199254740993""#);
    }

    #[test]
    fn test_js_safe_u64_from_string() {
        let json = r#""18446744073709551615""#;
        let value: JsSafeU64 = serde_json::from_str(json).unwrap();
        assert_eq!(value.into_inner(), u64::MAX);
    }

    #[test]
    fn test_js_safe_u64_from_number() {
        let json = "12345";
        let value: JsSafeU64 = serde_json::from_str(json).unwrap();
        assert_eq!(value.into_inner(), 12345);
    }

    #[test]
    fn test_js_safe_u64_serializes_as_string() {
        let value = JsSafeU64::from(9007199254740993_u64);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#""9007199254740993""#);
    }
}
