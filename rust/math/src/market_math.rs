use std::str::FromStr;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::direction::Side;
use crate::quantities::{
    BaseLots, BaseLotsPerTick, MathError, QuoteLots, QuoteLotsPerBaseLotPerTick, SignedBaseLots,
    SignedQuoteLots, SignedTicks, Ticks,
};

const QUOTE_LOT_DECIMALS: u8 = 6;

/// Rounding behavior for floating-point conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    Floor,
    Ceil,
    Nearest,
}

impl RoundingMode {
    fn apply(self, value: Decimal) -> Result<u64, MathError> {
        if value.is_sign_negative() && !value.is_zero() {
            return Err(MathError::Underflow);
        }
        let rounded = match self {
            RoundingMode::Floor => value.floor(),
            RoundingMode::Ceil => value.ceil(),
            RoundingMode::Nearest => {
                value.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
            }
        };
        rounded.to_u64().ok_or(MathError::Overflow)
    }
}

fn exact(x: f64) -> Result<Decimal, MathError> {
    if !x.is_finite() {
        return Err(MathError::Overflow);
    }
    if x < 0.0 {
        return Err(MathError::Underflow);
    }
    Decimal::from_str(&format!("{x}")).map_err(|_| MathError::Overflow)
}

fn checked_pow10(exp: u32) -> Option<Decimal> {
    10i128
        .checked_pow(exp)
        .and_then(|pow| Decimal::try_from_i128_with_scale(pow, 0).ok())
}

fn scale_pow10(value: Decimal, exp: i32) -> Result<Decimal, MathError> {
    let factor = checked_pow10(exp.unsigned_abs()).ok_or(MathError::Overflow)?;
    if exp >= 0 {
        value.checked_mul(factor)
    } else {
        value.checked_div(factor)
    }
    .ok_or(MathError::Overflow)
}

fn quote_lots_floor(value: Decimal) -> Result<QuoteLots, MathError> {
    let lots = scale_pow10(value, QUOTE_LOT_DECIMALS as i32)?;
    QuoteLots::new_checked(RoundingMode::Floor.apply(lots)?)
}

/// Convert quote units (e.g. USD) → quote lots, rounding down.
pub fn quote_usd_to_quote_lots(usd: f64) -> Result<QuoteLots, MathError> {
    quote_lots_floor(exact(usd)?)
}

/// Parse a non-negative decimal string of quote units (e.g. USD) into quote
/// lots, rounding down.
pub fn quote_lots_from_decimal_str(value: &str) -> Result<QuoteLots, MathError> {
    let value = value.trim();
    if !value
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'+' | b'-'))
    {
        return Err(MathError::Overflow);
    }
    let parsed = Decimal::from_str_exact(value).map_err(|_| MathError::Overflow)?;
    quote_lots_floor(parsed)
}

/// Utility struct that encapsulates per-market math conversions.
///
/// ## Field semantics
/// - `base_lot_decimals`: exponent translating 1 *human* base unit into
///   `10^base_lot_decimals` base lots. Negative values mean each base lot is a
///   bundle of units (e.g. `-4` ⇒ 1 lot = 10_000 units).
/// - `quote_lot_decimals`: exponent for quote lots; currently fixed to 6
///   (micro‑USD).
/// - `tick_size`: `QuoteLotsPerBaseLotPerTick`, the quote-lot value of one tick
///   for a single base lot.
///
/// ## Handy formulas (implemented as methods)
/// - Price → ticks: `ticks = (price * 10^{quote_dec}) / (tick_size *
///   10^{base_dec})` → `price_to_ticks`.
/// - Ticks → price: `price = ticks * tick_size * 10^{base_dec} /
///   10^{quote_dec}` → `ticks_to_price`.
/// - Base units → lots: `lots = base_units * 10^{base_dec}` (or divide when
///   `base_dec` is negative) → `base_units_to_lots`.
/// - Quote USD → quote lots: `ql = usd * 10^{quote_dec}` →
///   `quote_usd_to_quote_lots`.
/// - Quote budget → base lots at price: `base_lots = (quote_usd / price) *
///   10^{base_dec}` → `quote_budget_to_base_lots`.
/// - Quote units → ticks (alias for price-to-ticks): `quote_units_to_ticks`.
/// - Depth density (lots per tick) → base units per quote unit:
///   `base_lots_density_to_f64`.
///
/// ## Example
/// ```
/// use phoenix_rise_math::{
///     MarketCalculator, QuoteLotsPerBaseLotPerTick, RoundingMode, quote_usd_to_quote_lots,
/// };
///
/// let calc = MarketCalculator::new(4, QuoteLotsPerBaseLotPerTick::new(100)); // BTC-like
/// let ticks = calc.price_to_ticks(50_000.0).unwrap();
/// let lots = calc
///     .base_units_to_lots(0.25, RoundingMode::Nearest)
///     .unwrap();
/// let usd = calc.quote_lots_to_usd(quote_usd_to_quote_lots(123.45).unwrap());
/// ```
///
/// All helpers perform bounds/finite checks and return `MathError` on overflow,
/// underflow, or invalid input, so call sites stay lightweight.
#[derive(Debug, Clone, Copy)]
pub struct MarketCalculator {
    pub base_lot_decimals: i8,
    pub quote_lot_decimals: u8,
    pub tick_size: QuoteLotsPerBaseLotPerTick,
}

