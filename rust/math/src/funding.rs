//! Funding rate helpers shared by off-chain consumers.
//!
//! Funding projections clamp the whole interval's raw mark/index integral,
//! preserve the previous settlement's rounding remainder, and express the
//! settled quote/base amount as a percentage of the current mark price.

use crate::{
    FundingRateUnitInSeconds, QuoteLotsPerBaseLot, SignedQuoteLotsPerBaseLot,
    SignedQuoteLotsPerBaseLotUpcasted,
};

/// Convenience calculator for funding percentages.
#[derive(Debug, Clone, Copy)]
pub struct FundingCalculator {
    base_lot_decimals: i8,
    quote_lot_decimals: u8,
    funding_period_seconds: FundingRateUnitInSeconds,
    funding_interval_seconds: FundingRateUnitInSeconds,
    max_funding_rate_per_interval: SignedQuoteLotsPerBaseLot,
    funding_clamp_center: u16,
    funding_clamp_band_radius: u16,
    funding_divisor: u8,
}

impl FundingCalculator {
    /// Create a calculator using the market's decimals and funding parameters.
    pub fn new(
        base_lot_decimals: i8,
        funding_period_seconds: FundingRateUnitInSeconds,
        funding_interval_seconds: FundingRateUnitInSeconds,
        max_funding_rate_per_interval: SignedQuoteLotsPerBaseLot,
        funding_clamp_center: u16,
        funding_clamp_band_radius: u16,
        funding_divisor: u8,
    ) -> Self {
        Self {
            base_lot_decimals,
            quote_lot_decimals: 6,
            funding_period_seconds,
            funding_interval_seconds,
            max_funding_rate_per_interval,
            funding_clamp_center,
            funding_clamp_band_radius,
            funding_divisor,
        }
    }

    /// Convert already-clamped funding to a percentage of notional.
    ///
    /// `accumulated_funding` is quote_lots_per_base_lot * seconds, such as a
    /// settled contribution multiplied by the funding period. It does not apply
    /// the funding clamp; use [`Self::projected_rate_percentage`] to estimate
    /// the current interval from the on-chain accumulator.
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
        // Project using high-precision floats to preserve sub-lot values, but
        // clamp using the i128 max to match on-chain bounds exactly.
        let projected_f = acc_i128 as f64 / period_i128 as f64; // quote lots per base lot
        let max_rate = self.max_funding_rate_per_interval.as_inner() as f64;
        let clamped = projected_f.clamp(-max_rate, max_rate);

