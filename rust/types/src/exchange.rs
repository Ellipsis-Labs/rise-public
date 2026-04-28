//! Exchange configuration types for Phoenix API.
//!
//! These types represent exchange-level configuration including authority keys,
//! global config, and market configurations for order construction.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::js_safe_ints::JsSafeU64;
use crate::types::market::MarketStatus;

// ============================================================================
// Authority and Keys
// ============================================================================

/// Authority set containing all authority pubkeys.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoritySetView {
    pub root_authority: String,
    pub risk_authority: String,
    pub market_authority: String,
    pub oracle_authority: String,
}

/// Response for the `/view/exchange-keys` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeKeysView {
    pub global_config: String,
    pub current_authorities: AuthoritySetView,
    pub pending_authorities: AuthoritySetView,
    pub canonical_mint: String,
    pub global_vault: String,
    pub perp_asset_map: String,
    pub global_trader_index: Vec<String>,
    pub active_trader_buffer: Vec<String>,
    pub withdraw_queue: String,
}

// ============================================================================
// Exchange-specific Types (f64 percentages)
// ============================================================================

/// Leverage tier with risk factors as f64 percentages.
/// Used by the `/v1/exchange` endpoint.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeLeverageTier {
    pub max_leverage: f64,
    pub max_size_base_lots: u64,
    /// The limit order risk factor as a percentage (e.g., 60.0 = 60%).
    pub limit_order_risk_factor: f64,
}

/// Risk factors as f64 percentages.
/// Used by the `/v1/exchange` endpoint.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRiskFactors {
    /// Maintenance margin risk factor as a percentage (e.g., 50.0 = 50%).
    pub maintenance: f64,
    /// Backstop liquidation risk factor as a percentage.
    pub backstop: f64,
    /// High risk threshold as a percentage.
    pub high_risk: f64,
    /// Risk factor for positive unrealized PnL penalty as a percentage.
    pub upnl: f64,
    /// Risk factor for positive unrealized PnL penalty during withdrawals as a
    /// percentage.
    pub upnl_for_withdrawals: f64,
    /// Cancel order risk factor as a percentage.
    pub cancel_order: f64,
}

// ============================================================================
// Exchange Configuration
// ============================================================================

/// Static market configuration without live data like prices or open interest.
/// Used by the `/v1/exchange` endpoint to return market parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeMarketConfig {
    pub symbol: String,
    pub asset_id: u32,
    pub market_status: MarketStatus,
    /// The orderbook account pubkey (base58 encoded).
    pub market_pubkey: String,
    /// The spline collection PDA (derived from market_pubkey).
    pub spline_pubkey: String,
    pub tick_size: u64,
    pub base_lots_decimals: i8,
    /// Taker fee as a decimal (e.g., 0.0005 = 0.05%).
    pub taker_fee: f64,
    /// Maker fee as a decimal (can be negative for rebates).
    pub maker_fee: f64,
    pub leverage_tiers: Vec<ExchangeLeverageTier>,
    pub risk_factors: ExchangeRiskFactors,
    pub funding_interval_seconds: u32,
    pub funding_period_seconds: u32,
    pub max_funding_rate_per_interval: f64,
    pub open_interest_cap_base_lots: JsSafeU64,
    pub max_liquidation_size_base_lots: JsSafeU64,
    /// Whether this market only supports isolated margin positions.
    pub isolated_only: bool,
}

/// Raw response from the `/v1/exchange` endpoint.
/// Markets are returned as a Vec from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeResponse {
    pub keys: ExchangeKeysView,
    pub markets: Vec<ExchangeMarketConfig>,
}

/// Exchange configuration containing keys and market configs.
///
/// This struct is populated by querying the Phoenix API for exchange keys
/// and market info for supported symbols. Markets are stored in a HashMap
/// for efficient lookup by symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeView {
    pub keys: ExchangeKeysView,
    pub markets: HashMap<String, ExchangeMarketConfig>,
}

impl ExchangeView {
    /// Get market configuration by symbol (case-insensitive).
    pub fn get_market(&self, symbol: &str) -> Option<&ExchangeMarketConfig> {
        self.markets.get(&symbol.to_ascii_uppercase())
    }
}

