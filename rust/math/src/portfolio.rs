//! Portfolio-level types and aggregation
//!
//! This module provides types for representing a trader's portfolio across
//! multiple markets, computing portfolio-level margin, and liquidation pricing.

use std::collections::{HashMap, HashSet};

#[cfg(feature = "solana")]
pub use solana_pubkey::Pubkey;
#[cfg(not(feature = "solana"))]
pub type Pubkey = [u8; 32];

use crate::direction::{Direction, Side, StopLossOrderKind};
use crate::errors::PhoenixStateError;
use crate::margin::{LimitOrder, Margin, MarketMargin, MarketPosition};
use crate::market_math::MarketCalculator;
use crate::perp_metadata::PerpAssetMetadata;
use crate::quantities::{
    BaseLots, BasisPoints, MathError, QuoteLots, SequenceNumberU8, SignedBaseLots, SignedQuoteLots,
    Ticks,
};
use crate::risk::{MarginError, MarginState, RiskAction, RiskState, RiskTier};
use crate::trader_position::TraderPosition;

/// Trait for providing perp asset metadata needed for margin calculations.
/// This allows different implementations (PhoenixState, PerpAssetMap, off-chain
/// caches) to provide the necessary market data for computing position margin.
pub trait PerpMetadataProvider {
    /// Get the perp asset metadata for a given symbol
    fn get_perp_metadata(&self, symbol: &str) -> Option<&PerpAssetMetadata>;

    fn get_all_markets(&self) -> Vec<String>;
}

impl PerpMetadataProvider for HashMap<String, PerpAssetMetadata> {
    fn get_perp_metadata(&self, symbol: &str) -> Option<&PerpAssetMetadata> {
        self.get(symbol)
    }

    fn get_all_markets(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StopLossInfo {
    pub(crate) funder_key: Pubkey,
    pub(crate) trader_key: Pubkey,
    pub(crate) asset_id: u64,
    pub(crate) trigger_price: Ticks,
    pub(crate) execution_price: Ticks,
    pub(crate) slot: u64,
    pub(crate) order_kind: StopLossOrderKind,
    pub(crate) position_sequence_number: u8,
    pub(crate) execution_direction: Direction,
    pub(crate) trade_side: Side,
}

/// A trader's complete portfolio across all markets.
/// Contains positions, limit orders, and collateral but no computed margin.
#[derive(Default, Debug, Clone)]
pub struct TraderPortfolio {
    pub authority: Pubkey,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,

    pub quote_lot_collateral: SignedQuoteLots,

    pub positions: HashMap<String, TraderPosition>,
    /// Individual limit orders per market
    pub limit_orders: HashMap<String, Vec<LimitOrder>>,
    pub stop_losses: Vec<StopLossInfo>,
}

/// Builder for constructing a [`TraderPortfolio`] incrementally.
#[derive(Default, Debug, Clone)]
pub struct TraderPortfolioBuilder {
    authority: Pubkey,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    quote_lot_collateral: SignedQuoteLots,
    positions: HashMap<String, TraderPosition>,
    limit_orders: HashMap<String, Vec<LimitOrder>>,
    stop_losses: Vec<StopLossInfo>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum MarginSimulationMode {
    /// Keep all markets in the portfolio when projecting margin.
    #[default]
    Cross,
    /// Project only the target market against an isolated collateral balance.
    Isolated,
}

/// Inputs for [`TraderPortfolio::simulate_position_fill`].
#[derive(Debug, Clone, Copy)]
pub struct SimulatePositionFillParams<'a> {
    /// Market symbol to simulate.
    pub symbol: &'a str,
    /// Fill side. Bids increase base lots; asks decrease base lots.
    pub side: Side,
    /// Fill size in base lots.
    pub base_lots: BaseLots,
    /// Fill price in ticks.
    pub price_ticks: Ticks,
    /// Trading fee deducted from collateral after the fill. When omitted,
    /// `simulate_position_fill` assumes the caller is modeling a zero-fee path.
    pub fee_quote_lots: Option<QuoteLots>,
    /// Whether to project cross-margin or isolated-margin state.
    pub margin_mode: MarginSimulationMode,
    /// Isolated collateral balance to use when `margin_mode` is isolated.
    /// When omitted, the portfolio's current collateral balance is used.
    pub isolated_collateral: Option<SignedQuoteLots>,
}

/// Action to apply to a cloned portfolio in
/// [`TraderPortfolio::simulate_margin`].
#[derive(Debug, Clone)]
pub enum MarginSimulationAction {
    /// Apply a fully filled position change. `price_ticks` is treated as the
    /// exact fill price; slippage is intentionally ignored. `fee_quote_lots`
    /// is deducted from projected collateral after realized PnL.
    FillPosition {
        symbol: String,
        side: Side,
        base_lots: BaseLots,
        price_ticks: Ticks,
        fee_quote_lots: QuoteLots,
    },
    /// Close part or all of the current position with a simulated fill. The
    /// close size is `|position| * fraction_bps / 10_000` (floor) and the side
    /// opposes the current position. When `price_ticks` is omitted, the
    /// current scenario mark price is used.
    ClosePosition {
        symbol: String,
        fraction_bps: BasisPoints,
        price_ticks: Option<Ticks>,
        fee_quote_lots: QuoteLots,
    },
    /// Add or replace a resting limit order. The simulator does not match this
    /// order against the book.
    PlaceLimitOrder { symbol: String, order: LimitOrder },
    /// Remove one resting limit order by sequence number.
    CancelOrder {
        symbol: String,
        order_sequence_number: u64,
    },
    /// Remove all resting limit orders, optionally scoped to one market.
    CancelAllOrders { symbol: Option<String> },
    /// Add a signed collateral delta. Positive values model deposits or
    /// transfers in; negative values model withdrawals or transfers out.
    AdjustCollateral { quote_lots_delta: SignedQuoteLots },
    /// Replace the simulated collateral balance.
    SetCollateral {
        quote_lot_collateral: SignedQuoteLots,
    },
    /// Settle current unsettled funding into collateral, optionally scoped to
    /// one market in cross-margin mode.
    SettleFunding { symbol: Option<String> },
    /// Add caller-estimated funding to simulated unsettled funding. Positive
    /// values increase effective collateral.
    ApplyFundingPayment {
        symbol: String,
        quote_lots: SignedQuoteLots,
    },
    /// Override the mark price for subsequent actions and final margin.
    SetMarkPrice { symbol: String, mark_price: Ticks },
    /// Move the mark price by a signed basis-point delta relative to the
    /// current scenario mark price: `new = current * (10_000 + delta) /
    /// 10_000` (floor). The result must stay a positive valid tick price.
    MoveMarkPrice {
        symbol: String,
        percent_delta_bps: i64,
    },
}

/// Per-action details returned by [`TraderPortfolio::simulate_margin`].
#[derive(Debug, Clone)]
pub enum MarginSimulationActionReport {
    FillPosition {
        symbol: String,
        price_ticks: Ticks,
        price_usd: f64,
        realized_pnl: SignedQuoteLots,
        fee: QuoteLots,
        settled_funding: SignedQuoteLots,
        projected_position: Option<TraderPosition>,
    },
    ClosePosition {
        symbol: String,
        closed_base_lots: BaseLots,
        price_ticks: Ticks,
        price_usd: f64,
        realized_pnl: SignedQuoteLots,
        fee: QuoteLots,
        settled_funding: SignedQuoteLots,
        projected_position: Option<TraderPosition>,
    },
    PlaceLimitOrder {
        symbol: String,
        order_sequence_number: u64,
        settled_funding: SignedQuoteLots,
    },
    CancelOrder {
        symbol: String,
        order_sequence_number: u64,
        removed: bool,
        settled_funding: SignedQuoteLots,
    },
    CancelAllOrders {
        symbol: Option<String>,
        removed: usize,
        settled_funding: SignedQuoteLots,
    },
    AdjustCollateral {
        quote_lots_delta: SignedQuoteLots,
    },
    SetCollateral {
        previous_quote_lot_collateral: SignedQuoteLots,
        new_quote_lot_collateral: SignedQuoteLots,
    },
    SettleFunding {
        symbol: Option<String>,
        settled_funding: SignedQuoteLots,
    },
    ApplyFundingPayment {
        symbol: String,
        funding_payment: SignedQuoteLots,
    },
    SetMarkPrice {
        symbol: String,
        previous_mark_price: Ticks,
        new_mark_price: Ticks,
    },
    MoveMarkPrice {
        symbol: String,
        percent_delta_bps: i64,
        previous_mark_price: Ticks,
        new_mark_price: Ticks,
    },
}

/// Inputs for [`TraderPortfolio::simulate_margin`].
#[derive(Debug, Clone, Default)]
pub struct SimulateMarginParams {
    pub margin_mode: MarginSimulationMode,
    pub isolated_symbol: Option<String>,
    pub isolated_collateral: Option<SignedQuoteLots>,
    /// Mark prices to use instead of the provider marks for the whole
    /// simulation, including the `before` margin. Use this to compute against
    /// caller-supplied prices (e.g. API price overrides). To model a mark move
    /// as part of the scenario itself, use the `SetMarkPrice` or
    /// `MoveMarkPrice` actions instead.
    pub mark_price_overrides: HashMap<String, Ticks>,
    pub actions: Vec<MarginSimulationAction>,
}

/// A labeled action list applied on top of the shared base state in
/// [`TraderPortfolio::simulate_margin_scenarios`].
#[derive(Debug, Clone)]
pub struct MarginScenario {
    pub label: String,
    pub actions: Vec<MarginSimulationAction>,
}

/// Inputs for [`TraderPortfolio::simulate_margin_scenarios`].
#[derive(Debug, Clone, Default)]
pub struct SimulateMarginScenariosParams {
    pub margin_mode: MarginSimulationMode,
    pub isolated_symbol: Option<String>,
    pub isolated_collateral: Option<SignedQuoteLots>,
    /// Mark prices to use instead of the provider marks for the whole batch,
    /// including the `before` and base margins.
    pub mark_price_overrides: HashMap<String, Ticks>,
    /// Actions applied once to form the shared baseline every scenario starts
    /// from.
    pub base_actions: Vec<MarginSimulationAction>,
    pub scenarios: Vec<MarginScenario>,
}

/// One scenario outcome from [`TraderPortfolio::simulate_margin_scenarios`].
/// `simulation.before` is the shared post-base-action margin.
#[derive(Debug, Clone)]
pub struct MarginScenarioResult {
    pub label: String,
    pub simulation: SimulatedMargin,
}

/// Result of a batched margin simulation over independent scenarios.
#[derive(Debug, Clone)]
pub struct SimulatedMarginScenarios {
    pub margin_mode: MarginSimulationMode,
    /// Margin before base actions and scenarios (after mark-price overrides).
    pub before: TraderPortfolioMargin,
    /// Margin after base actions; every scenario branches from this state.
    pub base: TraderPortfolioMargin,
    pub base_action_reports: Vec<MarginSimulationActionReport>,
    pub scenarios: Vec<MarginScenarioResult>,
}

/// Quote-lot deltas between the pre-action and post-action margin results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginSimulationDelta {
    pub collateral: SignedQuoteLots,
    pub effective_collateral: SignedQuoteLots,
    pub initial_margin: SignedQuoteLots,
    pub maintenance_margin: SignedQuoteLots,
    pub unrealized_pnl: SignedQuoteLots,
    pub unsettled_funding: SignedQuoteLots,
}

/// Result of an action-based margin simulation.
#[derive(Debug, Clone)]
pub struct SimulatedMargin {
    pub margin_mode: MarginSimulationMode,
    pub before: TraderPortfolioMargin,
    pub after: TraderPortfolioMargin,
    pub delta: MarginSimulationDelta,
    pub projected_portfolio: TraderPortfolio,
    pub liquidation_prices_usd: HashMap<String, Option<f64>>,
    pub action_reports: Vec<MarginSimulationActionReport>,
    /// Funding adjustments that were applied to simulated unsettled funding but
    /// not settled into collateral. [`TraderPortfolio`] mirrors account data
    /// and does not have an explicit free-form unsettled-funding field, so
    /// callers that persist projected state should carry these adjustments
    /// alongside `projected_portfolio`.
    pub unsettled_funding_adjustments: HashMap<String, SignedQuoteLots>,
}

/// Result of a simulated position fill.
#[derive(Debug, Clone)]
pub struct SimulatedPositionFill {
    pub margin_mode: MarginSimulationMode,
    pub symbol: String,
    pub price_ticks: Ticks,
    pub price_usd: f64,
    pub realized_pnl: SignedQuoteLots,
    pub fee: QuoteLots,
    pub projected_position: Option<TraderPosition>,
    pub projected_portfolio: TraderPortfolio,
    pub margin: TraderPortfolioMargin,
    pub liquidation_price_usd: Option<f64>,
}

/// Inputs for the closed-form liquidation price equation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalculateLiquidationPriceUsdInput {
    pub position_size: f64,
    pub entry_price_usd: f64,
    pub leverage: f64,
    pub maintenance_margin_bps: f64,
    pub upnl_risk_factor_bps: f64,
    pub collateral_usd: f64,
    pub other_asset_unrealized_pnl_usd: f64,
    pub other_asset_maintenance_margin_usd: f64,
    pub allow_non_positive: bool,
}

