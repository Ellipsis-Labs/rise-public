//! Type-safe quantity system for preventing arithmetic errors and overflows.
//!
//! This module provides newtype wrappers around primitive numeric types (u64,
//! i64, i128) to enforce dimensional correctness and prevent common arithmetic
//! errors at compile time.
//!
//! # Benefits
//!
//! - **Type Safety**: Prevents mixing incompatible units (e.g., can't add
//!   BaseLots to QuoteLots)
//! - **Overflow Protection**: Provides checked arithmetic operations with
//!   explicit error handling
//! - **Bounds Enforcement**: Certain types enforce value ranges (e.g., BaseLots
//!   limited to u32::MAX)
//! - **Zero-cost Abstraction**: All types are #[repr(transparent)] with no
//!   runtime overhead
//!
//! # Example Usage
//!
//! ```ignore
//! use phoenix_rise_math::quantities::{BaseLots, QuoteLots, Ticks};
//!
//! // Type system prevents incorrect operations
//! let base = BaseLots::new(100);
//! let quote = QuoteLots::new(1000);
//! // let invalid = base + quote; // Compile error!
//!
//! // Safe arithmetic with overflow handling
//! let a = BaseLots::new(u32::MAX as u64);
//! let b = BaseLots::new(1);
//! let sum = a.checked_add(b); // Returns None on overflow
//! let saturated = a.saturating_add(b); // Saturates at MAX
//!
//! // Bounds checking
//! let large = BaseLots::new(u64::MAX);
//! assert!(!large.is_in_bounds()); // BaseLots limited to u32::MAX
//! ```
//!
//! # Architecture
//!
//! The module is organized into three sub-modules:
//! - `traits`: Core traits for wrapper types and bounds checking
//! - `macros`: Macro definitions for generating type-safe wrapper structs
//! - `types`: Concrete type definitions for various quantity types

// Re-export the macros at the crate level
#[macro_use]
mod macros;

pub mod errors;
#[cfg(feature = "serde")]
pub(crate) mod serde_numeric;
pub mod traits;
pub mod types;

