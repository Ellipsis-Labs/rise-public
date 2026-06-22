//! Exchange metadata caching for Phoenix SDK.

use std::collections::{HashMap, HashSet};

use crate::phoenix_rise_math::{
    BaseLots, BasisPoints, Constant, LeverageTier, LeverageTiers, MarketCalculator,
    PerpAssetMetadata, QuoteLotsPerBaseLotPerTick, WrapperNum,
};
use crate::types::{
    ExchangeKeysView, ExchangeLeverageTier, ExchangeMarketConfig, ExchangeView, MarketStatsUpdate,
};

/// Consolidated exchange metadata for Phoenix SDK.
#[derive(Debug, Clone)]
pub struct PhoenixMetadata {
    exchange: ExchangeView,
    market_calculators: HashMap<String, MarketCalculator>,
    perp_asset_metadata: HashMap<String, PerpAssetMetadata>,
    isolated_only_markets: HashSet<String>,
}

impl PhoenixMetadata {
    pub fn new(exchange: ExchangeView) -> Self {
        let mut market_calculators = HashMap::new();
        let mut isolated_only_markets = HashSet::new();

        for (symbol, config) in &exchange.markets {
            let calc = MarketCalculator::new(
                config.base_lots_decimals,
                QuoteLotsPerBaseLotPerTick::new(config.tick_size),
            );
            market_calculators.insert(symbol.clone(), calc);
            if config.isolated_only {
                isolated_only_markets.insert(symbol.clone());
            }
        }

        Self {
            exchange,
            market_calculators,
            perp_asset_metadata: HashMap::new(),
            isolated_only_markets,
        }
    }

    pub fn exchange(&self) -> &ExchangeView {
        &self.exchange
    }

    pub fn keys(&self) -> &ExchangeKeysView {
        &self.exchange.keys
    }

    pub fn get_market(&self, symbol: &str) -> Option<&ExchangeMarketConfig> {
        self.exchange.get_market(symbol)
    }

    pub fn is_isolated_only(&self, symbol: &str) -> bool {
        self.isolated_only_markets
            .contains(&symbol.to_ascii_uppercase())
    }

    pub fn get_market_calculator(&self, symbol: &str) -> Option<&MarketCalculator> {
        self.market_calculators.get(&symbol.to_ascii_uppercase())
    }

    pub fn get_perp_asset_metadata(&self, symbol: &str) -> Option<&PerpAssetMetadata> {
        self.perp_asset_metadata.get(&symbol.to_ascii_uppercase())
    }

    pub fn get_perp_asset_metadata_mut(&mut self, symbol: &str) -> Option<&mut PerpAssetMetadata> {
        self.perp_asset_metadata
            .get_mut(&symbol.to_ascii_uppercase())
    }

    pub fn all_perp_asset_metadata(&self) -> &HashMap<String, PerpAssetMetadata> {
        &self.perp_asset_metadata
    }

    pub fn all_perp_asset_metadata_mut(&mut self) -> &mut HashMap<String, PerpAssetMetadata> {
        &mut self.perp_asset_metadata
    }

    pub fn symbols(&self) -> impl Iterator<Item = &String> {
        self.exchange.markets.keys()
    }

    pub fn apply_market_stats(&mut self, stats: &MarketStatsUpdate) -> Result<(), String> {
        let symbol = stats.symbol.to_ascii_uppercase();

        let config = self
            .exchange
            .get_market(&symbol)
            .ok_or_else(|| format!("Unknown symbol: {}", symbol))?;
        let calc = self
            .market_calculators
            .get(&symbol)
            .ok_or_else(|| format!("Missing calculator for: {}", symbol))?;

        if let Some(metadata) = self.perp_asset_metadata.get_mut(&symbol) {
            let mark_price_ticks = calc
                .price_to_ticks(stats.mark_price)
                .map_err(|e| format!("Failed to convert mark price: {:?}", e))?;
            metadata.set_mark_price(mark_price_ticks);
        } else {
            let metadata = perp_asset_metadata_from_exchange_config(config, stats, calc)?;
            self.perp_asset_metadata.insert(symbol, metadata);
        }

        Ok(())
    }

    pub fn has_perp_asset_metadata(&self, symbol: &str) -> bool {
        self.perp_asset_metadata
            .contains_key(&symbol.to_ascii_uppercase())
    }

    pub fn initialized_market_count(&self) -> usize {
        self.perp_asset_metadata.len()
    }
}