/// Solve the SDK liquidation price preview equation in USD.
///
/// Assumptions: target-market limit-order maintenance is supplied as a fixed
/// outside term by callers, soft-stale oracle margin inflation is not modeled,
/// and positive target-market uPnL is discounted with `upnl_risk_factor_bps`.
pub fn calculate_liquidation_price_usd(input: CalculateLiquidationPriceUsdInput) -> Option<f64> {
    const EPSILON: f64 = 1e-12;
    const BPS_DENOMINATOR: f64 = 10_000.0;

    if input.position_size.abs() < EPSILON
        || input.leverage <= 0.0
        || input.maintenance_margin_bps <= 0.0
    {
        return None;
    }

    let price = solve_liquidation_price_usd(&input, 1.0)?;
    let target_unrealized_pnl = input.position_size * (price - input.entry_price_usd);
    let price = if target_unrealized_pnl > EPSILON && input.upnl_risk_factor_bps < BPS_DENOMINATOR {
        solve_liquidation_price_usd(&input, input.upnl_risk_factor_bps / BPS_DENOMINATOR)?
    } else {
        price
    };

    if !price.is_finite() || (!input.allow_non_positive && price <= 0.0) {
        return None;
    }

    Some(price)
}

fn solve_liquidation_price_usd(
    input: &CalculateLiquidationPriceUsdInput,
    target_upnl_multiplier: f64,
) -> Option<f64> {
    const EPSILON: f64 = 1e-12;
    const BPS_DENOMINATOR: f64 = 10_000.0;

    let maintenance_ratio = input.maintenance_margin_bps / BPS_DENOMINATOR / input.leverage;
    let maintenance_coefficient = input.position_size.abs() * maintenance_ratio;
    let numerator = input.collateral_usd + input.other_asset_unrealized_pnl_usd
        - input.other_asset_maintenance_margin_usd
        - target_upnl_multiplier * input.entry_price_usd * input.position_size;
    let denominator = maintenance_coefficient - target_upnl_multiplier * input.position_size;

    if denominator.abs() < EPSILON {
        return None;
    }

    let price = numerator / denominator;
    price.is_finite().then_some(price)
}

impl TraderPortfolioBuilder {
    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = authority;
        self
    }

    pub fn trader_pda_index(mut self, index: u8) -> Self {
        self.trader_pda_index = index;
        self
    }

    pub fn trader_subaccount_index(mut self, index: u8) -> Self {
        self.trader_subaccount_index = index;
        self
    }

    pub fn quote_lot_collateral(mut self, collateral: SignedQuoteLots) -> Self {
        self.quote_lot_collateral = collateral;
        self
    }

    pub fn position(mut self, symbol: impl Into<String>, position: TraderPosition) -> Self {
        self.positions.insert(symbol.into(), position);
        self
    }

    pub fn limit_orders(mut self, symbol: impl Into<String>, orders: Vec<LimitOrder>) -> Self {
        self.limit_orders.insert(symbol.into(), orders);
        self
    }

    pub fn stop_loss(mut self, stop_loss: StopLossInfo) -> Self {
        self.stop_losses.push(stop_loss);
        self
    }

    pub fn build(self) -> TraderPortfolio {
        TraderPortfolio {
            authority: self.authority,
            trader_pda_index: self.trader_pda_index,
            trader_subaccount_index: self.trader_subaccount_index,
            quote_lot_collateral: self.quote_lot_collateral,
            positions: self.positions,
            limit_orders: self.limit_orders,
            stop_losses: self.stop_losses,
        }
    }
}

impl TraderPortfolio {
    pub fn builder() -> TraderPortfolioBuilder {
        TraderPortfolioBuilder::default()
    }

    fn get_positions(&self) -> HashMap<String, MarketPosition> {
        let mut trader_positions = HashMap::new();

        let mut limit_orders = self.limit_orders.clone();
        for (symbol, position) in self.positions.iter() {
            trader_positions.insert(
                symbol.clone(),
                MarketPosition {
                    position: Some(*position),
                    limit_orders: limit_orders.remove(symbol).unwrap_or_default(),
                },
            );
        }
        for (symbol, orders) in limit_orders {
            trader_positions.insert(
                symbol.clone(),
                MarketPosition {
                    position: None,
                    limit_orders: orders,
                },
            );
        }

        trader_positions
    }

    /// Compute margin and PnL margin across all markets in this portfolio.
    /// Uses the provided metadata provider to fetch market data.
    pub fn compute_margin(
        &self,
        provider: &impl PerpMetadataProvider,
    ) -> Result<TraderPortfolioMargin, PhoenixStateError> {
        let positions: HashMap<String, MarketMargin> = self
            .get_positions()
            .into_iter()
            .map(|(symbol, position)| {
                let perp_asset_metadata = provider.get_perp_metadata(&symbol).ok_or_else(|| {
                    PhoenixStateError::MarketNotFound {
                        symbol: symbol.clone(),
                        markets: vec![],
                    }
                })?;

                let margin = position.compute_margin(&symbol, provider)?;
                let limit_orders_with_margin =
                    position.compute_limit_orders_margin(perp_asset_metadata)?;

                Ok((
                    symbol.clone(),
                    MarketMargin {
                        position: position.position,
                        limit_orders: limit_orders_with_margin,
                        margin,
                    },
                ))
            })
            .collect::<Result<HashMap<String, MarketMargin>, PhoenixStateError>>()?;

        let margin: Margin = positions.values().map(|p| p.margin).sum();

        Ok(TraderPortfolioMargin {
            authority: self.authority,
            trader_pda_index: self.trader_pda_index,
            trader_subaccount_index: self.trader_subaccount_index,
            quote_lot_collateral: self.quote_lot_collateral,
            margin,
            positions,
            stop_losses: self.stop_losses.clone(),
        })
    }

    /// Simulate a fully filled position change and recompute margin plus the
    /// target market's liquidation price. `price_ticks` is treated as the exact
    /// fill price; slippage is intentionally ignored.
    pub fn simulate_position_fill(
        &self,
        params: SimulatePositionFillParams<'_>,
        provider: &impl PerpMetadataProvider,
    ) -> Result<SimulatedPositionFill, PhoenixStateError> {
        let symbol = params.symbol.to_string();
        let simulation = self.simulate_margin(
            SimulateMarginParams {
                margin_mode: params.margin_mode,
                isolated_symbol: Some(symbol.clone()),
                isolated_collateral: params.isolated_collateral,
                mark_price_overrides: HashMap::new(),
                actions: vec![MarginSimulationAction::FillPosition {
                    symbol: symbol.clone(),
                    side: params.side,
                    base_lots: params.base_lots,
                    price_ticks: params.price_ticks,
                    fee_quote_lots: params.fee_quote_lots.unwrap_or(QuoteLots::ZERO),
                }],
            },
            provider,
        )?;

        let fill_report = simulation
            .action_reports
            .iter()
            .find_map(|report| match report {
                MarginSimulationActionReport::FillPosition {
                    symbol: report_symbol,
                    price_ticks,
                    price_usd,
                    realized_pnl,
                    fee,
                    projected_position,
                    ..
                } if report_symbol == &symbol => Some((
                    *price_ticks,
                    *price_usd,
                    *realized_pnl,
                    *fee,
                    *projected_position,
                )),
                _ => None,
            })
            .ok_or_else(|| PhoenixStateError::InvalidMarginSimulationInput {
                reason: format!("missing fill simulation report for {symbol}"),
            })?;

        Ok(SimulatedPositionFill {
            margin_mode: simulation.margin_mode,
            symbol: symbol.clone(),
            price_ticks: fill_report.0,
            price_usd: fill_report.1,
            realized_pnl: fill_report.2,
            fee: fill_report.3,
            projected_position: fill_report.4,
            projected_portfolio: simulation.projected_portfolio,
            margin: simulation.after,
            liquidation_price_usd: simulation
                .liquidation_prices_usd
                .get(&symbol)
                .copied()
                .flatten(),
        })
    }

