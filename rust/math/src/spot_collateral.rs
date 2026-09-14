//! Spot collateral valuation.
//!
//! Spot collateral is collateral held in an asset other than the quote token.
//! Native SOL is the only spot asset today. Its margin value is the balance
//! marked at the bound perp market's **index** price (not its mark price),
//! haircut by a balance-dependent discount curve:
//!
//! ```text
//! lamports_per_base_lot = 10^(decimals - base_lot_decimals)
//! price_qlpbl           = index_price_ticks * tick_size
//! notional              = floor(balance * price_qlpbl / lamports_per_base_lot)
//! retained_bps(b)       = (10000 - min_margin_discount)
//!                       - floor((max_margin_discount - min_margin_discount)
//!                               * min(b, max_global_balance) / max_global_balance)
//! discounted            = floor(notional * retained_bps / 10000)
//! ```
//!
//! Both floors are load bearing, and `retained_bps` floors the interpolation
//! term, which rounds ≤1 bps in the trader's favor. The discount denominator is
//! the exchange-wide cap while the numerator is the trader's own balance, so
//! another trader's deposit never changes your haircut.
//!
//! Spot collateral adds to effective collateral and portfolio value. It never
//! adds a margin requirement, and it is excluded from effective collateral for
//! *quote* withdrawals because it cannot back one.

use crate::quantities::{
    BasisPoints, QuoteLots, QuoteLotsPerBaseLot, QuoteLotsPerBaseLotPerTick, Ticks,
};

/// Index of native SOL's entry in the trader position map.
///
/// Native SOL predates the dedicated spot-position index range and lives in the
/// trader header extension, so it has its own reserved key.
pub const NATIVE_SOL_ASSET_INDEX: u32 = 0xFFFF_0000;

/// Basis-point denominator, i.e. 100%.
const BPS_DENOMINATOR: u64 = 10_000;

/// The exchange's configuration for one spot collateral asset, reduced to what
/// margin valuation needs.
///
/// Callers build this from the on-chain global configuration (see
/// `phoenix_rise_accounts::spot_collateral::SpotCollateralMetadata`) or from an
/// API response.
///
/// There is deliberately no "is active" flag. Every instruction that can create
/// or move a balance — `SyncNative` above all, the only way one can first
/// appear — refuses to run unless the asset is active, and neither the rollout
/// bit nor the active flag has a disable path. An inactive asset therefore
/// implies a zero balance everywhere, and a zero balance already contributes
/// nothing. **If a deactivation path is ever added on-chain, this reasoning
/// breaks and the flag has to come back.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotCollateralParams {
    /// Raw asset-index key of the asset's entry in the trader position map.
    /// This is not the perp market asset id.
    pub asset_index: u32,
    /// Spot asset symbol ("SOL" for native SOL), not the perp market symbol.
    pub symbol: String,
    /// Symbol of the perp market whose index price values this collateral.
    pub perp_symbol: String,
    /// Native-unit decimals of the asset (9 for native SOL). Balances above and
    /// in [`crate::TraderPortfolio::spot_collateral`] are raw units at this
    /// scale — lamports only when this asset *is* native SOL — which is why
    /// they are plain `u64` rather than a unit-bearing quantity type.
    pub decimals: u32,
    /// Maximum balance a single trader may hold, in the asset's native units.
    pub max_per_trader_balance: u64,
    /// Maximum balance across all traders, in the asset's native units. Also
    /// the denominator of the discount curve.
    pub max_global_balance: u64,
    /// Current balance across all traders, in the asset's native units.
    /// Informational: it does not enter margin valuation.
    pub curr_global_balance: u64,
    /// Margin discount at a zero balance.
    pub min_margin_discount: BasisPoints,
    /// Margin discount at [`Self::max_global_balance`].
    pub max_margin_discount: BasisPoints,
}

