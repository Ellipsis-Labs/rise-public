//! Core margin calculation functions
//!
//! This module contains the formulas for computing margin requirements,
//! including initial margin, maintenance margin, and risk-tier-specific
//! margins for perpetual futures positions.

use crate::direction::Side;
use crate::limit_order_state::LimitOrderMarginState;
use crate::perp_metadata::PerpAssetMetadata;
use crate::quantities::{
    BaseLots, Constant, MathError, QuoteLots, QuoteLotsPerBaseLot, QuoteLotsPerBaseLotPerTick,
    SignedBaseLots, Ticks,
};
use crate::risk::{MarginError, RiskAction, RiskTier};
use crate::trader_position::TraderPosition;

// ============================================================================
// Position Margin Functions
// ============================================================================

/// Calculate cancel margin threshold for a position
///
/// This is the margin level at which risk-increasing orders can be
/// force-cancelled. Typically higher than maintenance margin (e.g., 55% vs
/// 50%).
pub fn position_cancel_margin(
    perp_asset_metadata: &PerpAssetMetadata,
    position_initial_margin: QuoteLots,
) -> Result<QuoteLots, MarginError> {
    perp_asset_metadata
        .cancel_order_risk_factor()
        .apply_to_quote_lots(position_initial_margin)
        .ok_or(MarginError::Overflow)
}

/// Calculate maintenance margin (liquidation threshold) for a position
///
/// When effective collateral falls below this level, the position
/// becomes liquidatable via market orders.
pub fn position_maintenance_margin(
    perp_asset_metadata: &PerpAssetMetadata,
    position_initial_margin: QuoteLots,
) -> Result<QuoteLots, MarginError> {
    perp_asset_metadata
        .get_risk_factor(RiskTier::Liquidatable)
        .apply_to_quote_lots(position_initial_margin)
        .ok_or(MarginError::Overflow)
}

/// Calculate backstop margin threshold for a position
///
/// Second-tier liquidation threshold, typically ~40% of initial margin.
pub fn position_backstop_margin(
    perp_asset_metadata: &PerpAssetMetadata,
    position_initial_margin: QuoteLots,
) -> Result<QuoteLots, MarginError> {
    perp_asset_metadata
        .get_risk_factor(RiskTier::BackstopLiquidatable)
        .apply_to_quote_lots(position_initial_margin)
        .ok_or(MarginError::Overflow)
}

/// Calculate high-risk margin threshold for a position
///
/// Lowest margin threshold before insurance fund intervention, typically ~30%.
pub fn position_high_risk_margin(
    perp_asset_metadata: &PerpAssetMetadata,
    position_initial_margin: QuoteLots,
) -> Result<QuoteLots, MarginError> {
    perp_asset_metadata
        .get_risk_factor(RiskTier::HighRisk)
        .apply_to_quote_lots(position_initial_margin)
        .ok_or(MarginError::Overflow)
}

// ============================================================================
// Initial Margin Calculation
// ============================================================================

/// Calculate initial margin for an asset position during normal trading
/// operations
///
/// This function calculates the standard margin requirements for position
/// opening, order placement, and regular margin checks. It uses leverage-based
/// calculations and applies risk factor discounts to limit orders for capital
/// efficiency.
///
/// For withdrawal validation, use `initial_margin_for_asset_for_withdrawals`
/// instead, which enforces stricter requirements to prevent
/// undercollateralization.
///
/// # Returns
/// The minimum collateral required to maintain the position and limit orders
pub fn initial_margin_for_asset(
    perp_asset_metadata: &PerpAssetMetadata,
    position_state: &TraderPosition,
    limit_order_state: &LimitOrderMarginState,
    risk_action: RiskAction,
) -> Result<QuoteLots, MarginError> {
    let asset_unit_price = perp_asset_metadata
        .try_get_mark_price(risk_action)
        .map_err(MarginError::MarkPrice)?
        * perp_asset_metadata.tick_size();
    initial_margin_for_asset_internal(
        perp_asset_metadata,
        position_state,
        limit_order_state,
        false,
        asset_unit_price,
        risk_action,
    )
}

pub fn initial_margin_for_asset_with_mark_price(
    perp_asset_metadata: &PerpAssetMetadata,
    position_state: &TraderPosition,
    limit_order_state: &LimitOrderMarginState,
    mark_price: Ticks,
    risk_action: RiskAction,
) -> Result<QuoteLots, MarginError> {
    let asset_unit_price = mark_price * perp_asset_metadata.tick_size();
    initial_margin_for_asset_internal(
        perp_asset_metadata,
        position_state,
        limit_order_state,
        false,
        asset_unit_price,
        risk_action,
    )
}