impl From<ExchangeResponse> for ExchangeView {
    fn from(response: ExchangeResponse) -> Self {
        let markets = response
            .markets
            .into_iter()
            .map(|m| (m.symbol.clone(), m))
            .collect();
        Self {
            keys: response.keys,
            markets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_exchange_keys_view() {
        let json = r#"{
            "globalConfig": "11111111111111111111111111111111",
            "currentAuthorities": {
                "rootAuthority": "22222222222222222222222222222222",
                "riskAuthority": "33333333333333333333333333333333",
                "marketAuthority": "44444444444444444444444444444444",
                "oracleAuthority": "55555555555555555555555555555555"
            },
            "pendingAuthorities": {
                "rootAuthority": "66666666666666666666666666666666",
                "riskAuthority": "77777777777777777777777777777777",
                "marketAuthority": "88888888888888888888888888888888",
                "oracleAuthority": "99999999999999999999999999999999"
            },
            "canonicalMint": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "globalVault": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
            "perpAssetMap": "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
            "globalTraderIndex": ["idx1", "idx2"],
            "activeTraderBuffer": ["buf1", "buf2"],
            "withdrawQueue": "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD"
        }"#;

        let view: ExchangeKeysView = serde_json::from_str(json).unwrap();
        assert_eq!(view.global_config, "11111111111111111111111111111111");
        assert_eq!(
            view.current_authorities.root_authority,
            "22222222222222222222222222222222"
        );
        assert_eq!(view.global_trader_index.len(), 2);
    }

    #[test]
    fn test_fee_fields() {
        let config = ExchangeMarketConfig {
            symbol: "SOL".to_string(),
            asset_id: 0,
            market_status: MarketStatus::Active,
            market_pubkey: "test".to_string(),
            spline_pubkey: "test".to_string(),
            tick_size: 1,
            base_lots_decimals: 6,
            taker_fee: 0.0005,  // 0.05%
            maker_fee: -0.0001, // -0.01% (rebate)
            leverage_tiers: vec![],
            risk_factors: ExchangeRiskFactors::default(),
            funding_interval_seconds: 3600,
            funding_period_seconds: 86400,
            max_funding_rate_per_interval: 0.001,
            open_interest_cap_base_lots: 1_000_000_u64.into(),
            max_liquidation_size_base_lots: 10_000_u64.into(),
            isolated_only: false,
        };

        assert!((config.taker_fee - 0.0005).abs() < 1e-10);
        assert!((config.maker_fee - (-0.0001)).abs() < 1e-10);
    }

    #[test]
    fn test_get_market_case_insensitive() {
        let mut markets = HashMap::new();
        markets.insert(
            "SOL".to_string(),
            ExchangeMarketConfig {
                symbol: "SOL".to_string(),
                asset_id: 0,
                market_status: MarketStatus::Active,
                market_pubkey: "test".to_string(),
                spline_pubkey: "test".to_string(),
                tick_size: 1,
                base_lots_decimals: 6,
                taker_fee: 0.0005,
                maker_fee: 0.0,
                leverage_tiers: vec![],
                risk_factors: ExchangeRiskFactors::default(),
                funding_interval_seconds: 3600,
                funding_period_seconds: 86400,
                max_funding_rate_per_interval: 0.001,
                open_interest_cap_base_lots: 1_000_000_u64.into(),
                max_liquidation_size_base_lots: 10_000_u64.into(),
                isolated_only: false,
            },
        );

        let view = ExchangeView {
            keys: ExchangeKeysView {
                global_config: "test".to_string(),
                current_authorities: AuthoritySetView {
                    root_authority: "test".to_string(),
                    risk_authority: "test".to_string(),
                    market_authority: "test".to_string(),
                    oracle_authority: "test".to_string(),
                },
                pending_authorities: AuthoritySetView {
                    root_authority: "test".to_string(),
                    risk_authority: "test".to_string(),
                    market_authority: "test".to_string(),
                    oracle_authority: "test".to_string(),
                },
                canonical_mint: "test".to_string(),
                global_vault: "test".to_string(),
                perp_asset_map: "test".to_string(),
                global_trader_index: vec![],
                active_trader_buffer: vec![],
                withdraw_queue: "test".to_string(),
            },
            markets,
        };

        assert!(view.get_market("SOL").is_some());
        assert!(view.get_market("sol").is_some());
        assert!(view.get_market("Sol").is_some());
        assert!(view.get_market("BTC").is_none());
    }
}
