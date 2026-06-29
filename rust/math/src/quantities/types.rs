//! Concrete type definitions for type-safe quantity arithmetic.
//!
//! This module contains all the specific quantity types used throughout
//! the Phoenix exchange system. Each type is designed to prevent common
//! arithmetic errors and ensure dimensional correctness.
//!
//! # Type Categories
//!
//! - **Lots**: Smallest tradeable units (BaseLots, QuoteLots)
//! - **Units**: Human-readable quantities (BaseUnits, QuoteUnits)
//! - **Prices**: Price representations (Ticks, QuoteLotsPerBaseLot)
//! - **Conversion Factors**: For converting between units and lots
//! - **Risk Factors**: For risk management calculations
//!
//! # Overflow Safety
//!
//! Many types have restricted ranges to prevent overflow:
//! - `BaseLots`: Limited to u32::MAX to prevent overflow in calculations
//! - `Ticks`: Limited to u32::MAX for safe price arithmetic
//! - Risk factors: Limited to 0-10,000 (representing 0.0 to 1.0)

use std::fmt::{Debug, Display};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};

use crate::quantities::traits::{ScalarBounds, WrapperNum};

// Slot
basic_u64_struct!(Slot);

// Generic numeric constants
basic_u64_struct!(Constant);
basic_i64_struct!(SignedConstant);

// Fundamental quantities
basic_u64_struct_with_bounds!(QuoteLots, 0, u64::MAX);
basic_u64_struct_with_bounds!(BaseLots, 0, u32::MAX as u64);
basic_i64_struct!(SignedBaseLots);
basic_i64_struct!(SignedQuoteLots);

impl core::convert::TryFrom<i128> for SignedQuoteLots {
    type Error = crate::quantities::MathError;

    fn try_from(value: i128) -> Result<Self, Self::Error> {
        i64::try_from(value)
            .map(Self::new)
            .map_err(|_| crate::quantities::MathError::Overflow)
    }
}

impl core::convert::TryFrom<i128> for SignedBaseLots {
    type Error = crate::quantities::MathError;

    fn try_from(value: i128) -> Result<Self, Self::Error> {
        i64::try_from(value)
            .map(Self::new)
            .map_err(|_| crate::quantities::MathError::Overflow)
    }
}

// Creates SignedBaseLotsUpcasted and SignedQuoteLotsUpcasted
basic_i128_struct!(SignedBaseLots);
basic_i128_struct!(SignedQuoteLots);

allow_checked_add_64bit!(QuoteLots, SignedQuoteLots);
allow_add_64bit!(BaseLots, SignedBaseLots);

allow_multiply!(Slot, QuoteLots, QuoteLots);

impl QuoteLots {
    /// Multiply by a Constant (generic u64 wrapper type like leverage)
    /// Returns None on overflow
    pub fn checked_mul<Multiplier: WrapperNum<u64>>(self, other: Multiplier) -> Option<Self> {
        self.inner.checked_mul(other.as_inner()).map(Self::new)
    }
}

// Intermediate conversions for extracting quote lots from book orders
basic_u64_struct_with_bounds!(QuoteLotsPerBaseLot, 0, u32::MAX as u64);
basic_i64_struct!(SignedQuoteLotsBaseLots);
basic_i128_struct!(SignedQuoteLotsBaseLots);

basic_i64_struct!(SignedQuoteLotsPerBaseLot);
basic_i128_struct!(SignedQuoteLotsPerBaseLot);
allow_add_64bit!(QuoteLotsPerBaseLot, SignedQuoteLotsPerBaseLot);

impl QuoteLotsPerBaseLot {
    /// Multiply by another value with overflow check
    /// Returns None on overflow
    pub fn checked_mul<Multiplier: WrapperNum<u64>>(self, rhs: Multiplier) -> Option<Self> {
        self.inner.checked_mul(rhs.as_inner()).map(Self::new)
    }
}

impl SignedQuoteLotsPerBaseLot {
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        self.inner.checked_mul(rhs.inner).map(Self::new)
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(self.inner.clamp(min.inner, max.inner))
    }
}

impl SignedQuoteLotsPerBaseLotUpcasted {
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        self.inner.checked_mul(rhs.inner).map(Self::new)
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(self.inner.clamp(min.inner, max.inner))
    }
}

allow_multiply!(SignedQuoteLotsPerBaseLot, SignedBaseLots, SignedQuoteLots);

allow_multiply!(
    SignedBaseLotsUpcasted,
    SignedQuoteLotsUpcasted,
    SignedQuoteLotsBaseLotsUpcasted
);