/// Calculate initial margin for an asset position when validating withdrawals
///
/// This function enforces stricter margin requirements than normal trading
/// operations to ensure traders cannot withdraw funds that would leave them
/// undercollateralized. It uses the MAXIMUM of leverage-based and
/// risk-factor-based requirements.
pub fn initial_margin_for_asset_for_withdrawals(
    perp_asset_metadata: &PerpAssetMetadata,
    position_state: &TraderPosition,
    limit_order_state: &LimitOrderMarginState,
    risk_action: RiskAction,
) -> Result<QuoteLots, MarginError> {
    let asset_unit_price = perp_asset_metadata
        .try_get_mark_price(risk_action)
        .map_err(MarginError::MarkPrice)?
        * perp_asset_metadata.tick_size();
    initial_margin_for_asset_internal(
        perp_asset_metadata,
        position_state,
        limit_order_state,
        true,
        asset_unit_price,
        risk_action,
    )
}

fn existing_position_margin(
    position: SignedBaseLots,
    asset_unit_price: QuoteLotsPerBaseLot,
    perp_asset_metadata: &PerpAssetMetadata,
) -> QuoteLots {
    if position.is_zero() {
        return QuoteLots::ZERO;
    }
    let absolute_position_size = position.abs_as_unsigned();
    let absolute_book_value = asset_unit_price * absolute_position_size;
    let leverage = perp_asset_metadata
        .leverage_tiers()
        .get_leverage_constant(absolute_position_size);
    absolute_book_value.div_ceil::<Constant>(leverage)
}

/// Internal function for calculating initial margin requirements for a position
///
/// This function implements two distinct calculation modes controlled by the
/// `bypass_risk_factor` parameter, which determines the strictness of margin
/// requirements.
///
/// # Parameters
/// - `perp_asset_metadata`: Market configuration including leverage tiers and
///   risk factors
/// - `position_state`: Current position size and state
/// - `limit_order_state`: Outstanding limit orders requiring margin
/// - `bypass_risk_factor`: Controls calculation mode (see below)
/// - `risk_action`: Context for the risk check (View, Withdrawal, etc.)
///
/// # Calculation Modes
///
/// ## Normal Operations (`bypass_risk_factor = false`)
/// Used for: Position opening, order placement, regular margin checks
/// - Calculates leverage-based margin using the position's leverage tier
/// - Applies risk factor discounts to limit orders (allows more capital
///   efficiency)
/// - Formula: `position_value / max_leverage`
///
/// ## Withdrawal Validation (`bypass_risk_factor = true`)
/// Used for: Validating that withdrawals won't leave trader undercollateralized
/// - Does NOT apply risk factor discounts to limit orders (maximum strictness)
/// - Leverage margin: `position_value with risk increasing limit orders filled
///   / max_leverage`
///
/// # Strictness Guarantees
/// - All division operations use ceiling division (`div_ceil`) to round up
/// - Risk factor applications use `apply_to_quote_lots_ceil` for maximum
///   strictness
/// - No safety buffers or tolerances - exact calculations only
fn initial_margin_for_asset_internal(
    perp_asset_metadata: &PerpAssetMetadata,
    position_state: &TraderPosition,
    limit_order_state: &LimitOrderMarginState,
    bypass_risk_factor: bool,
    asset_unit_price: QuoteLotsPerBaseLot,
    _risk_action: RiskAction,
) -> Result<QuoteLots, MarginError> {
    // Early return if no positions AND no resting limit orders (reduce-only
    // included). This fixes the bug where traders with closed positions couldn't
    // withdraw due to margin calculations being performed on zero positions.
    // Reduce-only orders are included in the guard because they still reserve
    // adverse-fill-loss margin below.
    if position_state.base_lot_position.is_zero()
        && limit_order_state.total_bid_base_lots().is_zero()
        && limit_order_state.total_ask_base_lots().is_zero()
    {
        return Ok(QuoteLots::ZERO);
    }

    let position_margin = existing_position_margin(
        position_state.base_lot_position,
        asset_unit_price,
        perp_asset_metadata,
    );

    // Calculate margin increase due to the bid limit orders, plus the reserve
    // against an adverse fill of the resting bids (a bid above mark filling).
    let (margin_bid, bid_fill_loss) = if limit_order_state.total_bid_base_lots() > BaseLots::ZERO {
        (
            margin_increase_for_bids_internal(
                position_state.base_lot_position,
                limit_order_state
                    .total_non_reduce_only_bid_base_lots
                    .as_signed(),
                asset_unit_price,
                perp_asset_metadata,
                position_margin,
                bypass_risk_factor,
            )
            .map_err(MarginError::from)?,
            limit_order_fill_loss(
                Side::Bid,
                limit_order_state.total_bid_base_lots(),
                limit_order_state.highest_bid(),
                asset_unit_price,
                perp_asset_metadata.tick_size(),
            )
            .map_err(MarginError::from)?,
        )
    } else {
        (QuoteLots::ZERO, QuoteLots::ZERO)
    };

    // Calculate margin increase due to the ask limit orders, plus the reserve
    // against an adverse fill of the resting asks (an ask below mark filling).
    let (margin_ask, ask_fill_loss) = if limit_order_state.total_ask_base_lots() > BaseLots::ZERO {
        (
            margin_increase_for_asks_internal(
                position_state.base_lot_position,
                limit_order_state
                    .total_non_reduce_only_ask_base_lots
                    .as_signed(),
                asset_unit_price,
                perp_asset_metadata,
                position_margin,
                bypass_risk_factor,
            )
            .map_err(MarginError::from)?,
            limit_order_fill_loss(
                Side::Ask,
                limit_order_state.total_ask_base_lots(),
                limit_order_state.lowest_ask(),
                asset_unit_price,
                perp_asset_metadata.tick_size(),
            )
            .map_err(MarginError::from)?,
        )
    } else {
        (QuoteLots::ZERO, QuoteLots::ZERO)
    };

    // Margin increase caused by limit orders: the larger of the two directional
    // margin increases, plus the adverse-fill-loss reserve for each side.
    let collateral_required = position_margin
        .checked_add(if margin_bid > margin_ask {
            margin_bid
        } else {
            margin_ask
        })
        .and_then(|x| x.checked_add(bid_fill_loss))
        .and_then(|x| x.checked_add(ask_fill_loss))
        .ok_or(MarginError::Overflow)?;

    Ok(collateral_required)
}

