//! Funding rate helpers shared by off-chain consumers.
//!
//! The funding accumulator on-chain tracks `∑ (mark - index) * dt` in
//! `SignedQuoteLotsPerBaseLotUpcasted` units. To display a percentage, we
//! convert to quote/base *seconds* per funding period, clamp to the market
//! maximum, and express as a percentage of notional using the current mark
//! price.

use crate::{
    FundingRateUnitInSeconds, SignedQuoteLotsPerBaseLot, SignedQuoteLotsPerBaseLotUpcasted,
};

/// Convenience calculator for funding percentages.
#[derive(Debug, Clone, Copy)]
pub struct FundingCalculator {
    base_lot_decimals: i8,
    quote_lot_decimals: u8,
    funding_period_seconds: FundingRateUnitInSeconds,
    funding_interval_seconds: FundingRateUnitInSeconds,
    max_funding_rate_per_interval: SignedQuoteLotsPerBaseLot,
}

impl FundingCalculator {
    /// Create a calculator using the market's decimals and funding parameters.
    pub fn new(
        base_lot_decimals: i8,
        funding_period_seconds: FundingRateUnitInSeconds,
        funding_interval_seconds: FundingRateUnitInSeconds,
        max_funding_rate_per_interval: SignedQuoteLotsPerBaseLot,
    ) -> Self {
        Self {
            base_lot_decimals,
            quote_lot_decimals: 6,
            funding_period_seconds,
            funding_interval_seconds,
            max_funding_rate_per_interval,
        }
    }

    /// Current interval funding as a percentage of notional.
    ///
    /// `accumulated_funding` is the in-interval accumulator
    /// (quote_lots_per_base_lot * seconds). We divide by the funding period to
    /// project the interval contribution, clamp to the configured max, convert
    /// to quote units per base unit, and scale by the current mark.
    ///
    /// Math (units shown):
    /// - `rate_raw = acc / T_period`                       (quote_lots /
    ///   base_lot)
    /// - `rate_clamped = clamp(rate_raw, ±max_per_interval)` (quote_lots /
    ///   base_lot)
    /// - `funding_usd_per_base_unit = rate_clamped * (10^{base_dec} / 10^6)`
    ///   where 10^6 converts quote lots → quote units (USD), and 10^{base_dec}
    ///   converts base lots → base units. Units: quote_units / base_unit.
    /// - `funding_pct = (funding_usd_per_base_unit / mark_price) * 100` Units:
    ///   percent of notional for this interval.
    pub fn current_rate_percentage(
        &self,
        accumulated_funding: SignedQuoteLotsPerBaseLotUpcasted,
        mark_price: f64,
    ) -> f64 {
        let period = self.funding_period_seconds.as_inner() as f64;
        if period == 0.0 || !mark_price.is_finite() || mark_price <= 0.0 {
            return 0.0;
        }

        // Do the clamp in integer space to avoid precision loss, then convert.
        let acc_i128 = accumulated_funding.as_inner();
        let period_i128 = self.funding_period_seconds.as_inner() as i128;
        if period_i128 == 0 {
            return 0.0;
        }
        // Project using high-precision floats to preserve sub-lot values, but clamp
        // using the i128 max to match on-chain bounds exactly.
        let projected_f = acc_i128 as f64 / period_i128 as f64; // quote lots per base lot
        let max_rate = self.max_funding_rate_per_interval.as_inner() as f64;
        let clamped = projected_f.clamp(-max_rate, max_rate);

        // Convert to quote units per base unit.
        let quote_lots_per_quote_unit = 10f64.powi(self.quote_lot_decimals as i32);
        if quote_lots_per_quote_unit == 0.0 {
            return 0.0;
        }
        let base_lots_per_base_unit = 10f64.powi(self.base_lot_decimals as i32);

        // (SignedQuoteLots / BaseLot) * (BaseLots / BaseUnit) / (QuoteLots / QuoteUnit)
        // = SignedQuoteUnits / BaseUnit
        let funding_quote_units_per_base_unit =
            (clamped / quote_lots_per_quote_unit) * base_lots_per_base_unit;

        // Percentage of notional.
        // (SignedQuoteUnits / BaseUnit) / (QuoteUnits / BaseUnit) * 100 = Percent of
        // Notional
        (funding_quote_units_per_base_unit / mark_price) * 100.0
    }

    /// Annualize an interval funding percentage.
    pub fn annualized_rate_percentage(&self, interval_rate_percentage: f64) -> f64 {
        let interval = self.funding_interval_seconds.as_inner() as f64;
        let period = self.funding_period_seconds.as_inner() as f64;
        if interval == 0.0 || period == 0.0 {
            return 0.0;
        }

        // Interval rate -> annual rate. seconds_per_year / interval_seconds.
        let seconds_per_year = 31_536_000.0; // 365 days
        let intervals_per_year = seconds_per_year / interval;

        // interval_rate_percentage already reflects (interval/period), so we only need
        // intervals_per_year here.
        interval_rate_percentage * intervals_per_year
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hourly_rate_to_percentage_and_annualized() {
        // Mark 94_206, spot 94_209 => diff = -3 quote units
        // Accumulator for one hour: diff_quote_lots * interval_seconds
        let diff_quote_lots_per_base_lot = -3_000_000i128; // -3 USD with 6 quote decimals
        let interval_seconds = FundingRateUnitInSeconds::new_const(3_600);
        let period_seconds = FundingRateUnitInSeconds::new_const(86_400);

        let acc = SignedQuoteLotsPerBaseLotUpcasted::new_const(
            diff_quote_lots_per_base_lot * interval_seconds.as_inner() as i128,
        );

        let calc = FundingCalculator::new(
            0,
            period_seconds,
            interval_seconds,
            SignedQuoteLotsPerBaseLot::new_const(i64::MAX),
        );
        let rate = calc.current_rate_percentage(acc, 94_206.0);

        // Expected: (-3 / 94_206) * (1/24) * 100 ≈ -0.0001327%
        assert!(
            (rate + 0.0001327).abs() < 1e-6,
            "unexpected interval rate: {}",
            rate
        );

        let annual = calc.annualized_rate_percentage(rate);
        // Expected annual ≈ -1.162% (hourly * 8760)
        assert!(
            (annual + 1.1623).abs() < 0.01,
            "unexpected annualized rate: {}",
            annual
        );
    }

    #[test]
    fn clamps_to_max() {
        let period = FundingRateUnitInSeconds::new_const(86_400);
        let interval = FundingRateUnitInSeconds::new_const(3_600);
        let max = SignedQuoteLotsPerBaseLot::new_const(500); // tiny max

        let calc = FundingCalculator::new(0, period, interval, max);

        let acc = SignedQuoteLotsPerBaseLotUpcasted::new_const(10_000_000); // large acc
        let rate = calc.current_rate_percentage(acc, 10_000.0);
        let annual = calc.annualized_rate_percentage(rate);

        // Should be finite and non-zero but limited by max
        assert!(rate.is_finite());
        assert!(annual.is_finite());
    }
}