impl MarketCalculator {
    pub fn new(base_lot_decimals: i8, tick_size: QuoteLotsPerBaseLotPerTick) -> Self {
        Self {
            base_lot_decimals,
            quote_lot_decimals: QUOTE_LOT_DECIMALS,
            tick_size,
        }
    }

    fn base_lots_per_base_unit(&self) -> f64 {
        10f64.powi(self.base_lot_decimals as i32)
    }

    fn quote_lots_per_quote_unit(&self) -> f64 {
        10f64.powi(self.quote_lot_decimals as i32)
    }

    fn pow10_decimal(exp: i32) -> rust_decimal::Decimal {
        match (checked_pow10(exp.unsigned_abs()), exp >= 0) {
            (Some(factor), true) => factor,
            (Some(factor), false) => Decimal::ONE / factor,
            (None, true) => Decimal::MAX,
            (None, false) => Decimal::ZERO,
        }
    }

    fn base_lots_per_base_unit_decimal(&self) -> rust_decimal::Decimal {
        Self::pow10_decimal(self.base_lot_decimals as i32)
    }

    fn quote_lots_per_quote_unit_decimal(&self) -> rust_decimal::Decimal {
        Self::pow10_decimal(self.quote_lot_decimals as i32)
    }

    /// Convert ticks → price in quote units (e.g. USD).
    pub fn ticks_to_price(&self, ticks: Ticks) -> f64 {
        self.ticks_f64_to_price(ticks.as_inner() as f64)
    }

    /// Convert signed ticks → price difference in quote units (e.g. USD).
    /// Used for values like EMA of price differences that can be negative.
    pub fn signed_ticks_to_price_diff(&self, ticks: SignedTicks) -> f64 {
        self.ticks_f64_to_price(ticks.as_inner() as f64)
    }

    fn ticks_f64_to_price(&self, ticks: f64) -> f64 {
        let tick_size_f = self.tick_size.as_inner() as f64;
        ticks * tick_size_f * self.base_lots_per_base_unit() / self.quote_lots_per_quote_unit()
    }

    pub fn ticks_to_decimal(&self, ticks: Ticks) -> rust_decimal::Decimal {
        let ticks_dec = rust_decimal::Decimal::from(ticks.as_inner());
        (ticks_dec
            * rust_decimal::Decimal::from(self.tick_size.as_inner())
            * self.base_lots_per_base_unit_decimal())
            / self.quote_lots_per_quote_unit_decimal()
    }

    /// Convert price in quote units (e.g. USD) → ticks (rounded to nearest).
    pub fn price_to_ticks(&self, price: f64) -> Result<Ticks, MathError> {
        self.price_to_ticks_rounded(price, RoundingMode::Nearest)
    }