/// Reserve for the immediate mark-to-market loss if resting orders on `side`
/// were to fill at their limit price. A resting bid above mark (or a resting
/// ask below mark) would fill "in the money" for the counterparty, so we
/// reserve `size * |limit_price - mark_price|` for that adverse move. Orders
/// resting on the passive side of mark contribute nothing.
fn limit_order_fill_loss(
    side: Side,
    size: BaseLots,
    limit_price_in_ticks: Option<Ticks>,
    mark_price: QuoteLotsPerBaseLot,
    tick_size: QuoteLotsPerBaseLotPerTick,
) -> Result<QuoteLots, MathError> {
    let Some(limit_price_in_ticks) = limit_price_in_ticks else {
        return Ok(QuoteLots::ZERO);
    };
    let limit_price = limit_price_in_ticks * tick_size;

    let price_difference = match side {
        Side::Bid => {
            if limit_price > mark_price {
                limit_price
                    .checked_sub(mark_price)
                    .ok_or(MathError::Overflow)?
            } else {
                return Ok(QuoteLots::ZERO);
            }
        }
        Side::Ask => {
            if limit_price < mark_price {
                mark_price
                    .checked_sub(limit_price)
                    .ok_or(MathError::Overflow)?
            } else {
                return Ok(QuoteLots::ZERO);
            }
        }
    };

    Ok(price_difference * size)
}

// ============================================================================
// Limit Order Margin Helpers
// ============================================================================

/// Compute incremental margin for bid orders (public interface)
pub fn margin_increase_for_bids(
    position: SignedBaseLots,
    bid_size: SignedBaseLots,
    asset_unit_price: QuoteLotsPerBaseLot,
    perp_asset_metadata: &PerpAssetMetadata,
) -> Result<QuoteLots, MathError> {
    let existing_position_margin_offset =
        existing_position_margin(position, asset_unit_price, perp_asset_metadata);
    margin_increase_for_bids_internal(
        position,
        bid_size,
        asset_unit_price,
        perp_asset_metadata,
        existing_position_margin_offset,
        false,
    )
}

