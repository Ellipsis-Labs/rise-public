use borsh::{BorshDeserialize, BorshSerialize};
use rust_decimal::RoundingStrategy;
use rust_decimal::prelude::*;

use crate::quantities::{MathError, Ticks};

/// Oracle-friendly price representation (mantissa + exponent).
///
/// `value * 10^{-expo}` yields quote units per base unit. The exponent is
/// stored as a positive integer to match on-chain layouts (e.g., Pyth‑style
/// mantissa/exponent).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct Price {
    /// Price mantissa
    pub value: u64,
    /// Decimal exponent (number of fractional digits)
    pub expo: u8,
}

impl Price {
    /// Rescale the mantissa to `target_decimals` fractional digits.
    pub fn to_scaled_value(&self, target_decimals: u8) -> Result<u64, MathError> {
        if self.expo < target_decimals {
            let factor = 10u64
                .checked_pow((target_decimals - self.expo) as u32)
                .ok_or(MathError::Overflow)?;
            self.value.checked_mul(factor).ok_or(MathError::Overflow)
        } else if self.expo > target_decimals {
            let factor = 10u64
                .checked_pow((self.expo - target_decimals) as u32)
                .ok_or(MathError::Overflow)?;
            self.value
                .checked_div(factor)
                .ok_or(MathError::DivisionByZero)
        } else {
            Ok(self.value)
        }
    }

    /// Convert this price into ticks given the market's tick size and decimals.
    pub fn to_ticks(
        &self,
        tick_size_in_quote_lots_per_base_lot: u64,
        base_lot_decimals: i8,
        quote_decimals: u8,
    ) -> Result<u64, MathError> {
        let price_in_quote_lots_per_base_unit = self.to_scaled_value(quote_decimals)?;
        if tick_size_in_quote_lots_per_base_lot == 0 {
            return Err(MathError::DivisionByZero);
        }

        if base_lot_decimals >= 0 {
            let base_lots_per_base_unit = 10u64
                .checked_pow(base_lot_decimals as u32)
                .ok_or(MathError::Overflow)?;
            price_in_quote_lots_per_base_unit
                .checked_div(
                    tick_size_in_quote_lots_per_base_lot
                        .checked_mul(base_lots_per_base_unit)
                        .ok_or(MathError::Overflow)?,
                )
                .ok_or(MathError::DivisionByZero)
        } else {
            let base_units_per_base_lot = 10u64
                .checked_pow((-base_lot_decimals) as u32)
                .ok_or(MathError::Overflow)?;
            price_in_quote_lots_per_base_unit
                .checked_mul(base_units_per_base_lot)
                .ok_or(MathError::Overflow)?
                .checked_div(tick_size_in_quote_lots_per_base_lot)
                .ok_or(MathError::DivisionByZero)
        }
    }

    /// Helper to wrap the tick result in the newtype.
    pub fn to_ticks_wrapped(
        &self,
        tick_size_in_quote_lots_per_base_lot: u64,
        base_lot_decimals: i8,
        quote_decimals: u8,
    ) -> Result<Ticks, MathError> {
        let t = self.to_ticks(
            tick_size_in_quote_lots_per_base_lot,
            base_lot_decimals,
            quote_decimals,
        )?;
        Ticks::new_checked(t).map_err(|_| MathError::Overflow)
    }
}