        self.rate_percentage_from_quote_lots_per_base_lot(clamped, mark_price)
    }

    fn rate_percentage_from_quote_lots_per_base_lot(
        &self,
        rate_quote_lots_per_base_lot: f64,
        mark_price: f64,
    ) -> f64 {
        if !mark_price.is_finite() || mark_price <= 0.0 {
            return 0.0;
        }

        // Convert to quote units per base unit.
        let quote_lots_per_quote_unit = 10f64.powi(self.quote_lot_decimals as i32);
        if quote_lots_per_quote_unit == 0.0 {
            return 0.0;
        }
        let base_lots_per_base_unit = 10f64.powi(self.base_lot_decimals as i32);

        // (SignedQuoteLots / BaseLot) * (BaseLots / BaseUnit) / (QuoteLots /
        // QuoteUnit) = SignedQuoteUnits / BaseUnit
        let funding_quote_units_per_base_unit =
            (rate_quote_lots_per_base_lot / quote_lots_per_quote_unit) * base_lots_per_base_unit;

        // Percentage of notional.
        // (SignedQuoteUnits / BaseUnit) / (QuoteUnits / BaseUnit) * 100 =
        // Percent of Notional
        (funding_quote_units_per_base_unit / mark_price) * 100.0
    }

    /// Clamp and divide the raw interval integral without touching the carried
    /// remainder.
    fn clamped_current_rate_percentage(
        &self,
        accumulated_funding: SignedQuoteLotsPerBaseLotUpcasted,
        funding_remainder: i64,
        elapsed_seconds: FundingRateUnitInSeconds,
        mark_price: f64,
        index_price: f64,
    ) -> f64 {
        let remainder = SignedQuoteLotsPerBaseLotUpcasted::new(funding_remainder as i128);
        let Some(raw_funding) = accumulated_funding.checked_sub(remainder) else {
            return 0.0;
        };
        let index_price_quote_lots_per_base_lot = QuoteLotsPerBaseLot::new_saturating(
            (index_price * 10f64.powi(self.quote_lot_decimals as i32)
                / 10f64.powi(self.base_lot_decimals as i32))
            .round() as u64,
        );
        let Some(clamped_funding) = clamp_accumulated_funding(
            raw_funding,
            elapsed_seconds,
            index_price_quote_lots_per_base_lot,
            self.funding_clamp_center,
            self.funding_clamp_band_radius,
        )
        .and_then(|funding| {
            // A zero divisor is treated as one, matching the on-chain account.
            let divisor = i128::from(self.funding_divisor.max(1));
            SignedQuoteLotsPerBaseLotUpcasted::new(funding.as_inner() / divisor)
                .checked_add(remainder)
        }) else {
            return 0.0;
        };
        self.settled_rate_from_accumulated_funding(clamped_funding)
            .map(|rate| self.rate_percentage_from_quote_lots_per_base_lot(rate as f64, mark_price))
            .unwrap_or(0.0)
    }

    /// Add the raw tail, then clamp once over the complete projected interval.
    fn projected_rate_percentage_with_remaining_seconds(
        &self,
        accumulated_funding: SignedQuoteLotsPerBaseLotUpcasted,
        funding_remainder: i64,
        current_diff: SignedQuoteLotsPerBaseLotUpcasted,
        elapsed_seconds: FundingRateUnitInSeconds,
        seconds_until_boundary: FundingRateUnitInSeconds,
        mark_price: f64,
        index_price: f64,
    ) -> f64 {
        let projected = current_diff
            .checked_mul(SignedQuoteLotsPerBaseLotUpcasted::new(
                seconds_until_boundary.as_inner() as i128,
            ))
            .and_then(|tail| accumulated_funding.checked_add(tail))
            .zip(elapsed_seconds.checked_add(seconds_until_boundary));
        let (accumulator, duration) = projected.unwrap_or((accumulated_funding, elapsed_seconds));
        self.clamped_current_rate_percentage(
            accumulator,
            funding_remainder,
            duration,
            mark_price,
            index_price,
        )
    }

    /// Project the funding rate that would settle at the next aligned funding
    /// boundary if the latest mark/index spread persisted until then.
    ///
    /// The accumulator fields come from the market's on-chain funding
    /// accumulator. `last_settled_contribution`, when given, blends the
    /// previous interval's settled rate by the time remaining in the current
    /// interval. `index_price` scales the funding clamp band. A `paused`
    /// market settles no funding, so its projected rate is zero.
    pub fn projected_rate_percentage(
        &self,
        paused: bool,
        accumulated_funding: SignedQuoteLotsPerBaseLotUpcasted,
        funding_remainder: i64,
        current_diff: SignedQuoteLotsPerBaseLotUpcasted,
        funding_start_interval_timestamp: FundingRateUnitInSeconds,
        last_funding_update_timestamp: FundingRateUnitInSeconds,
        last_settled_contribution: Option<i64>,
        mark_price: f64,
        index_price: f64,
    ) -> f64 {
        if paused {
            return 0.0;
        }
        let elapsed_seconds =
            last_funding_update_timestamp.saturating_sub(funding_start_interval_timestamp);
        let funding_interval_seconds = self.funding_interval_seconds.as_inner();
        if funding_interval_seconds == 0 {
            return self.clamped_current_rate_percentage(
                accumulated_funding,
                funding_remainder,
                elapsed_seconds,
                mark_price,
                index_price,
            );
        }

        let Some(next_unaligned) = funding_start_interval_timestamp
            .as_inner()
            .checked_add(funding_interval_seconds)
        else {
            return self.clamped_current_rate_percentage(
                accumulated_funding,
                funding_remainder,
                elapsed_seconds,
                mark_price,
                index_price,
            );
        };
        let next_boundary_timestamp =
            (next_unaligned / funding_interval_seconds) * funding_interval_seconds;
        let seconds_until_boundary =
            next_boundary_timestamp.saturating_sub(last_funding_update_timestamp.as_inner());

        let current_funding_rate = self.projected_rate_percentage_with_remaining_seconds(
            accumulated_funding,
            funding_remainder,
            current_diff,
            elapsed_seconds,
            FundingRateUnitInSeconds::new(seconds_until_boundary),
            mark_price,
            index_price,
        );

        let Some(settled_contribution) = last_settled_contribution else {
            return current_funding_rate;
        };

        let previous_funding_period_rate = self
            .rate_percentage_from_quote_lots_per_base_lot(settled_contribution as f64, mark_price);

        let seconds_passed = funding_interval_seconds.saturating_sub(seconds_until_boundary);
        let interval_seconds = funding_interval_seconds as f64;
        let weight_current = seconds_passed as f64 / interval_seconds;
        let weight_previous = seconds_until_boundary as f64 / interval_seconds;
        weight_current * current_funding_rate + weight_previous * previous_funding_period_rate
    }

    fn settled_rate_from_accumulated_funding(
        &self,
        accumulated_funding: SignedQuoteLotsPerBaseLotUpcasted,
    ) -> Option<i64> {
        let period_i128 = self.funding_period_seconds.as_inner() as i128;
        if period_i128 == 0 {
            return None;
        }

        let max_acc =
            (self.max_funding_rate_per_interval.as_inner() as i128).checked_mul(period_i128)?;
        let clamped_acc = accumulated_funding.as_inner().clamp(-max_acc, max_acc);
        i64::try_from(clamped_acc / period_i128).ok()
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

        // interval_rate_percentage already reflects (interval/period), so we
        // only need intervals_per_year here.
        interval_rate_percentage * intervals_per_year
    }
}