    /// Simulate a sequence of margin-affecting actions against a cloned
    /// portfolio. Position fills use the provided price exactly; slippage is
    /// intentionally ignored.
    pub fn simulate_margin(
        &self,
        params: SimulateMarginParams,
        provider: &impl PerpMetadataProvider,
    ) -> Result<SimulatedMargin, PhoenixStateError> {
        let scope = resolve_simulation_scope(&params)?;
        validate_actions_in_scope(&params.actions, &scope)?;
        let mut state = SimulationState::new(self, &scope, &params.actions, provider)?;
        state.apply_mark_price_overrides(&params.mark_price_overrides)?;
        let before = state.compute_margin()?;
        let mut action_reports = Vec::with_capacity(params.actions.len());

        for action in &params.actions {
            action_reports.push(state.apply_action(action, &scope)?);
        }

        let after = state.compute_margin()?;
        let liquidation_prices_usd = liquidation_prices_for_margin(&after, &state.markets)?;
        let delta = compute_margin_simulation_delta(&before, &after)?;

        Ok(SimulatedMargin {
            margin_mode: scope.margin_mode,
            before,
            after,
            delta,
            projected_portfolio: state.portfolio,
            liquidation_prices_usd,
            action_reports,
            unsettled_funding_adjustments: state.unsettled_funding_adjustments,
        })
    }

    /// Simulate independent labeled scenarios that all branch from one shared
    /// baseline (`base_actions` applied to this portfolio). Each scenario's
    /// `simulation.before` is the shared post-base margin, so scenario deltas
    /// isolate the scenario's own effect.
    pub fn simulate_margin_scenarios(
        &self,
        params: SimulateMarginScenariosParams,
        provider: &impl PerpMetadataProvider,
    ) -> Result<SimulatedMarginScenarios, PhoenixStateError> {
        let mut combined_actions = params.base_actions.clone();
        for scenario in &params.scenarios {
            combined_actions.extend(scenario.actions.iter().cloned());
        }
        let scope_params = SimulateMarginParams {
            margin_mode: params.margin_mode,
            isolated_symbol: params.isolated_symbol.clone(),
            isolated_collateral: params.isolated_collateral,
            mark_price_overrides: params.mark_price_overrides.clone(),
            actions: combined_actions.clone(),
        };
        let scope = resolve_simulation_scope(&scope_params)?;
        validate_actions_in_scope(&combined_actions, &scope)?;
        let mut state = SimulationState::new(self, &scope, &combined_actions, provider)?;
        state.apply_mark_price_overrides(&params.mark_price_overrides)?;
        let before = state.compute_margin()?;

        let mut base_action_reports = Vec::with_capacity(params.base_actions.len());
        for action in &params.base_actions {
            base_action_reports.push(state.apply_action(action, &scope)?);
        }
        let base = state.compute_margin()?;

        let mut scenarios = Vec::with_capacity(params.scenarios.len());
        for scenario in params.scenarios {
            let mut scenario_state = state.clone();
            let mut action_reports = Vec::with_capacity(scenario.actions.len());
            for action in &scenario.actions {
                action_reports.push(scenario_state.apply_action(action, &scope)?);
            }
            let after = scenario_state.compute_margin()?;
            let liquidation_prices_usd =
                liquidation_prices_for_margin(&after, &scenario_state.markets)?;
            let delta = compute_margin_simulation_delta(&base, &after)?;
            scenarios.push(MarginScenarioResult {
                label: scenario.label,
                simulation: SimulatedMargin {
                    margin_mode: scope.margin_mode,
                    before: base.clone(),
                    after,
                    delta,
                    projected_portfolio: scenario_state.portfolio,
                    liquidation_prices_usd,
                    action_reports,
                    unsettled_funding_adjustments: scenario_state.unsettled_funding_adjustments,
                },
            });
        }

        Ok(SimulatedMarginScenarios {
            margin_mode: scope.margin_mode,
            before,
            base,
            base_action_reports,
            scenarios,
        })
    }

