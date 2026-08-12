//! Pure Phoenix math helpers for Rise.
//!
//! This crate contains the SDK's dependency-light market math, margin/risk
//! calculations, funding helpers, portfolio aggregation, and fixed-point
//! quantity wrappers. [`MarketCalculator`] is the usual starting point for
//! converting between UI prices, ticks, base lots, quote lots, and notional
//! values. The margin modules are useful when simulating orders or positions
//! from account data before submitting instructions.

pub mod direction;
pub mod errors;
pub mod fixed;
pub mod funding;
pub mod leverage_tiers;
pub mod limit_order_state;
pub mod margin;
pub mod margin_calc;
pub mod market_math;
pub mod perp_metadata;
pub mod portfolio;
pub mod price;
pub mod quantities;
pub mod risk;
pub mod spot_collateral;
pub mod trader_position;

pub use direction::{Direction, Side, StopLossOrderKind};
pub use errors::PhoenixStateError;
pub use fixed::I80F48;
pub use funding::FundingCalculator;
pub use leverage_tiers::{LeverageTier, LeverageTiers};
pub use limit_order_state::{LimitOrderMarginState, LimitOrderSummary};
pub use margin::{LimitOrder, Margin, MarketMargin, MarketPosition, OrderMargin};
pub use margin_calc::{
    initial_margin_for_asset, initial_margin_for_asset_for_withdrawals,
    initial_margin_for_asset_with_mark_price, margin_increase_for_asks, margin_increase_for_bids,
    position_backstop_margin, position_cancel_margin, position_high_risk_margin,
    position_maintenance_margin,
};
pub use market_math::{MarketCalculator, RoundingMode};
pub use perp_metadata::PerpAssetMetadata;
pub use portfolio::{
    CalculateLiquidationPriceUsdInput, MarginScenario, MarginScenarioResult,
    MarginSimulationAction, MarginSimulationActionReport, MarginSimulationDelta,
    MarginSimulationMode, PerpMetadataProvider, ProjectedLiquidation, ProjectedLiquidationFill,
    ProjectedLiquidationParams, Pubkey as PortfolioPubkey, SimulateMarginParams,
    SimulateMarginScenariosParams, SimulatePositionFillParams, SimulatedMargin,
    SimulatedMarginScenarios, SimulatedPositionFill, SpotCollateralMargin,
    StaticLiquidationPriceParams, StopLossInfo, TraderPortfolio, TraderPortfolioBuilder,
    TraderPortfolioMargin, calculate_liquidation_price_usd,
    calculate_liquidation_price_usd_with_target_limit_order_maintenance,
    projected_liquidation_price, static_liquidation_price_ticks,
};
pub use price::{Price, dynamic_price_decimals};
pub use quantities::{
    BaseLots, BaseLotsPerBaseUnit, BaseLotsPerTick, BaseUnits, BasisPoints, BasisPointsU32,
    Constant, FeeRateMicro, FundingRateUnitInSeconds, Lamports, MathError, MicroDivisor, QuoteLots,
    QuoteLotsPerBaseLot, QuoteLotsPerBaseLotPerTick, QuoteLotsPerQuoteUnit, QuoteUnits,
    ScalarBounds, SequenceNumberU8, SignedBaseLots, SignedBaseLotsUpcasted, SignedConstant,
    SignedFeeRateMicro, SignedQuoteLots, SignedQuoteLotsBaseLots, SignedQuoteLotsBaseLotsUpcasted,
    SignedQuoteLotsI56, SignedQuoteLotsI56Error, SignedQuoteLotsPerBaseLot,
    SignedQuoteLotsPerBaseLotUpcasted, SignedQuoteLotsUpcasted, SignedTicks, Slot, Ticks,
    UPnlRiskFactor, WrapperNum,
};
pub use risk::{MarginError, MarginState, ProgramError, RiskAction, RiskState, RiskTier};
use sha2_const_stable::Sha256;
pub use spot_collateral::{
    NATIVE_SOL_ASSET_INDEX, SpotCollateralParams, SpotCollateralValuationError,
    discounted_spot_collateral, margin_retained_bps, notional_spot_collateral,
    spot_collateral_price,
};
pub use trader_position::{TraderPosition, calculate_unrealized_pnl};

pub const fn sha2_const(input: &[u8]) -> u64 {
    let hash = Sha256::new().update(input).finalize();
    u64::from_le_bytes([
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
    ])
}