allow_multiply!(SignedBaseLots, SignedQuoteLots, SignedQuoteLotsBaseLots);

// Discrete price unit (quote quantity per base quantity)
basic_u64_struct_with_bounds!(Ticks, 0, u32::MAX as u64);
basic_i64_struct_with_bounds!(SignedTicks, -i32::MAX as i64, i32::MAX as i64);
allow_checked_add_32bit!(Ticks, SignedTicks);

impl Ticks {
    /// Multiply by another value with overflow check
    /// Returns None on overflow
    pub fn checked_mul<Multiplier: WrapperNum<u64>>(self, rhs: Multiplier) -> Option<Self> {
        self.inner.checked_mul(rhs.as_inner()).map(Self::new)
    }
}

// Quantities
basic_u64_struct!(QuoteUnits);
basic_u64_struct!(BaseUnits);

// Dimensionless conversion factors
basic_u64_struct!(BaseLotsPerBaseUnit);
basic_u64_struct!(QuoteLotsPerQuoteUnit);

// Dimensionless tick sizes
basic_u64_struct_with_bounds!(QuoteLotsPerBaseLotPerTick, 0, 10_000);

// Basis points for general percentage calculations (0.0 to 1.0, where 10_000 =
// 100%)
basic_u64_struct_with_bounds!(BasisPoints, 0, 10_000);

// Risk factor (discount) for positive uPnL (0.0 to 1.0)
// This is just an alias to BasisPoints since they're semantically identical.
pub type UPnlRiskFactor = BasisPoints;

// Fee rates in micros (1/1,000,000). For example, 2500 = 0.25% fee
basic_u32_struct_with_bounds!(FeeRateMicro, 0, i32::MAX as u32);
basic_i32_struct!(SignedFeeRateMicro);

impl BasisPoints {
    /// The denominator for basis points (10,000 = 100%)
    pub const DENOMINATOR: u64 = 10_000;

    /// Apply basis points to a QuoteLots value, returning the result
    /// For example: 5000 basis points (50%) of 1000 QuoteLots = 500 QuoteLots
    pub fn apply_to_quote_lots(&self, value: QuoteLots) -> Option<QuoteLots> {
        let result = value
            .as_inner()
            .checked_mul(self.as_inner())?
            .checked_div(Self::DENOMINATOR)?;
        Some(QuoteLots::new(result))
    }

    /// Apply basis points to a QuoteLots value with ceiling division
    /// For example: 5001 basis points of 1000 QuoteLots = 501 QuoteLots (rounds
    /// up)
    pub fn apply_to_quote_lots_ceil(&self, value: QuoteLots) -> Option<QuoteLots> {
        // Try checked arithmetic first
        if let Some(numerator) = value.as_inner().checked_mul(self.as_inner()) {
            if let Some(result) = numerator.checked_add(Self::DENOMINATOR - 1) {
                return Some(QuoteLots::new(result / Self::DENOMINATOR));
            }
        }

        // Upcast to u128 for intermediate calculations
        let numerator = value.as_inner() as u128 * self.as_inner() as u128;
        let result = numerator.div_ceil(Self::DENOMINATOR as u128);

        // Try to downcast back to u64
        u64::try_from(result).ok().map(QuoteLots::new)
    }

    /// Apply basis points to a Ticks value
    pub fn apply_to_ticks(&self, value: Ticks) -> Option<Ticks> {
        value
            .as_inner()
            .checked_mul(self.as_inner())?
            .checked_div(Self::DENOMINATOR)
            .map(Ticks::new)
    }

    /// Create from a u16 value (common for risk factors stored as u16)
    pub fn from_u16(value: u16) -> Option<Self> {
        if value <= Self::UPPER_BOUND as u16 {
            Some(Self::new(value as u64))
        } else {
            None
        }
    }

    pub fn to_u16(&self) -> u16 {
        if self.as_inner() > Self::UPPER_BOUND {
            Self::UPPER_BOUND as u16
        } else {
            self.as_inner() as u16
        }
    }
}

// Divisor for micro units (1,000,000)
basic_u64_struct_with_bounds!(MicroDivisor, 1_000_000, 1_000_000);

// Funding rate unit in seconds
basic_u64_struct!(FundingRateUnitInSeconds);

impl FundingRateUnitInSeconds {
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        self.inner.checked_mul(rhs.inner).map(Self::new)
    }
}

impl MicroDivisor {
    /// The standard micro divisor constant
    pub const MICRO: Self = Self { inner: 1_000_000 };
}

