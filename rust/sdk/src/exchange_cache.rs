use std::collections::HashMap;

use thiserror::Error;

use crate::phoenix_rise_types::{
    ExchangeDeltaMessage, ExchangeDeltaOp, ExchangeMarketParameterUpdate, ExchangeMarketSnapshot,
    ExchangeSnapshotMessage, ExchangeSnapshotView, ExchangeStateSnapshot, MarketPublicMetadata,
    MarketStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeCacheSnapshotSource {
    Http,
    Websocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeCacheExchangeChangeKind {
    Keys,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeCacheMarketChangeKind {
    Status,
    Closed,
    Tombstoned,
    CancelRiskFactor,
    IsolatedOnly,
    LeverageTiers,
    MarkPriceParameters,
    OpenInterestCap,
    UpnlRiskFactor,
    UpnlRiskFactorForWithdrawals,
    FundingParameters,
    MarketFees,
    CommodityMetadata,
    Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeCacheEvent {
    SnapshotApplied {
        source: ExchangeCacheSnapshotSource,
        slot: u64,
        slot_index: u32,
        market_count: usize,
    },
    ExchangeUpdated {
        change: ExchangeCacheExchangeChangeKind,
        slot: u64,
        slot_index: u32,
    },
    MarketAdded {
        symbol: String,
        slot: u64,
        slot_index: u32,
    },
    MarketUpdated {
        symbol: String,
        change: ExchangeCacheMarketChangeKind,
        slot: u64,
        slot_index: u32,
    },
    MarketRemoved {
        symbol: String,
        asset_id: u32,
        slot: u64,
        slot_index: u32,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExchangeCacheApplyError {
    #[error("exchange delta {found} cannot be applied before a sequenced snapshot")]
    MissingSequenceBaseline { found: u64 },

    #[error("exchange delta sequence gap: expected {expected} but received {found}")]
    SequenceGap { expected: u64, found: u64 },
}

#[derive(Debug, Clone, Copy)]
pub struct ExchangeInstructionContext<'a> {
    pub exchange: &'a ExchangeStateSnapshot,
    pub market: &'a ExchangeMarketSnapshot,
}

#[derive(Debug, Clone)]
pub struct PhoenixExchangeCacheStore {
    snapshot: ExchangeSnapshotView,
    markets_by_symbol: HashMap<String, usize>,
    markets_by_asset_id: HashMap<u32, usize>,
    markets_by_pubkey: HashMap<String, usize>,
}

impl PhoenixExchangeCacheStore {
    pub fn new(initial_snapshot: ExchangeSnapshotView) -> Self {
        Self::from_snapshot(initial_snapshot)
    }

    pub fn snapshot(&self) -> &ExchangeSnapshotView {
        &self.snapshot
    }

    pub fn market(&self, symbol: &str) -> Option<&ExchangeMarketSnapshot> {
        self.markets_by_symbol
            .get(&normalize_symbol(symbol))
            .and_then(|index| self.snapshot.markets.get(*index))
    }

    pub fn market_by_asset_id(&self, asset_id: u32) -> Option<&ExchangeMarketSnapshot> {
        self.markets_by_asset_id
            .get(&asset_id)
            .and_then(|index| self.snapshot.markets.get(*index))
    }

    pub fn market_by_pubkey(&self, pubkey: &str) -> Option<&ExchangeMarketSnapshot> {
        self.markets_by_pubkey
            .get(pubkey)
            .and_then(|index| self.snapshot.markets.get(*index))
    }

    pub fn market_metadata(&self, symbol: &str) -> Option<&MarketPublicMetadata> {
        self.market(symbol)
            .and_then(|market| market.metadata.as_ref())
    }

    pub fn market_metadata_by_asset_id(&self, asset_id: u32) -> Option<&MarketPublicMetadata> {
        self.market_by_asset_id(asset_id)
            .and_then(|market| market.metadata.as_ref())
    }

    pub fn market_metadata_by_pubkey(&self, pubkey: &str) -> Option<&MarketPublicMetadata> {
        self.market_by_pubkey(pubkey)
            .and_then(|market| market.metadata.as_ref())
    }

    pub fn instruction_context(&self, symbol: &str) -> Option<ExchangeInstructionContext<'_>> {
        let market = self.market(symbol)?;
        Some(ExchangeInstructionContext {
            exchange: &self.snapshot.exchange,
            market,
        })
    }

    pub fn apply_snapshot(
        &mut self,
        snapshot: ExchangeSnapshotView,
        source: ExchangeCacheSnapshotSource,
    ) -> Vec<ExchangeCacheEvent> {
        self.replace_snapshot(snapshot);
        vec![ExchangeCacheEvent::SnapshotApplied {
            source,
            slot: self.snapshot.slot,
            slot_index: self.snapshot.slot_index,
            market_count: self.snapshot.markets.len(),
        }]
    }

    pub fn apply_snapshot_message(
        &mut self,
        message: &ExchangeSnapshotMessage,
    ) -> Vec<ExchangeCacheEvent> {
        self.apply_snapshot(
            ExchangeSnapshotView::from(message),
            ExchangeCacheSnapshotSource::Websocket,
        )
    }

    pub fn apply_delta(
        &mut self,
        delta: &ExchangeDeltaMessage,
    ) -> Result<Vec<ExchangeCacheEvent>, ExchangeCacheApplyError> {
        let previous_sequence = self
            .snapshot
            .sequence_number
            .map(|sequence| sequence.into_inner())
            .ok_or(ExchangeCacheApplyError::MissingSequenceBaseline {
                found: delta.sequence_number.into_inner(),
            })?;
        let expected_sequence = previous_sequence + 1;
        let actual_sequence = delta.sequence_number.into_inner();
        if actual_sequence != expected_sequence {
            return Err(ExchangeCacheApplyError::SequenceGap {
                expected: expected_sequence,
                found: actual_sequence,
            });
        }

        let mut next_snapshot = self.snapshot.clone();
        let mut events = Vec::with_capacity(delta.ops.len() + 1);
        next_snapshot.version = delta.version;
        next_snapshot.sequence_number = Some(delta.sequence_number);
        next_snapshot.slot = delta.slot;
        next_snapshot.slot_index = delta.slot_index;

        for op in &delta.ops {
            match op {
                ExchangeDeltaOp::ExchangeKeysUpdated { exchange } => {
                    next_snapshot.exchange = exchange.clone();
                    events.push(ExchangeCacheEvent::ExchangeUpdated {
                        change: ExchangeCacheExchangeChangeKind::Keys,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::ExchangeStatusChanged {
                    new_bits,
                    new_features,
                    active,
                    gated,
                    ..
                } => {
                    next_snapshot.exchange.exchange_status_bits = *new_bits;
                    next_snapshot.exchange.exchange_status_features = new_features.clone();
                    next_snapshot.exchange.active = *active;
                    next_snapshot.exchange.gated = *gated;
                    events.push(ExchangeCacheEvent::ExchangeUpdated {
                        change: ExchangeCacheExchangeChangeKind::Status,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::MarketAdded { market } => {
                    upsert_market(&mut next_snapshot.markets, market.clone());
                    events.push(ExchangeCacheEvent::MarketAdded {
                        symbol: market.symbol.clone(),
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::MarketStatusChanged {
                    symbol,
                    new_market_status,
                    ..
                } => {
                    if let Some(market) = find_market_mut(&mut next_snapshot.markets, symbol) {
                        market.market_status = *new_market_status;
                    }
                    events.push(ExchangeCacheEvent::MarketUpdated {
                        symbol: symbol.clone(),
                        change: ExchangeCacheMarketChangeKind::Status,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::MarketClosed { symbol, .. } => {
                    if let Some(market) = find_market_mut(&mut next_snapshot.markets, symbol) {
                        market.market_status = MarketStatus::Closed;
                    }
                    events.push(ExchangeCacheEvent::MarketUpdated {
                        symbol: symbol.clone(),
                        change: ExchangeCacheMarketChangeKind::Closed,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::MarketTombstoned { symbol, .. } => {
                    if let Some(market) = find_market_mut(&mut next_snapshot.markets, symbol) {
                        market.market_status = MarketStatus::Tombstoned;
                    }
                    events.push(ExchangeCacheEvent::MarketUpdated {
                        symbol: symbol.clone(),
                        change: ExchangeCacheMarketChangeKind::Tombstoned,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::MarketDeleted { symbol, asset_id } => {
                    next_snapshot.markets.retain(|market| {
                        normalize_symbol(&market.symbol) != normalize_symbol(symbol)
                    });
                    events.push(ExchangeCacheEvent::MarketRemoved {
                        symbol: symbol.clone(),
                        asset_id: *asset_id,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::MarketParameterUpdated { symbol, update } => {
                    if let Some(change) = market_change_kind(update) {
                        if let Some(market) = find_market_mut(&mut next_snapshot.markets, symbol) {
                            apply_market_parameter_update(market, update);
                        }
                        events.push(ExchangeCacheEvent::MarketUpdated {
                            symbol: symbol.clone(),
                            change,
                            slot: delta.slot,
                            slot_index: delta.slot_index,
                        });
                    }
                }
                ExchangeDeltaOp::MarketMetadataUpdated { symbol, metadata } => {
                    if let Some(market) = find_market_mut(&mut next_snapshot.markets, symbol) {
                        market.metadata = metadata.clone();
                    }
                    events.push(ExchangeCacheEvent::MarketUpdated {
                        symbol: symbol.clone(),
                        change: ExchangeCacheMarketChangeKind::Metadata,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::Unknown => {}
            }
        }

        self.replace_snapshot(next_snapshot);
        events.insert(
            0,
            ExchangeCacheEvent::SnapshotApplied {
                source: ExchangeCacheSnapshotSource::Websocket,
                slot: self.snapshot.slot,
                slot_index: self.snapshot.slot_index,
                market_count: self.snapshot.markets.len(),
            },
        );
        Ok(events)
    }

    fn from_snapshot(mut snapshot: ExchangeSnapshotView) -> Self {
        sort_markets(&mut snapshot.markets);

        let mut markets_by_symbol = HashMap::with_capacity(snapshot.markets.len());
        let mut markets_by_asset_id = HashMap::with_capacity(snapshot.markets.len());
        let mut markets_by_pubkey = HashMap::with_capacity(snapshot.markets.len());

        for (index, market) in snapshot.markets.iter().enumerate() {
            markets_by_symbol.insert(normalize_symbol(&market.symbol), index);
            markets_by_asset_id.insert(market.asset_id, index);
            markets_by_pubkey.insert(market.market_pubkey.clone(), index);
        }

        Self {
            snapshot,
            markets_by_symbol,
            markets_by_asset_id,
            markets_by_pubkey,
        }
    }

    fn replace_snapshot(&mut self, snapshot: ExchangeSnapshotView) {
        *self = Self::from_snapshot(snapshot);
    }
}

fn apply_market_parameter_update(
    market: &mut ExchangeMarketSnapshot,
    update: &ExchangeMarketParameterUpdate,
) {
    match update {
        ExchangeMarketParameterUpdate::CancelRiskFactorUpdated { new, new_bps, .. } => {
            market.risk_factors.cancel_order = *new;
            market.risk_factors.cancel_order_bps =
                (*new_bps).or_else(|| percent_to_basis_points(*new));
        }
        ExchangeMarketParameterUpdate::IsolatedOnlyUpdated { new, .. } => {
            market.isolated_only = *new;
        }
        ExchangeMarketParameterUpdate::LeverageTiersUpdated { new, .. } => {
            market.leverage_tiers = new.clone();
        }
        ExchangeMarketParameterUpdate::MarkPriceParametersUpdated { new, .. } => {
            market.mark_price_parameters = new.clone();
        }
        ExchangeMarketParameterUpdate::OpenInterestCapUpdated { new_base_lots, .. } => {
            market.open_interest_cap_base_lots = *new_base_lots;
        }
        ExchangeMarketParameterUpdate::UpnlRiskFactorUpdated { new, new_bps, .. } => {
            market.risk_factors.upnl = *new;
            market.risk_factors.upnl_bps = (*new_bps).or_else(|| percent_to_basis_points(*new));
        }
        ExchangeMarketParameterUpdate::UpnlRiskFactorForWithdrawalsUpdated {
            new, new_bps, ..
        } => {
            market.risk_factors.upnl_for_withdrawals = *new;
            market.risk_factors.upnl_for_withdrawals_bps =
                (*new_bps).or_else(|| percent_to_basis_points(*new));
        }
        ExchangeMarketParameterUpdate::FundingParametersUpdated { new, .. } => {
            market.funding_config = new.clone();
        }
        ExchangeMarketParameterUpdate::MarketFeesUpdated { new, .. } => {
            market.taker_fee = new.taker_fee;
            market.maker_fee = new.maker_fee;
        }
        ExchangeMarketParameterUpdate::CommodityMetadataUpdated { new, .. } => {
            market.commodity_metadata = Some(new.clone());
        }
        ExchangeMarketParameterUpdate::Unknown => {}
    }
}

fn percent_to_basis_points(value: f64) -> Option<u16> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let bps = value * 100.0;
    if bps > u16::MAX as f64 {
        return None;
    }
    Some(bps.round() as u16)
}

fn market_change_kind(
    update: &ExchangeMarketParameterUpdate,
) -> Option<ExchangeCacheMarketChangeKind> {
    match update {
        ExchangeMarketParameterUpdate::CancelRiskFactorUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::CancelRiskFactor)
        }
        ExchangeMarketParameterUpdate::IsolatedOnlyUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::IsolatedOnly)
        }
        ExchangeMarketParameterUpdate::LeverageTiersUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::LeverageTiers)
        }
        ExchangeMarketParameterUpdate::MarkPriceParametersUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::MarkPriceParameters)
        }
        ExchangeMarketParameterUpdate::OpenInterestCapUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::OpenInterestCap)
        }
        ExchangeMarketParameterUpdate::UpnlRiskFactorUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::UpnlRiskFactor)
        }
        ExchangeMarketParameterUpdate::UpnlRiskFactorForWithdrawalsUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::UpnlRiskFactorForWithdrawals)
        }
        ExchangeMarketParameterUpdate::FundingParametersUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::FundingParameters)
        }
        ExchangeMarketParameterUpdate::MarketFeesUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::MarketFees)
        }
        ExchangeMarketParameterUpdate::CommodityMetadataUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::CommodityMetadata)
        }
        ExchangeMarketParameterUpdate::Unknown => None,
    }
}

fn find_market_mut<'a>(
    markets: &'a mut [ExchangeMarketSnapshot],
    symbol: &str,
) -> Option<&'a mut ExchangeMarketSnapshot> {
    let normalized = normalize_symbol(symbol);
    markets
        .iter_mut()
        .find(|market| normalize_symbol(&market.symbol) == normalized)
}

fn upsert_market(markets: &mut Vec<ExchangeMarketSnapshot>, updated: ExchangeMarketSnapshot) {
    let normalized = normalize_symbol(&updated.symbol);
    if let Some(existing) = markets
        .iter_mut()
        .find(|market| normalize_symbol(&market.symbol) == normalized)
    {
        *existing = updated;
    } else {
        markets.push(updated);
    }
    sort_markets(markets);
}

fn sort_markets(markets: &mut [ExchangeMarketSnapshot]) {
    markets.sort_by(|left, right| {
        left.asset_id
            .cmp(&right.asset_id)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoenix_rise_types::{
        AuthoritySet, CommodityMarketState, Decimal, ExchangeMarketParameterUpdate,
        ExchangeSnapshotMessage, ExchangeWsCommodityMetadata, ExchangeWsFundingConfig,
        ExchangeWsLeverageTier, ExchangeWsMarkPriceParameters, ExchangeWsMarketPriceBand,
        ExchangeWsMarketPriceBand as MarketPriceBand, ExchangeWsRiskActionPriceValidityRules,
        ExchangeWsValidationRule, JsSafeU64, MarketCalendar,
    };

    fn decimal(value: i64, decimals: i8, ui: &str) -> Decimal {
        Decimal {
            value,
            decimals,
            ui: ui.to_string(),
        }
    }

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

    fn build_snapshot(slot: u64, slot_index: u32) -> ExchangeSnapshotView {
        ExchangeSnapshotView {
            version: 1,
            sequence_number: None,
            slot,
            slot_index,
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
                exchange_status_features: vec!["initialized".to_string(), "active".to_string()],
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
                risk_factors: crate::phoenix_rise_types::ExchangeRiskFactors {
                    maintenance: 5_000.0,
                    maintenance_bps: Some(5_000),
                    backstop: 4_000.0,
                    backstop_bps: Some(4_000),
                    high_risk: 3_000.0,
                    high_risk_bps: Some(3_000),
                    upnl: 10_000.0,
                    upnl_bps: Some(10_000),
                    upnl_for_withdrawals: 100.0,
                    upnl_for_withdrawals_bps: Some(100),
                    cancel_order: 5_500.0,
                    cancel_order_bps: Some(5_500),
                },
                funding_config: ExchangeWsFundingConfig {
                    funding_interval_seconds: 3600,
                    funding_period_seconds: 28800,
                    max_funding_rate_per_interval: 2_500,
                },
                open_interest_cap_base_lots: 5_000_u64.into(),
                max_liquidation_size_base_lots: 250_u64.into(),
                isolated_only: false,
                mark_price_parameters: ExchangeWsMarkPriceParameters {
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
                    risk_action_price_validity_rules: sample_risk_action_price_validity_rules(),
                    oracle_divergence_radius: 500,
                    oracle_hard_stale_multiplier: 0,
                    min_oracle_responses: 1,
                },
                commodity_metadata: Some(ExchangeWsCommodityMetadata {
                    is_commodity: true,
                    is_reopen: false,
                    is_after_hours: false,
                    status: CommodityMarketState::Active,
                    after_hours_radius: decimal(150, 0, "150"),
                    last_known_index_price: Some(decimal(742500, 2, "7425.00")),
                    mark_price_band: Some(MarketPriceBand {
                        lower: decimal(741000, 2, "7410.00"),
                        upper: decimal(744000, 2, "7440.00"),
                    }),
                    execution_price_band: Some(ExchangeWsMarketPriceBand {
                        lower: decimal(741500, 2, "7415.00"),
                        upper: decimal(743500, 2, "7435.00"),
                    }),
                    last_index_expiry_timestamp: Some(1_713_200_000),
                }),
            }],
        }
    }

    fn sample_risk_action_price_validity_rules() -> ExchangeWsRiskActionPriceValidityRules {
        [[[ExchangeWsValidationRule::Ignore; 8]; 4]; 8]
    }

    fn build_snapshot_message(sequence_number: u64) -> ExchangeSnapshotMessage {
        let snapshot = build_snapshot(2, 1);
        ExchangeSnapshotMessage {
            channel: "exchange".to_string(),
            version: snapshot.version,
            sequence_number: sequence_number.into(),
            slot: snapshot.slot,
            slot_index: snapshot.slot_index,
            reason: crate::phoenix_rise_types::ExchangeSnapshotReason::Snapshot,
            exchange: snapshot.exchange,
            markets: snapshot.markets,
        }
    }

    fn build_open_interest_cap_delta(
        sequence_number: u64,
        new_base_lots: u64,
    ) -> ExchangeDeltaMessage {
        ExchangeDeltaMessage {
            channel: "exchange".to_string(),
            version: 1,
            sequence_number: sequence_number.into(),
            slot: 3,
            slot_index: 2,
            ops: vec![ExchangeDeltaOp::MarketParameterUpdated {
                symbol: "SOL-PERP".to_string(),
                update: ExchangeMarketParameterUpdate::OpenInterestCapUpdated {
                    previous_base_lots: 5_000_u64.into(),
                    new_base_lots: new_base_lots.into(),
                },
            }],
        }
    }

    fn sample_public_metadata(name: &str) -> MarketPublicMetadata {
        MarketPublicMetadata {
            name: Some(name.to_string()),
            description: Some("Solana perpetual market".to_string()),
            logo_uri: Some("https://example.com/sol.png".to_string()),
            coin_gecko_id: Some("solana".to_string()),
            coin_market_cap_id: Some(5426),
            tokens_xyz_asset_id: Some("solana".to_string()),
            calendar: Some(MarketCalendar {
                id: "cme_commodities".to_string(),
                description: "CME commodities".to_string(),
                calendar_uri: "https://phoenixcdn.trade/calendars/cme_commodities.json".to_string(),
                content_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
                next_market_transition_utc: Some(
                    chrono::DateTime::parse_from_rfc3339("2026-04-24T21:00:00Z")
                        .expect("fixture timestamp")
                        .with_timezone(&chrono::Utc),
                ),
            }),
            display_color: Some("#14f195".to_string()),
        }
    }

    fn build_market_metadata_delta(
        sequence_number: u64,
        metadata: Option<MarketPublicMetadata>,
    ) -> ExchangeDeltaMessage {
        ExchangeDeltaMessage {
            channel: "exchange".to_string(),
            version: 1,
            sequence_number: sequence_number.into(),
            slot: 4,
            slot_index: 0,
            ops: vec![ExchangeDeltaOp::MarketMetadataUpdated {
                symbol: "SOL-PERP".to_string(),
                metadata,
            }],
        }
    }

    fn build_unknown_delta(sequence_number: u64) -> ExchangeDeltaMessage {
        ExchangeDeltaMessage {
            channel: "exchange".to_string(),
            version: 1,
            sequence_number: sequence_number.into(),
            slot: 5,
            slot_index: 0,
            ops: vec![ExchangeDeltaOp::Unknown],
        }
    }

    fn build_unknown_market_parameter_delta(sequence_number: u64) -> ExchangeDeltaMessage {
        ExchangeDeltaMessage {
            channel: "exchange".to_string(),
            version: 1,
            sequence_number: sequence_number.into(),
            slot: 5,
            slot_index: 0,
            ops: vec![ExchangeDeltaOp::MarketParameterUpdated {
                symbol: "SOL-PERP".to_string(),
                update: ExchangeMarketParameterUpdate::Unknown,
            }],
        }
    }

    #[test]
    fn applies_snapshot_then_delta_updates() {
        let mut cache = PhoenixExchangeCacheStore::new(build_snapshot(1, 0));

        let snapshot_events = cache.apply_snapshot_message(&build_snapshot_message(10));
        assert_eq!(snapshot_events.len(), 1);
        assert_eq!(
            cache.snapshot().sequence_number.map(JsSafeU64::into_inner),
            Some(10)
        );

        let delta_events = cache
            .apply_delta(&build_open_interest_cap_delta(11, 7_500))
            .expect("delta should apply");
        assert_eq!(
            cache.snapshot().sequence_number.map(JsSafeU64::into_inner),
            Some(11)
        );
        assert_eq!(
            cache
                .market("sol-perp")
                .expect("market")
                .open_interest_cap_base_lots
                .into_inner(),
            7_500
        );
        assert!(matches!(
            delta_events.get(1),
            Some(ExchangeCacheEvent::MarketUpdated {
                change: ExchangeCacheMarketChangeKind::OpenInterestCap,
                ..
            })
        ));

        let metadata = sample_public_metadata("Updated Solana Perp");
        let metadata_events = cache
            .apply_delta(&build_market_metadata_delta(12, Some(metadata.clone())))
            .expect("metadata delta should apply");
        assert_eq!(
            cache.snapshot().sequence_number.map(JsSafeU64::into_inner),
            Some(12)
        );
        assert_eq!(cache.market_metadata("sol-perp"), Some(&metadata));
        assert_eq!(cache.market_metadata_by_asset_id(1), Some(&metadata));
        assert_eq!(
            cache.market_metadata_by_pubkey("sol-market"),
            Some(&metadata)
        );
        assert!(matches!(
            metadata_events.get(1),
            Some(ExchangeCacheEvent::MarketUpdated {
                change: ExchangeCacheMarketChangeKind::Metadata,
                ..
            })
        ));
    }

    #[test]
    fn ignores_unknown_delta_ops_while_advancing_sequence() {
        let mut cache = PhoenixExchangeCacheStore::new(build_snapshot(1, 0));
        cache.apply_snapshot_message(&build_snapshot_message(10));

        let events = cache
            .apply_delta(&build_unknown_delta(11))
            .expect("unknown delta op should not reject the delta message");

        assert_eq!(
            cache.snapshot().sequence_number.map(JsSafeU64::into_inner),
            Some(11)
        );
        assert_eq!(
            cache
                .market("sol-perp")
                .expect("market")
                .open_interest_cap_base_lots
                .into_inner(),
            5_000
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.first(),
            Some(ExchangeCacheEvent::SnapshotApplied {
                source: ExchangeCacheSnapshotSource::Websocket,
                ..
            })
        ));
    }

    #[test]
    fn ignores_unknown_market_parameter_updates_while_advancing_sequence() {
        let mut cache = PhoenixExchangeCacheStore::new(build_snapshot(1, 0));
        cache.apply_snapshot_message(&build_snapshot_message(10));

        let events = cache
            .apply_delta(&build_unknown_market_parameter_delta(11))
            .expect("unknown market parameter update should not reject the delta message");

        assert_eq!(
            cache.snapshot().sequence_number.map(JsSafeU64::into_inner),
            Some(11)
        );
        assert_eq!(
            cache
                .market("sol-perp")
                .expect("market")
                .open_interest_cap_base_lots
                .into_inner(),
            5_000
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.first(),
            Some(ExchangeCacheEvent::SnapshotApplied {
                source: ExchangeCacheSnapshotSource::Websocket,
                ..
            })
        ));
    }

    #[test]
    fn rejects_delta_without_sequence_baseline() {
        let mut cache = PhoenixExchangeCacheStore::new(build_snapshot(1, 0));

        let error = cache
            .apply_delta(&build_open_interest_cap_delta(2, 7_500))
            .expect_err("delta should fail without websocket snapshot");
        assert_eq!(
            error,
            ExchangeCacheApplyError::MissingSequenceBaseline { found: 2 }
        );
    }

    #[test]
    fn rejects_sequence_gaps() {
        let mut cache = PhoenixExchangeCacheStore::new(build_snapshot(1, 0));
        cache.apply_snapshot_message(&build_snapshot_message(10));

        let error = cache
            .apply_delta(&build_open_interest_cap_delta(12, 7_500))
            .expect_err("delta should fail on sequence gap");
        assert_eq!(
            error,
            ExchangeCacheApplyError::SequenceGap {
                expected: 11,
                found: 12,
            }
        );
    }
}
