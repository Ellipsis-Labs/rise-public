//! Portfolio-level types and aggregation
//!
//! This module provides types for representing a trader's portfolio across
//! multiple markets, computing portfolio-level margin, and liquidation pricing.

use std::collections::HashMap;

#[cfg(feature = "solana")]
pub use solana_pubkey::Pubkey;
#[cfg(not(feature = "solana"))]
pub type Pubkey = [u8; 32];

use crate::direction::{Direction, Side, StopLossOrderKind};
use crate::errors::PhoenixStateError;
use crate::margin::{LimitOrder, Margin, MarketMargin, MarketPosition};
use crate::perp_metadata::PerpAssetMetadata;
use crate::quantities::{QuoteLots, SignedQuoteLots, Ticks};
use crate::risk::{MarginError, MarginState, RiskState, RiskTier};
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
}