impl FeeRateMicro {
    pub fn to_signed_fee_rate_micro(self) -> SignedFeeRateMicro {
        SignedFeeRateMicro::new(self.as_inner() as i32)
    }

    /// Apply fee rate to quote lots, rounding up (ceiling division)
    /// Returns the fee amount in quote lots
    pub fn apply_to_quote_lots(self, quote_lots: QuoteLots) -> Option<QuoteLots> {
        let fee = quote_lots
            .as_inner()
            .checked_mul(self.as_u64())?
            .div_ceil(MicroDivisor::MICRO.as_inner());
        Some(QuoteLots::new(fee))
    }

    /// Apply fee rate using saturating arithmetic
    pub fn apply_to_quote_lots_saturating(self, quote_lots: QuoteLots) -> QuoteLots {
        let fee = quote_lots
            .as_inner()
            .saturating_mul(self.as_u64())
            .div_ceil(MicroDivisor::MICRO.as_inner());
        QuoteLots::new(fee)
    }

    /// Adjust quote budget for fees (for buy orders)
    pub fn adjust_quote_budget(self, quote_lot_budget: QuoteLots) -> QuoteLots {
        let divisor = MicroDivisor::MICRO;
        let fee_adjusted_budget = quote_lot_budget
            .as_inner()
            .saturating_mul(divisor.as_inner())
            .saturating_div(divisor.as_inner().saturating_add(self.as_u64()));
        QuoteLots::new(fee_adjusted_budget)
    }
}

impl SignedFeeRateMicro {
    pub fn from_i8_bps(bps: i8) -> Self {
        Self::new(bps as i32 * 100)
    }

    pub fn to_unsigned_fee_rate_micro(self) -> FeeRateMicro {
        FeeRateMicro::new(self.as_inner() as u32)
    }

    /// Apply signed fee rate to quote lots
    /// For positive fees: rounds up (div_ceil)
    /// For negative fees (rebates): rounds down (regular division)
    pub fn apply_to_quote_lots(self, quote_lots: QuoteLots) -> Option<SignedQuoteLots> {
        let divisor_i64 = MicroDivisor::MICRO.as_inner() as i64;
        let size_i64 = quote_lots.as_inner() as i64;
        let product = size_i64.checked_mul(self.as_i64())?;

        let fee = if self.as_inner() >= 0 {
            // Positive fee - round up
            (product as u64).div_ceil(MicroDivisor::MICRO.as_inner()) as i64
        } else {
            // Negative fee (rebate) - round down
            product / divisor_i64
        };

        Some(SignedQuoteLots::new(fee))
    }

    /// Add an unsigned fee rate to this signed fee rate, checking for negative
    /// total
    pub fn add_unsigned_checked(self, unsigned: FeeRateMicro) -> Option<Self> {
        let unsigned_i32 = i32::try_from(unsigned.as_inner()).ok()?;
        let total = self.as_inner().checked_add(unsigned_i32)?;
        if total < 0 {
            None
        } else {
            Some(SignedFeeRateMicro::new(total))
        }
    }

    /// Multiply by a signed scalar, returning None on overflow
    pub fn checked_mul_i32(self, scalar: i32) -> Option<Self> {
        let product = (self.as_inner() as i64).checked_mul(scalar as i64)?;
        i32::try_from(product).ok().map(SignedFeeRateMicro::new)
    }

    /// Divide by a signed scalar, returning None on division by zero or
    /// overflow
    pub fn checked_div_i32(self, scalar: i32) -> Option<Self> {
        if scalar == 0 {
            return None;
        }
        let quotient = (self.as_inner() as i64).checked_div(scalar as i64)?;
        i32::try_from(quotient).ok().map(SignedFeeRateMicro::new)
    }
}

// Conversions from units to lots
allow_multiply!(BaseUnits, BaseLotsPerBaseUnit, BaseLots);
allow_multiply!(QuoteUnits, QuoteLotsPerQuoteUnit, QuoteLots);

// Conversion between units of tick size
allow_multiply!(QuoteLotsPerBaseLotPerTick, Ticks, QuoteLotsPerBaseLot);

// Intermediate conversions for extracting quote lots from book orders
allow_multiply!(QuoteLotsPerBaseLot, BaseLots, QuoteLots);

// Density of liquidity per tick in DMM
basic_u64_struct!(BaseLotsPerTick);

allow_multiply!(BaseLotsPerTick, Ticks, BaseLots);

