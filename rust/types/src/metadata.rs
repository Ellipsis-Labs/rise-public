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
        (config.risk_factors.maintenance * 100.0) as u16,
        (config.risk_factors.backstop * 100.0) as u16,
        (config.risk_factors.high_risk * 100.0) as u16,
    ];

    let cancel_order_risk_factor = (config.risk_factors.cancel_order * 100.0) as u16;
    let upnl_risk_factor = (config.risk_factors.upnl * 100.0) as u16;
    let upnl_risk_factor_for_withdrawals =
        (config.risk_factors.upnl_for_withdrawals * 100.0) as u16;

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

    let convert_tier = |tier: &ExchangeLeverageTier| -> LeverageTier {
        LeverageTier {
            upper_bound_size: BaseLots::new(tier.max_size_base_lots),
            max_leverage: Constant::new(tier.max_leverage as u64),
            limit_order_risk_factor: BasisPoints::new(
                (tier.limit_order_risk_factor * 100.0) as u64,
            ),
        }
    };

    let tiers: [LeverageTier; 4] = [
        convert_tier(&api_tiers[0]),
        convert_tier(&api_tiers[1]),
        convert_tier(&api_tiers[2]),
        convert_tier(&api_tiers[3]),
    ];

    LeverageTiers::new(tiers).map_err(|e| e.to_string())
}