/// Build PerpAssetMetadata from exchange config and market stats.
fn perp_asset_metadata_from_exchange_config(
    config: &ExchangeMarketConfig,
    stats: &MarketStatsUpdate,
    calc: &MarketCalculator,
) -> Result<PerpAssetMetadata, String> {
    let mark_price_ticks = calc
        .price_to_ticks(stats.mark_price)
        .map_err(|e| format!("Failed to convert mark price: {:?}", e))?;

    let leverage_tiers = convert_leverage_tiers(&config.leverage_tiers)?;

    let risk_factors = [
        risk_factor_bps_or_percent_to_bps(
            config.risk_factors.maintenance_bps,
            config.risk_factors.maintenance,
            "maintenance",
        )?,
        risk_factor_bps_or_percent_to_bps(
            config.risk_factors.backstop_bps,
            config.risk_factors.backstop,
            "backstop",
        )?,
        risk_factor_bps_or_percent_to_bps(
            config.risk_factors.high_risk_bps,
            config.risk_factors.high_risk,
            "high_risk",
        )?,
    ];

    let cancel_order_risk_factor = risk_factor_bps_or_percent_to_bps(
        config.risk_factors.cancel_order_bps,
        config.risk_factors.cancel_order,
        "cancel_order",
    )?;
    let upnl_risk_factor = risk_factor_bps_or_percent_to_bps(
        config.risk_factors.upnl_bps,
        config.risk_factors.upnl,
        "upnl",
    )?;
    let upnl_risk_factor_for_withdrawals = risk_factor_bps_or_percent_to_bps(
        config.risk_factors.upnl_for_withdrawals_bps,
        config.risk_factors.upnl_for_withdrawals,
        "upnl_for_withdrawals",
    )?;

    let tick_size = QuoteLotsPerBaseLotPerTick::new(config.tick_size);

    Ok(PerpAssetMetadata::new(
        config.symbol.clone(),
        config.asset_id as u64,
        config.base_lots_decimals as i8,
        mark_price_ticks,
        tick_size,
        leverage_tiers,
        risk_factors,
        cancel_order_risk_factor,
        upnl_risk_factor,
        upnl_risk_factor_for_withdrawals,
    ))
}

/// Convert Exchange API leverage tiers to margin calculation leverage tiers.
fn convert_leverage_tiers(api_tiers: &[ExchangeLeverageTier]) -> Result<LeverageTiers, String> {
    if api_tiers.len() != 4 {
        return Err(format!(
            "Expected exactly 4 leverage tiers, got {}",
            api_tiers.len()
        ));
    }

    let convert_tier = |tier: &ExchangeLeverageTier| -> Result<LeverageTier, String> {
        Ok(LeverageTier {
            upper_bound_size: BaseLots::new(tier.max_size_base_lots),
            max_leverage: Constant::new(tier.max_leverage as u64),
            limit_order_risk_factor: BasisPoints::new(risk_factor_bps_or_percent_to_bps(
                tier.limit_order_risk_factor_bps,
                tier.limit_order_risk_factor,
                "leverage_tier.limit_order_risk_factor",
            )? as u64),
        })
    };

    let tiers: [LeverageTier; 4] = [
        convert_tier(&api_tiers[0])?,
        convert_tier(&api_tiers[1])?,
        convert_tier(&api_tiers[2])?,
        convert_tier(&api_tiers[3])?,
    ];

    LeverageTiers::new(tiers).map_err(|e| e.to_string())
}

fn risk_factor_bps_or_percent_to_bps(
    value_bps: Option<u16>,
    value_percent: f64,
    field: &str,
) -> Result<u16, String> {
    if let Some(value_bps) = value_bps {
        if value_bps > 10_000 {
            return Err(format!(
                "{field} risk factor must be at most 10000 bps, got {value_bps}",
            ));
        }
        return Ok(value_bps);
    }

    risk_factor_percent_to_bps(value_percent, field)
}