impl Price {
    /// Convert a positive `f64` into `Price` with a caller-provided decimal
    /// cap.
    pub fn from_f64_with_max_decimals(value: f64, max_decimals: u8) -> Result<Self, MathError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(MathError::Underflow);
        }

        let expo = dynamic_price_decimals(value).min(max_decimals);
        let scale = 10f64.powi(expo as i32);
        let scaled = (value * scale).round();

        if !scaled.is_finite() {
            return Err(MathError::Overflow);
        }
        if scaled <= 0.0 {
            return Err(MathError::Underflow);
        }
        if scaled > u64::MAX as f64 {
            return Err(MathError::Overflow);
        }

        Ok(Price {
            value: scaled as u64,
            expo,
        })
    }

    /// Convert using the default `DEFAULT_MAX_DYNAMIC_DECIMALS` cap.
    pub fn from_f64(value: f64) -> Result<Self, MathError> {
        Self::from_f64_with_max_decimals(value, DEFAULT_MAX_DYNAMIC_DECIMALS)
    }

    /// High-precision conversion from `rust_decimal::Decimal`, keeping as much
    /// precision as possible without overflowing `u64` while avoiding
    /// over-scaling for large prices.
    pub fn from_decimal(decimal: Decimal) -> Result<Self, MathError> {
        Self::from_decimal_with_max_decimals(decimal, DEFAULT_MAX_DYNAMIC_DECIMALS)
    }

    /// Variant that allows configuring the max decimals cap.
    pub fn from_decimal_with_max_decimals(
        decimal: Decimal,
        max_decimals: u8,
    ) -> Result<Self, MathError> {
        if decimal.is_sign_negative() || decimal.is_zero() {
            return Err(MathError::Underflow);
        }

        let value_f64 = decimal.to_f64().ok_or(MathError::Overflow)?;
        let dynamic_expo = dynamic_price_decimals(value_f64) as u32;
        let source_scale = decimal.scale();

        // Target exponent: keep at least the source scale, respect the dynamic
        // heuristic, and never exceed `max_decimals`.
        let target_expo = std::cmp::min(
            max_decimals as u32,
            std::cmp::max(source_scale, dynamic_expo),
        );

        let mut mantissa = decimal.mantissa();
        let mut scale = source_scale;

        if scale < target_expo {
            // Increase precision by appending zeros to the mantissa.
            let diff = target_expo - scale;
            let factor = ten_pow_i128(diff).ok_or(MathError::Overflow)?;
            mantissa = mantissa.checked_mul(factor).ok_or(MathError::Overflow)?;
            scale = target_expo;
        } else if scale > target_expo {
            // Round rather than truncate when reducing precision.
            let rounded =
                decimal.round_dp_with_strategy(target_expo, RoundingStrategy::MidpointAwayFromZero);
            mantissa = rounded.mantissa();
            scale = rounded.scale();
        }

        debug_assert_eq!(scale, target_expo);

        if mantissa <= 0 {
            return Err(MathError::Underflow);
        }
        if mantissa > u64::MAX as i128 {
            return Err(MathError::Overflow);
        }

        Ok(Price {
            value: mantissa as u64,
            expo: target_expo as u8,
        })
    }
}

/// Choose a reasonable number of decimals for a positive price to avoid
/// over/under-scaling while preserving precision for micro assets.
pub fn dynamic_price_decimals(value: f64) -> u8 {
    if value <= 0.0 || !value.is_finite() {
        return 6;
    }

    if value >= 1.0 {
        // Large prices: reduce decimals as magnitude grows, but keep at least 4 to
        // avoid under-reporting precision for high-value assets.
        let floor_log = value.log10().floor() as i32;
        let decimals = 6 - floor_log - 1;
        return decimals.clamp(4, 6) as u8;
    }

    // Sub-dollar: add decimals as the value shrinks, but cap to keep mantissas
    // reasonable. Micro assets can afford a wider cap to preserve precision.
    let magnitude = (-value.log10()).ceil() as i32;
    let decimals = 6 + magnitude;
    decimals.clamp(6, DEFAULT_MAX_DYNAMIC_DECIMALS as i32) as u8
}

const DEFAULT_MAX_DYNAMIC_DECIMALS: u8 = 12;