/// Calculates the net change (i.e., increase) in margin required for bid orders
///
/// Formula: CV_bid = max(N_bid + x_i - |x_i|, 0) * p_i^mark * r_i^LO
fn margin_increase_for_bids_internal(
    position: SignedBaseLots,
    bid_size: SignedBaseLots,
    asset_unit_price: QuoteLotsPerBaseLot,
    perp_asset_metadata: &PerpAssetMetadata,
    existing_position_margin_offset: QuoteLots,
    bypass_risk_factor: bool,
) -> Result<QuoteLots, MathError> {
    // CV_bid = max(N_bid + x_i - |x_i|, 0) * p_i^mark * r_i^LO
    let new_exposure_signed = bid_size
        .checked_add(position)
        .ok_or(MathError::Overflow)?
        .checked_sub(position.abs())
        .ok_or(MathError::Overflow)?;

    if new_exposure_signed <= SignedBaseLots::ZERO {
        return Ok(QuoteLots::ZERO);
    }

    let total_exposure_signed = position.checked_add(bid_size).ok_or(MathError::Overflow)?;
    let total_exposure = total_exposure_signed.abs_as_unsigned();
    let total_gross_value = asset_unit_price * total_exposure;
    let total_leverage = perp_asset_metadata
        .leverage_tiers()
        .get_leverage_constant(total_exposure);
    let total_margin = total_gross_value.div_ceil::<Constant>(total_leverage);

    let incremental_margin = total_margin
        .checked_sub(existing_position_margin_offset)
        .unwrap_or(QuoteLots::ZERO);

    if bypass_risk_factor {
        Ok(incremental_margin)
    } else {
        let bid_risk_factor = perp_asset_metadata
            .leverage_tiers()
            .get_limit_order_risk_factor(total_exposure);
        bid_risk_factor
            .apply_to_quote_lots_ceil(incremental_margin)
            .ok_or(MathError::Overflow)
    }
}

/// Compute incremental margin for ask orders (public interface)
pub fn margin_increase_for_asks(
    position: SignedBaseLots,
    ask_size: SignedBaseLots,
    asset_unit_price: QuoteLotsPerBaseLot,
    perp_asset_metadata: &PerpAssetMetadata,
) -> Result<QuoteLots, MathError> {
    let existing_position_margin_offset =
        existing_position_margin(position, asset_unit_price, perp_asset_metadata);
    margin_increase_for_asks_internal(
        position,
        ask_size,
        asset_unit_price,
        perp_asset_metadata,
        existing_position_margin_offset,
        false,
    )
}