#[repr(transparent)]
#[derive(
    Pod, Zeroable, Debug, Default, Copy, Clone, PartialEq, BorshDeserialize, BorshSerialize, Eq,
)]
pub struct SequenceNumberU8(u8);

impl SequenceNumberU8 {
    pub fn from(value: u8) -> Self {
        Self(value)
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

/// Error type for SignedQuoteLotsI56 conversions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedQuoteLotsI56Error {
    /// Value does not fit in 56-bit signed integer (exceeds ±2^55)
    Overflow,
}

impl core::fmt::Display for SignedQuoteLotsI56Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SignedQuoteLotsI56Error::Overflow => {
                write!(f, "Value does not fit in 56-bit signed integer")
            }
        }
    }
}

#[repr(C)]
#[derive(
    Pod, Zeroable, Debug, Default, Copy, Clone, PartialEq, BorshDeserialize, BorshSerialize, Eq,
)]
pub struct SignedQuoteLotsI56 {
    data: [u8; 7],
}

impl SignedQuoteLotsI56 {
    /// Convenience method: converts to SignedQuoteLots
    pub fn to_signed_quote_lots(&self) -> SignedQuoteLots {
        (*self).into()
    }

    pub fn checked_add(self, quote_lots: SignedQuoteLots) -> Option<Self> {
        let temp: SignedQuoteLots = self.into();
        let sum = temp.checked_add(quote_lots)?;
        Self::try_from(sum).ok()
    }

    pub fn clear(&mut self) {
        self.data = [0; 7];
    }
}

// Infallible conversion: i56 -> i64 (always safe)
impl From<SignedQuoteLotsI56> for SignedQuoteLots {
    fn from(value: SignedQuoteLotsI56) -> Self {
        let mut temp: [u8; 8] = [0; 8];
        temp[..7].copy_from_slice(&value.data);
        // For signed little-endian, extended bytes should match MSB
        temp[7] = if value.data[6] >> 7 > 0 { u8::MAX } else { 0 };
        SignedQuoteLots::new(i64::from_le_bytes(temp))
    }
}

// Fallible conversion: i64 -> i56 (can overflow)
impl TryFrom<SignedQuoteLots> for SignedQuoteLotsI56 {
    type Error = SignedQuoteLotsI56Error;

