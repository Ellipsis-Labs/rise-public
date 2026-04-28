//! Core traits for type-safe numeric wrappers.
//!
//! This module defines the fundamental traits that enable type-safe arithmetic
//! and bounds checking for numeric wrapper types.

use std::ops::RangeInclusive;

/// Generic trait for creating newtype wrappers around numeric types.
///
/// This trait provides the foundation for type-safe arithmetic by wrapping
/// primitive numeric types (u64, i64, i128) in strongly-typed structs.
///
/// # Type Safety
///
/// By implementing this trait for different wrapper types, we ensure that
/// only compatible types can be used in arithmetic operations, preventing
/// dimensional errors at compile time.
///
/// # Example
///
/// ```ignore
/// struct Price { inner: u64 }
/// impl WrapperNum<u64> for Price {
///     type Inner = u64;
///     fn new(value: u64) -> Self { Price { inner: value } }
///     fn as_inner(&self) -> u64 { self.inner }
/// }
/// ```
pub trait WrapperNum<T> {
    type Inner;
    fn new(value: T) -> Self;
    fn as_inner(&self) -> T;
}

/// Trait for enforcing value bounds on numeric wrapper types.
///
/// This trait enables compile-time and runtime bounds checking for types
/// that need to restrict their valid value ranges. This is crucial for
/// preventing overflow errors and ensuring data integrity.
///
/// # Overflow Prevention
///
/// By defining upper bounds smaller than the underlying type's maximum,
/// we can catch potential overflows before they occur:
///
/// ```ignore
/// // BaseLots is limited to u32::MAX even though it wraps u64
/// impl ScalarBounds<u64> for BaseLots {
///     const LOWER_BOUND: u64 = 0;
///     const UPPER_BOUND: u64 = u32::MAX as u64;
/// }
///
/// let valid = BaseLots::new(1000);
/// assert!(valid.is_in_bounds());
///
/// let too_large = BaseLots::new(u64::MAX);
/// assert!(!too_large.is_in_bounds()); // Caught before overflow!
/// ```
pub trait ScalarBounds<Inner>: WrapperNum<Inner>
where
    Inner: PartialOrd,
{
    const LOWER_BOUND: Inner;
    const UPPER_BOUND: Inner;

    /// Checks if the current value is within the defined bounds.
    ///
    /// This should be called after arithmetic operations to ensure
    /// the result hasn't exceeded the type's valid range.
    fn is_in_bounds(&self) -> bool {
        self.as_inner() >= Self::LOWER_BOUND && self.as_inner() <= Self::UPPER_BOUND
    }

    fn bounds() -> RangeInclusive<Inner> {
        Self::LOWER_BOUND..=Self::UPPER_BOUND
    }

    fn lower_bound() -> Inner {
        Self::LOWER_BOUND
    }

    fn upper_bound() -> Inner {
        Self::UPPER_BOUND
    }
}