    fn isolated_projection_shell(
        &self,
        symbol: &str,
        collateral: SignedQuoteLots,
        asset_id: u64,
    ) -> TraderPortfolio {
        let mut limit_orders = HashMap::new();
        if let Some(orders) = self.limit_orders.get(symbol) {
            limit_orders.insert(symbol.to_string(), orders.clone());
        }
        let mut positions = HashMap::new();
        if let Some(position) = self.positions.get(symbol) {
            positions.insert(symbol.to_string(), *position);
        }

        TraderPortfolio {
            authority: self.authority,
            trader_pda_index: self.trader_pda_index,
            trader_subaccount_index: self.trader_subaccount_index,
            quote_lot_collateral: collateral,
            positions,
            limit_orders,
            stop_losses: self
                .stop_losses
                .iter()
                .filter(|stop_loss| stop_loss.asset_id == asset_id)
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedSimulationScope {
    margin_mode: MarginSimulationMode,
    isolated_symbol: Option<String>,
    isolated_collateral: Option<SignedQuoteLots>,
}

#[derive(Clone)]
struct SimulationState {
    portfolio: TraderPortfolio,
    markets: HashMap<String, PerpAssetMetadata>,
    unsettled_funding_adjustments: HashMap<String, SignedQuoteLots>,
}

impl SimulationState {
    fn new(
        source_portfolio: &TraderPortfolio,
        scope: &ResolvedSimulationScope,
        actions: &[MarginSimulationAction],
        provider: &impl PerpMetadataProvider,
    ) -> Result<Self, PhoenixStateError> {
        let required_symbols = required_market_symbols(source_portfolio, actions, scope);
        let markets = clone_provider_markets(provider, &required_symbols)?;
        let portfolio = match scope.margin_mode {
            MarginSimulationMode::Cross => source_portfolio.clone(),
            MarginSimulationMode::Isolated => {
                let symbol = scope.isolated_symbol.as_deref().ok_or_else(|| {
                    invalid_margin_simulation_input(
                        "isolated_symbol is required for isolated margin simulation",
                    )
                })?;
                let metadata = metadata_for_symbol(&markets, symbol)?;
                source_portfolio.isolated_projection_shell(
                    symbol,
                    scope
                        .isolated_collateral
                        .unwrap_or(source_portfolio.quote_lot_collateral),
                    metadata.asset_id(),
                )
            }
        };

        Ok(Self {
            portfolio,
            markets,
            unsettled_funding_adjustments: HashMap::new(),
        })
    }

    fn compute_margin(&self) -> Result<TraderPortfolioMargin, PhoenixStateError> {
        let mut margin = self.portfolio.compute_margin(&self.markets)?;
        apply_unsettled_funding_adjustments(&mut margin, &self.unsettled_funding_adjustments)?;
        Ok(margin)
    }

    fn apply_action(
        &mut self,
        action: &MarginSimulationAction,
        scope: &ResolvedSimulationScope,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        match action {
            MarginSimulationAction::FillPosition {
                symbol,
                side,
                base_lots,
                price_ticks,
                fee_quote_lots,
            } => self.apply_fill_position(
                scope,
                symbol,
                *side,
                *base_lots,
                *price_ticks,
                *fee_quote_lots,
            ),
            MarginSimulationAction::ClosePosition {
                symbol,
                fraction_bps,
                price_ticks,
                fee_quote_lots,
            } => self.apply_close_position(
                scope,
                symbol,
                *fraction_bps,
                *price_ticks,
                *fee_quote_lots,
            ),
            MarginSimulationAction::PlaceLimitOrder { symbol, order } => {
                self.apply_place_limit_order(scope, symbol, *order)
            }
            MarginSimulationAction::CancelOrder {
                symbol,
                order_sequence_number,
            } => self.apply_cancel_order(scope, symbol, *order_sequence_number),
            MarginSimulationAction::CancelAllOrders { symbol } => {
                self.apply_cancel_all_orders(scope, symbol.as_deref())
            }
            MarginSimulationAction::AdjustCollateral { quote_lots_delta } => {
                self.apply_adjust_collateral(*quote_lots_delta)
            }
            MarginSimulationAction::SetCollateral {
                quote_lot_collateral,
            } => Ok(self.apply_set_collateral(*quote_lot_collateral)),
            MarginSimulationAction::SettleFunding { symbol } => {
                self.apply_settle_funding(scope, symbol.as_deref())
            }
            MarginSimulationAction::ApplyFundingPayment { symbol, quote_lots } => {
                self.apply_funding_payment(symbol, *quote_lots)
            }
            MarginSimulationAction::SetMarkPrice { symbol, mark_price } => {
                self.apply_set_mark_price(symbol, *mark_price)
            }
            MarginSimulationAction::MoveMarkPrice {
                symbol,
                percent_delta_bps,
            } => self.apply_move_mark_price(symbol, *percent_delta_bps),
        }
    }

    fn apply_fill_position(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: &str,
        side: Side,
        base_lots: BaseLots,
        price_ticks: Ticks,
        fee_quote_lots: QuoteLots,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        if base_lots == BaseLots::ZERO || price_ticks == Ticks::ZERO {
            return Err(MathError::Underflow.into());
        }

        let metadata = metadata_for_symbol(&self.markets, symbol)?.clone();
        let settled_funding = self.settle_funding_for_action_scope(scope, None)?;
        let signed_base_lots = base_lots.as_signed();
        let delta_base_lots = match side {
            Side::Bid => signed_base_lots,
            Side::Ask => -signed_base_lots,
        };
        let current_position = self
            .portfolio
            .positions
            .get(symbol)
            .copied()
            .unwrap_or_else(|| empty_position_for_metadata(&metadata));
        let fill_quote_lots_per_base_lot = price_ticks * metadata.tick_size();
        let (projected_position, realized_pnl) = project_position_after_fill(
            current_position,
            delta_base_lots,
            fill_quote_lots_per_base_lot.as_inner(),
        )?;

        let stop_losses_invalidated = current_position.base_lot_position != SignedBaseLots::ZERO
            && projected_position.is_none_or(|position| {
                position.base_lot_position.as_inner().signum()
                    != current_position.base_lot_position.as_inner().signum()
            });
        // Mirrors TradersView::inc_position_sequence_numbers: a fill that
        // flips through zero starts a new position, so detached stop-losses
        // must not match the projected sequence number.
        let projected_position = projected_position.map(|mut position| {
            if stop_losses_invalidated {
                position.position_sequence_number = SequenceNumberU8::from(
                    position.position_sequence_number.as_u8().wrapping_add(1),
                );
            }
            position
        });

        if let Some(position) = projected_position {
            self.portfolio
                .positions
                .insert(symbol.to_string(), position);
        } else {
            self.portfolio.positions.remove(symbol);
            prune_empty_market(&mut self.portfolio, symbol);
        }
        if stop_losses_invalidated {
            self.portfolio
                .stop_losses
                .retain(|stop_loss| stop_loss.asset_id != metadata.asset_id());
        }

        let fee = fee_quote_lots.checked_as_signed()?;
        let collateral_delta = realized_pnl.checked_sub(fee).ok_or(MathError::Overflow)?;
        self.portfolio.quote_lot_collateral =
            add_signed_quote_lots(self.portfolio.quote_lot_collateral, collateral_delta)?;

        let calculator = MarketCalculator::new(metadata.base_lot_decimals(), metadata.tick_size());
        Ok(MarginSimulationActionReport::FillPosition {
            symbol: symbol.to_string(),
            price_ticks,
            price_usd: calculator.ticks_to_price(price_ticks),
            realized_pnl,
            fee: fee_quote_lots,
            settled_funding,
            projected_position,
        })
    }

    fn apply_place_limit_order(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: &str,
        order: LimitOrder,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        if order.base_lot_size == BaseLots::ZERO || order.price == Ticks::ZERO {
            return Err(MathError::Underflow.into());
        }
        metadata_for_symbol(&self.markets, symbol)?;

        let settled_funding = self.settle_funding_for_action_scope(scope, None)?;
        let orders = self
            .portfolio
            .limit_orders
            .entry(symbol.to_string())
            .or_default();
        orders.retain(|existing| existing.order_sequence_number != order.order_sequence_number);
        orders.push(order);

        Ok(MarginSimulationActionReport::PlaceLimitOrder {
            symbol: symbol.to_string(),
            order_sequence_number: order.order_sequence_number,
            settled_funding,
        })
    }

    fn apply_cancel_order(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: &str,
        order_sequence_number: u64,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        let settled_funding = self.settle_funding_for_action_scope(scope, None)?;
        let removed = if let Some(orders) = self.portfolio.limit_orders.get_mut(symbol) {
            let original_len = orders.len();
            orders.retain(|order| order.order_sequence_number != order_sequence_number);
            original_len != orders.len()
        } else {
            false
        };
        prune_empty_market(&mut self.portfolio, symbol);

        Ok(MarginSimulationActionReport::CancelOrder {
            symbol: symbol.to_string(),
            order_sequence_number,
            removed,
            settled_funding,
        })
    }

    fn apply_cancel_all_orders(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: Option<&str>,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        let settled_funding = self.settle_funding_for_action_scope(scope, None)?;
        let removed = if let Some(symbol) = symbol {
            let removed = self.portfolio.limit_orders.get(symbol).map_or(0, Vec::len);
            self.portfolio.limit_orders.remove(symbol);
            removed
        } else {
            let removed = self.portfolio.limit_orders.values().map(Vec::len).sum();
            self.portfolio.limit_orders.clear();
            removed
        };

        Ok(MarginSimulationActionReport::CancelAllOrders {
            symbol: symbol.map(str::to_string),
            removed,
            settled_funding,
        })
    }

    fn apply_adjust_collateral(
        &mut self,
        quote_lots_delta: SignedQuoteLots,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        self.portfolio.quote_lot_collateral =
            add_signed_quote_lots(self.portfolio.quote_lot_collateral, quote_lots_delta)?;

        Ok(MarginSimulationActionReport::AdjustCollateral { quote_lots_delta })
    }

    fn apply_set_collateral(
        &mut self,
        quote_lot_collateral: SignedQuoteLots,
    ) -> MarginSimulationActionReport {
        let previous_quote_lot_collateral = self.portfolio.quote_lot_collateral;
        self.portfolio.quote_lot_collateral = quote_lot_collateral;

        MarginSimulationActionReport::SetCollateral {
            previous_quote_lot_collateral,
            new_quote_lot_collateral: quote_lot_collateral,
        }
    }

    fn apply_settle_funding(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: Option<&str>,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        Ok(MarginSimulationActionReport::SettleFunding {
            symbol: symbol.map(str::to_string),
            settled_funding: self.settle_funding_for_action_scope(scope, symbol)?,
        })
    }

    fn apply_funding_payment(
        &mut self,
        symbol: &str,
        quote_lots: SignedQuoteLots,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        metadata_for_symbol(&self.markets, symbol)?;
        if !self.portfolio.positions.contains_key(symbol) {
            return Err(invalid_margin_simulation_input(format!(
                "cannot apply funding payment for {symbol} without an open position"
            )));
        }

        add_unsettled_funding_adjustment(
            &mut self.unsettled_funding_adjustments,
            symbol,
            quote_lots,
        )?;

        Ok(MarginSimulationActionReport::ApplyFundingPayment {
            symbol: symbol.to_string(),
            funding_payment: quote_lots,
        })
    }

    fn apply_set_mark_price(
        &mut self,
        symbol: &str,
        mark_price: Ticks,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        if mark_price == Ticks::ZERO {
            return Err(MathError::Underflow.into());
        }
        let available_markets = self.markets.keys().cloned().collect::<Vec<_>>();
        let market =
            self.markets
                .get_mut(symbol)
                .ok_or_else(|| PhoenixStateError::MarketNotFound {
                    symbol: symbol.to_string(),
                    markets: available_markets,
                })?;
        let previous_mark_price = market.mark_price;
        market.set_mark_price(mark_price);

        Ok(MarginSimulationActionReport::SetMarkPrice {
            symbol: symbol.to_string(),
            previous_mark_price,
            new_mark_price: mark_price,
        })
    }

    fn apply_close_position(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: &str,
        fraction_bps: BasisPoints,
        price_ticks: Option<Ticks>,
        fee_quote_lots: QuoteLots,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        let position = self
            .portfolio
            .positions
            .get(symbol)
            .copied()
            .ok_or_else(|| {
                invalid_margin_simulation_input(format!(
                    "cannot close position for {symbol} without an open position"
                ))
            })?;
        let close_base_lots_inner =
            (u128::from(position.base_lot_position.abs_as_unsigned().as_inner())
                * u128::from(fraction_bps.as_inner())
                / 10_000) as u64;
        let close_base_lots = BaseLots::new_checked(close_base_lots_inner)?;
        if close_base_lots == BaseLots::ZERO {
            return Err(invalid_margin_simulation_input(format!(
                "close fraction {} bps of position for {symbol} rounds to zero base lots",
                fraction_bps.as_inner()
            )));
        }
        let side = if position.base_lot_position > SignedBaseLots::ZERO {
            Side::Ask
        } else {
            Side::Bid
        };
        let price_ticks = match price_ticks {
            Some(price_ticks) => price_ticks,
            None => metadata_for_symbol(&self.markets, symbol)?.mark_price,
        };

        match self.apply_fill_position(
            scope,
            symbol,
            side,
            close_base_lots,
            price_ticks,
            fee_quote_lots,
        )? {
            MarginSimulationActionReport::FillPosition {
                symbol,
                price_ticks,
                price_usd,
                realized_pnl,
                fee,
                settled_funding,
                projected_position,
            } => Ok(MarginSimulationActionReport::ClosePosition {
                symbol,
                closed_base_lots: close_base_lots,
                price_ticks,
                price_usd,
                realized_pnl,
                fee,
                settled_funding,
                projected_position,
            }),
            report => Err(invalid_margin_simulation_input(format!(
                "unexpected close-position fill report: {report:?}"
            ))),
        }
    }

    fn apply_move_mark_price(
        &mut self,
        symbol: &str,
        percent_delta_bps: i64,
    ) -> Result<MarginSimulationActionReport, PhoenixStateError> {
        let current_mark_price = metadata_for_symbol(&self.markets, symbol)?.mark_price;
        let scaled = i128::from(current_mark_price.as_inner())
            * (10_000i128 + i128::from(percent_delta_bps))
            / 10_000;
        let new_mark_price = u64::try_from(scaled)
            .ok()
            .and_then(|value| Ticks::new_checked(value).ok())
            .filter(|value| *value != Ticks::ZERO)
            .ok_or_else(|| {
                invalid_margin_simulation_input(format!(
                    "mark price move of {percent_delta_bps} bps for {symbol} does not produce a \
                     positive in-range tick price"
                ))
            })?;

        match self.apply_set_mark_price(symbol, new_mark_price)? {
            MarginSimulationActionReport::SetMarkPrice {
                symbol,
                previous_mark_price,
                new_mark_price,
            } => Ok(MarginSimulationActionReport::MoveMarkPrice {
                symbol,
                percent_delta_bps,
                previous_mark_price,
                new_mark_price,
            }),
            report => Err(invalid_margin_simulation_input(format!(
                "unexpected mark price move report: {report:?}"
            ))),
        }
    }

    fn apply_mark_price_overrides(
        &mut self,
        overrides: &HashMap<String, Ticks>,
    ) -> Result<(), PhoenixStateError> {
        for (symbol, mark_price) in overrides {
            if *mark_price == Ticks::ZERO {
                return Err(invalid_margin_simulation_input(format!(
                    "mark price override for {symbol} must be positive"
                )));
            }
            let available_markets = self.markets.keys().cloned().collect::<Vec<_>>();
            let market =
                self.markets
                    .get_mut(symbol)
                    .ok_or_else(|| PhoenixStateError::MarketNotFound {
                        symbol: symbol.clone(),
                        markets: available_markets,
                    })?;
            market.set_mark_price(*mark_price);
        }
        Ok(())
    }

    fn settle_funding_for_action_scope(
        &mut self,
        scope: &ResolvedSimulationScope,
        symbol: Option<&str>,
    ) -> Result<SignedQuoteLots, PhoenixStateError> {
        let target_symbol = match scope.margin_mode {
            MarginSimulationMode::Cross => symbol.map(str::to_string),
            MarginSimulationMode::Isolated => {
                Some(scope.isolated_symbol.clone().ok_or_else(|| {
                    invalid_margin_simulation_input(
                        "isolated_symbol is required for isolated margin simulation",
                    )
                })?)
            }
        };
        let symbols = if let Some(symbol) = target_symbol {
            vec![symbol]
        } else {
            self.portfolio
                .positions
                .keys()
                .chain(self.unsettled_funding_adjustments.keys())
                .cloned()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect()
        };
        let mut total_payment = SignedQuoteLots::ZERO;

        for symbol in symbols {
            let mut payment = self
                .unsettled_funding_adjustments
                .remove(&symbol)
                .unwrap_or(SignedQuoteLots::ZERO);
            if let Some(position) = self.portfolio.positions.get_mut(&symbol) {
                let metadata =
                    self.markets
                        .get(&symbol)
                        .ok_or_else(|| PhoenixStateError::MarketNotFound {
                            symbol: symbol.clone(),
                            markets: self.markets.keys().cloned().collect(),
                        })?;
                let (settled_position, funding_payment) =
                    settle_funding_for_position(*position, metadata);
                *position = settled_position;
                payment = add_signed_quote_lots(payment, funding_payment)?;
            }
            total_payment = add_signed_quote_lots(total_payment, payment)?;
        }

        self.portfolio.quote_lot_collateral =
            add_signed_quote_lots(self.portfolio.quote_lot_collateral, total_payment)?;
        Ok(total_payment)
    }
}

fn resolve_simulation_scope(
    params: &SimulateMarginParams,
) -> Result<ResolvedSimulationScope, PhoenixStateError> {
    let isolated_symbol = params.isolated_symbol.clone().or_else(|| {
        if params.margin_mode == MarginSimulationMode::Isolated {
            infer_single_action_symbol(&params.actions)
        } else {
            None
        }
    });

    if params.margin_mode == MarginSimulationMode::Isolated && isolated_symbol.is_none() {
        return Err(invalid_margin_simulation_input(
            "isolated_symbol is required when simulating isolated margin",
        ));
    }

    Ok(ResolvedSimulationScope {
        margin_mode: params.margin_mode,
        isolated_symbol,
        isolated_collateral: params.isolated_collateral,
    })
}

fn validate_actions_in_scope(
    actions: &[MarginSimulationAction],
    scope: &ResolvedSimulationScope,
) -> Result<(), PhoenixStateError> {
    if scope.margin_mode != MarginSimulationMode::Isolated {
        return Ok(());
    }
    let isolated_symbol = scope
        .isolated_symbol
        .as_ref()
        .ok_or_else(|| invalid_margin_simulation_input("missing isolated symbol"))?;

    for symbol in action_symbols(actions) {
        if &symbol != isolated_symbol {
            return Err(invalid_margin_simulation_input(format!(
                "isolated margin simulation for {isolated_symbol} cannot include action for \
                 {symbol}"
            )));
        }
    }

    Ok(())
}

fn infer_single_action_symbol(actions: &[MarginSimulationAction]) -> Option<String> {
    let symbols = action_symbols(actions);
    if symbols.len() == 1 {
        symbols.into_iter().next()
    } else {
        None
    }
}

fn action_symbols(actions: &[MarginSimulationAction]) -> HashSet<String> {
    actions
        .iter()
        .filter_map(|action| match action {
            MarginSimulationAction::FillPosition { symbol, .. }
            | MarginSimulationAction::ClosePosition { symbol, .. }
            | MarginSimulationAction::PlaceLimitOrder { symbol, .. }
            | MarginSimulationAction::CancelOrder { symbol, .. }
            | MarginSimulationAction::ApplyFundingPayment { symbol, .. }
            | MarginSimulationAction::SetMarkPrice { symbol, .. }
            | MarginSimulationAction::MoveMarkPrice { symbol, .. } => Some(symbol.clone()),
            MarginSimulationAction::CancelAllOrders { symbol }
            | MarginSimulationAction::SettleFunding { symbol } => symbol.clone(),
            MarginSimulationAction::AdjustCollateral { .. }
            | MarginSimulationAction::SetCollateral { .. } => None,
        })
        .collect()
}

fn required_market_symbols(
    portfolio: &TraderPortfolio,
    actions: &[MarginSimulationAction],
    scope: &ResolvedSimulationScope,
) -> HashSet<String> {
    let mut symbols = action_symbols(actions);
    if let Some(symbol) = &scope.isolated_symbol {
        symbols.insert(symbol.clone());
    }
    if scope.margin_mode == MarginSimulationMode::Cross {
        symbols.extend(portfolio.positions.keys().cloned());
        symbols.extend(portfolio.limit_orders.keys().cloned());
    }
    symbols
}

fn clone_provider_markets(
    provider: &impl PerpMetadataProvider,
    required_symbols: &HashSet<String>,
) -> Result<HashMap<String, PerpAssetMetadata>, PhoenixStateError> {
    let mut markets = HashMap::new();
    for symbol in provider.get_all_markets() {
        if let Some(metadata) = provider.get_perp_metadata(&symbol) {
            markets.insert(symbol, metadata.clone());
        }
    }
    for symbol in required_symbols {
        if !markets.contains_key(symbol) {
            let metadata = provider.get_perp_metadata(symbol).ok_or_else(|| {
                PhoenixStateError::MarketNotFound {
                    symbol: symbol.clone(),
                    markets: provider.get_all_markets(),
                }
            })?;
            markets.insert(symbol.clone(), metadata.clone());
        }
    }

    Ok(markets)
}

fn metadata_for_symbol<'a>(
    markets: &'a HashMap<String, PerpAssetMetadata>,
    symbol: &str,
) -> Result<&'a PerpAssetMetadata, PhoenixStateError> {
    markets
        .get(symbol)
        .ok_or_else(|| PhoenixStateError::MarketNotFound {
            symbol: symbol.to_string(),
            markets: markets.keys().cloned().collect(),
        })
}

fn prune_empty_market(portfolio: &mut TraderPortfolio, symbol: &str) {
    if portfolio.positions.contains_key(symbol) {
        return;
    }
    if portfolio
        .limit_orders
        .get(symbol)
        .is_some_and(|orders| orders.is_empty())
    {
        portfolio.limit_orders.remove(symbol);
    }
}

fn add_unsettled_funding_adjustment(
    adjustments: &mut HashMap<String, SignedQuoteLots>,
    symbol: &str,
    quote_lots: SignedQuoteLots,
) -> Result<(), PhoenixStateError> {
    let next = add_signed_quote_lots(
        adjustments
            .get(symbol)
            .copied()
            .unwrap_or(SignedQuoteLots::ZERO),
        quote_lots,
    )?;
    if next == SignedQuoteLots::ZERO {
        adjustments.remove(symbol);
    } else {
        adjustments.insert(symbol.to_string(), next);
    }
    Ok(())
}

fn apply_unsettled_funding_adjustments(
    margin: &mut TraderPortfolioMargin,
    adjustments: &HashMap<String, SignedQuoteLots>,
) -> Result<(), PhoenixStateError> {
    for (symbol, adjustment) in adjustments {
        if *adjustment == SignedQuoteLots::ZERO {
            continue;
        }
        if let Some(market_margin) = margin.positions.get_mut(symbol) {
            market_margin.margin.unsettled_funding =
                add_signed_quote_lots(market_margin.margin.unsettled_funding, *adjustment)?;
        }
        margin.margin.unsettled_funding =
            add_signed_quote_lots(margin.margin.unsettled_funding, *adjustment)?;
    }

    Ok(())
}

fn compute_margin_simulation_delta(
    before: &TraderPortfolioMargin,
    after: &TraderPortfolioMargin,
) -> Result<MarginSimulationDelta, PhoenixStateError> {
    Ok(MarginSimulationDelta {
        collateral: signed_quote_lots_delta(
            after.quote_lot_collateral,
            before.quote_lot_collateral,
        )?,
        effective_collateral: signed_quote_lots_delta(
            after.effective_collateral(),
            before.effective_collateral(),
        )?,
        initial_margin: quote_lots_delta(
            after.margin.initial_margin,
            before.margin.initial_margin,
        )?,
        maintenance_margin: quote_lots_delta(
            after.margin.maintenance_margin,
            before.margin.maintenance_margin,
        )?,
        unrealized_pnl: signed_quote_lots_delta(
            after.margin.unrealized_pnl,
            before.margin.unrealized_pnl,
        )?,
        unsettled_funding: signed_quote_lots_delta(
            after.margin.unsettled_funding,
            before.margin.unsettled_funding,
        )?,
    })
}

fn liquidation_prices_for_margin(
    margin: &TraderPortfolioMargin,
    provider: &impl PerpMetadataProvider,
) -> Result<HashMap<String, Option<f64>>, PhoenixStateError> {
    let mut prices = HashMap::new();
    for (symbol, market_margin) in &margin.positions {
        if market_margin.position.is_none() {
            continue;
        }
        let metadata = provider.get_perp_metadata(symbol).ok_or_else(|| {
            PhoenixStateError::MarketNotFound {
                symbol: symbol.clone(),
                markets: provider.get_all_markets(),
            }
        })?;
        prices.insert(
            symbol.clone(),
            liquidation_price_for_market(market_margin, margin, metadata)?,
        );
    }
    Ok(prices)
}

fn quote_lots_delta(after: QuoteLots, before: QuoteLots) -> Result<SignedQuoteLots, MathError> {
    SignedQuoteLots::try_from(i128::from(after.as_inner()) - i128::from(before.as_inner()))
}

fn signed_quote_lots_delta(
    after: SignedQuoteLots,
    before: SignedQuoteLots,
) -> Result<SignedQuoteLots, MathError> {
    SignedQuoteLots::try_from(i128::from(after.as_inner()) - i128::from(before.as_inner()))
}

fn invalid_margin_simulation_input(reason: impl Into<String>) -> PhoenixStateError {
    PhoenixStateError::InvalidMarginSimulationInput {
        reason: reason.into(),
    }
}

fn project_position_after_fill(
    current_position: TraderPosition,
    delta_base_lots: SignedBaseLots,
    fill_quote_lots_per_base_lot: u64,
) -> Result<(Option<TraderPosition>, SignedQuoteLots), PhoenixStateError> {
    let current_base_lots = current_position.base_lot_position;
    let fill_virtual_quote_lots = -i128::from(delta_base_lots.as_inner())
        .checked_mul(i128::from(fill_quote_lots_per_base_lot))
        .ok_or(MathError::Overflow)?;

    if current_base_lots == SignedBaseLots::ZERO
        || current_base_lots.as_inner().signum() == delta_base_lots.as_inner().signum()
    {
        let projected_base_lots = current_base_lots
            .checked_add(delta_base_lots)
            .ok_or(MathError::Overflow)?;
        let projected_virtual_quote_lots =
            i128::from(current_position.virtual_quote_lot_position.as_inner())
                .checked_add(fill_virtual_quote_lots)
                .ok_or(MathError::Overflow)?;
        return Ok((
            projected_position_from_parts(
                current_position,
                projected_base_lots,
                signed_quote_lots_from_i128(projected_virtual_quote_lots)?,
                current_base_lots == SignedBaseLots::ZERO,
            ),
            SignedQuoteLots::ZERO,
        ));
    }

    let current_base_lots_i128 = i128::from(current_base_lots.as_inner());
    let delta_base_lots_i128 = i128::from(delta_base_lots.as_inner());
    let current_virtual_quote_lots_i128 =
        i128::from(current_position.virtual_quote_lot_position.as_inner());
    let base_lots_to_close_position = if delta_base_lots_i128.abs() > current_base_lots_i128.abs() {
        -current_base_lots_i128
    } else {
        delta_base_lots_i128
    };
    let base_lots_remaining_from_fill = delta_base_lots_i128
        .checked_sub(base_lots_to_close_position)
        .ok_or(MathError::Overflow)?;
    let quote_lots_to_close_position = checked_mul_div_i128(
        fill_virtual_quote_lots,
        base_lots_to_close_position,
        delta_base_lots_i128,
    )?;
    let quote_lots_remaining_from_fill = fill_virtual_quote_lots
        .checked_sub(quote_lots_to_close_position)
        .ok_or(MathError::Overflow)?;
    let quote_lots_to_enter_position = checked_mul_div_i128(
        current_virtual_quote_lots_i128,
        base_lots_to_close_position,
        current_base_lots_i128,
    )?;
    let mut projected_base_lots_i128 = current_base_lots_i128
        .checked_add(base_lots_to_close_position)
        .ok_or(MathError::Overflow)?;
    let mut projected_virtual_quote_lots = current_virtual_quote_lots_i128
        .checked_add(quote_lots_to_enter_position)
        .ok_or(MathError::Overflow)?;
    let mut realized_pnl = quote_lots_to_close_position
        .checked_sub(quote_lots_to_enter_position)
        .ok_or(MathError::Overflow)?;
    let flipped = projected_base_lots_i128 == 0;

    if flipped {
        realized_pnl = realized_pnl
            .checked_add(projected_virtual_quote_lots)
            .ok_or(MathError::Overflow)?;
        projected_base_lots_i128 = base_lots_remaining_from_fill;
        projected_virtual_quote_lots = quote_lots_remaining_from_fill;
    }

    Ok((
        projected_position_from_parts(
            current_position,
            signed_base_lots_from_i128(projected_base_lots_i128)?,
            signed_quote_lots_from_i128(projected_virtual_quote_lots)?,
            flipped,
        ),
        signed_quote_lots_from_i128(realized_pnl)?,
    ))
}

fn projected_position_from_parts(
    current_position: TraderPosition,
    base_lot_position: SignedBaseLots,
    virtual_quote_lot_position: SignedQuoteLots,
    reset_accumulated_funding: bool,
) -> Option<TraderPosition> {
    if base_lot_position == SignedBaseLots::ZERO {
        return None;
    }

    let mut projected_position = TraderPosition {
        base_lot_position,
        virtual_quote_lot_position,
        ..current_position
    };
    if reset_accumulated_funding {
        projected_position
            .accumulated_funding_for_active_position
            .clear();
    }

    Some(projected_position)
}

fn signed_base_lots_from_i128(value: i128) -> Result<SignedBaseLots, MathError> {
    SignedBaseLots::try_from(value)
}

fn signed_quote_lots_from_i128(value: i128) -> Result<SignedQuoteLots, MathError> {
    SignedQuoteLots::try_from(value)
}

fn checked_mul_div_i128(
    multiplicand: i128,
    multiplier: i128,
    divisor: i128,
) -> Result<i128, MathError> {
    if divisor == 0 {
        return Err(MathError::DivisionByZero);
    }

    multiplicand
        .checked_mul(multiplier)
        .and_then(|value| value.checked_div(divisor))
        .ok_or(MathError::Overflow)
}

fn empty_position_for_metadata(perp_asset_metadata: &PerpAssetMetadata) -> TraderPosition {
    TraderPosition {
        cumulative_funding_snapshot: perp_asset_metadata.cumulative_funding_rate(),
        ..TraderPosition::new()
    }
}

fn settle_funding_for_position(
    mut position: TraderPosition,
    perp_asset_metadata: &PerpAssetMetadata,
) -> (TraderPosition, SignedQuoteLots) {
    let payment = -(perp_asset_metadata.cumulative_funding_rate()
        - position.cumulative_funding_snapshot)
        * position.base_lot_position;
    position.cumulative_funding_snapshot = perp_asset_metadata.cumulative_funding_rate();
    (position, payment)
}

fn add_signed_quote_lots(
    lhs: SignedQuoteLots,
    rhs: SignedQuoteLots,
) -> Result<SignedQuoteLots, MathError> {
    lhs.checked_add(rhs).ok_or(MathError::Overflow)
}

fn liquidation_price_for_market(
    market_margin: &MarketMargin,
    portfolio_margin: &TraderPortfolioMargin,
    perp_asset_metadata: &PerpAssetMetadata,
) -> Result<Option<f64>, PhoenixStateError> {
    let Some(position) = market_margin.position else {
        return Ok(None);
    };
    if position.base_lot_position == SignedBaseLots::ZERO {
        return Ok(None);
    }

    let calculator = MarketCalculator::new(
        perp_asset_metadata.base_lot_decimals(),
        perp_asset_metadata.tick_size(),
    );
    let base_position_units = calculator.signed_base_lots_to_units(position.base_lot_position);
    if !base_position_units.is_finite() || base_position_units.abs() == 0.0 {
        return Ok(None);
    }
    if !entry_price_sign_invariant_holds(&position) {
        return Err(PhoenixStateError::InvalidMarginSimulationInput {
            reason: format!(
                "invalid position sign invariant for {}: base and virtual quote positions must \
                 have opposite signs",
                &perp_asset_metadata.symbol
            ),
        });
    }

    let entry_price_usd = calculator
        .quote_lots_to_usd(position.virtual_quote_lot_position.abs_as_unsigned())
        / base_position_units.abs();
    let collateral_with_funding = portfolio_margin
        .quote_lot_collateral
        .checked_add(portfolio_margin.margin.unsettled_funding)
        .ok_or(MathError::Overflow)?;
    let other_asset_unrealized_pnl = portfolio_margin
        .margin
        .discounted_unrealized_pnl
        .checked_sub(market_margin.margin.discounted_unrealized_pnl)
        .ok_or(MathError::Overflow)?;
    let maintenance_margin_factor = perp_asset_metadata.get_risk_factor(RiskTier::Liquidatable);
    let discounted_limit_order_maintenance_margin = maintenance_margin_factor
        .apply_to_quote_lots(market_margin.margin.limit_order_margin)
        .ok_or(MathError::Overflow)?;
    let target_position_maintenance_margin = market_margin
        .margin
        .maintenance_margin
        .checked_sub(discounted_limit_order_maintenance_margin)
        .ok_or(MathError::Underflow)?;
    let other_asset_maintenance_margin = portfolio_margin
        .margin
        .maintenance_margin
        .checked_sub(target_position_maintenance_margin)
        .ok_or(MathError::Underflow)?;
    let leverage = perp_asset_metadata
        .leverage_tiers()
        .get_leverage_constant(position.base_lot_position.abs_as_unsigned())
        .as_inner() as f64;

    Ok(calculate_liquidation_price_usd(
        CalculateLiquidationPriceUsdInput {
            position_size: base_position_units,
            entry_price_usd,
            leverage,
            maintenance_margin_bps: maintenance_margin_factor.as_inner() as f64,
            upnl_risk_factor_bps: perp_asset_metadata
                .upnl_risk_factor(RiskAction::View)
                .as_inner() as f64,
            collateral_usd: calculator.signed_quote_lots_to_usd(collateral_with_funding),
            other_asset_unrealized_pnl_usd: calculator
                .signed_quote_lots_to_usd(other_asset_unrealized_pnl),
            other_asset_maintenance_margin_usd: calculator
                .quote_lots_to_usd(other_asset_maintenance_margin),
            allow_non_positive: false,
        },
    ))
}

fn entry_price_sign_invariant_holds(position: &TraderPosition) -> bool {
    let base = position.base_lot_position.as_inner();
    let quote = position.virtual_quote_lot_position.as_inner();
    base == 0 || quote == 0 || base.signum() != quote.signum()
}

/// A trader's portfolio with computed margin and PnL across all markets.
/// Includes per-market breakdown and aggregated totals.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TraderPortfolioMargin {
    pub authority: Pubkey,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,

