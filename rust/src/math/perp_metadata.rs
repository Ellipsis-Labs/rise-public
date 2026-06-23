//! Simplified perpetual asset metadata for margin calculations
//!
//! This module provides a minimal PerpAssetMetadata struct containing only
//! the fields required for offline margin calculations, without the complex
//! oracle and funding logic from the on-chain program.

use crate::math::leverage_tiers::LeverageTiers;
use crate::math::quantities::{
    BasisPoints, QuoteLotsPerBaseLotPerTick, SignedQuoteLotsPerBaseLot, Ticks, WrapperNum,
};
use crate::math::risk::{ProgramError, RiskAction, RiskTier};

/// Simplified perpetual asset metadata for margin calculations
///
/// This struct contains the minimal set of parameters needed to compute
/// margin requirements offline. It is designed to be constructed from
/// HTTP API data (ExchangeMarketConfig) combined with WebSocket updates
/// (MarketStatsUpdate for mark price).
///
/// # Fields Required for Margin Calculation
///
/// - **mark_price**: Current mark price in ticks (from WebSocket)
/// - **tick_size**: Conversion factor between ticks and quote lots
/// - **leverage_tiers**: Position-size-dependent maximum leverage
/// - **risk_factors**: Margin multipliers for different risk tiers
///   [maintenance, backstop, high_risk]
/// - **cancel_order_risk_factor**: Threshold for forced order cancellation
#[derive(Debug, Clone)]
pub struct PerpAssetMetadata {
    /// Symbol identifier (e.g., "SOL", "BTC")
    pub symbol: String,

    /// Asset identifier
    pub asset_id: u64,

    /// Number of decimals for base lot conversions
    pub base_lot_decimals: i8,

    /// Current mark price in ticks (updated from WebSocket)
    pub mark_price: Ticks,

    /// Tick size: quote lots per base lot per tick
    /// Static from HTTP API config
    pub tick_size: QuoteLotsPerBaseLotPerTick,

    /// Leverage tiers defining max leverage by position size
    /// Static from HTTP API config
    pub leverage_tiers: LeverageTiers,

    /// Risk factors in basis points: [maintenance, backstop, high_risk]
    /// Maintenance: ~5000 (50%) - liquidation threshold
    /// Backstop: ~4000 (40%) - backstop liquidation
    /// High_risk: ~3000 (30%) - high risk threshold
    /// Static from HTTP API config
    pub risk_factors: [u16; 3],

    /// Cancel order risk factor in basis points (e.g., 5500 = 55%)
    /// Orders can be force-cancelled when margin falls below this threshold
    /// Static from HTTP API config
    pub cancel_order_risk_factor: u16,

    /// UPnL risk factor for normal operations (basis points)
    /// Used to discount unrealized PnL in effective collateral calculation
    pub upnl_risk_factor: u16,

    /// UPnL risk factor for withdrawals (basis points)
    /// Typically stricter than normal operations to prevent
    /// undercollateralization
    pub upnl_risk_factor_for_withdrawals: u16,

    /// Cumulative funding rate for this market
    pub cumulative_funding_rate: SignedQuoteLotsPerBaseLot,
}

impl PerpAssetMetadata {
    /// Create a new perpetual asset metadata
    pub fn new(
        symbol: String,
        asset_id: u64,
        base_lot_decimals: i8,
        mark_price: Ticks,
        tick_size: QuoteLotsPerBaseLotPerTick,
        leverage_tiers: LeverageTiers,
        risk_factors: [u16; 3],
        cancel_order_risk_factor: u16,
        upnl_risk_factor: u16,
        upnl_risk_factor_for_withdrawals: u16,
    ) -> Self {
        Self {
            symbol,
            asset_id,
            base_lot_decimals,
            mark_price,
            tick_size,
            leverage_tiers,
            risk_factors,
            cancel_order_risk_factor,
            upnl_risk_factor,
            upnl_risk_factor_for_withdrawals,
            cumulative_funding_rate: SignedQuoteLotsPerBaseLot::ZERO,
        }
    }

    /// Get the current mark price
    #[inline(always)]
    pub fn try_get_mark_price(&self, _risk_action: RiskAction) -> Result<Ticks, ProgramError> {
        Ok(self.mark_price)
    }

    /// Get the base lot decimals
    #[inline(always)]
    pub fn base_lot_decimals(&self) -> i8 {
        self.base_lot_decimals
    }

    /// Get the asset identifier
    #[inline(always)]
    pub fn asset_id(&self) -> u64 {
        self.asset_id
    }

    /// Get the cumulative funding rate
    #[inline(always)]
    pub fn cumulative_funding_rate(&self) -> SignedQuoteLotsPerBaseLot {
        self.cumulative_funding_rate
    }

    /// Update the mark price (call when WebSocket update received)
    #[inline]
    pub fn set_mark_price(&mut self, new_price: Ticks) {
        self.mark_price = new_price;
    }

