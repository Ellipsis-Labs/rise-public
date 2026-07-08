//! Core margin types and per-market margin computation
//!
//! This module provides types for representing margin requirements and
//! computing per-market margin from positions and limit orders.

use std::iter::Sum;
use std::ops::Add;

use crate::direction::Side;
use crate::errors::PhoenixStateError;
use crate::limit_order_state::{LimitOrderMarginState, LimitOrderSummary};
use crate::margin_calc::{
    initial_margin_for_asset, initial_margin_for_asset_for_withdrawals, margin_increase_for_asks,
    margin_increase_for_bids, position_backstop_margin, position_cancel_margin,
    position_high_risk_margin, position_maintenance_margin,
};
use crate::market_math::MarketCalculator;
use crate::perp_metadata::PerpAssetMetadata;
use crate::portfolio::PerpMetadataProvider;
use crate::quantities::{
    BaseLots, BasisPoints, MathError, QuoteLots, QuoteLotsPerBaseLotPerTick, ScalarBounds,
    SignedBaseLots, SignedQuoteLots, Ticks, UPnlRiskFactor,
};
use crate::risk::{MarginError, RiskAction, RiskTier};
use crate::trader_position::TraderPosition;

pub(crate) fn unrealized_pnl_for_position(
    base_lot_position: SignedBaseLots,
    virtual_quote_lot_position: SignedQuoteLots,
    settlement_price: Ticks,
    tick_size_in_quote_lots_per_base_lot: QuoteLotsPerBaseLotPerTick,
) -> SignedQuoteLots {
    let calculator = MarketCalculator::new(0, tick_size_in_quote_lots_per_base_lot);
    virtual_quote_lot_position
        + calculator.position_value_for_position(base_lot_position, settlement_price)
}