/// Calculates the net change (i.e., increase) in margin required for ask orders
///
/// Formula: margin_ask = max(N_ask - x_i - |x_i|, 0) * p_i^mark * r_i^LO
fn margin_increase_for_asks_internal(
    position: SignedBaseLots,
    ask_size: SignedBaseLots,
    asset_unit_price: QuoteLotsPerBaseLot,
    perp_asset_metadata: &PerpAssetMetadata,
    existing_position_margin_offset: QuoteLots,
    bypass_risk_factor: bool,
) -> Result<QuoteLots, MathError> {
    // If ask_size - position - |position| <= 0, the order reduces risk → no margin.
    // Otherwise: total_exposure = |position - ask_size|, and margin is
    // (total_margin(total_exposure) - existing_position_margin) *
    // risk_factor(total_exposure)
    let new_exposure_signed = ask_size
        .checked_sub(position)
        .ok_or(MathError::Overflow)?
        .checked_sub(position.abs())
        .ok_or(MathError::Overflow)?;

    if new_exposure_signed <= SignedBaseLots::ZERO {
        return Ok(QuoteLots::ZERO);
    }

    let total_exposure_signed = position.checked_sub(ask_size).ok_or(MathError::Overflow)?;
    let total_exposure = total_exposure_signed.abs_as_unsigned();
    let total_gross_value = asset_unit_price * total_exposure;
    let total_leverage = perp_asset_metadata
        .leverage_tiers()
        .get_leverage_constant(total_exposure);
    let total_margin = total_gross_value.div_ceil::<Constant>(total_leverage);

    let incremental_margin = total_margin
        .checked_sub(existing_position_margin_offset)
        .unwrap_or(QuoteLots::ZERO);

    if bypass_risk_factor {
        Ok(incremental_margin)
    } else {
        let ask_risk_factor = perp_asset_metadata
            .leverage_tiers()
            .get_limit_order_risk_factor(total_exposure);
        ask_risk_factor
            .apply_to_quote_lots_ceil(incremental_margin)
            .ok_or(MathError::Overflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limit_order_state::LimitOrderSummary;

    fn create_test_metadata() -> PerpAssetMetadata {
        PerpAssetMetadata::default()
    }

    #[test]
    fn test_no_position_no_orders() {
        let metadata = create_test_metadata();
        let position = TraderPosition::new();
        let orders = LimitOrderMarginState::empty();

        let margin = initial_margin_for_asset(&metadata, &position, &orders, RiskAction::View)
            .expect("Should calculate margin");

        assert_eq!(margin, QuoteLots::ZERO);
    }

    #[test]
    fn test_position_initial_margin() {
        let metadata = create_test_metadata();
        let mut position = TraderPosition::new();
        position.base_lot_position = SignedBaseLots::new(1_000_000); // 1M base lots
        let orders = LimitOrderMarginState::empty();

        let margin = initial_margin_for_asset(&metadata, &position, &orders, RiskAction::View)
            .expect("Should calculate margin");

        assert!(margin > QuoteLots::ZERO);
    }

    #[test]
    fn test_risk_factor_margins() {
        let metadata = create_test_metadata();
        let initial_margin = QuoteLots::new(10_000);

        // Maintenance margin should be less than initial
        let maintenance = position_maintenance_margin(&metadata, initial_margin)
            .expect("Should calculate maintenance margin");
        assert!(maintenance < initial_margin);
        assert_eq!(maintenance, QuoteLots::new(5_000)); // 50% of initial

        // Backstop should be less than maintenance
        let backstop = position_backstop_margin(&metadata, initial_margin)
            .expect("Should calculate backstop margin");
        assert!(backstop < maintenance);
        assert_eq!(backstop, QuoteLots::new(4_000)); // 40% of initial

        // High risk should be less than backstop
        let high_risk = position_high_risk_margin(&metadata, initial_margin)
            .expect("Should calculate high risk margin");
        assert!(high_risk < backstop);
        assert_eq!(high_risk, QuoteLots::new(3_000)); // 30% of initial
    }

    /// Metadata mirroring the canonical `soft_stale_test_metadata`: tick size
    /// 10, mark price 100 ticks (= 1000 quote lots per base lot), flat 10x
    /// leverage, and a 100% limit-order risk factor so the leverage-based
    /// order margin is undiscounted.
    fn fill_loss_test_metadata() -> PerpAssetMetadata {
        use crate::leverage_tiers::{LeverageTier, LeverageTiers};
        use crate::quantities::BasisPoints;

        let tier = |upper_bound_size| LeverageTier {
            upper_bound_size: BaseLots::new(upper_bound_size),
            max_leverage: Constant::new(10),
            limit_order_risk_factor: BasisPoints::new(10_000),
        };
        let leverage_tiers = LeverageTiers::new([
            tier(1_000_000),
            tier(2_000_000),
            tier(3_000_000),
            tier(4_000_000),
        ])
        .expect("valid leverage tiers");

        PerpAssetMetadata::new(
            "TEST".to_string(),
            0,
            0,
            Ticks::new(100),
            QuoteLotsPerBaseLotPerTick::new(10),
            leverage_tiers,
            [5_000, 4_000, 3_000],
            5_500,
            5_000,
            7_500,
        )
    }

    /// A resting bid above mark (or ask below mark) must reserve the immediate
    /// mark-to-market loss it would incur on an adverse fill, on top of the
    /// leverage-based order margin. Mirrors canonical
    /// `limit_order_margin_includes_adverse_fill_loss_against_mark`.
    #[test]
    fn limit_order_margin_includes_adverse_fill_loss_against_mark() {
        let metadata = fill_loss_test_metadata();
        let flat_position = TraderPosition::new();

        // Bid at 200 ticks, mark at 100 ticks: adverse fill loss of
        // 10 lots * (2000 - 1000) = 10_000, plus leverage margin 1_000.
        let bid_above_mark = LimitOrderMarginState::from_orders(
            [LimitOrderSummary {
                side: Side::Bid,
                price_in_ticks: Ticks::new(200),
                base_lots_remaining: BaseLots::new(10),
                reduce_only: false,
            }],
            flat_position.base_lot_position,
        );
        // Ask at 50 ticks, mark at 100 ticks: adverse fill loss of
        // 10 lots * (1000 - 500) = 5_000, plus leverage margin 1_000.
        let ask_below_mark = LimitOrderMarginState::from_orders(
            [LimitOrderSummary {
                side: Side::Ask,
                price_in_ticks: Ticks::new(50),
                base_lots_remaining: BaseLots::new(10),
                reduce_only: false,
            }],
            flat_position.base_lot_position,
        );

        let bid_margin =
            initial_margin_for_asset(&metadata, &flat_position, &bid_above_mark, RiskAction::View)
                .expect("bid margin");
        let ask_margin =
            initial_margin_for_asset(&metadata, &flat_position, &ask_below_mark, RiskAction::View)
                .expect("ask margin");

        assert_eq!(bid_margin, QuoteLots::new(11_000));
        assert_eq!(ask_margin, QuoteLots::new(6_000));
    }

    fn position_at(base_lots: i64) -> TraderPosition {
        let mut position = TraderPosition::new();
        position.base_lot_position = SignedBaseLots::new(base_lots);
        position
    }

    fn reduce_only_order(side: Side, price_ticks: u64, size: u64) -> crate::margin::LimitOrder {
        crate::margin::LimitOrder {
            price: Ticks::new(price_ticks),
            side,
            order_sequence_number: 0,
            base_lot_size: BaseLots::new(size),
            initial_trade_size: BaseLots::new(size),
            reduce_only: true,
            is_stop_loss: false,
        }
    }

    /// A large reduce-only order against a small opposite position must only
    /// reserve fill-loss margin for the lots that could actually reduce the
    /// position. A reduce-only ask can reduce at most the size of a long
    /// position, so a 1_000-lot reduce-only ask against a 5-lot long is capped
    /// to 5 lots. Without the cap the reserve would be 1_000 * 500 = 500_000.
    #[test]
    fn reduce_only_fill_loss_capped_by_small_opposite_position() {
        use crate::margin::LimitOrder;

        let metadata = fill_loss_test_metadata();
        let position = position_at(5); // long 5 lots

        // Reduce-only ask of 1_000 lots at 50 ticks (limit price 500 < mark 1000).
        let orders = vec![reduce_only_order(Side::Ask, 50, 1_000)];
        let state = LimitOrder::aggregate_margin_state(&orders, position.base_lot_position);

        // Aggregation caps the reduce-only ask to the long position size (5).
        assert_eq!(state.total_reduce_only_ask_base_lots, BaseLots::new(5));

        let margin = initial_margin_for_asset(&metadata, &position, &state, RiskAction::View)
            .expect("margin");
        // position margin ceil(1000 * 5 / 10) = 500, plus fill loss 5 * 500 = 2_500.
        assert_eq!(margin, QuoteLots::new(3_000));
    }

    /// A reduce-only order on the same side as the position can only increase
    /// exposure, so it can never fill as a reduce and reserves nothing. A
    /// reduce-only bid against a long position is capped to zero.
    #[test]
    fn reduce_only_on_position_side_reserves_nothing() {
        use crate::margin::LimitOrder;

        let metadata = fill_loss_test_metadata();
        let position = position_at(5); // long 5 lots

        // Reduce-only bid of 1_000 lots at 200 ticks (above mark) — a bid would
        // increase the long, so it is not a valid reduce and is capped to zero.
        let orders = vec![reduce_only_order(Side::Bid, 200, 1_000)];
        let state = LimitOrder::aggregate_margin_state(&orders, position.base_lot_position);

        assert_eq!(state.total_reduce_only_bid_base_lots, BaseLots::ZERO);

        let margin = initial_margin_for_asset(&metadata, &position, &state, RiskAction::View)
            .expect("margin");
        // Only the position margin ceil(1000 * 5 / 10) = 500; no fill-loss reserve.
        assert_eq!(margin, QuoteLots::new(500));
    }

    /// A reduce-only order that fits entirely within the opposite position is
    /// not capped and reserves the full fill loss. A 10-lot reduce-only bid
    /// against a 10-lot short reserves 10 * (2000 - 1000) = 10_000 on top
    /// of position margin.
    #[test]
    fn reduce_only_within_opposite_position_reserves_full_fill_loss() {
        use crate::margin::LimitOrder;

        let metadata = fill_loss_test_metadata();
        let position = position_at(-10); // short 10 lots

        let orders = vec![reduce_only_order(Side::Bid, 200, 10)];
        let state = LimitOrder::aggregate_margin_state(&orders, position.base_lot_position);

        assert_eq!(state.total_reduce_only_bid_base_lots, BaseLots::new(10));

        let margin = initial_margin_for_asset(&metadata, &position, &state, RiskAction::View)
            .expect("margin");
        // position margin ceil(1000 * 10 / 10) = 1_000, plus fill loss 10 * 1000 =
        // 10_000.
        assert_eq!(margin, QuoteLots::new(11_000));
    }
}