    /// Get the tick size
    #[inline]
    pub fn tick_size(&self) -> QuoteLotsPerBaseLotPerTick {
        self.tick_size
    }

    /// Get leverage tiers
    #[inline]
    pub fn leverage_tiers(&self) -> &LeverageTiers {
        &self.leverage_tiers
    }

    /// Get risk factor for a specific risk tier
    ///
    /// # Risk Tier to Index Mapping
    /// - Liquidatable (Tier 3) -> risk_factors[0] (maintenance margin)
    /// - BackstopLiquidatable (Tier 4) -> risk_factors[1] (backstop margin)
    /// - HighRisk (Tier 5) -> risk_factors[2] (high risk margin)
    #[inline]
    pub fn get_risk_factor(&self, risk_tier: RiskTier) -> BasisPoints {
        let index = match risk_tier {
            RiskTier::Liquidatable => 0,          // Maintenance margin
            RiskTier::BackstopLiquidatable => 1,  // Backstop margin
            RiskTier::HighRisk => 2,              // High risk margin
            _ => return BasisPoints::new(10_000), // Other tiers use 100%
        };

        BasisPoints::from_u16(self.risk_factors[index]).unwrap_or_else(|| BasisPoints::new(10_000))
    }

    /// Get cancel order risk factor
    #[inline]
    pub fn cancel_order_risk_factor(&self) -> BasisPoints {
        BasisPoints::from_u16(self.cancel_order_risk_factor)
            .unwrap_or_else(|| BasisPoints::new(10_000))
    }

    /// Get UPnL risk factor based on risk action
    #[inline]
    pub fn upnl_risk_factor(&self, risk_action: RiskAction) -> BasisPoints {
        let factor = match risk_action {
            RiskAction::Withdrawal { .. } => self.upnl_risk_factor_for_withdrawals,
            _ => self.upnl_risk_factor,
        };
        BasisPoints::from_u16(factor).unwrap_or_else(|| BasisPoints::new(10_000))
    }
}

impl Default for PerpAssetMetadata {
    fn default() -> Self {
        Self {
            symbol: String::from("UNKNOWN"),
            asset_id: 0,
            base_lot_decimals: 0,
            mark_price: Ticks::new(1_000_000), // Default to reasonable price
            tick_size: QuoteLotsPerBaseLotPerTick::new(1),
            leverage_tiers: LeverageTiers::default(),
            risk_factors: [5_000, 4_000, 3_000], // 50%, 40%, 30%
            cancel_order_risk_factor: 5_500,     // 55%
            upnl_risk_factor: 5_000,             // 50%
            upnl_risk_factor_for_withdrawals: 7_500, // 75%
            cumulative_funding_rate: SignedQuoteLotsPerBaseLot::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_get_mark_price() {
        let metadata = PerpAssetMetadata::default();
        let price = metadata
            .try_get_mark_price(RiskAction::View)
            .expect("Should return mark price");
        assert_eq!(price, Ticks::new(1_000_000));
    }

    #[test]
    fn test_set_mark_price() {
        let mut metadata = PerpAssetMetadata::default();
        metadata.set_mark_price(Ticks::new(2_000_000));
        assert_eq!(metadata.mark_price, Ticks::new(2_000_000));
    }

    #[test]
    fn test_get_risk_factor() {
        let metadata = PerpAssetMetadata::default();

        // Maintenance margin (50%)
        assert_eq!(
            metadata.get_risk_factor(RiskTier::Liquidatable),
            BasisPoints::new(5_000)
        );

        // Backstop margin (40%)
        assert_eq!(
            metadata.get_risk_factor(RiskTier::BackstopLiquidatable),
            BasisPoints::new(4_000)
        );

        // High risk margin (30%)
        assert_eq!(
            metadata.get_risk_factor(RiskTier::HighRisk),
            BasisPoints::new(3_000)
        );

        // Other tiers return 100%
        assert_eq!(
            metadata.get_risk_factor(RiskTier::Safe),
            BasisPoints::new(10_000)
        );
    }

    #[test]
    fn test_cancel_order_risk_factor() {
        let metadata = PerpAssetMetadata::default();
        assert_eq!(metadata.cancel_order_risk_factor(), BasisPoints::new(5_500));
    }

    #[test]
    fn test_upnl_risk_factor() {
        let metadata = PerpAssetMetadata::default();

        // Normal operations use standard UPnL factor
        assert_eq!(
            metadata.upnl_risk_factor(RiskAction::View),
            BasisPoints::new(5_000)
        );

        // Withdrawals use stricter factor
        assert_eq!(
            metadata.upnl_risk_factor(RiskAction::Withdrawal {
                current_slot: crate::math::quantities::Slot::new(1000)
            }),
            BasisPoints::new(7_500)
        );
    }
}