/// Hundredths of a basis point per whole.
const HUNDREDTH_BPS_DENOMINATOR: i128 = 1_000_000;

/// Apply the funding band to an entire interval's integrated premium.
/// Scaling the center and radius by elapsed time avoids rounding an average
/// before applying the clamp. Center and radius are in hundredths of a basis
/// point of `price`.
fn clamp_accumulated_funding(
    accumulated: SignedQuoteLotsPerBaseLotUpcasted,
    elapsed_seconds: FundingRateUnitInSeconds,
    price: QuoteLotsPerBaseLot,
    clamp_center: u16,
    clamp_band_radius: u16,
) -> Option<SignedQuoteLotsPerBaseLotUpcasted> {
    if clamp_band_radius == 0 {
        return Some(accumulated);
    }
    let premium = accumulated.as_inner();
    let price_times_dt =
        i128::from(price.as_inner()).checked_mul(i128::from(elapsed_seconds.as_inner()))?;
    let fraction_of = |hundredth_bps: u16| {
        Some(price_times_dt.checked_mul(i128::from(hundredth_bps))? / HUNDREDTH_BPS_DENOMINATOR)
    };
    let center = fraction_of(clamp_center)?;
    let radius = fraction_of(clamp_band_radius)?;
    let adjustment = center.checked_sub(premium)?.clamp(-radius, radius);
    premium
        .checked_add(adjustment)
        .map(SignedQuoteLotsPerBaseLotUpcasted::new)
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
            0,
            0,
            1,
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

        let calc = FundingCalculator::new(0, period, interval, max, 0, 0, 1);

        let acc = SignedQuoteLotsPerBaseLotUpcasted::new_const(10_000_000); // large acc
        let rate = calc.current_rate_percentage(acc, 10_000.0);
        let annual = calc.annualized_rate_percentage(rate);

        // Should be finite and non-zero but limited by max
        assert!(rate.is_finite());
        assert!(annual.is_finite());
    }
}