pub(crate) fn discounted_unrealized_pnl_for_position_for_withdrawals(
    base_lot_position: SignedBaseLots,
    virtual_quote_lot_position: SignedQuoteLots,
    settlement_price: Ticks,
    tick_size_in_quote_lots_per_base_lot: QuoteLotsPerBaseLotPerTick,
    perp_asset_metadata: &PerpAssetMetadata,
) -> Result<SignedQuoteLots, MathError> {
    let raw_pnl = unrealized_pnl_for_position(
        base_lot_position,
        virtual_quote_lot_position,
        settlement_price,
        tick_size_in_quote_lots_per_base_lot,
    );

    // Apply withdrawal risk factor penalty only to positive uPnL
    if raw_pnl > SignedQuoteLots::ZERO {
        let raw_pnl_unsigned = raw_pnl.checked_as_unsigned()?;
        let discounted = perp_asset_metadata
            .upnl_risk_factor(RiskAction::Withdrawal {
                current_slot: crate::quantities::Slot::ZERO,
            })
            .apply_to_quote_lots_ceil(raw_pnl_unsigned)
            .ok_or(MathError::Overflow)?;
        discounted.checked_as_signed()
    } else {
        Ok(raw_pnl) // No penalty for negative uPnL
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Margin {
    /// Total initial margin requirement including both positions and limit
    /// orders.
    pub initial_margin: QuoteLots,

    /// Maintenance margin (liquidation margin) threshold.
    pub maintenance_margin: QuoteLots,

    /// Initial margin requirement used specifically for withdrawal validation.
    pub initial_margin_for_withdrawals: QuoteLots,

    /// Margin requirement specifically from outstanding limit orders.
    pub limit_order_margin: QuoteLots,

    /// Backstop liquidation margin threshold.
    pub backstop_requirement: QuoteLots,

    /// High-risk margin threshold.
    pub high_risk_margin: QuoteLots,

    /// At-risk margin threshold - same as initial margin (100%).
    pub at_risk_margin: QuoteLots,

    /// Cancellation margin threshold.
    pub cancel_margin: QuoteLots,

    /// Raw unrealized profit or loss based on current mark price.
    pub unrealized_pnl: SignedQuoteLots,

    /// Unrealized PnL with risk-based discounting applied.
    pub discounted_unrealized_pnl: SignedQuoteLots,

    /// Unrealized PnL with stricter discounting for withdrawal validation.
    pub discounted_pnl_for_withdrawals: SignedQuoteLots,

    /// Funding payments that have not yet been settled to the trader's account.
    pub unsettled_funding: SignedQuoteLots,

    /// Funding payments that have been accrued but not yet settled.
    pub accumulated_funding: SignedQuoteLots,

    /// Position value at current mark price
    pub position_value: SignedQuoteLots,
}

impl Margin {
    /// Initial margin requirement for positions only, excluding limit orders.
    pub fn position_only_initial_margin(&self) -> QuoteLots {
        self.initial_margin - self.limit_order_margin
    }

    pub fn position_only_maintenance_margin(
        &self,
        limit_order_risk_factor: BasisPoints,
    ) -> QuoteLots {
        let discounted_limit_order_margin = limit_order_risk_factor
            .apply_to_quote_lots(self.limit_order_margin)
            .expect("limit order risk factor application should not overflow");
        self.maintenance_margin
            .checked_sub(discounted_limit_order_margin)
            .expect("maintenance margin should be >= discounted limit order margin")
    }

    pub fn risk_tier(
        &self,
        effective_collateral: SignedQuoteLots,
    ) -> Result<RiskTier, MarginError> {
        if effective_collateral < SignedQuoteLots::ZERO {
            return Ok(RiskTier::HighRisk);
        }
        let effective_collateral = effective_collateral
            .checked_as_unsigned()
            .map_err(|_| MarginError::Overflow)?;

        Ok(if effective_collateral < self.high_risk_margin {
            RiskTier::HighRisk
        } else if effective_collateral < self.backstop_requirement {
            RiskTier::BackstopLiquidatable
        } else if effective_collateral < self.maintenance_margin {
            RiskTier::Liquidatable
        } else if effective_collateral < self.cancel_margin {
            RiskTier::Cancellable
        } else if effective_collateral < self.at_risk_margin {
            RiskTier::AtRisk
        } else {
            RiskTier::Safe
        })
    }
}

impl Add for Margin {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            maintenance_margin: self.maintenance_margin + other.maintenance_margin,
            initial_margin: self.initial_margin + other.initial_margin,
            initial_margin_for_withdrawals: self.initial_margin_for_withdrawals
                + other.initial_margin_for_withdrawals,

            limit_order_margin: self.limit_order_margin + other.limit_order_margin,
            backstop_requirement: self.backstop_requirement + other.backstop_requirement,
            high_risk_margin: self.high_risk_margin + other.high_risk_margin,
            at_risk_margin: self.at_risk_margin + other.at_risk_margin,
            cancel_margin: self.cancel_margin + other.cancel_margin,

            unrealized_pnl: self.unrealized_pnl + other.unrealized_pnl,
            discounted_unrealized_pnl: self.discounted_unrealized_pnl
                + other.discounted_unrealized_pnl,
            discounted_pnl_for_withdrawals: self.discounted_pnl_for_withdrawals
                + other.discounted_pnl_for_withdrawals,
            unsettled_funding: self.unsettled_funding + other.unsettled_funding,
            accumulated_funding: self.accumulated_funding + other.accumulated_funding,
            position_value: self.position_value + other.position_value,
        }
    }
}

impl Sum for Margin {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, item| acc + item)
    }
}

/// Individual limit order details.
/// Represents a single resting order in the orderbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitOrder {
    /// Order price in ticks
    pub price: Ticks,
    /// Order side (Bid or Ask)
    pub side: Side,
    /// Unique order sequence number
    pub order_sequence_number: u64,
    /// Order size in base lots
    pub base_lot_size: BaseLots,
    /// Initial trade size when order was placed
    pub initial_trade_size: BaseLots,
    /// Whether the order was placed with the reduce-only flag
    pub reduce_only: bool,
    /// Whether the order was placed from a stop loss
    pub is_stop_loss: bool,
}