fn ten_pow_i128(exp: u32) -> Option<i128> {
    10i128.checked_pow(exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_to_scaled_value_rounds_correctly() {
        let p = Price {
            value: 15_000,
            expo: 0,
        };
        assert_eq!(p.to_scaled_value(6).unwrap(), 15_000_000_000);
        let p = Price {
            value: 15_000_000,
            expo: 3,
        };
        assert_eq!(p.to_scaled_value(6).unwrap(), 15_000_000_000);
        let p = Price {
            value: 15_000_000_000,
            expo: 9,
        };
        assert_eq!(p.to_scaled_value(6).unwrap(), 15_000_000);
    }

    #[test]
    fn price_to_ticks_behaves_for_positive_and_negative_decimals() {
        let p = Price {
            value: 15_000_000,
            expo: 3,
        };
        assert_eq!(p.to_ticks(10_000_000, 3, 6).unwrap(), 1);

        let p2 = Price {
            value: 150,
            expo: 0,
        };
        assert_eq!(p2.to_ticks(1_000_000, 0, 6).unwrap(), 150);

        let p3 = Price {
            value: 11_460,
            expo: 8,
        };
        // price = 0.00011460, base_lot_decimals = -4
        let ticks = p3.to_ticks(1, -4, 6).unwrap();
        assert!(ticks > 0);
    }

    #[test]
    fn dynamic_price_decimals_limits() {
        assert_eq!(dynamic_price_decimals(0.000009706), 12);
        assert_eq!(dynamic_price_decimals(50_000.0), 4);
        assert_eq!(dynamic_price_decimals(1.0), 5);
        assert_eq!(dynamic_price_decimals(-1.0), 6);
    }

    #[test]
    fn quantize_price_micro_asset() {
        let price = 0.000009706;
        let q = Price::from_f64(price).unwrap();
        assert_eq!(q.expo, 12);
        assert_eq!(q.value, 9_706_000);

        let ticks = q.to_ticks(1, /* tick size */ 0, /* base lot dec */ 6).err();
        assert!(ticks.is_none());

        let ticks_wrapped = q.to_ticks_wrapped(1, 0, 6).expect("ticks should compute");
        assert_eq!(ticks_wrapped.as_inner(), 9);
    }

    #[test]
    fn quantize_price_large_value_rounds() {
        let price = 50_000.123_4;
        let p = Price::from_f64(price).unwrap();
        assert_eq!(p.expo, 4); // reduced decimals with floor at 4 for large price
        let ticks = p
            .to_ticks_wrapped(
                100, // tick size quote lots per base lot
                4,   // base lot dec
                6,
            )
            .unwrap();
        assert!(ticks.as_inner() > 0);
    }

    #[test]
    fn rounding_boundaries_half_up_behavior() {
        // Value slightly below the .5 boundary should round down
        let expo = 5;
        let scale = 10f64.powi(expo);
        let just_below = (123_456_f64 + 0.4999) / scale;
        let p_down = Price::from_f64_with_max_decimals(just_below, expo as u8).unwrap();
        assert_eq!(p_down.value, 123_456);

        // Value slightly above the .5 boundary should round up
        let just_above = (123_456_f64 + 0.5001) / scale;
        let p_up = Price::from_f64_with_max_decimals(just_above, expo as u8).unwrap();
        assert_eq!(p_up.value, 123_457);
    }

    #[test]
    fn sub_dollar_micro_precision_scaling() {
        // Very small price should keep high precision and sensible ticks
        let price = 0.000009706_f64;
        let p = Price::from_f64(price).unwrap();
        assert_eq!(p.expo, 12);
        assert_eq!(p.value, 9_706_000);

        // Convert to ticks for a 1-quote-lot tick, base lot decimals 0, quote_dec=6
        let ticks = p.to_ticks_wrapped(1, 0, 6).unwrap();
        assert_eq!(ticks.as_inner(), 9);

        // Slightly larger price should produce strictly greater ticks
        let price_up = 0.0000101_f64; // enough to move to tick 10
        let p_up = Price::from_f64(price_up).unwrap();
        let ticks_up = p_up.to_ticks_wrapped(1, 0, 6).unwrap();
        assert!(ticks_up.as_inner() > ticks.as_inner());
    }

    #[test]
    fn from_decimal_micro_price_scales_up_to_cap() {
        let d = rust_decimal::Decimal::from_str("0.000009706").unwrap();
        let p = Price::from_decimal(d).unwrap();
        assert_eq!(p.expo, 12);
        assert_eq!(p.value, 9_706_000);
    }

    #[test]
    fn from_decimal_respects_dynamic_and_max() {
        let d = rust_decimal::Decimal::from_str("99443.1002232312").unwrap();
        let p = Price::from_decimal_with_max_decimals(d, 12).unwrap();
        // Source scale is 10; dynamic suggests 4; we keep source scale (10)
        assert_eq!(p.expo, 10);
        assert!(p.value > 0);
    }

    #[test]
    fn quantize_price_overflow_rejected() {
        let too_large = f64::MAX;
        assert!(Price::from_f64(too_large).is_err());
    }
}
