//! Core margin calculation functions
//!
//! This module contains the formulas for computing margin requirements,
//! including initial margin, maintenance margin, and risk-tier-specific
//! margins for perpetual futures positions.

use crate::limit_order_state::LimitOrderMarginState;
use crate::perp_metadata::PerpAssetMetadata;
use crate::quantities::{
    BaseLots, Constant, MathError, QuoteLots, QuoteLotsPerBaseLot, SignedBaseLots,
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
    initial_margin_for_asset_internal(
        perp_asset_metadata,
        position_state,
        limit_order_state,
        false,
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
    initial_margin_for_asset_internal(
        perp_asset_metadata,
        position_state,
        limit_order_state,
        true,
        risk_action,
    )
}

fn existing_position_margin(
    position: SignedBaseLots,
    asset_unit_price: QuoteLotsPerBaseLot,
    perp_asset_metadata: &PerpAssetMetadata,
) -> QuoteLots {
    if position == SignedBaseLots::ZERO {
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
    risk_action: RiskAction,
) -> Result<QuoteLots, MarginError> {
    // Early return if no positions AND no non-reduce-only limit orders
    // This fixes the bug where traders with closed positions couldn't withdraw
    // due to margin calculations being performed on zero positions
    if position_state.base_lot_position == SignedBaseLots::ZERO
        && limit_order_state.total_non_reduce_only_bid_base_lots == BaseLots::ZERO
        && limit_order_state.total_non_reduce_only_ask_base_lots == BaseLots::ZERO
    {
        return Ok(QuoteLots::ZERO);
    }

    let asset_unit_price = perp_asset_metadata
        .try_get_mark_price(risk_action)
        .map_err(MarginError::MarkPrice)?
        * perp_asset_metadata.tick_size();
    let position_margin = existing_position_margin(
        position_state.base_lot_position,
        asset_unit_price,
        perp_asset_metadata,
    );
    let mut collateral_required = position_margin;

    // Calculate margin increase due to the bid limit orders
    let margin_bid = if limit_order_state.total_non_reduce_only_bid_base_lots > BaseLots::ZERO {
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
        .map_err(MarginError::from)?
    } else {
        QuoteLots::ZERO
    };

    // Calculate margin increase due to the ask limit orders
    let margin_ask = if limit_order_state.total_non_reduce_only_ask_base_lots > BaseLots::ZERO {
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
        .map_err(MarginError::from)?
    } else {
        QuoteLots::ZERO
    };

    // Calculate margin increase caused by limit orders
    collateral_required = collateral_required
        .checked_add(if margin_bid > margin_ask {
            margin_bid
        } else {
            margin_ask
        })
        .ok_or(MarginError::Overflow)?;

    Ok(collateral_required)
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
}