impl LimitOrder {
    /// Aggregate a list of orders into a LimitOrderMarginState via the
    /// canonical [`LimitOrderMarginState::from_orders`] construction, which
    /// caps reduce-only totals by the position they can reduce.
    pub fn aggregate_margin_state(
        orders: &[LimitOrder],
        position: SignedBaseLots,
    ) -> LimitOrderMarginState {
        LimitOrderMarginState::from_orders(
            orders.iter().map(|order| LimitOrderSummary {
                side: order.side,
                price_in_ticks: order.price,
                base_lots_remaining: order.base_lot_size,
                reduce_only: order.reduce_only,
            }),
            position,
        )
    }
}

/// Order with computed margin requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderMargin {
    /// Order price in ticks
    pub price: Ticks,
    /// Order side (Bid or Ask)
    pub side: Side,
    /// Unique order sequence number
    pub order_sequence_number: u64,
    /// Initial order size when placed
    pub initial_trade_size: BaseLots,
    /// Remaining unfilled size
    pub trade_size_remaining: BaseLots,
    /// Margin required for this specific order.
    pub margin_requirement: QuoteLots,
    /// The margin factor applied to this order's notional value.
    pub margin_factor: BasisPoints,
    /// Whether the originating order was reduce-only
    pub reduce_only: bool,
    /// Whether the originating order was placed from a stop-loss trigger
    pub is_stop_loss: bool,
}

/// Raw position and limit order data for a single market.
/// Contains no computed margin or PnL.
pub struct MarketPosition {
    pub position: Option<TraderPosition>,
    /// Individual limit orders for this market
    pub limit_orders: Vec<LimitOrder>,
}

impl MarketPosition {
    /// Get aggregated limit order margin state (calculated from individual
    /// orders)
    pub fn limit_order_margin(&self) -> Option<LimitOrderMarginState> {
        if self.limit_orders.is_empty() {
            return None;
        }
        let position = self
            .position
            .map(|p| p.base_lot_position)
            .unwrap_or(SignedBaseLots::ZERO);
        Some(LimitOrder::aggregate_margin_state(
            &self.limit_orders,
            position,
        ))
    }

    /// Compute margin requirements for each individual limit order.
    pub(crate) fn compute_limit_orders_margin(
        &self,
        perp_asset_metadata: &PerpAssetMetadata,
    ) -> Result<Vec<OrderMargin>, PhoenixStateError> {
        let mark_price = perp_asset_metadata
            .try_get_mark_price(RiskAction::View)
            .map_err(PhoenixStateError::MarkPriceError)?;
        let asset_unit_price = mark_price * perp_asset_metadata.tick_size();

        let trader_position = self
            .position
            .map(|p| p.base_lot_position)
            .unwrap_or(SignedBaseLots::ZERO);

        let mut bids: Vec<&LimitOrder> = self
            .limit_orders
            .iter()
            .filter(|o| o.side == Side::Bid)
            .collect();
        let mut asks: Vec<&LimitOrder> = self
            .limit_orders
            .iter()
            .filter(|o| o.side == Side::Ask)
            .collect();

        // Sort bids by price (descending) then sequence number (ascending)
        bids.sort_by(|a, b| {
            b.price
                .cmp(&a.price)
                .then_with(|| a.order_sequence_number.cmp(&b.order_sequence_number))
        });

        // Sort asks by price (ascending) then sequence number (ascending)
        asks.sort_by(|a, b| {
            a.price
                .cmp(&b.price)
                .then_with(|| a.order_sequence_number.cmp(&b.order_sequence_number))
        });

        let mut result = Vec::new();

        // Process bids
        for order in bids {
            let remaining_base_lots = if order.reduce_only {
                BaseLots::ZERO
            } else {
                order.base_lot_size
            };
            let order_size = remaining_base_lots.as_signed();

            let margin_req = margin_increase_for_bids(
                trader_position,
                order_size,
                asset_unit_price,
                perp_asset_metadata,
            )?;

            let limit_order_risk_factor = if margin_req.is_zero() {
                BasisPoints::ZERO
            } else {
                let total_exposure_signed = trader_position
                    .checked_add(order_size)
                    .ok_or(MathError::Overflow)?;
                let total_exposure = total_exposure_signed.abs_as_unsigned();
                perp_asset_metadata
                    .leverage_tiers()
                    .get_limit_order_risk_factor(total_exposure)
            };

            result.push(OrderMargin {
                price: order.price,
                side: order.side,
                order_sequence_number: order.order_sequence_number,
                initial_trade_size: order.initial_trade_size,
                trade_size_remaining: order.base_lot_size,
                margin_requirement: margin_req,
                margin_factor: limit_order_risk_factor,
                reduce_only: order.reduce_only,
                is_stop_loss: order.is_stop_loss,
            });
        }

        // Process asks
        for order in asks {
            let remaining_base_lots = if order.reduce_only {
                BaseLots::ZERO
            } else {
                order.base_lot_size
            };
            let order_size = remaining_base_lots.as_signed();

            let margin_req = margin_increase_for_asks(
                trader_position,
                order_size,
                asset_unit_price,
                perp_asset_metadata,
            )?;

            let limit_order_risk_factor = if margin_req.is_zero() {
                BasisPoints::ZERO
            } else {
                let total_exposure_signed = trader_position
                    .checked_sub(order_size)
                    .ok_or(MathError::Overflow)?;
                let total_exposure = total_exposure_signed.abs_as_unsigned();
                perp_asset_metadata
                    .leverage_tiers()
                    .get_limit_order_risk_factor(total_exposure)
            };

            result.push(OrderMargin {
                price: order.price,
                side: order.side,
                order_sequence_number: order.order_sequence_number,
                initial_trade_size: order.initial_trade_size,
                trade_size_remaining: order.base_lot_size,
                margin_requirement: margin_req,
                margin_factor: limit_order_risk_factor,
                reduce_only: order.reduce_only,
                is_stop_loss: order.is_stop_loss,
            });
        }

        Ok(result)
    }