// Re-export commonly used items at the module level
pub use errors::MathError;
pub use traits::{ScalarBounds, WrapperNum};
pub use types::{
    BaseLots, BaseLotsPerBaseUnit, BaseLotsPerTick, BaseUnits, BasisPoints, BasisPointsU32,
    Constant, FeeRateMicro, FundingRateUnitInSeconds, Lamports, MicroDivisor, QuoteLots,
    QuoteLotsPerBaseLot, QuoteLotsPerBaseLotPerTick, QuoteLotsPerQuoteUnit, QuoteUnits,
    SequenceNumberU8, SignedBaseLots, SignedBaseLotsUpcasted, SignedConstant, SignedFeeRateMicro,
    SignedQuoteLots, SignedQuoteLotsBaseLots, SignedQuoteLotsBaseLotsUpcasted, SignedQuoteLotsI56,
    SignedQuoteLotsI56Error, SignedQuoteLotsPerBaseLot, SignedQuoteLotsPerBaseLotUpcasted,
    SignedQuoteLotsUpcasted, SignedTicks, Slot, Ticks, UPnlRiskFactor,
};

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_new_constructor_macro() {
        let base_lots_1 = BaseLots::new(5);
        let base_lots_2 = BaseLots::new(10);

        assert_eq!(base_lots_1 + base_lots_2, BaseLots::new(15));

        // Below code (correctly) fails to compile.
        // let quote_lots_1 = QuoteLots::new(5);
        // let result = quote_lots_1 + base_lots_1;
    }

    #[test]
    fn test_multiply_macro() {
        let base_units = BaseUnits::new(5);
        let base_lots_per_base_unit = BaseLotsPerBaseUnit::new(100);
        assert_eq!(base_units * base_lots_per_base_unit, BaseLots::new(500));

        // Below code (correctly) fails to compile.
        // let quote_units = QuoteUnits::new(5);
        // let result = quote_units * base_lots_per_base_unit;
    }
    #[test]
    fn test_bounds_range_quote_lots() {
        let bounds = QuoteLots::bounds();
        assert_eq!(*bounds.start(), 0);
        assert_eq!(*bounds.end(), u64::MAX);
        assert_eq!(QuoteLots::lower_bound(), 0);
        assert_eq!(QuoteLots::upper_bound(), u64::MAX);
    }

    #[test]
    fn test_bounds_range_base_lots() {
        let bounds = BaseLots::bounds();
        assert_eq!(*bounds.start(), 0);
        assert_eq!(*bounds.end(), u32::MAX as u64);
        assert_eq!(BaseLots::lower_bound(), 0);
        assert_eq!(BaseLots::upper_bound(), u32::MAX as u64);
    }

    #[test]
    fn test_bounds_range_ticks() {
        let bounds = Ticks::bounds();
        assert_eq!(*bounds.start(), 0);
        assert_eq!(*bounds.end(), u32::MAX as u64);
        assert_eq!(Ticks::lower_bound(), 0);
        assert_eq!(Ticks::upper_bound(), u32::MAX as u64);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn quantity_wrappers_serialize_as_json_strings() {
        assert_eq!(
            serde_json::to_string(&QuoteLots::new(9_007_199_254_740_993)).unwrap(),
            r#""9007199254740993""#
        );
        assert_eq!(
            serde_json::to_string(&SignedQuoteLotsPerBaseLotUpcasted::new(
                -9_007_199_254_740_993
            ))
            .unwrap(),
            r#""-9007199254740993""#
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn quantity_wrappers_deserialize_from_strings_or_numbers() {
        let from_string: QuoteLots = serde_json::from_str(r#""18446744073709551615""#).unwrap();
        assert_eq!(from_string, QuoteLots::new(u64::MAX));

        let from_number: SignedBaseLots = serde_json::from_str("-12345").unwrap();
        assert_eq!(from_number, SignedBaseLots::new(-12345));
    }

    proptest! {
        #[test]
        fn test_quote_lots_is_in_bounds(value in 0..=u32::MAX as u64) {
            let quote_lots = QuoteLots::new(value);
            prop_assert!(quote_lots.is_in_bounds());
            prop_assert_eq!(quote_lots.as_inner(), value);
        }

        #[test]
        fn test_base_lots_is_in_bounds(value in 0..=u32::MAX as u64) {
            let base_lots = BaseLots::new(value);
            prop_assert!(base_lots.is_in_bounds());
            prop_assert_eq!(base_lots.as_inner(), value);
        }

        #[test]
        fn test_base_lots_out_of_bounds(value in (u32::MAX as u64 + 1)..=u64::MAX) {
            let base_lots = BaseLots::new(value);
            prop_assert!(!base_lots.is_in_bounds());
        }

        #[test]
        fn test_ticks_is_in_bounds(value in 0..=u32::MAX as u64) {
            let ticks = Ticks::new(value);
            prop_assert!(ticks.is_in_bounds());
            prop_assert_eq!(ticks.as_inner(), value);
        }

        #[test]
        fn test_ticks_out_of_bounds(value in (u32::MAX as u64 + 1)..=u64::MAX) {
            let ticks = Ticks::new(value);
            prop_assert!(!ticks.is_in_bounds());
        }



        #[test]
        fn test_arithmetic_preserves_bounds_add(a in 0..=u16::MAX as u64, b in 0..=u16::MAX as u64) {
            let quote_lots_a = QuoteLots::new(a);
            let quote_lots_b = QuoteLots::new(b);
            let sum = quote_lots_a + quote_lots_b;

            // Sum of two values each <= u16::MAX should be <= u32::MAX
            prop_assert!(sum.is_in_bounds());
            prop_assert_eq!(sum.as_inner(), a + b);
        }

        #[test]
        fn test_arithmetic_preserves_bounds_sub(a in 0..=u32::MAX as u64, b in 0..=u32::MAX as u64) {
            // Skip test if b > a since we can't subtract in that case
            prop_assume!(b <= a);

            let base_lots_a = BaseLots::new(a);
            let base_lots_b = BaseLots::new(b);
            let diff = base_lots_a - base_lots_b;

            prop_assert!(diff.is_in_bounds());
            prop_assert_eq!(diff.as_inner(), a - b);
        }

        #[test]
        fn test_saturating_sub_preserves_bounds(a in 0..=u32::MAX as u64, b in 0..=u32::MAX as u64) {
            let base_lots_a = BaseLots::new(a);
            let base_lots_b = BaseLots::new(b);
            let result = base_lots_a.saturating_sub(base_lots_b);

            prop_assert!(result.is_in_bounds());
            prop_assert_eq!(result.as_inner(), a.saturating_sub(b));
        }

        #[test]
        fn test_multiply_preserves_bounds(ticks in 0..=100u64, base_lots_per_tick in 0..=1000u64) {
            let t = Ticks::new(ticks);
            let blpt = BaseLotsPerTick::new(base_lots_per_tick);

            // Only test if the result would be in bounds
            if ticks * base_lots_per_tick <= u32::MAX as u64 {
                let result = blpt * t;
                prop_assert!(result.is_in_bounds());
                prop_assert_eq!(result.as_inner(), ticks * base_lots_per_tick);
            }
        }
    }

    #[test]
    fn test_checked_division() {
        let a = BaseLots::new(100);
        let b = BaseLots::new(10);
        let zero = BaseLots::new(0);

        // Normal division works
        assert_eq!(a.checked_div(b), Some(BaseLots::new(10)));

        // Division by zero returns None
        assert_eq!(a.checked_div(zero), None);
    }

    #[test]
    fn test_div_ceil_correct() {
        let a = BaseLots::new(10);
        let b = BaseLots::new(3);
        let c = BaseLots::new(12);

        // 10 / 3 = 3.33... -> 4
        assert_eq!(a.div_ceil(b), BaseLots::new(4));

        // 12 / 3 = 4 exactly -> 4 (not 5!)
        assert_eq!(c.div_ceil(b), BaseLots::new(4));

        // Test checked version
        assert_eq!(a.checked_div_ceil(b), Some(BaseLots::new(4)));
        assert_eq!(c.checked_div_ceil(b), Some(BaseLots::new(4)));
        assert_eq!(a.checked_div_ceil(BaseLots::new(0)), None);
    }

    #[test]
    fn test_checked_add_signed() {
        let unsigned = QuoteLots::new(100);
        let positive = SignedQuoteLots::new(50);
        let negative = SignedQuoteLots::new(-30);
        let large_negative = SignedQuoteLots::new(-150);

        // Adding positive works
        assert_eq!(
            unsigned.checked_add_signed(positive),
            Some(QuoteLots::new(150))
        );

        // Subtracting smaller negative works
        assert_eq!(
            unsigned.checked_add_signed(negative),
            Some(QuoteLots::new(70))
        );

        // Subtracting larger negative returns None (underflow)
        assert_eq!(unsigned.checked_add_signed(large_negative), None);
    }

    #[test]
    fn test_saturating_add_signed() {
        let unsigned = QuoteLots::new(100);
        let positive = SignedQuoteLots::new(50);
        let negative = SignedQuoteLots::new(-30);
        let large_negative = SignedQuoteLots::new(-150);

        // Adding positive works
        assert_eq!(
            unsigned.saturating_add_signed(positive),
            QuoteLots::new(150)
        );

        // Subtracting smaller negative works
        assert_eq!(unsigned.saturating_add_signed(negative), QuoteLots::new(70));

        // Subtracting larger negative saturates at 0
        assert_eq!(
            unsigned.saturating_add_signed(large_negative),
            QuoteLots::new(0)
        );
    }

    #[test]
    fn test_overflow_safe_arithmetic() {
        // In debug mode, overflow detection is enabled via debug_assert
        // In release mode, operations saturate

        // Test checked methods always work
        let max = QuoteLots::new(u64::MAX);
        let one = QuoteLots::new(1);

        assert_eq!(max.checked_add(one), None);
        assert_eq!(max.saturating_add(one), max);

        let zero = QuoteLots::new(0);
        assert_eq!(zero.checked_sub(one), None);
        assert_eq!(zero.saturating_sub(one), zero);

        // Test wrapping methods for when wrapping is desired
        assert_eq!(max.wrapping_add(one).as_inner(), 0);
        assert_eq!(zero.wrapping_sub(one).as_inner(), u64::MAX);

        // Test normal range operations still work
        let mid = QuoteLots::new(1000);
        let small = QuoteLots::new(100);
        assert_eq!((mid + small).as_inner(), 1100);
        assert_eq!((mid - small).as_inner(), 900);
    }

    #[test]
    fn test_checked_constructors() {
        // Test new_checked for bounded types
        assert_eq!(BaseLots::new_checked(100), Ok(BaseLots::new(100)));
        assert_eq!(
            BaseLots::new_checked(u32::MAX as u64),
            Ok(BaseLots::new(u32::MAX as u64))
        );
        assert!(BaseLots::new_checked(u64::MAX).is_err());

        // Test new_saturating for bounded types
        assert_eq!(BaseLots::new_saturating(100).as_inner(), 100);
        assert_eq!(
            BaseLots::new_saturating(u64::MAX).as_inner(),
            u32::MAX as u64
        );
        assert_eq!(Ticks::new_saturating(u64::MAX).as_inner(), u32::MAX as u64);

        // Test that QuoteLots with u64::MAX bounds doesn't have the issue
        assert_eq!(
            QuoteLots::new_checked(u64::MAX),
            Ok(QuoteLots::new(u64::MAX))
        );
    }

    #[test]
    fn test_checked_div_by_types() {
        let tick_size = QuoteLotsPerBaseLotPerTick::new(5);
        let ticks = Ticks::new(20);

        // QuoteLotsPerBaseLotPerTick * Ticks = QuoteLotsPerBaseLot
        let price = tick_size * ticks;

        // Test the auto-generated checked division methods
        // The macro generates snake_case method names from the type names
        assert_eq!(
            price.checked_div_by_quote_lots_per_base_lot_per_tick(tick_size),
            Some(ticks)
        );
        assert_eq!(price.checked_div_by_ticks(ticks), Some(tick_size));

        // Test division by zero
        assert_eq!(
            price.checked_div_by_quote_lots_per_base_lot_per_tick(QuoteLotsPerBaseLotPerTick::new(
                0
            )),
            None
        );
        assert_eq!(price.checked_div_by_ticks(Ticks::new(0)), None);
    }

    #[test]
    #[should_panic(expected = "Underflow in add operation")]
    fn test_add_signed_panic_compatibility() {
        // Test that the old Add implementation still panics for backward compatibility
        let unsigned = QuoteLots::new(50);
        let large_negative = SignedQuoteLots::new(-100);
        let _ = unsigned + large_negative; // Should panic
    }

    // Property tests for div_ceil correctness
    proptest! {
        /// Property: div_ceil(a, b) should always be >= regular division
        #[test]
        fn test_div_ceil_always_gte_regular_division(a in 1u64..=u64::MAX/2, b in 1u64..=u64::MAX/2) {
            let regular_div = a / b;
            let ceil_div = a.div_ceil(b);

            prop_assert!(ceil_div >= regular_div,
                "div_ceil({}, {}) = {} should be >= regular division = {}",
                a, b, ceil_div, regular_div);
        }

        /// Property: div_ceil(a, b) should be exactly regular division when a % b == 0
        #[test]
        fn test_div_ceil_exact_when_no_remainder(a in 1u64..=1_000_000u64, b in 1u64..=1_000_000u64) {
            if a % b == 0 {
                let regular_div = a / b;
                let ceil_div = a.div_ceil(b);

                prop_assert_eq!(ceil_div, regular_div,
                    "When {} % {} == 0, div_ceil should equal regular division", a, b);
            }
        }

        /// Property: div_ceil(a, b) should be regular division + 1 when a % b != 0
        #[test]
        fn test_div_ceil_plus_one_when_remainder(a in 1u64..=1_000_000u64, b in 1u64..=1_000_000u64) {
            if a % b != 0 {
                let regular_div = a / b;
                let ceil_div = a.div_ceil(b);

                prop_assert_eq!(ceil_div, regular_div + 1,
                    "When {} % {} != 0, div_ceil should be regular division + 1", a, b);
            }
        }

        /// Property: div_ceil(a, 1) should always equal a
        #[test]
        fn test_div_ceil_by_one_identity(a in 0u64..=u64::MAX) {
            let result = a.div_ceil(1);
            prop_assert_eq!(result, a, "div_ceil({}, 1) should equal {}", a, a);
        }

        /// Property: div_ceil(0, b) should always be 0
        #[test]
        fn test_div_ceil_zero_numerator(b in 1u64..=u64::MAX) {
            let result = 0u64.div_ceil(b);
            prop_assert_eq!(result, 0, "div_ceil(0, {}) should be 0", b);
        }

        /// Property: For any a <= b, div_ceil(a, b) should be 0 or 1
        #[test]
        fn test_div_ceil_small_numerator(a in 1u64..=1_000_000u64, b in 1u64..=1_000_000u64) {
            if a <= b {
                let result = a.div_ceil(b);
                prop_assert!(result <= 1,
                    "When {} <= {}, div_ceil should be 0 or 1, got {}", a, b, result);

                // Should be 1 unless a == 0
                if a > 0 {
                    prop_assert_eq!(result, 1);
                }
            }
        }

        /// Property: div_ceil should be consistent with mathematical definition
        /// ceil(a/b) = floor((a + b - 1) / b) for positive integers
        #[test]
        fn test_div_ceil_mathematical_definition(a in 1u64..=u64::MAX/2, b in 1u64..=1_000_000u64) {
            let ceil_div = a.div_ceil(b);

            // Alternative formula: (a + b - 1) / b
            // But we need to check for overflow
            if let Some(sum) = a.checked_add(b - 1) {
                let alternative = sum / b;
                prop_assert_eq!(ceil_div, alternative,
                    "div_ceil({}, {}) = {} should match alternative formula = {}",
                    a, b, ceil_div, alternative);
            }
        }
    }

    // Regression test for the margin calculation bug
    #[test]
    fn test_margin_calculation_regression() {
        // This test captures the exact scenario from the failing SDK test
        let book_value = BaseLots::new(1_000_000_000); // $1000 in base lots
        let leverage = Constant::new(1);

        // The correct margin should be exactly 1_000_000_000
        let margin = book_value.div_ceil(leverage);
        assert_eq!(
            margin.as_inner(),
            1_000_000_000,
            "Margin for $1000 with leverage 1 should be exactly $1000, not $1000.000001"
        );

        // With the old buggy implementation, this would have been 1_000_000_001
        // which would make a trader with exactly $1000 collateral fail the
        // margin check
    }

    // Test the behavior difference between old and new implementations
    #[test]
    fn test_old_vs_new_div_ceil_behavior() {
        // Old (buggy) implementation: a / b + 1
        let old_div_ceil = |a: u64, b: u64| -> u64 { a / b + 1 };

        // New (correct) implementation
        let new_div_ceil = |a: u64, b: u64| -> u64 { a.div_ceil(b) };

        // Test exact divisions - these should NOT have +1
        assert_ne!(
            old_div_ceil(10, 5),
            new_div_ceil(10, 5),
            "Old implementation incorrectly adds 1 for exact divisions"
        );
        assert_eq!(new_div_ceil(10, 5), 2, "Correct result for 10/5");
        assert_eq!(old_div_ceil(10, 5), 3, "Old buggy result for 10/5");

        // Test divisions with remainder - these should be the same
        assert_eq!(
            old_div_ceil(11, 5),
            new_div_ceil(11, 5),
            "Both should round up for divisions with remainder"
        );
        assert_eq!(new_div_ceil(11, 5), 3, "Correct result for 11/5");
    }

    #[test]
    fn test_signed_fee_rate_overflow_safety() {
        use crate::quantities::types::{
            FeeRateMicro, QuoteLots, SignedFeeRateMicro, SignedQuoteLots,
        };

        // Test add_unsigned_checked with large unsigned value
        let signed_fee = SignedFeeRateMicro::new(100);
        let unsigned_fee = FeeRateMicro::new(u32::MAX);

        // Should return None because u64::MAX can't be converted to i64
        assert!(signed_fee.add_unsigned_checked(unsigned_fee).is_none());

        // Test with value that fits in i64 but would overflow when added
        let unsigned_fee_large = FeeRateMicro::new(i32::MAX as u32);
        assert!(
            signed_fee
                .add_unsigned_checked(unsigned_fee_large)
                .is_none(),
            "Should fail due to addition overflow"
        );

        // Test with value that fits and doesn't overflow
        let unsigned_fee_small = FeeRateMicro::new(1000);
        assert!(
            signed_fee
                .add_unsigned_checked(unsigned_fee_small)
                .is_some()
        );
        assert_eq!(
            signed_fee
                .add_unsigned_checked(unsigned_fee_small)
                .unwrap()
                .as_inner(),
            1100
        );

        // Test QuoteLots::saturating_add_signed with i64::MIN
        let signed_lots = SignedQuoteLots::new(i64::MIN);
        let base = QuoteLots::new(1000);

        // Should handle i64::MIN correctly using unsigned_abs()
        let result = base.saturating_add_signed(signed_lots);
        // i64::MIN has absolute value of 2^63, which will saturate when subtracting
        // from 1000
        assert_eq!(result, QuoteLots::new(0));

        // Test with normal negative value
        let signed_lots_normal = SignedQuoteLots::new(-500);
        let result_normal = base.saturating_add_signed(signed_lots_normal);
        assert_eq!(result_normal, QuoteLots::new(500));
    }

    #[test]
    fn test_quote_lots_checked_as_signed_bounds() {
        let at_max = QuoteLots::new(i64::MAX as u64);
        assert_eq!(
            at_max.checked_as_signed(),
            Ok(SignedQuoteLots::new(i64::MAX))
        );

        let overflow = QuoteLots::new(i64::MAX as u64 + 1);
        assert_eq!(overflow.checked_as_signed(), Err(MathError::Overflow));
    }

    #[test]
    fn test_signed_quote_lots_checked_as_unsigned() {
        let positive = SignedQuoteLots::new(10);
        assert_eq!(positive.checked_as_unsigned(), Ok(QuoteLots::new(10)));

        let negative = SignedQuoteLots::new(-1);
        assert_eq!(negative.checked_as_unsigned(), Err(MathError::Underflow));
    }

    #[test]
    fn test_signed_quote_lots_abs_as_unsigned() {
        let value = SignedQuoteLots::new(-10);
        assert_eq!(value.abs_as_unsigned(), QuoteLots::new(10));
    }

    #[test]
    fn test_signed_quote_lots_abs_as_unsigned_min_overflow() {
        let min_value = SignedQuoteLots::new(i64::MIN);
        assert_eq!(
            min_value.abs_as_unsigned(),
            QuoteLots::new(i64::MIN.unsigned_abs())
        );
    }

    #[test]
    #[should_panic(expected = "Overflow in abs for signed 64-bit wrapper")]
    fn test_signed_quote_lots_abs_panics_on_min() {
        let _ = SignedQuoteLots::new(i64::MIN).abs();
    }

    #[test]
    #[should_panic(expected = "Overflow in neg for signed 64-bit wrapper")]
    fn test_signed_quote_lots_neg_panics_on_min() {
        let _ = -SignedQuoteLots::new(i64::MIN);
    }

    #[test]
    #[should_panic(expected = "Overflow in abs for signed 32-bit wrapper")]
    fn test_signed_fee_rate_abs_panics_on_min() {
        let _ = SignedFeeRateMicro::new(i32::MIN).abs();
    }

    #[test]
    #[should_panic(expected = "Overflow in neg for signed 32-bit wrapper")]
    fn test_signed_fee_rate_neg_panics_on_min() {
        let _ = -SignedFeeRateMicro::new(i32::MIN);
    }

    #[test]
    #[should_panic(expected = "Overflow in add: result exceeds signed bounds")]
    fn test_signed_quote_lots_add_unsigned_panics_on_overflow() {
        let signed = SignedQuoteLots::new(i64::MAX);
        let _ = signed + QuoteLots::new(1);
    }

    #[test]
    #[should_panic(expected = "Overflow in sub: result exceeds signed bounds")]
    fn test_signed_quote_lots_sub_unsigned_panics_on_underflow() {
        let signed = SignedQuoteLots::new(i64::MIN);
        let _ = signed - QuoteLots::new(1);
    }

    #[test]
    fn test_ticks_checked_as_signed_bounds() {
        let in_range = Ticks::new(i32::MAX as u64);
        assert!(in_range.checked_as_signed().is_ok());

        let overflow = Ticks::new(i32::MAX as u64 + 1);
        assert_eq!(overflow.checked_as_signed(), Err(MathError::Overflow));
    }

    #[test]
    fn test_base_lots_checked_as_signed_bounds() {
        // BaseLots is bounded to u32::MAX, which always fits in i64
        let at_bound = BaseLots::new(u32::MAX as u64);
        assert!(at_bound.checked_as_signed().is_ok());
        assert_eq!(
            at_bound.checked_as_signed(),
            Ok(SignedBaseLots::new(u32::MAX as i64))
        );

        // Test that an artificially large value (beyond i64::MAX) would fail.
        // This verifies the check exists even though valid BaseLots can't reach this.
        let overflow = BaseLots::new(i64::MAX as u64 + 1);
        assert_eq!(overflow.checked_as_signed(), Err(MathError::Overflow));
    }

    #[test]
    fn test_signed_base_lots_checked_as_unsigned_bounds() {
        // Value within BaseLots bounds (u32::MAX) should succeed
        let in_range = SignedBaseLots::new(u32::MAX as i64);
        assert_eq!(
            in_range.checked_as_unsigned(),
            Ok(BaseLots::new(u32::MAX as u64))
        );

        // Value exceeding BaseLots::UPPER_BOUND (u32::MAX) should fail
        let overflow = SignedBaseLots::new(u32::MAX as i64 + 1);
        assert_eq!(overflow.checked_as_unsigned(), Err(MathError::Overflow));

        // Large i64 value should also fail
        let large_overflow = SignedBaseLots::new(i64::MAX);
        assert_eq!(
            large_overflow.checked_as_unsigned(),
            Err(MathError::Overflow)
        );

        // Negative value should fail with Underflow
        let negative = SignedBaseLots::new(-1);
        assert_eq!(negative.checked_as_unsigned(), Err(MathError::Underflow));
    }
}