    /// Convert price in quote units (e.g. USD) → ticks with selectable
    /// rounding.
    pub fn price_to_ticks_rounded(
        &self,
        price: f64,
        rounding: RoundingMode,
    ) -> Result<Ticks, MathError> {
        if price <= 0.0 || !price.is_finite() {
            return Err(MathError::Underflow);
        }
        if self.tick_size.as_inner() == 0 {
            return Err(MathError::DivisionByZero);
        }
        let scaled = scale_pow10(
            exact(price)?,
            self.quote_lot_decimals as i32 - self.base_lot_decimals as i32,
        )?;
        let ticks = scaled
            .checked_div(Decimal::from(self.tick_size.as_inner()))
            .ok_or(MathError::Overflow)?;
        Ticks::new_checked(rounding.apply(ticks)?).map_err(|_| MathError::Overflow)
    }

    /// Convert an order price in quote units (e.g. USD) → ticks. Bids round
    /// down and asks round up; a price that resolves to zero ticks is an
    /// error.
    pub fn order_price_to_ticks(&self, price: f64, side: Side) -> Result<Ticks, MathError> {
        let rounding = match side {
            Side::Bid => RoundingMode::Floor,
            Side::Ask => RoundingMode::Ceil,
        };
        let ticks = self.price_to_ticks_rounded(price, rounding)?;
        if ticks.as_inner() == 0 {
            return Err(MathError::Underflow);
        }
        Ok(ticks)
    }

    /// Synonym for `price_to_ticks` when callers conceptually start from a
    /// quote amount (quote units per base unit).
    pub fn quote_units_to_ticks(&self, quote_units: f64) -> Result<Ticks, MathError> {
        self.price_to_ticks(quote_units)
    }

    /// Convert human-sized base units → base lots with selectable rounding.
    pub fn base_units_to_lots(
        &self,
        base_units: f64,
        rounding: RoundingMode,
    ) -> Result<BaseLots, MathError> {
        if base_units <= 0.0 || !base_units.is_finite() {
            return Err(MathError::Underflow);
        }
        self.base_units_decimal_to_lots(exact(base_units)?, rounding)
    }

    fn base_units_decimal_to_lots(
        &self,
        base_units: Decimal,
        rounding: RoundingMode,
    ) -> Result<BaseLots, MathError> {
        let lots = scale_pow10(base_units, self.base_lot_decimals as i32)?;
        BaseLots::new_checked(rounding.apply(lots)?)
    }

    /// Convert base lots back to human base units.
    pub fn base_lots_to_units(&self, lots: BaseLots) -> f64 {
        let divisor = self.base_lots_per_base_unit();
        if divisor == 0.0 {
            return 0.0;
        }
        lots.as_inner() as f64 / divisor
    }

    /// Convert signed base lots back to human base units (preserves sign).
    pub fn signed_base_lots_to_units(&self, lots: SignedBaseLots) -> f64 {
        let divisor = self.base_lots_per_base_unit();
        if divisor == 0.0 {
            return 0.0;
        }
        lots.as_inner() as f64 / divisor
    }

    pub fn base_lots_to_decimal(&self, lots: BaseLots) -> rust_decimal::Decimal {
        let lots_dec = rust_decimal::Decimal::from(lots.as_inner());
        let denom = self.base_lots_per_base_unit_decimal();
        if denom.is_zero() {
            rust_decimal::Decimal::ZERO
        } else {
            lots_dec / denom
        }
    }

    /// Convert quote lots → quote units (e.g. USD).
    pub fn quote_lots_to_usd(&self, lots: QuoteLots) -> f64 {
        lots.as_inner() as f64 / self.quote_lots_per_quote_unit()
    }

    pub fn signed_quote_lots_to_usd(&self, lots: SignedQuoteLots) -> f64 {
        lots.as_inner() as f64 / self.quote_lots_per_quote_unit()
    }

    pub fn quote_lots_to_decimal(&self, lots: QuoteLots) -> rust_decimal::Decimal {
        let lots_dec = rust_decimal::Decimal::from(lots.as_inner());
        lots_dec / self.quote_lots_per_quote_unit_decimal()
    }

    pub fn signed_quote_lots_to_decimal(&self, lots: SignedQuoteLots) -> rust_decimal::Decimal {
        let lots_dec = rust_decimal::Decimal::from(lots.as_inner());
        lots_dec / self.quote_lots_per_quote_unit_decimal()
    }