fn risk_factor_percent_to_bps(value: f64, field: &str) -> Result<u16, String> {
    if !value.is_finite() {
        return Err(format!("{field} risk factor is not finite"));
    }
    if value < 0.0 || value > 100.0 {
        return Err(format!(
            "{field} risk factor must be between 0 and 100 percent, got {value}",
        ));
    }

    let basis_points = value * 100.0;
    let rounded = basis_points.round();
    if (rounded - basis_points).abs() > 1e-9 {
        return Err(format!(
            "{field} risk factor must resolve to an integer basis point value, got {value}",
        ));
    }

    Ok(rounded as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExchangeRiskFactors, MarketStatus};

    fn market_config() -> ExchangeMarketConfig {
        ExchangeMarketConfig {
            symbol: "SOL-PERP".to_string(),
            asset_id: 1,
            market_status: MarketStatus::Active,
            metadata: None,
            market_pubkey: "market".to_string(),
            spline_pubkey: "spline".to_string(),
            tick_size: 1,
            base_lots_decimals: 0,
            taker_fee: 0.0,
            maker_fee: 0.0,
            leverage_tiers: vec![
                ExchangeLeverageTier {
                    max_leverage: 20.0,
                    max_size_base_lots: 1_000,
                    limit_order_risk_factor: 60.0,
                    limit_order_risk_factor_bps: Some(6_000),
                },
                ExchangeLeverageTier {
                    max_leverage: 10.0,
                    max_size_base_lots: 2_000,
                    limit_order_risk_factor: 70.0,
                    limit_order_risk_factor_bps: Some(7_000),
                },
                ExchangeLeverageTier {
                    max_leverage: 5.0,
                    max_size_base_lots: 3_000,
                    limit_order_risk_factor: 80.0,
                    limit_order_risk_factor_bps: Some(8_000),
                },
                ExchangeLeverageTier {
                    max_leverage: 2.0,
                    max_size_base_lots: 4_000,
                    limit_order_risk_factor: 100.0,
                    limit_order_risk_factor_bps: Some(10_000),
                },
            ],
            risk_factors: ExchangeRiskFactors {
                maintenance: 50.0,
                maintenance_bps: Some(5_000),
                backstop: 20.0,
                backstop_bps: Some(2_000),
                high_risk: 10.0,
                high_risk_bps: Some(1_000),
                upnl: 100.0,
                upnl_bps: Some(10_000),
                upnl_for_withdrawals: 1.0,
                upnl_for_withdrawals_bps: Some(100),
                cancel_order: 70.0,
                cancel_order_bps: Some(7_000),
            },
            funding_interval_seconds: 3_600,
            funding_period_seconds: 86_400,
            max_funding_rate_per_interval: 0.0,
            open_interest_cap_base_lots: 1_000_000_u64.into(),
            max_liquidation_size_base_lots: 10_000_u64.into(),
            isolated_only: false,
            stats_snapshot: None,
        }
    }

    fn market_stats() -> MarketStatsUpdate {
        MarketStatsUpdate {
            symbol: "SOL-PERP".to_string(),
            open_interest: 0.0,
            mark_price: 121.0,
            mid_price: 121.0,
            oracle_price: 121.0,
            prev_day_mark_price: 121.0,
            day_volume_usd: 0.0,
            funding_rate: 0.0,
        }
    }

    #[test]
    fn exchange_config_risk_factors_convert_percentages_to_basis_points() {
        let config = market_config();
        let calc = MarketCalculator::new(
            config.base_lots_decimals,
            QuoteLotsPerBaseLotPerTick::new(config.tick_size),
        );
        let metadata =
            perp_asset_metadata_from_exchange_config(&config, &market_stats(), &calc).unwrap();

        assert_eq!(metadata.risk_factors, [5_000, 2_000, 1_000]);
        assert_eq!(metadata.cancel_order_risk_factor, 7_000);
        assert_eq!(metadata.upnl_risk_factor, 10_000);
        assert_eq!(metadata.upnl_risk_factor_for_withdrawals, 100);
        assert_eq!(
            metadata
                .leverage_tiers()
                .get_limit_order_risk_factor(BaseLots::new(1)),
            BasisPoints::new(6_000)
        );
    }

    #[test]
    fn rejects_out_of_range_risk_factor_values() {
        let mut config = market_config();
        config.risk_factors.maintenance_bps = None;
        config.risk_factors.maintenance = 500.0;
        let calc = MarketCalculator::new(
            config.base_lots_decimals,
            QuoteLotsPerBaseLotPerTick::new(config.tick_size),
        );
        let err =
            perp_asset_metadata_from_exchange_config(&config, &market_stats(), &calc).unwrap_err();

        assert!(err.contains("maintenance risk factor must be between 0 and 100 percent"));
    }

    #[test]
    fn exchange_config_prefers_basis_point_risk_factors() {
        let mut config = market_config();
        config.risk_factors.maintenance = 0.5;
        config.risk_factors.maintenance_bps = Some(5_000);
        config.leverage_tiers[0].limit_order_risk_factor = 0.6;
        config.leverage_tiers[0].limit_order_risk_factor_bps = Some(6_000);
        let calc = MarketCalculator::new(
            config.base_lots_decimals,
            QuoteLotsPerBaseLotPerTick::new(config.tick_size),
        );
        let metadata =
            perp_asset_metadata_from_exchange_config(&config, &market_stats(), &calc).unwrap();

        assert_eq!(metadata.risk_factors[0], 5_000);
        assert_eq!(
            metadata
                .leverage_tiers()
                .get_limit_order_risk_factor(BaseLots::new(1)),
            BasisPoints::new(6_000)
        );
    }
}