    /// Compute margin and PnL margin for this market position using the
    /// provided metadata.
    pub fn compute_margin(
        &self,
        symbol: &str,
        provider: &impl PerpMetadataProvider,
    ) -> Result<Margin, PhoenixStateError> {
        let perp_asset_metadata = provider.get_perp_metadata(symbol).ok_or_else(|| {
            PhoenixStateError::MarketNotFound {
                symbol: symbol.to_string(),
                markets: vec![],
            }
        })?;

        let position = self.position.unwrap_or_default();
        let limit_order_state = self.limit_order_margin().unwrap_or_default();
        compute_market_margin(position, limit_order_state, perp_asset_metadata)
    }
}

/// Market position with computed margin and PnL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketMargin {
    /// The trader's position in this market.
    pub position: Option<TraderPosition>,
    /// Individual limit orders with their margin requirements
    pub limit_orders: Vec<OrderMargin>,
    /// The trader's margin requirements for this market.
    pub margin: Margin,
}

impl MarketMargin {
    pub fn limit_order_margin(&self) -> LimitOrderMarginState {
        LimitOrderMarginState::from_orders(
            self.limit_orders.iter().map(|order| LimitOrderSummary {
                side: order.side,
                price_in_ticks: order.price,
                base_lots_remaining: order.trade_size_remaining,
                reduce_only: order.reduce_only,
            }),
            self.position
                .map(|p| p.base_lot_position)
                .unwrap_or(SignedBaseLots::ZERO),
        )
    }

    pub fn recompute_margin(
        &mut self,
        perp_asset_metadata: &PerpAssetMetadata,
    ) -> Result<(), PhoenixStateError> {
        let position = self.position.unwrap_or_default();
        let limit_order_margin = self.limit_order_margin();
        self.margin = compute_market_margin(position, limit_order_margin, perp_asset_metadata)?;

        Ok(())
    }
}

