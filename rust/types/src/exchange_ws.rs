//! Exchange snapshot and websocket delta types for the public Phoenix SDK.

use serde::{Deserialize, Serialize};

use crate::types::core::Decimal;
use crate::types::exchange::ExchangeRiskFactors;
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
    pub max_leverage: f64,
    pub max_size_base_lots: JsSafeU64,
    pub limit_order_risk_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeWsFundingConfig {
    pub funding_interval_seconds: u32,
    pub funding_period_seconds: u32,
    pub max_funding_rate_per_interval: f64,
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
    pub perp_price_stale_threshold: JsSafeU64,
    pub risk_action_price_validity_rules: ExchangeWsRiskActionPriceValidityRules,
    pub oracle_divergence_radius: u64,
    pub min_oracle_responses: u32,
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
    pub exchange_status_bits: u32,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSnapshotView {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<JsSafeU64>,
    pub slot: JsSafeU64,
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
    pub version: u32,
    pub sequence_number: JsSafeU64,
    pub slot: JsSafeU64,
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
    pub version: u32,
    pub sequence_number: JsSafeU64,
    pub slot: JsSafeU64,
    pub slot_index: u32,
    pub reason: ExchangeSnapshotReason,
    pub encoding: ExchangeSnapshotEncoding,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeDeltaMessage {
    pub channel: String,
    pub version: u32,
    pub sequence_number: JsSafeU64,
    pub slot: JsSafeU64,
    pub slot_index: u32,
    pub ops: Vec<ExchangeDeltaOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExchangeMarketParameterUpdate {
    CancelRiskFactorUpdated {
        previous: f64,
        new: f64,
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
    UpnlRiskFactorUpdated {
        previous: f64,
        new: f64,
    },
    UpnlRiskFactorForWithdrawalsUpdated {
        previous: f64,
        new: f64,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExchangeDeltaOp {
    ExchangeKeysUpdated {
        exchange: ExchangeStateSnapshot,
    },
    ExchangeStatusChanged {
        previous_bits: u32,
        new_bits: u32,
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
}
