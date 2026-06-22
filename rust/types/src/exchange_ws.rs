//! Exchange snapshot and websocket delta types for the public Phoenix SDK.

use serde::{Deserialize, Serialize};

use crate::types::core::Decimal;
use crate::types::exchange::{
    AuthoritySetView, ExchangeKeysView, ExchangeLeverageTier, ExchangeMarketConfig,
    ExchangeResponse, ExchangeRiskFactors, MarketPublicMetadata,
};
use crate::types::js_safe_ints::JsSafeU64;
use crate::types::market::MarketStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoritySet {
    pub root_authority: String,
    pub risk_authority: String,
    pub market_authority: String,
    pub oracle_authority: String,
    pub adl_authority: String,
    pub cancel_authority: String,
    pub backstop_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsLeverageTier {
    pub max_leverage: u64,
    pub max_size_base_lots: JsSafeU64,
    pub limit_order_risk_factor: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsFundingConfig {
    pub funding_interval_seconds: u64,
    pub funding_period_seconds: u64,
    pub max_funding_rate_per_interval: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsFeeConfig {
    pub taker_fee: f64,
    pub maker_fee: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsMarkPriceParameters {
    pub ema_period_slots: JsSafeU64,
    pub ema_diff_radius: JsSafeU64,
    pub book_price_radius: JsSafeU64,
    pub commodities_after_hours_radius: JsSafeU64,
    pub commodities_after_hours_radius_bps: JsSafeU64,
    pub adjusted_exchange_spot_price_weight: JsSafeU64,
    pub book_price_weight: JsSafeU64,
    pub exchange_perp_price_weight: JsSafeU64,
    pub spot_price_stale_threshold: JsSafeU64,
    pub book_price_stale_threshold: JsSafeU64,
    #[serde(default)]
    pub book_hard_stale_multiplier: u8,
    pub perp_price_stale_threshold: JsSafeU64,
    pub risk_action_price_validity_rules: ExchangeWsRiskActionPriceValidityRules,
    pub oracle_divergence_radius: u16,
    #[serde(default)]
    pub oracle_hard_stale_multiplier: u8,
    pub min_oracle_responses: u8,
}

pub type ExchangeWsRiskActionPriceValidityRules = [[[ExchangeWsValidationRule; 8]; 4]; 8];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExchangeWsValidationRule {
    #[default]
    Ignore,
    Require,
    Forbid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsMarketPriceBand {
    pub lower: Decimal,
    pub upper: Decimal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommodityMarketState {
    Active,
    Reopen,
    AfterHours,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsCommodityMetadata {
    pub is_commodity: bool,
    pub is_reopen: bool,
    pub is_after_hours: bool,
    pub status: CommodityMarketState,
    pub after_hours_radius: Decimal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_index_price: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark_price_band: Option<ExchangeWsMarketPriceBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_price_band: Option<ExchangeWsMarketPriceBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_index_expiry_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeStateSnapshot {
    pub program_id: String,
    pub global_config: String,
    pub current_authorities: AuthoritySet,
    pub canonical_mint: String,
    pub usdc_mint: String,
    pub global_vault: String,
    pub perp_asset_map: String,
    pub global_trader_index: Vec<String>,
    pub active_trader_buffer: Vec<String>,
    pub withdraw_queue: String,
    pub exchange_status_bits: u8,
    pub exchange_status_features: Vec<String>,
    pub active: bool,
    pub gated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeMarketSnapshot {
    pub symbol: String,
    pub asset_id: u32,
    pub market_status: MarketStatus,
    pub market_pubkey: String,
    pub spline_pubkey: String,
    pub tick_size: u64,
    pub base_lots_decimals: i8,
    pub taker_fee: f64,
    pub maker_fee: f64,
    pub leverage_tiers: Vec<ExchangeWsLeverageTier>,
    pub risk_factors: ExchangeRiskFactors,
    pub funding_config: ExchangeWsFundingConfig,
    pub open_interest_cap_base_lots: JsSafeU64,
    pub max_liquidation_size_base_lots: JsSafeU64,
    pub isolated_only: bool,
    pub mark_price_parameters: ExchangeWsMarkPriceParameters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commodity_metadata: Option<ExchangeWsCommodityMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MarketPublicMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSnapshotView {
    pub version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<JsSafeU64>,
    pub slot: u64,
    pub slot_index: u32,
    pub exchange: ExchangeStateSnapshot,
    pub markets: Vec<ExchangeMarketSnapshot>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExchangeSnapshotReason {
    Snapshot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExchangeSnapshotEncoding {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "base64+zstd")]
    Base64Zstd,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSnapshotMessage {
    pub channel: String,
    pub version: u16,
    pub sequence_number: JsSafeU64,
    pub slot: u64,
    pub slot_index: u32,
    pub reason: ExchangeSnapshotReason,
    pub exchange: ExchangeStateSnapshot,
    pub markets: Vec<ExchangeMarketSnapshot>,
}

impl From<&ExchangeSnapshotMessage> for ExchangeSnapshotView {
    fn from(message: &ExchangeSnapshotMessage) -> Self {
        Self {
            version: message.version,
            sequence_number: Some(message.sequence_number),
            slot: message.slot,
            slot_index: message.slot_index,
            exchange: message.exchange.clone(),
            markets: message.markets.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeEncodedSnapshotMessage {
    pub channel: String,
    pub version: u16,
    pub sequence_number: JsSafeU64,
    pub slot: u64,
    pub slot_index: u32,
    pub reason: ExchangeSnapshotReason,
    pub encoding: ExchangeSnapshotEncoding,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeDeltaMessage {
    pub channel: String,
    pub version: u16,
    pub sequence_number: JsSafeU64,
    pub slot: u64,
    pub slot_index: u32,
    pub ops: Vec<ExchangeDeltaOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExchangeMarketParameterUpdate {
    CancelRiskFactorUpdated {
        previous: f64,
        new: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_bps: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        new_bps: Option<u16>,
    },
    IsolatedOnlyUpdated {
        previous: bool,
        new: bool,
    },
    LeverageTiersUpdated {
        previous: Vec<ExchangeWsLeverageTier>,
        new: Vec<ExchangeWsLeverageTier>,
    },
    MarkPriceParametersUpdated {
        previous: ExchangeWsMarkPriceParameters,
        new: ExchangeWsMarkPriceParameters,
    },
    OpenInterestCapUpdated {
        previous_base_lots: JsSafeU64,
        new_base_lots: JsSafeU64,
    },
    MaxLiquidationSizeUpdated {
        previous_base_lots: JsSafeU64,
        new_base_lots: JsSafeU64,
    },
    UpnlRiskFactorUpdated {
        previous: f64,
        new: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_bps: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        new_bps: Option<u16>,
    },
    UpnlRiskFactorForWithdrawalsUpdated {
        previous: f64,
        new: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_bps: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        new_bps: Option<u16>,
    },
    FundingParametersUpdated {
        previous: ExchangeWsFundingConfig,
        new: ExchangeWsFundingConfig,
    },
    MarketFeesUpdated {
        previous: ExchangeWsFeeConfig,
        new: ExchangeWsFeeConfig,
    },
    CommodityMetadataUpdated {
        previous: ExchangeWsCommodityMetadata,
        new: ExchangeWsCommodityMetadata,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExchangeDeltaOp {
    ExchangeKeysUpdated {
        exchange: ExchangeStateSnapshot,
    },
    ExchangeStatusChanged {
        previous_bits: u8,
        new_bits: u8,
        previous_features: Vec<String>,
        new_features: Vec<String>,
        enabled_features: Vec<String>,
        disabled_features: Vec<String>,
        active: bool,
        gated: bool,
    },
    MarketAdded {
        market: ExchangeMarketSnapshot,
    },
    MarketStatusChanged {
        symbol: String,
        previous_market_status: MarketStatus,
        new_market_status: MarketStatus,
    },
    MarketClosed {
        symbol: String,
        previous_market_status: MarketStatus,
        finalized_mark_price: JsSafeU64,
    },
    MarketTombstoned {
        symbol: String,
        previous_market_status: MarketStatus,
        final_sequence_number: JsSafeU64,
        final_trade_sequence_number: JsSafeU64,
        final_order_sequence_number: JsSafeU64,
    },
    MarketDeleted {
        symbol: String,
        asset_id: u32,
    },
    MarketParameterUpdated {
        symbol: String,
        update: ExchangeMarketParameterUpdate,
    },
    MarketMetadataUpdated {
        symbol: String,
        metadata: Option<MarketPublicMetadata>,
    },
    #[serde(other)]
    Unknown,
}

impl ExchangeSnapshotView {
    /// Exchange keys converted for instruction metadata.
    ///
    /// The snapshot carries the active authority set; `pending_authorities` is
    /// mirrored from the active set for compatibility with `ExchangeKeysView`.
    pub fn keys(&self) -> ExchangeKeysView {
        ExchangeKeysView::from(&self.exchange)
    }

    /// Market configs converted into the same shape used by `PhoenixMetadata`.
    pub fn market_configs(&self) -> Vec<ExchangeMarketConfig> {
        self.markets
            .iter()
            .map(ExchangeMarketConfig::from)
            .collect()
    }

    /// Convert the snapshot into exchange metadata for instruction builders.
    pub fn into_exchange_response(self) -> ExchangeResponse {
        ExchangeResponse {
            keys: ExchangeKeysView::from(&self.exchange),
            markets: self
                .markets
                .iter()
                .map(ExchangeMarketConfig::from)
                .collect(),
        }
    }
}

impl From<ExchangeSnapshotView> for ExchangeResponse {
    fn from(snapshot: ExchangeSnapshotView) -> Self {
        snapshot.into_exchange_response()
    }
}

impl From<&AuthoritySet> for AuthoritySetView {
    fn from(authorities: &AuthoritySet) -> Self {
        Self {
            root_authority: authorities.root_authority.clone(),
            risk_authority: authorities.risk_authority.clone(),
            market_authority: authorities.market_authority.clone(),
            oracle_authority: authorities.oracle_authority.clone(),
        }
    }
}

impl From<&ExchangeStateSnapshot> for ExchangeKeysView {
    fn from(exchange: &ExchangeStateSnapshot) -> Self {
        // `/v1/exchange/snapshot` does not include pending authorities; these
        // keys are unused by instruction builders, so mirror the active set.
        let current_authorities = AuthoritySetView::from(&exchange.current_authorities);
        Self {
            program_id: Some(exchange.program_id.clone()),
            global_config: exchange.global_config.clone(),
            current_authorities: current_authorities.clone(),
            pending_authorities: current_authorities,
            canonical_mint: exchange.canonical_mint.clone(),
            global_vault: exchange.global_vault.clone(),
            perp_asset_map: exchange.perp_asset_map.clone(),
            global_trader_index: exchange.global_trader_index.clone(),
            active_trader_buffer: exchange.active_trader_buffer.clone(),
            withdraw_queue: exchange.withdraw_queue.clone(),
        }
    }
}

impl From<&ExchangeWsLeverageTier> for ExchangeLeverageTier {
    fn from(tier: &ExchangeWsLeverageTier) -> Self {
        Self {
            max_leverage: tier.max_leverage as f64,
            max_size_base_lots: tier.max_size_base_lots.into_inner(),
            limit_order_risk_factor: tier.limit_order_risk_factor as f64 / 100.0,
            limit_order_risk_factor_bps: Some(tier.limit_order_risk_factor),
        }
    }
}

impl From<&ExchangeMarketSnapshot> for ExchangeMarketConfig {
    fn from(market: &ExchangeMarketSnapshot) -> Self {
        Self {
            symbol: market.symbol.clone(),
            asset_id: market.asset_id,
            market_status: market.market_status,
            metadata: market.metadata.clone(),
            market_pubkey: market.market_pubkey.clone(),
            spline_pubkey: market.spline_pubkey.clone(),
            tick_size: market.tick_size,
            base_lots_decimals: market.base_lots_decimals,
            taker_fee: market.taker_fee,
            maker_fee: market.maker_fee,
            leverage_tiers: market
                .leverage_tiers
                .iter()
                .map(ExchangeLeverageTier::from)
                .collect(),
            risk_factors: market.risk_factors,
            funding_interval_seconds: clamp_u64_to_u32(
                market.funding_config.funding_interval_seconds,
            ),
            funding_period_seconds: clamp_u64_to_u32(market.funding_config.funding_period_seconds),
            max_funding_rate_per_interval: market.funding_config.max_funding_rate_per_interval
                as f64,
            open_interest_cap_base_lots: market.open_interest_cap_base_lots,
            max_liquidation_size_base_lots: market.max_liquidation_size_base_lots,
            isolated_only: market.isolated_only,
            stats_snapshot: None,
        }
    }
}

fn clamp_u64_to_u32(value: u64) -> u32 {
    value.min(u32::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_set() -> AuthoritySet {
        AuthoritySet {
            root_authority: "root".to_string(),
            risk_authority: "risk".to_string(),
            market_authority: "market".to_string(),
            oracle_authority: "oracle".to_string(),
            adl_authority: "adl".to_string(),
            cancel_authority: "cancel".to_string(),
            backstop_authority: "backstop".to_string(),
        }
    }

    fn mark_price_parameters() -> ExchangeWsMarkPriceParameters {
        ExchangeWsMarkPriceParameters {
            ema_period_slots: 375_u64.into(),
            ema_diff_radius: 250_u64.into(),
            book_price_radius: 500_u64.into(),
            commodities_after_hours_radius: 0_u64.into(),
            commodities_after_hours_radius_bps: 0_u64.into(),
            adjusted_exchange_spot_price_weight: 2_000_u64.into(),
            book_price_weight: 4_000_u64.into(),
            exchange_perp_price_weight: 4_000_u64.into(),
            spot_price_stale_threshold: 120_u64.into(),
            book_price_stale_threshold: 15_u64.into(),
            book_hard_stale_multiplier: 0,
            perp_price_stale_threshold: 15_u64.into(),
            risk_action_price_validity_rules: [[[ExchangeWsValidationRule::Ignore; 8]; 4]; 8],
            oracle_divergence_radius: 500,
            oracle_hard_stale_multiplier: 0,
            min_oracle_responses: 1,
        }
    }

    fn snapshot() -> ExchangeSnapshotView {
        ExchangeSnapshotView {
            version: 1,
            sequence_number: None,
            slot: 42,
            slot_index: 7,
            exchange: ExchangeStateSnapshot {
                program_id: "program".to_string(),
                global_config: "global-config".to_string(),
                current_authorities: authority_set(),
                canonical_mint: "canonical".to_string(),
                usdc_mint: "usdc".to_string(),
                global_vault: "vault".to_string(),
                perp_asset_map: "perp-map".to_string(),
                global_trader_index: vec!["gti-0".to_string()],
                active_trader_buffer: vec!["atb-0".to_string()],
                withdraw_queue: "withdraw-queue".to_string(),
                exchange_status_bits: 129,
                exchange_status_features: vec!["active".to_string()],
                active: true,
                gated: false,
            },
            markets: vec![ExchangeMarketSnapshot {
                symbol: "SOL-PERP".to_string(),
                asset_id: 1,
                market_status: MarketStatus::Active,
                metadata: None,
                market_pubkey: "sol-market".to_string(),
                spline_pubkey: "sol-spline".to_string(),
                tick_size: 100,
                base_lots_decimals: 3,
                taker_fee: 0.0005,
                maker_fee: -0.0001,
                leverage_tiers: vec![ExchangeWsLeverageTier {
                    max_leverage: 20,
                    max_size_base_lots: 1_000_u64.into(),
                    limit_order_risk_factor: 2_500,
                }],
                risk_factors: ExchangeRiskFactors {
                    maintenance: 5.0,
                    maintenance_bps: Some(500),
                    backstop: 8.0,
                    backstop_bps: Some(800),
                    high_risk: 12.0,
                    high_risk_bps: Some(1_200),
                    upnl: 90.0,
                    upnl_bps: Some(9_000),
                    upnl_for_withdrawals: 80.0,
                    upnl_for_withdrawals_bps: Some(8_000),
                    cancel_order: 2.5,
                    cancel_order_bps: Some(250),
                },
                funding_config: ExchangeWsFundingConfig {
                    funding_interval_seconds: 3_600,
                    funding_period_seconds: 28_800,
                    max_funding_rate_per_interval: 2_500,
                },
                open_interest_cap_base_lots: 5_000_u64.into(),
                max_liquidation_size_base_lots: 250_u64.into(),
                isolated_only: false,
                mark_price_parameters: mark_price_parameters(),
                commodity_metadata: None,
            }],
        }
    }

    #[test]
    fn deserializes_v1_exchange_snapshot_shape() {
        let value = serde_json::to_value(snapshot()).unwrap();
        let decoded: ExchangeSnapshotView = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.slot, 42);
        assert_eq!(
            decoded.exchange.current_authorities.cancel_authority,
            "cancel"
        );
        assert_eq!(decoded.markets[0].leverage_tiers[0].max_leverage, 20);
        assert_eq!(
            decoded.markets[0]
                .funding_config
                .max_funding_rate_per_interval,
            2_500
        );
    }

    #[test]
    fn deserializes_unknown_exchange_delta_ops_as_unknown() {
        let value = serde_json::json!({
            "channel": "exchange",
            "version": 1,
            "sequenceNumber": 11,
            "slot": 43,
            "slotIndex": 0,
            "ops": [
                {
                    "kind": "futureMarketThingUpdated",
                    "symbol": "SOL-PERP",
                    "futureField": true
                }
            ]
        });

        let decoded: ExchangeDeltaMessage = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.ops, vec![ExchangeDeltaOp::Unknown]);
    }

    #[test]
    fn deserializes_unknown_exchange_market_parameter_updates_as_unknown() {
        let value = serde_json::json!({
            "channel": "exchange",
            "version": 1,
            "sequenceNumber": 11,
            "slot": 43,
            "slotIndex": 0,
            "ops": [
                {
                    "kind": "marketParameterUpdated",
                    "symbol": "SOL-PERP",
                    "update": {
                        "kind": "futureParameterUpdated",
                        "futureField": true
                    }
                }
            ]
        });

        let decoded: ExchangeDeltaMessage = serde_json::from_value(value).unwrap();
        assert_eq!(
            decoded.ops,
            vec![ExchangeDeltaOp::MarketParameterUpdated {
                symbol: "SOL-PERP".to_string(),
                update: ExchangeMarketParameterUpdate::Unknown,
            }]
        );
    }

    #[test]
    fn converts_snapshot_into_exchange_metadata_response() {
        let response = snapshot().into_exchange_response();
        assert_eq!(response.keys.global_config, "global-config");
        assert_eq!(response.keys.perp_asset_map, "perp-map");
        assert_eq!(response.markets.len(), 1);
        assert_eq!(response.markets[0].symbol, "SOL-PERP");
        assert_eq!(response.markets[0].leverage_tiers[0].max_leverage, 20.0);
        assert_eq!(
            response.markets[0].leverage_tiers[0].limit_order_risk_factor,
            25.0
        );
        assert_eq!(
            response.markets[0].leverage_tiers[0].limit_order_risk_factor_bps,
            Some(2_500)
        );
        assert_eq!(response.markets[0].risk_factors.maintenance_bps, Some(500));
        assert_eq!(response.markets[0].funding_interval_seconds, 3_600);
    }
}