    /// Given a quote budget and price, compute base lots purchasable.
    pub fn quote_budget_to_base_lots(
        &self,
        quote_usd: f64,
        price: f64,
        rounding: RoundingMode,
    ) -> Result<BaseLots, MathError> {
        if quote_usd <= 0.0 || price <= 0.0 {
            return Err(MathError::Underflow);
        }
        let base_units = exact(quote_usd)?
            .checked_div(exact(price)?)
            .ok_or(MathError::Overflow)?;
        self.base_units_decimal_to_lots(base_units, rounding)
    }

    /// USD-per-tick multiplier (helpful for formatting book depths).
    pub fn tick_size_multiplier(&self) -> f64 {
        self.quote_lots_per_quote_unit()
            / (self.base_lots_per_base_unit() * self.tick_size.as_inner() as f64)
    }

    pub fn tick_size_multiplier_decimal(&self) -> rust_decimal::Decimal {
        let denom =
            self.base_lots_per_base_unit_decimal() * Decimal::from(self.tick_size.as_inner());
        if denom.is_zero() {
            Decimal::ZERO
        } else {
            self.quote_lots_per_quote_unit_decimal() / denom
        }
    }

    /// Convert a base-lot density (lots per tick) into a human-friendly ratio
    /// of base units per quote unit.
    pub fn base_lots_density_to_f64(&self, base_lots_density: BaseLotsPerTick) -> f64 {
        let base_units_per_tick =
            self.base_lots_to_units(BaseLots::new(base_lots_density.as_inner()));
        let dollars_per_tick = self.ticks_to_price(Ticks::new(1));
        if dollars_per_tick == 0.0 {
            0.0
        } else {
            base_units_per_tick / dollars_per_tick
        }
    }

    pub fn base_lots_density_to_decimal(
        &self,
        base_lots_density: BaseLotsPerTick,
    ) -> rust_decimal::Decimal {
        let base_units_per_tick =
            self.base_lots_to_decimal(BaseLots::new(base_lots_density.as_inner()));
        let dollars_per_tick = self.ticks_to_decimal(Ticks::new(1));
        if dollars_per_tick.is_zero() {
            Decimal::ZERO
        } else {
            base_units_per_tick / dollars_per_tick
        }
    }

    /// Calculate price in USD from base lots and quote lots.
    /// Returns the price per base unit in USD.
    /// Returns 0.0 if base_lots is zero to avoid division by zero.
    pub fn price_from_lots(&self, base_lots: BaseLots, quote_lots: QuoteLots) -> f64 {
        let base_units = self.base_lots_to_units(base_lots);
        if base_units == 0.0 {
            return 0.0;
        }
        let quote_usd = self.quote_lots_to_usd(quote_lots);
        quote_usd / base_units
    }