    pub quote_lot_collateral: SignedQuoteLots,

    pub margin: Margin,
    pub positions: HashMap<String, MarketMargin>,
    pub stop_losses: Vec<StopLossInfo>,
}

impl TraderPortfolioMargin {
    pub fn effective_collateral(&self) -> SignedQuoteLots {
        self.quote_lot_collateral
            + self.margin.discounted_unrealized_pnl
            + self.margin.unsettled_funding
    }

    pub fn effective_collateral_for_withdrawals(&self) -> SignedQuoteLots {
        self.quote_lot_collateral
            + self.margin.discounted_pnl_for_withdrawals
            + self.margin.unsettled_funding
    }

    pub fn portfolio_value(&self) -> SignedQuoteLots {
        self.quote_lot_collateral + self.margin.unrealized_pnl
    }

    pub fn initial_margin(&self) -> QuoteLots {
        self.margin.initial_margin
    }

    pub fn risk_state(&self) -> Result<RiskState, MarginError> {
        let effective_collateral = self.effective_collateral();
        let margin_state = MarginState::new(self.margin.initial_margin, effective_collateral);
        margin_state.risk_state()
    }

    pub fn risk_tier(&self) -> Result<RiskTier, MarginError> {
        let effective_collateral = self.effective_collateral();
        self.margin.risk_tier(effective_collateral)
    }