/// Why a spot collateral balance could not be valued.
///
/// Callers should treat this as "worth zero for now" rather than a hard error:
/// under-valuing is the conservative direction, and any on-chain action that
/// depended on the price would revert anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotCollateralValuationError {
    /// No index price was supplied, or it was zero. Rise cannot derive the
    /// index price itself — it is a median over the market's oracles — so it
    /// must be provided, and must never silently fall back to the mark price.
    MissingIndexPrice,
    /// The asset has fewer decimals than the perp market's base lot, so
    /// lamports cannot be converted into base lots.
    NegativeDecimalsDifference {
        decimals: u32,
        base_lot_decimals: i8,
    },
    /// The decimals gap does not fit a `u64` power of ten.
    DecimalsDifferenceTooLarge { difference: u32 },
    /// The configured discount curve is unusable: a zero global cap, or a
    /// minimum discount above the maximum.
    InvalidDiscountCurve,
    /// The valuation overflowed.
    Overflow,
}

/// Resolve the index price of one native unit's worth of base lots.
///
/// Returns the index price in quote lots per base lot and the number of native
/// units in one base lot.
pub fn spot_collateral_price(
    params: &SpotCollateralParams,
    index_price_ticks: Ticks,
    tick_size: QuoteLotsPerBaseLotPerTick,
    base_lot_decimals: i8,
) -> Result<(QuoteLotsPerBaseLot, u64), SpotCollateralValuationError> {
    if index_price_ticks == Ticks::ZERO {
        return Err(SpotCollateralValuationError::MissingIndexPrice);
    }

    let difference =
        i32::try_from(params.decimals).unwrap_or(i32::MAX) - i32::from(base_lot_decimals);
    if difference < 0 {
        return Err(SpotCollateralValuationError::NegativeDecimalsDifference {
            decimals: params.decimals,
            base_lot_decimals,
        });
    }
    let difference = difference as u32;
    let native_units_per_base_lot = 10u64
        .checked_pow(difference)
        .ok_or(SpotCollateralValuationError::DecimalsDifferenceTooLarge { difference })?;

    Ok((index_price_ticks * tick_size, native_units_per_base_lot))
}

/// Undiscounted value of a balance, in quote lots.
///
/// Equivalent to the on-chain base-lot-plus-dust form, floored: sub-base-lot
/// dust keeps its value instead of being truncated away.
pub fn notional_spot_collateral(
    price: QuoteLotsPerBaseLot,
    native_units_per_base_lot: u64,
    balance: u64,
) -> Result<QuoteLots, SpotCollateralValuationError> {
    if balance == 0 {
        return Ok(QuoteLots::ZERO);
    }
    let divisor = u128::from(native_units_per_base_lot);
    if divisor == 0 {
        return Err(SpotCollateralValuationError::Overflow);
    }

    let notional = u128::from(balance)
        .checked_mul(u128::from(price.as_inner()))
        .ok_or(SpotCollateralValuationError::Overflow)?
        / divisor;

    u64::try_from(notional)
        .map(QuoteLots::new)
        .map_err(|_| SpotCollateralValuationError::Overflow)
}

/// Fraction of notional value retained after the margin haircut, in basis
/// points, for a trader holding `balance`.
///
/// This is a *retained* multiplier, not a haircut: a configured
/// `min_margin_discount` of 500 bps means 9,500 bps of value is retained at a
/// zero balance.
pub fn margin_retained_bps(
    params: &SpotCollateralParams,
    balance: u64,
) -> Result<BasisPoints, SpotCollateralValuationError> {
    interpolate_retained_bps(
        balance,
        params.max_global_balance,
        params.min_margin_discount.as_inner(),
        params.max_margin_discount.as_inner(),
    )
}

/// Value of a balance after the margin haircut, in quote lots. This is the
/// term spot collateral contributes to effective collateral.
pub fn discounted_spot_collateral(
    params: &SpotCollateralParams,
    price: QuoteLotsPerBaseLot,
    native_units_per_base_lot: u64,
    balance: u64,
) -> Result<QuoteLots, SpotCollateralValuationError> {
    let notional = notional_spot_collateral(price, native_units_per_base_lot, balance)?;
    let retained = margin_retained_bps(params, balance)?;
    retained
        .apply_to_quote_lots(notional)
        .ok_or(SpotCollateralValuationError::Overflow)
}