    /// Value of a signed base-lot position at the given settlement price in
    /// quote lots.
    pub fn position_value_for_position(
        &self,
        base_lot_position: SignedBaseLots,
        settlement_price: Ticks,
    ) -> SignedQuoteLots {
        let sign = SignedQuoteLots::new(base_lot_position.signum().as_inner());
        let absolute_base_lots = base_lot_position.abs_as_unsigned();
        let unsigned_value = absolute_base_lots * (self.tick_size * settlement_price);
        let signed_value = unsigned_value
            .checked_as_signed()
            // tick_size and settlement_price are bounded and absolute_base_lots is u64,
            // so overflow here would signal a misconfigured market
            .expect("quote lot value fits in SignedQuoteLots");
        signed_value * sign
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantities::QuoteLotsPerBaseLotPerTick;

    #[test]
    fn price_tick_round_trip_positive_decimals() {
        let calc = MarketCalculator::new(4, QuoteLotsPerBaseLotPerTick::new(100));
        let price = 42_000.25;
        let ticks = calc.price_to_ticks(price).unwrap();
        let back = calc.ticks_to_price(ticks);
        assert!((back - price).abs() < 1.0);
    }

    #[test]
    fn quote_units_to_ticks_matches_price_to_ticks() {
        let calc = MarketCalculator::new(2, QuoteLotsPerBaseLotPerTick::new(50));
        let price = 123.45;
        let a = calc.price_to_ticks(price).unwrap();
        let b = calc.quote_units_to_ticks(price).unwrap();
        assert_eq!(a.as_inner(), b.as_inner());
    }

    #[test]
    fn base_lots_density_conversion_behaves() {
        let calc = MarketCalculator::new(3, QuoteLotsPerBaseLotPerTick::new(100));
        let density = BaseLotsPerTick::new(500); // 0.5 base units per tick
        let ratio = calc.base_lots_density_to_f64(density);
        assert!(ratio > 0.0);
    }

    #[test]
    fn position_value_matches_sign() {
        let calc = MarketCalculator::new(2, QuoteLotsPerBaseLotPerTick::new(100));
        let price_ticks = Ticks::new(10); // arbitrary
        let long_val = calc
            .position_value_for_position(SignedBaseLots::new(5), price_ticks)
            .as_inner();
        let short_val = calc
            .position_value_for_position(SignedBaseLots::new(-5), price_ticks)
            .as_inner();
        assert_eq!(long_val, -(short_val));
        assert!(long_val > 0);
    }

    #[test]
    fn price_tick_round_trip_negative_decimals() {
        let calc = MarketCalculator::new(-4, QuoteLotsPerBaseLotPerTick::new(1));
        let price = 0.00001146;
        let ticks = calc.price_to_ticks(price).unwrap();
        let back = calc.ticks_to_price(ticks);
        assert!((back - price).abs() < 1e-12);
    }

    #[test]
    fn base_unit_conversions_negative_decimals() {
        let calc = MarketCalculator::new(-4, QuoteLotsPerBaseLotPerTick::new(1));
        let base_units = 20_000.0;
        let lots = calc
            .base_units_to_lots(base_units, RoundingMode::Nearest)
            .unwrap();
        assert_eq!(lots.as_inner(), 2);
        let units_back = calc.base_lots_to_units(lots);
        assert!((units_back - 20_000.0).abs() < 1e-6);
    }

    #[test]
    fn quote_budget_to_lots_matches_expectation() {
        // 1000.0 / 0.00001146 = 87260034.904
        // 87260034.904 / e-4 = 8726
        let calc = MarketCalculator::new(-4, QuoteLotsPerBaseLotPerTick::new(1));
        let lots = calc
            .quote_budget_to_base_lots(1000.0, 0.00001146, RoundingMode::Floor)
            .unwrap();
        println!("lots: {:?}", lots.as_inner());
        assert_eq!(lots.as_inner(), 8726);
    }

    #[test]
    fn btc_mainnet_config_behaves_as_expected() {
        let calc = MarketCalculator::new(4, QuoteLotsPerBaseLotPerTick::new(100));
        let price = 50_000.0;
        let ticks = calc.price_to_ticks(price).unwrap();
        assert!(ticks.as_inner() > 0);
        let price_back = calc.ticks_to_price(ticks);
        assert!((price_back - price).abs() < 1.0);

        let lots = calc
            .base_units_to_lots(0.1234, RoundingMode::Nearest)
            .unwrap();
        assert_eq!(lots.as_inner(), 1234);

        let quote_budget_lots = calc
            .quote_budget_to_base_lots(1000.0, price, RoundingMode::Floor)
            .unwrap();
        assert_eq!(quote_budget_lots.as_inner(), 200);
    }

    #[test]
    fn eth_mainnet_config_behaves_as_expected() {
        let calc = MarketCalculator::new(3, QuoteLotsPerBaseLotPerTick::new(100));
        let price = 3_500.0;
        let ticks = calc.price_to_ticks(price).unwrap();
        let price_back = calc.ticks_to_price(ticks);
        assert!((price_back - price).abs() < 0.5);

        let lots = calc.base_units_to_lots(1.5, RoundingMode::Nearest).unwrap();
        assert_eq!(lots.as_inner(), 1500);
    }

    #[test]
    fn sol_mainnet_config_behaves_as_expected() {
        let calc = MarketCalculator::new(2, QuoteLotsPerBaseLotPerTick::new(100));
        let price = 150.0;
        let ticks = calc.price_to_ticks(price).unwrap();
        let back = calc.ticks_to_price(ticks);
        assert!((back - price).abs() < 0.05);

        let lots = calc
            .base_units_to_lots(25.25, RoundingMode::Nearest)
            .unwrap();
        assert_eq!(lots.as_inner(), 2525);
    }

    fn calc(bl_dec: i8, tick_q_per_bl_per_tick: u64) -> MarketCalculator {
        MarketCalculator::new(
            bl_dec,
            QuoteLotsPerBaseLotPerTick::new(tick_q_per_bl_per_tick),
        )
    }

    #[test]
    fn base_units_round_trip_various() {
        let c = calc(3, 10);
        let size = 1.2345;
        let lots = c.base_units_to_lots(size, RoundingMode::Nearest).unwrap();
        let size_back = c.base_lots_to_units(lots);
        assert!((size_back - size).abs() < 0.001);
    }

    #[test]
    fn quote_lots_round_trip() {
        let c = calc(4, 100);
        let usd = 123.456789;
        let ql = quote_usd_to_quote_lots(usd).unwrap();
        let usd_back = c.quote_lots_to_usd(ql);
        assert!((usd_back - usd).abs() < 0.000001);
    }

    #[test]
    fn rounding_behavior_base_units() {
        let c = calc(2, 1);
        let size = 1.7;
        let r = c.base_units_to_lots(size, RoundingMode::Nearest).unwrap();
        assert_eq!(r.as_inner(), 170);

        let size_frac = 1.234;
        let r_frac = c
            .base_units_to_lots(size_frac, RoundingMode::Nearest)
            .unwrap();
        assert_eq!(r_frac.as_inner(), 123);

        let size_up = 1.236;
        let r_up = c
            .base_units_to_lots(size_up, RoundingMode::Nearest)
            .unwrap();
        assert_eq!(r_up.as_inner(), 124);
    }

    #[test]
    fn exact_conversions_and_order_price_rounding() {
        for d in 0..=6i8 {
            let c = calc(d, 1);
            let scale = 10f64.powi(d as i32);
            for n in 1..=100_000u64 {
                let lots = c
                    .base_units_to_lots(n as f64 / scale, RoundingMode::Floor)
                    .unwrap();
                assert_eq!(lots.as_inner(), n, "d={d} n={n}");
            }
        }
        for n in 1..=100_000u64 {
            let two_dp = quote_usd_to_quote_lots(n as f64 / 100.0).unwrap();
            assert_eq!(two_dp.as_inner(), n * 10_000, "n={n}");
            let six_dp = quote_usd_to_quote_lots(n as f64 / 1_000_000.0).unwrap();
            assert_eq!(six_dp.as_inner(), n, "n={n}");
        }

        let c = calc(2, 100);
        for n in 1..=100_000u64 {
            let price = n as f64 / 100.0;
            let ticks = [
                c.price_to_ticks(price),
                c.price_to_ticks_rounded(price, RoundingMode::Floor),
                c.price_to_ticks_rounded(price, RoundingMode::Ceil),
                c.order_price_to_ticks(price, Side::Bid),
                c.order_price_to_ticks(price, Side::Ask),
            ];
            for t in ticks {
                assert_eq!(t.unwrap().as_inner(), n, "n={n}");
            }
        }
        for (price, bid, ask, nearest) in [
            (135.87, 13587, 13587, 13587),
            (135.874, 13587, 13588, 13587),
            (135.875, 13587, 13588, 13588),
        ] {
            let ticks = |side| c.order_price_to_ticks(price, side).unwrap().as_inner();
            assert_eq!(ticks(Side::Bid), bid, "price={price}");
            assert_eq!(ticks(Side::Ask), ask, "price={price}");
            assert_eq!(c.price_to_ticks(price).unwrap().as_inner(), nearest);
        }
        assert!(matches!(
            c.order_price_to_ticks(0.001, Side::Bid),
            Err(MathError::Underflow)
        ));
        assert_eq!(
            c.order_price_to_ticks(0.001, Side::Ask).unwrap().as_inner(),
            1
        );

        let parse = |s| quote_lots_from_decimal_str(s).map(|lots| lots.as_inner());
        assert_eq!(parse("1000.5").unwrap(), 1_000_500_000);
        assert_eq!(parse("0.0000019").unwrap(), 1);
        for bad in [
            "-1",
            "NaN",
            "abc",
            "1e3",
            "1_000",
            "99999999999999999999999999",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn invalid_inputs_should_error() {
        let c = calc(4, 100);
        assert!(c.price_to_ticks(-1.0).is_err());
        assert!(c.base_units_to_lots(-0.5, RoundingMode::Nearest).is_err());
        assert!(quote_usd_to_quote_lots(-10.0).is_err());
        assert!(c.price_to_ticks(f64::NAN).is_err());
        assert!(
            c.base_units_to_lots(f64::INFINITY, RoundingMode::Nearest)
                .is_err()
        );
    }

    #[test]
    fn zero_and_tiny_inputs() {
        let c = calc(4, 100);
        assert!(c.base_units_to_lots(0.0, RoundingMode::Nearest).is_err());
        assert_eq!(quote_usd_to_quote_lots(0.0).unwrap().as_inner(), 0);
        let tiny = 0.000001;
        assert!(c.price_to_ticks(tiny).is_ok());
        assert!(c.base_units_to_lots(tiny, RoundingMode::Nearest).is_ok());
        assert!(quote_usd_to_quote_lots(tiny).is_ok());
    }

    #[test]
    fn extreme_prices_and_quantities_behave() {
        let c = calc(6, 1000);
        let price = 100_000_000.0;
        let ticks = c.price_to_ticks(price).unwrap();
        let back = c.ticks_to_price(ticks);
        let rel_err = (back - price).abs() / price;
        assert!(rel_err < 1e-6);

        let big_price = 9_007_199_254_740.0;
        assert!(c.price_to_ticks(big_price).is_err());

        let doge = calc(8, 1);
        let near_max_qty = 42.0;
        let lots = doge
            .base_units_to_lots(near_max_qty, RoundingMode::Nearest)
            .unwrap();
        let back_units = doge.base_lots_to_units(lots);
        assert!((back_units - near_max_qty).abs() / near_max_qty < 1e-8);

        let over_max_qty = 50.0;
        assert!(
            doge.base_units_to_lots(over_max_qty, RoundingMode::Nearest)
                .is_err()
        );
    }

    #[test]
    fn cross_product_precision() {
        let c = calc(6, 1000);
        let price = 50_000.0;
        let qty = 1_000.0;
        let ticks = c.price_to_ticks(price).unwrap();
        let lots = c.base_units_to_lots(qty, RoundingMode::Nearest).unwrap();

        let back_price = c.ticks_to_price(ticks);
        let back_qty = c.base_lots_to_units(lots);
        let notional = back_price * back_qty;
        let expected = price * qty;
        assert!((notional - expected).abs() / expected < 1e-6);
    }

    #[test]
    fn signed_ticks_to_price_diff_matches_unsigned_for_positive() {
        // BTC config: 10^4 base lots per BTC, tick_size=100
        let c = MarketCalculator::new(4, QuoteLotsPerBaseLotPerTick::new(100));
        let unsigned_ticks = Ticks::new(1000);
        let signed_ticks = SignedTicks::new(1000);

        let unsigned_price = c.ticks_to_price(unsigned_ticks);
        let signed_price = c.signed_ticks_to_price_diff(signed_ticks);

        assert!((unsigned_price - signed_price).abs() < 1e-12);
    }

    #[test]
    fn signed_ticks_to_price_diff_negative() {
        // SOL config: 10^2 base lots per SOL, tick_size=100
        // At this config, 1 tick = $0.01, so 500 ticks = $5.00
        let c = MarketCalculator::new(2, QuoteLotsPerBaseLotPerTick::new(100));
        let positive = SignedTicks::new(500);
        let negative = SignedTicks::new(-500);

        let pos_price = c.signed_ticks_to_price_diff(positive);
        let neg_price = c.signed_ticks_to_price_diff(negative);

        assert!(pos_price > 0.0);
        assert!(neg_price < 0.0);
        assert!((pos_price + neg_price).abs() < 1e-12);
        // Verify actual dollar value
        assert!((pos_price - 5.0).abs() < 1e-10);
    }

    #[test]
    fn signed_ticks_to_price_diff_zero() {
        let c = MarketCalculator::new(4, QuoteLotsPerBaseLotPerTick::new(100));
        let zero = SignedTicks::new(0);
        assert_eq!(c.signed_ticks_to_price_diff(zero), 0.0);
    }
}