    fn try_from(quote_lots: SignedQuoteLots) -> Result<Self, Self::Error> {
        let raw_i64 = quote_lots.as_inner();
        // Check if value fits in 56 bits (±2^55)
        if raw_i64.abs() >= (1 << 55) {
            return Err(SignedQuoteLotsI56Error::Overflow);
        }
        let temp: [u8; 8] = raw_i64.to_le_bytes();
        Ok(Self {
            data: [
                temp[0], temp[1], temp[2], temp[3], temp[4], temp[5], temp[6],
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversions_and_roundtrip() {
        // Zero and default
        let zero: SignedQuoteLotsI56 = Default::default();
        assert_eq!(zero.to_signed_quote_lots(), SignedQuoteLots::ZERO);

        // Comprehensive roundtrip testing including boundaries
        let test_values = [
            0,
            1,
            -1,
            100,
            -100,
            1000,
            -1000,
            1_000_000,
            -1_000_000,
            1_000_000_000,
            -1_000_000_000,
            1_000_000_000_000,
            -1_000_000_000_000,
            (1i64 << 54),
            -(1i64 << 54),
            // Exact boundaries (max safe values)
            (1i64 << 55) - 1,
            -((1i64 << 55) - 1),
            // Just within boundaries
            (1i64 << 55) - 2,
            -((1i64 << 55) - 2),
        ];
        for &value in &test_values {
            let quote_lots = SignedQuoteLots::new(value);
            let i56: SignedQuoteLotsI56 = quote_lots.try_into().unwrap();
            assert_eq!(
                i56.to_signed_quote_lots(),
                quote_lots,
                "Roundtrip failed for {}",
                value
            );
        }

        // Sign extension for negative values
        let neg: SignedQuoteLotsI56 = SignedQuoteLots::new(-1).try_into().unwrap();
        assert_eq!(neg.to_signed_quote_lots().as_inner(), -1);

        let neg_pattern: SignedQuoteLotsI56 = SignedQuoteLots::new(-12345678).try_into().unwrap();
        assert_eq!(
            neg_pattern.to_signed_quote_lots(),
            SignedQuoteLots::new(-12345678)
        );

        // Reset
        let mut i56: SignedQuoteLotsI56 = SignedQuoteLots::new(100).try_into().unwrap();
        i56.clear();
        assert_eq!(i56.to_signed_quote_lots(), SignedQuoteLots::ZERO);
    }

    #[test]
    fn test_pod_compatibility() {
        let i56: SignedQuoteLotsI56 = Default::default();
        let bytes = bytemuck::bytes_of(&i56);
        assert_eq!(bytes.len(), 7);

        let i56_from_bytes: &SignedQuoteLotsI56 = bytemuck::from_bytes(bytes);
        assert_eq!(i56_from_bytes.to_signed_quote_lots(), SignedQuoteLots::ZERO);
    }

    #[test]
    #[should_panic(expected = "Overflow")]
    fn test_overflow_positive_panic() {
        let _: SignedQuoteLotsI56 = SignedQuoteLots::new(1i64 << 55).try_into().unwrap();
    }

    #[test]
    #[should_panic(expected = "Overflow")]
    fn test_overflow_negative_panic() {
        let _: SignedQuoteLotsI56 = SignedQuoteLots::new(-(1i64 << 55)).try_into().unwrap();
    }

    #[test]
    fn test_checked_conversions() {
        // Valid conversions
        assert!(SignedQuoteLotsI56::try_from(SignedQuoteLots::new(1000)).is_ok());
        assert!(SignedQuoteLotsI56::try_from(SignedQuoteLots::new((1i64 << 55) - 1)).is_ok());

        // Overflow at boundaries
        assert!(SignedQuoteLotsI56::try_from(SignedQuoteLots::new(1i64 << 55)).is_err());
        assert!(SignedQuoteLotsI56::try_from(SignedQuoteLots::new(-(1i64 << 55))).is_err());
    }

    #[test]
    fn test_checked_add() {
        // Basic addition
        let a: SignedQuoteLotsI56 = SignedQuoteLots::new(100).try_into().unwrap();
        assert_eq!(
            a.checked_add(SignedQuoteLots::new(200))
                .unwrap()
                .to_signed_quote_lots(),
            SignedQuoteLots::new(300)
        );
        assert_eq!(
            a.checked_add(SignedQuoteLots::new(-50))
                .unwrap()
                .to_signed_quote_lots(),
            SignedQuoteLots::new(50)
        );

        // Large values that fit
        let large: SignedQuoteLotsI56 = SignedQuoteLots::new(1i64 << 54).try_into().unwrap();
        assert!(
            large
                .checked_add(SignedQuoteLots::new(1i64 << 53))
                .is_some()
        );

        // Boundary overflow - positive
        let near_max: SignedQuoteLotsI56 =
            SignedQuoteLots::new((1i64 << 55) - 2).try_into().unwrap();
        assert!(
            near_max.checked_add(SignedQuoteLots::new(10)).is_none(),
            "Should overflow at positive boundary"
        );

        // Boundary overflow - negative
        let near_min: SignedQuoteLotsI56 = SignedQuoteLots::new(-((1i64 << 55) - 2))
            .try_into()
            .unwrap();
        assert!(
            near_min.checked_add(SignedQuoteLots::new(-10)).is_none(),
            "Should overflow at negative boundary"
        );
    }

    #[test]
    fn test_trait_conversions() {
        // Test From trait (infallible i56 -> i64)
        let i56: SignedQuoteLotsI56 = SignedQuoteLots::new(12345).try_into().unwrap();
        let i64_result: SignedQuoteLots = i56.into();
        assert_eq!(i64_result, SignedQuoteLots::new(12345));

        // Test TryFrom trait (fallible i64 -> i56)
        let quote_lots = SignedQuoteLots::new(67890);
        let i56_result: Result<SignedQuoteLotsI56, _> = quote_lots.try_into();
        assert!(i56_result.is_ok());
        assert_eq!(i56_result.unwrap().to_signed_quote_lots(), quote_lots);

        // Test TryFrom with overflow
        let overflow_value = SignedQuoteLots::new(1i64 << 55);
        let overflow_result: Result<SignedQuoteLotsI56, _> = overflow_value.try_into();
        assert!(overflow_result.is_err());
        assert_eq!(
            overflow_result.unwrap_err(),
            SignedQuoteLotsI56Error::Overflow
        );

        // Test turbofish syntax
        let i56_turbofish = SignedQuoteLotsI56::try_from(SignedQuoteLots::new(-999));
        assert!(i56_turbofish.is_ok());
        assert_eq!(
            i56_turbofish.unwrap().to_signed_quote_lots(),
            SignedQuoteLots::new(-999)
        );
    }
}