    pub fn calculate_transferable_collateral(&self) -> Result<u64, MarginError> {
        let available_collateral = self.effective_collateral_for_withdrawals();

        // If trader has no positions or limit orders, all collateral is transferable
        if self.positions.is_empty() {
            return Ok(available_collateral.max(SignedQuoteLots::ZERO).as_inner() as u64);
        }

        // Use the pre-calculated initial_margin_for_withdrawals which includes
        // margin requirements for both positions AND open limit orders
        let total_margin_required = self
            .margin
            .initial_margin_for_withdrawals
            .checked_as_signed()?;

        // Transferable amount = withdrawal effective collateral - required
        // margin, matching on-chain withdrawal checks.
        if available_collateral >= total_margin_required {
            Ok((available_collateral - total_margin_required)
                .max(SignedQuoteLots::ZERO)
                .as_inner() as u64)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leverage_tiers::{LeverageTier, LeverageTiers};
    use crate::quantities::{
        BasisPoints, Constant, QuoteLotsPerBaseLotPerTick, SignedQuoteLotsI56,
        SignedQuoteLotsPerBaseLot,
    };

    const MICRO_USD: f64 = 1_000_000.0;
    const BASE_LOTS_PER_UNIT: f64 = 100.0;

    fn quote_lots(usd: f64) -> SignedQuoteLots {
        SignedQuoteLots::new((usd * MICRO_USD).round() as i64)
    }

    fn price_ticks(price_usd: f64) -> Ticks {
        Ticks::new(((price_usd * MICRO_USD) / BASE_LOTS_PER_UNIT).round() as u64)
    }

    fn market(
        symbol: &str,
        asset_id: u64,
        mark_price_usd: f64,
        max_leverage: u64,
    ) -> PerpAssetMetadata {
        let tier = LeverageTier {
            upper_bound_size: BaseLots::new(1_000_000_000),
            max_leverage: Constant::new(max_leverage),
            limit_order_risk_factor: BasisPoints::new(5_000),
        };
        PerpAssetMetadata::new(
            symbol.to_string(),
            asset_id,
            2,
            price_ticks(mark_price_usd),
            QuoteLotsPerBaseLotPerTick::new(1),
            LeverageTiers::new_unchecked([tier; 4]),
            [5_000, 10_000, 10_000],
            5_000,
            10_000,
            10_000,
        )
    }

    fn position(base_units: f64, virtual_quote_usd: f64) -> TraderPosition {
        TraderPosition {
            base_lot_position: SignedBaseLots::new((base_units * BASE_LOTS_PER_UNIT) as i64),
            virtual_quote_lot_position: quote_lots(virtual_quote_usd),
            cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot::ZERO,
            position_sequence_number: Default::default(),
            accumulated_funding_for_active_position: SignedQuoteLotsI56::default(),
        }
    }

    fn limit_order(
        order_sequence_number: u64,
        side: Side,
        price_usd: f64,
        base_lots: u64,
        reduce_only: bool,
    ) -> LimitOrder {
        LimitOrder {
            price: price_ticks(price_usd),
            side,
            order_sequence_number,
            base_lot_size: BaseLots::new(base_lots),
            initial_trade_size: BaseLots::new(base_lots),
            reduce_only,
            is_stop_loss: false,
        }
    }

    fn stop_loss(asset_id: u64) -> StopLossInfo {
        StopLossInfo {
            funder_key: Default::default(),
            trader_key: Default::default(),
            asset_id,
            trigger_price: price_ticks(90.0),
            execution_price: price_ticks(89.0),
            slot: 0,
            order_kind: StopLossOrderKind::Limit,
            position_sequence_number: 0,
            execution_direction: Direction::LessThan,
            trade_side: Side::Ask,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {actual} to be close to {expected}"
        );
    }

    #[test]
    fn liquidation_price_discounts_target_market_positive_upnl() {
        let long = calculate_liquidation_price_usd(CalculateLiquidationPriceUsdInput {
            position_size: 1.0,
            entry_price_usd: 100.0,
            leverage: 10.0,
            maintenance_margin_bps: 500.0,
            upnl_risk_factor_bps: 5_000.0,
            collateral_usd: 100.0,
            other_asset_unrealized_pnl_usd: -100.0,
            other_asset_maintenance_margin_usd: 0.0,
            allow_non_positive: false,
        })
        .unwrap();
        let short = calculate_liquidation_price_usd(CalculateLiquidationPriceUsdInput {
            position_size: -1.0,
            entry_price_usd: 100.0,
            leverage: 10.0,
            maintenance_margin_bps: 500.0,
            upnl_risk_factor_bps: 5_000.0,
            collateral_usd: 0.0,
            other_asset_unrealized_pnl_usd: -10.0,
            other_asset_maintenance_margin_usd: 0.0,
            allow_non_positive: false,
        })
        .unwrap();

        assert_close(long, 101.01010101010101);
        assert_close(short, 79.20792079207921);
    }

    #[test]
    fn transferable_collateral_uses_withdrawal_effective_collateral_without_positions() {
        let margin = TraderPortfolioMargin {
            quote_lot_collateral: SignedQuoteLots::new(100),
            margin: Margin {
                discounted_pnl_for_withdrawals: SignedQuoteLots::new(-25),
                unsettled_funding: SignedQuoteLots::new(5),
                ..Margin::default()
            },
            ..TraderPortfolioMargin::default()
        };

        assert_eq!(margin.calculate_transferable_collateral().unwrap(), 80);
    }

    #[test]
    fn transferable_collateral_uses_withdrawal_effective_collateral_with_margin() {
        let margin = TraderPortfolioMargin {
            quote_lot_collateral: SignedQuoteLots::new(100),
            margin: Margin {
                initial_margin_for_withdrawals: QuoteLots::new(35),
                discounted_pnl_for_withdrawals: SignedQuoteLots::new(-25),
                unsettled_funding: SignedQuoteLots::new(5),
                ..Margin::default()
            },
            positions: HashMap::from([(
                "SOL".to_string(),
                MarketMargin {
                    position: None,
                    limit_orders: Vec::new(),
                    margin: Margin::default(),
                },
            )]),
            ..TraderPortfolioMargin::default()
        };

        assert_eq!(margin.calculate_transferable_collateral().unwrap(), 45);
    }

    #[test]
    fn simulate_position_fill_increases_cross_position() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 200.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(1.0, -100.0))
            .build();

        let result = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Bid,
                    base_lots: BaseLots::new(100),
                    price_ticks: price_ticks(200.0),
                    fee_quote_lots: None,
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_collateral: None,
                },
                &provider,
            )
            .unwrap();
        let projected_position = result.projected_position.unwrap();

        assert_eq!(result.realized_pnl, SignedQuoteLots::ZERO);
        assert_eq!(
            projected_position.base_lot_position,
            SignedBaseLots::new(200)
        );
        assert_eq!(
            projected_position.virtual_quote_lot_position,
            quote_lots(-300.0)
        );
        assert_eq!(result.margin.quote_lot_collateral, quote_lots(100.0));
        assert_close(result.liquidation_price_usd.unwrap(), 101.01010101010101);
    }

    #[test]
    fn simulate_position_fill_flips_position_and_realizes_pnl() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 120.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(2.0, -200.0))
            .build();

        let result = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Ask,
                    base_lots: BaseLots::new(300),
                    price_ticks: price_ticks(120.0),
                    fee_quote_lots: None,
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_collateral: None,
                },
                &provider,
            )
            .unwrap();
        let projected_position = result.projected_position.unwrap();

        assert_eq!(result.realized_pnl, quote_lots(40.0));
        assert_eq!(result.margin.quote_lot_collateral, quote_lots(140.0));
        assert_eq!(
            projected_position.base_lot_position,
            SignedBaseLots::new(-100)
        );
        assert_eq!(
            projected_position.virtual_quote_lot_position,
            quote_lots(120.0)
        );
        assert_close(result.liquidation_price_usd.unwrap(), 257.4257425742574);
    }

    #[test]
    fn simulate_position_fill_deducts_fee_from_projected_collateral() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 120.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(2.0, -200.0))
            .build();

        let result = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Ask,
                    base_lots: BaseLots::new(300),
                    price_ticks: price_ticks(120.0),
                    fee_quote_lots: Some(QuoteLots::new(1_000_000)),
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_collateral: None,
                },
                &provider,
            )
            .unwrap();

        assert_eq!(result.realized_pnl, quote_lots(40.0));
        assert_eq!(result.fee, QuoteLots::new(1_000_000));
        assert_eq!(result.margin.quote_lot_collateral, quote_lots(139.0));
    }

    #[test]
    fn simulate_position_fill_detaches_stop_losses_when_position_flips() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 120.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(2.0, -200.0))
            .stop_loss(stop_loss(1))
            .build();

        let result = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Ask,
                    base_lots: BaseLots::new(300),
                    price_ticks: price_ticks(120.0),
                    fee_quote_lots: None,
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_collateral: None,
                },
                &provider,
            )
            .unwrap();

        assert!(result.projected_position.is_some());
        assert!(result.projected_portfolio.stop_losses.is_empty());
    }

    #[test]
    fn simulate_position_fill_settles_current_funding_into_projected_collateral() {
        let mut sol = market("SOL-PERP", 1, 120.0, 50);
        sol.cumulative_funding_rate = SignedQuoteLotsPerBaseLot::new(50_000);
        let provider = HashMap::from([("SOL-PERP".to_string(), sol)]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(1.0, -100.0))
            .build();

        let result = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Ask,
                    base_lots: BaseLots::new(100),
                    price_ticks: price_ticks(120.0),
                    fee_quote_lots: None,
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_collateral: None,
                },
                &provider,
            )
            .unwrap();

        assert_eq!(result.realized_pnl, quote_lots(20.0));
        assert!(result.projected_position.is_none());
        assert_eq!(result.margin.quote_lot_collateral, quote_lots(115.0));
        assert_eq!(
            result.margin.margin.unsettled_funding,
            SignedQuoteLots::ZERO
        );
        assert!(result.liquidation_price_usd.is_none());
    }

    #[test]
    fn simulate_position_fill_isolated_excludes_other_markets() {
        let provider = HashMap::from([
            ("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 50)),
            ("ETH-PERP".to_string(), market("ETH-PERP", 2, 100.0, 10)),
        ]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(1.0, -100.0))
            .position("ETH-PERP", position(10.0, -1050.0))
            .build();

        let cross = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Bid,
                    base_lots: BaseLots::new(100),
                    price_ticks: price_ticks(100.0),
                    fee_quote_lots: None,
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_collateral: None,
                },
                &provider,
            )
            .unwrap();
        let isolated = portfolio
            .simulate_position_fill(
                SimulatePositionFillParams {
                    symbol: "SOL-PERP",
                    side: Side::Bid,
                    base_lots: BaseLots::new(100),
                    price_ticks: price_ticks(100.0),
                    fee_quote_lots: None,
                    margin_mode: MarginSimulationMode::Isolated,
                    isolated_collateral: Some(quote_lots(100.0)),
                },
                &provider,
            )
            .unwrap();

        assert!(cross.margin.positions.contains_key("ETH-PERP"));
        assert!(!isolated.margin.positions.contains_key("ETH-PERP"));
        assert_eq!(isolated.margin.quote_lot_collateral, quote_lots(100.0));
        assert!(cross.liquidation_price_usd.unwrap() > isolated.liquidation_price_usd.unwrap());
        assert_close(isolated.liquidation_price_usd.unwrap(), 50.505050505050505);
    }

    #[test]
    fn simulate_margin_cancel_order_removes_margin_requirement() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .limit_orders(
                "SOL-PERP",
                vec![limit_order(10, Side::Bid, 100.0, 100, false)],
            )
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_symbol: None,
                    isolated_collateral: None,
                    mark_price_overrides: HashMap::new(),
                    actions: vec![MarginSimulationAction::CancelOrder {
                        symbol: "SOL-PERP".to_string(),
                        order_sequence_number: 10,
                    }],
                },
                &provider,
            )
            .unwrap();

        assert!(result.before.margin.initial_margin > QuoteLots::ZERO);
        assert_eq!(result.after.margin.initial_margin, QuoteLots::ZERO);
        assert!(result.after.positions.is_empty());
        assert!(matches!(
            result.action_reports.as_slice(),
            [MarginSimulationActionReport::CancelOrder {
                symbol,
                order_sequence_number: 10,
                removed: true,
                settled_funding
            }] if symbol == "SOL-PERP" && *settled_funding == SignedQuoteLots::ZERO
        ));
    }

    #[test]
    fn simulate_margin_cancel_order_settles_funding() {
        let mut sol = market("SOL-PERP", 1, 100.0, 50);
        sol.cumulative_funding_rate = SignedQuoteLotsPerBaseLot::new(-50_000);
        let provider = HashMap::from([("SOL-PERP".to_string(), sol)]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(1.0, -100.0))
            .limit_orders(
                "SOL-PERP",
                vec![limit_order(10, Side::Bid, 90.0, 100, false)],
            )
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_symbol: None,
                    isolated_collateral: None,
                    mark_price_overrides: HashMap::new(),
                    actions: vec![MarginSimulationAction::CancelOrder {
                        symbol: "SOL-PERP".to_string(),
                        order_sequence_number: 10,
                    }],
                },
                &provider,
            )
            .unwrap();

        assert_eq!(result.after.quote_lot_collateral, quote_lots(105.0));
        assert_eq!(result.after.margin.unsettled_funding, SignedQuoteLots::ZERO);
        assert!(matches!(
            result.action_reports.as_slice(),
            [MarginSimulationActionReport::CancelOrder {
                settled_funding,
                removed: true,
                ..
            }] if *settled_funding == quote_lots(5.0)
        ));
    }

    #[test]
    fn simulate_margin_reduce_only_orders_do_not_increase_margin() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .build();

        let reduce_only = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_symbol: None,
                    isolated_collateral: None,
                    mark_price_overrides: HashMap::new(),
                    actions: vec![MarginSimulationAction::PlaceLimitOrder {
                        symbol: "SOL-PERP".to_string(),
                        order: limit_order(11, Side::Ask, 100.0, 100, true),
                    }],
                },
                &provider,
            )
            .unwrap();
        let non_reduce_only = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_symbol: None,
                    isolated_collateral: None,
                    mark_price_overrides: HashMap::new(),
                    actions: vec![MarginSimulationAction::PlaceLimitOrder {
                        symbol: "SOL-PERP".to_string(),
                        order: limit_order(12, Side::Bid, 100.0, 100, false),
                    }],
                },
                &provider,
            )
            .unwrap();

        assert_eq!(reduce_only.after.margin.initial_margin, QuoteLots::ZERO);
        assert!(non_reduce_only.after.margin.initial_margin > QuoteLots::ZERO);
    }

    #[test]
    fn simulate_margin_mark_funding_and_collateral_changes() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(1.0, -100.0))
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_symbol: None,
                    isolated_collateral: None,
                    mark_price_overrides: HashMap::new(),
                    actions: vec![
                        MarginSimulationAction::SetMarkPrice {
                            symbol: "SOL-PERP".to_string(),
                            mark_price: price_ticks(120.0),
                        },
                        MarginSimulationAction::ApplyFundingPayment {
                            symbol: "SOL-PERP".to_string(),
                            quote_lots: quote_lots(5.0),
                        },
                        MarginSimulationAction::SettleFunding {
                            symbol: Some("SOL-PERP".to_string()),
                        },
                        MarginSimulationAction::AdjustCollateral {
                            quote_lots_delta: quote_lots(-10.0),
                        },
                    ],
                },
                &provider,
            )
            .unwrap();
        let sol_margin = result.after.positions.get("SOL-PERP").unwrap();

        assert_eq!(result.after.quote_lot_collateral, quote_lots(95.0));
        assert_eq!(result.after.margin.unsettled_funding, SignedQuoteLots::ZERO);
        assert_eq!(sol_margin.margin.unrealized_pnl, quote_lots(20.0));
        assert_eq!(result.delta.collateral, quote_lots(-5.0));
        assert!(result.unsettled_funding_adjustments.is_empty());
    }

    #[test]
    fn simulate_margin_can_leave_estimated_funding_unsettled() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 50))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(100.0))
            .position("SOL-PERP", position(1.0, -100.0))
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    margin_mode: MarginSimulationMode::Cross,
                    isolated_symbol: None,
                    isolated_collateral: None,
                    mark_price_overrides: HashMap::new(),
                    actions: vec![MarginSimulationAction::ApplyFundingPayment {
                        symbol: "SOL-PERP".to_string(),
                        quote_lots: quote_lots(5.0),
                    }],
                },
                &provider,
            )
            .unwrap();

        assert_eq!(result.after.quote_lot_collateral, quote_lots(100.0));
        assert_eq!(result.after.margin.unsettled_funding, quote_lots(5.0));
        assert_eq!(
            result
                .unsettled_funding_adjustments
                .get("SOL-PERP")
                .copied(),
            Some(quote_lots(5.0))
        );
    }

    #[test]
    fn close_position_action_closes_full_position_at_scenario_mark() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 10))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .position("SOL-PERP", position(1.0, -90.0))
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    actions: vec![MarginSimulationAction::ClosePosition {
                        symbol: "SOL-PERP".to_string(),
                        fraction_bps: BasisPoints::new(10_000),
                        price_ticks: None,
                        fee_quote_lots: QuoteLots::ZERO,
                    }],
                    ..Default::default()
                },
                &provider,
            )
            .unwrap();

        match &result.action_reports[0] {
            MarginSimulationActionReport::ClosePosition {
                closed_base_lots,
                price_ticks,
                realized_pnl,
                projected_position,
                ..
            } => {
                assert_eq!(*closed_base_lots, BaseLots::new(100));
                assert_eq!(*price_ticks, self::price_ticks(100.0));
                // Long 1 unit from $90 closed at the $100 mark realizes +$10.
                assert_eq!(*realized_pnl, quote_lots(10.0));
                assert!(projected_position.is_none());
            }
            report => panic!("unexpected report: {report:?}"),
        }
        assert_eq!(result.after.quote_lot_collateral, quote_lots(1_010.0));
        assert_eq!(result.after.margin.initial_margin, QuoteLots::ZERO);
    }

    #[test]
    fn close_position_action_closes_fraction_with_fee_at_explicit_price() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 10))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .position("SOL-PERP", position(1.0, -90.0))
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    actions: vec![MarginSimulationAction::ClosePosition {
                        symbol: "SOL-PERP".to_string(),
                        fraction_bps: BasisPoints::new(5_000),
                        price_ticks: Some(price_ticks(80.0)),
                        fee_quote_lots: QuoteLots::new(1_000_000),
                    }],
                    ..Default::default()
                },
                &provider,
            )
            .unwrap();

        match &result.action_reports[0] {
            MarginSimulationActionReport::ClosePosition {
                closed_base_lots,
                realized_pnl,
                fee,
                projected_position,
                ..
            } => {
                assert_eq!(*closed_base_lots, BaseLots::new(50));
                // Closing half the long at $80 realizes -$5 plus a $1 fee.
                assert_eq!(*realized_pnl, quote_lots(-5.0));
                assert_eq!(*fee, QuoteLots::new(1_000_000));
                let projected_position = projected_position.unwrap();
                assert_eq!(
                    projected_position.base_lot_position,
                    SignedBaseLots::new(50)
                );
                assert_eq!(
                    projected_position.virtual_quote_lot_position,
                    quote_lots(-45.0)
                );
            }
            report => panic!("unexpected report: {report:?}"),
        }
        assert_eq!(result.after.quote_lot_collateral, quote_lots(994.0));
    }

    #[test]
    fn move_mark_price_action_scales_the_scenario_mark() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 10))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .position("SOL-PERP", position(1.0, -90.0))
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    actions: vec![MarginSimulationAction::MoveMarkPrice {
                        symbol: "SOL-PERP".to_string(),
                        percent_delta_bps: 1_000,
                    }],
                    ..Default::default()
                },
                &provider,
            )
            .unwrap();

        match &result.action_reports[0] {
            MarginSimulationActionReport::MoveMarkPrice {
                previous_mark_price,
                new_mark_price,
                ..
            } => {
                assert_eq!(*previous_mark_price, price_ticks(100.0));
                assert_eq!(*new_mark_price, price_ticks(110.0));
            }
            report => panic!("unexpected report: {report:?}"),
        }
        assert_eq!(result.before.margin.unrealized_pnl, quote_lots(10.0));
        assert_eq!(result.after.margin.unrealized_pnl, quote_lots(20.0));
    }

    #[test]
    fn move_mark_price_action_rejects_non_positive_results() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 10))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .position("SOL-PERP", position(1.0, -90.0))
            .build();

        let result = portfolio.simulate_margin(
            SimulateMarginParams {
                actions: vec![MarginSimulationAction::MoveMarkPrice {
                    symbol: "SOL-PERP".to_string(),
                    percent_delta_bps: -10_000,
                }],
                ..Default::default()
            },
            &provider,
        );

        assert!(matches!(
            result,
            Err(PhoenixStateError::InvalidMarginSimulationInput { .. })
        ));
    }

    #[test]
    fn mark_price_overrides_apply_to_the_whole_simulation() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 10))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .position("SOL-PERP", position(1.0, -90.0))
            .build();

        let result = portfolio
            .simulate_margin(
                SimulateMarginParams {
                    mark_price_overrides: HashMap::from([(
                        "SOL-PERP".to_string(),
                        price_ticks(120.0),
                    )]),
                    ..Default::default()
                },
                &provider,
            )
            .unwrap();

        assert_eq!(result.before.margin.unrealized_pnl, quote_lots(30.0));
        assert_eq!(result.after.margin.unrealized_pnl, quote_lots(30.0));
        // The caller's provider must stay untouched.
        assert_eq!(provider["SOL-PERP"].mark_price, price_ticks(100.0));
    }

    #[test]
    fn simulate_margin_scenarios_branch_from_one_shared_baseline() {
        let provider = HashMap::from([("SOL-PERP".to_string(), market("SOL-PERP", 1, 100.0, 10))]);
        let portfolio = TraderPortfolio::builder()
            .quote_lot_collateral(quote_lots(1_000.0))
            .position("SOL-PERP", position(1.0, -90.0))
            .build();

        let result = portfolio
            .simulate_margin_scenarios(
                SimulateMarginScenariosParams {
                    base_actions: vec![MarginSimulationAction::AdjustCollateral {
                        quote_lots_delta: quote_lots(500.0),
                    }],
                    scenarios: vec![
                        MarginScenario {
                            label: "close everything".to_string(),
                            actions: vec![MarginSimulationAction::ClosePosition {
                                symbol: "SOL-PERP".to_string(),
                                fraction_bps: BasisPoints::new(10_000),
                                price_ticks: None,
                                fee_quote_lots: QuoteLots::ZERO,
                            }],
                        },
                        MarginScenario {
                            label: "price drops 20%".to_string(),
                            actions: vec![MarginSimulationAction::MoveMarkPrice {
                                symbol: "SOL-PERP".to_string(),
                                percent_delta_bps: -2_000,
                            }],
                        },
                    ],
                    ..Default::default()
                },
                &provider,
            )
            .unwrap();

        assert_eq!(result.before.quote_lot_collateral, quote_lots(1_000.0));
        assert_eq!(result.base.quote_lot_collateral, quote_lots(1_500.0));
        assert_eq!(result.base_action_reports.len(), 1);
        assert_eq!(result.scenarios.len(), 2);

        let close_scenario = &result.scenarios[0].simulation;
        let drawdown_scenario = &result.scenarios[1].simulation;
        // Every scenario starts from the shared post-base state.
        assert_eq!(close_scenario.before, result.base);
        assert_eq!(drawdown_scenario.before, result.base);

        // Scenario one closes the position at the $100 mark.
        assert_eq!(
            close_scenario.after.quote_lot_collateral,
            quote_lots(1_510.0)
        );
        assert_eq!(close_scenario.after.margin.initial_margin, QuoteLots::ZERO);
        assert!(close_scenario.liquidation_prices_usd.is_empty());

        // Scenario two still holds the position; the close in scenario one
        // must not leak into it.
        assert_eq!(
            drawdown_scenario.after.quote_lot_collateral,
            quote_lots(1_500.0)
        );
        assert_eq!(
            drawdown_scenario.after.margin.unrealized_pnl,
            quote_lots(-10.0)
        );
        assert_eq!(drawdown_scenario.liquidation_prices_usd.len(), 1);
    }
}