/// Linear interpolation of the retained fraction between the endpoints, then
/// floored.
///
/// `min_discount_bps` applies at a zero balance and `max_discount_bps` at
/// `max_balance`; balances above `max_balance` clamp rather than extrapolate.
fn interpolate_retained_bps(
    balance: u64,
    max_balance: u64,
    min_discount_bps: u64,
    max_discount_bps: u64,
) -> Result<BasisPoints, SpotCollateralValuationError> {
    if max_balance == 0 || min_discount_bps > max_discount_bps || max_discount_bps > BPS_DENOMINATOR
    {
        return Err(SpotCollateralValuationError::InvalidDiscountCurve);
    }

    let upper = u128::from(BPS_DENOMINATOR - min_discount_bps);
    let lower = u128::from(BPS_DENOMINATOR - max_discount_bps);

    let target = u128::from(balance.min(max_balance));
    let retained = if target == 0 {
        upper
    } else if target == u128::from(max_balance) {
        lower
    } else {
        // Line through (0, upper) and (max_balance, lower), evaluated at
        // `target`. The subtracted term floors, so the result rounds up: at
        // most 1 bps in the trader's favor.
        upper - ((upper - lower) * target / u128::from(max_balance))
    };

    u64::try_from(retained)
        .map(BasisPoints::new)
        .map_err(|_| SpotCollateralValuationError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOL: 9 decimals against a market with 2 base-lot decimals, so one base
    /// lot is 0.01 SOL.
    const BASE_LOT_DECIMALS: i8 = 2;
    const LAMPORTS_PER_BASE_LOT: u64 = 10_000_000;

    fn params(
        min_discount_bps: u64,
        max_discount_bps: u64,
        max_global: u64,
    ) -> SpotCollateralParams {
        SpotCollateralParams {
            asset_index: 0xFFFF_0000,
            symbol: "SOL".to_string(),
            perp_symbol: "SOL".to_string(),
            decimals: 9,
            max_per_trader_balance: max_global,
            max_global_balance: max_global,
            curr_global_balance: 0,
            min_margin_discount: BasisPoints::new(min_discount_bps),
            max_margin_discount: BasisPoints::new(max_discount_bps),
        }
    }

    fn price(ticks: u64) -> (QuoteLotsPerBaseLot, u64) {
        spot_collateral_price(
            &params(0, 0, 1),
            Ticks::new(ticks),
            QuoteLotsPerBaseLotPerTick::new(1),
            BASE_LOT_DECIMALS,
        )
        .unwrap()
    }

    #[test]
    fn price_resolution_derives_lamports_per_base_lot_from_the_decimals_gap() {
        let (price, units) = price(1_000);
        assert_eq!(price, QuoteLotsPerBaseLot::new(1_000));
        assert_eq!(units, LAMPORTS_PER_BASE_LOT);
    }

    #[test]
    fn price_resolution_rejects_a_missing_index_price() {
        assert_eq!(
            spot_collateral_price(
                &params(0, 0, 1),
                Ticks::ZERO,
                QuoteLotsPerBaseLotPerTick::new(1),
                BASE_LOT_DECIMALS,
            ),
            Err(SpotCollateralValuationError::MissingIndexPrice)
        );
    }

    #[test]
    fn price_resolution_rejects_a_negative_decimals_gap() {
        assert_eq!(
            spot_collateral_price(
                &params(0, 0, 1),
                Ticks::new(1_000),
                QuoteLotsPerBaseLotPerTick::new(1),
                10,
            ),
            Err(SpotCollateralValuationError::NegativeDecimalsDifference {
                decimals: 9,
                base_lot_decimals: 10,
            })
        );
    }

    /// Sub-base-lot dust keeps its value: the notional is a single floor over
    /// the whole balance, not per whole base lot.
    #[test]
    fn notional_floors_once_over_the_whole_balance() {
        let (price, units) = price(1_000);
        for lamports in [1u64, 9, 10, 999_999_999, 1_000_000_000, 1_000_000_001] {
            let expected =
                (u128::from(lamports) * 1_000u128 / u128::from(LAMPORTS_PER_BASE_LOT)) as u64;
            assert_eq!(
                notional_spot_collateral(price, units, lamports).unwrap(),
                QuoteLots::new(expected),
                "lamports={lamports}"
            );
        }
    }

    #[test]
    fn notional_is_zero_for_a_zero_balance() {
        let (price, units) = price(1_000);
        assert_eq!(
            notional_spot_collateral(price, units, 0).unwrap(),
            QuoteLots::ZERO
        );
    }

    /// A balance at the global cap gets the configured maximum discount.
    #[test]
    fn a_balance_at_the_cap_retains_exactly_the_max_discount_complement() {
        let max_global = 100_000_000_000_000;
        let params = params(0, 2_000, max_global);
        let (price, units) = price(1_000);

        let notional = notional_spot_collateral(price, units, max_global).unwrap();
        let discounted = discounted_spot_collateral(&params, price, units, max_global).unwrap();

        assert_eq!(
            margin_retained_bps(&params, max_global).unwrap(),
            BasisPoints::new(8_000)
        );
        assert_eq!(
            discounted,
            QuoteLots::new(notional.as_inner() * 8_000 / 10_000)
        );
    }

    /// A small balance is still haircut by `min_margin_discount_bps` — it is
    /// not valued at par.
    #[test]
    fn a_small_balance_is_haircut_by_the_minimum_discount() {
        let params = params(500, 1_000, 100_000_000_000_000);
        let (price, units) = price(1_000);
        let balance = 20_000_000_000;

        let notional = notional_spot_collateral(price, units, balance).unwrap();
        let discounted = discounted_spot_collateral(&params, price, units, balance).unwrap();

        assert_eq!(
            margin_retained_bps(&params, balance).unwrap(),
            BasisPoints::new(9_500)
        );
        assert_eq!(
            discounted,
            QuoteLots::new(notional.as_inner() * 9_500 / 10_000)
        );
    }

    /// The interpolation term floors, so a balance one unit below the halfway
    /// point retains 1 bps more than the exact midpoint.
    #[test]
    fn the_interpolation_term_floors_in_the_traders_favor() {
        let max_global = 100_000_000_000_000;
        let params = params(500, 1_000, max_global);

        assert_eq!(
            margin_retained_bps(&params, max_global / 2).unwrap(),
            BasisPoints::new(9_250)
        );
        assert_eq!(
            margin_retained_bps(&params, max_global / 2 - 10_000_000).unwrap(),
            BasisPoints::new(9_251)
        );
    }

    /// Balances above the cap clamp to the maximum discount rather than
    /// extrapolating past it.
    #[test]
    fn balances_above_the_cap_clamp_to_the_max_discount() {
        let max_global = 1_000_000;
        let params = params(500, 1_000, max_global);

        assert_eq!(
            margin_retained_bps(&params, max_global * 5).unwrap(),
            BasisPoints::new(9_000)
        );
    }

    /// The curve reads the trader's own balance against the global cap, so it
    /// is a pure function of that balance: pool utilization by other traders
    /// cannot move it.
    #[test]
    fn the_discount_depends_only_on_the_traders_own_balance() {
        let max_global = 100_000_000_000_000;
        let mut params = params(500, 1_000, max_global);
        let balance = 20_000_000_000;

        let before = margin_retained_bps(&params, balance).unwrap();
        params.curr_global_balance = max_global / 2;
        assert_eq!(margin_retained_bps(&params, balance).unwrap(), before);
    }

    #[test]
    fn a_zero_global_cap_is_an_invalid_curve() {
        assert_eq!(
            margin_retained_bps(&params(0, 0, 0), 1),
            Err(SpotCollateralValuationError::InvalidDiscountCurve)
        );
    }

    #[test]
    fn a_minimum_discount_above_the_maximum_is_an_invalid_curve() {
        assert_eq!(
            margin_retained_bps(&params(2_000, 1_000, 1_000), 1),
            Err(SpotCollateralValuationError::InvalidDiscountCurve)
        );
    }
}