pub(crate) fn compute_market_margin(
    position: TraderPosition,
    limit_order_margin: LimitOrderMarginState,
    perp_asset_metadata: &PerpAssetMetadata,
) -> Result<Margin, PhoenixStateError> {
    let mark_price = perp_asset_metadata
        .try_get_mark_price(RiskAction::View)
        .map_err(PhoenixStateError::MarkPriceError)?;

    let unrealized_pnl = unrealized_pnl_for_position(
        position.base_lot_position,
        position.virtual_quote_lot_position,
        mark_price,
        perp_asset_metadata.tick_size(),
    );

    let discounted_unrealized_pnl = if unrealized_pnl > SignedQuoteLots::ZERO {
        let upnl_risk_factor = perp_asset_metadata
            .upnl_risk_factor(RiskAction::View)
            .as_inner() as u128;
        let numerator = (unrealized_pnl.as_inner() as u128).saturating_mul(upnl_risk_factor);
        let denom = UPnlRiskFactor::UPPER_BOUND as u128;
        let discounted_u128 = numerator
            .saturating_add(denom.saturating_sub(1))
            .saturating_div(denom);
        let discounted_u64 = discounted_u128.min(u64::MAX as u128) as u64;
        QuoteLots::new(discounted_u64)
            .checked_as_signed()
            .map_err(PhoenixStateError::MathError)?
    } else {
        unrealized_pnl
    };

    let discounted_pnl_for_withdrawals = discounted_unrealized_pnl_for_position_for_withdrawals(
        position.base_lot_position,
        position.virtual_quote_lot_position,
        mark_price,
        perp_asset_metadata.tick_size(),
        perp_asset_metadata,
    )?;

    let total_initial_margin = initial_margin_for_asset(
        perp_asset_metadata,
        &position,
        &limit_order_margin,
        RiskAction::View,
    )
    .map_err(PhoenixStateError::MarginError)?;

    let position_only_initial_margin = initial_margin_for_asset(
        perp_asset_metadata,
        &position,
        &LimitOrderMarginState::default(),
        RiskAction::View,
    )
    .map_err(PhoenixStateError::MarginError)?;

    let initial_margin_for_withdrawals = initial_margin_for_asset_for_withdrawals(
        perp_asset_metadata,
        &position,
        &limit_order_margin,
        RiskAction::View,
    )
    .map_err(PhoenixStateError::MarginError)?;

    let limit_order_margin_amount =
        total_initial_margin.saturating_sub(position_only_initial_margin);

    let maintenance_margin_amount =
        position_maintenance_margin(perp_asset_metadata, total_initial_margin)?;

    let backstop_requirement_amount =
        position_backstop_margin(perp_asset_metadata, total_initial_margin)?;

    let high_risk_margin_amount =
        position_high_risk_margin(perp_asset_metadata, total_initial_margin)?;

    let cancel_margin_requirement =
        position_cancel_margin(perp_asset_metadata, total_initial_margin)?;

    // The sign is flipped to match the contract's `calculate_funding_payment`
    // semantics:
    // - Positive value = funding you will receive when settled (increases
    //   collateral)
    // - Negative value = funding you owe when settled (decreases collateral)
    let unsettled_funding = -(perp_asset_metadata.cumulative_funding_rate()
        - position.cumulative_funding_snapshot)
        * position.base_lot_position;

    let accumulated_funding: SignedQuoteLots =
        position.accumulated_funding_for_active_position.into();

    let calculator = MarketCalculator::new(
        perp_asset_metadata.base_lot_decimals(),
        perp_asset_metadata.tick_size(),
    );
    let position_value =
        calculator.position_value_for_position(position.base_lot_position, mark_price);

    Ok(Margin {
        maintenance_margin: maintenance_margin_amount,
        initial_margin: total_initial_margin,
        initial_margin_for_withdrawals,

        limit_order_margin: limit_order_margin_amount,
        backstop_requirement: backstop_requirement_amount,
        high_risk_margin: high_risk_margin_amount,
        at_risk_margin: total_initial_margin,
        cancel_margin: cancel_margin_requirement,

        unrealized_pnl,
        discounted_unrealized_pnl,
        discounted_pnl_for_withdrawals,
        unsettled_funding,
        accumulated_funding,
        position_value,
    })
}
