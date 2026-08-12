use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use parking_lot::RwLock;
use phoenix_rise_types::prelude::{
    CollateralAssetMetadata, ExchangeDeltaMessage, ExchangeDeltaOp, ExchangeMarketParameterUpdate,
    ExchangeMarketSnapshot, ExchangeSnapshotMessage, ExchangeSnapshotView, ExchangeStateSnapshot,
    MarketPublicMetadata, MarketStatus,
};
use solana_pubkey::Pubkey;
use thiserror::Error;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeCacheSnapshotSource {
    Http,
    Websocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeCacheExchangeChangeKind {
    Keys,
    Status,
    SpotCollaterals,
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
    MaxLiquidationSize,
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

    pub fn spot_collaterals(&self) -> &[CollateralAssetMetadata] {
        &self.snapshot.spot_collaterals
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
                    withdrawals_available,
                    ..
                } => {
                    next_snapshot.exchange.exchange_status_bits = *new_bits;
                    next_snapshot.exchange.exchange_status_features = new_features.clone();
                    next_snapshot.exchange.active = *active;
                    next_snapshot.exchange.gated = *gated;
                    next_snapshot.exchange.withdrawals_available = *withdrawals_available;
                    events.push(ExchangeCacheEvent::ExchangeUpdated {
                        change: ExchangeCacheExchangeChangeKind::Status,
                        slot: delta.slot,
                        slot_index: delta.slot_index,
                    });
                }
                ExchangeDeltaOp::SpotCollateralsUpdated { assets } => {
                    next_snapshot.spot_collaterals = assets.clone();
                    events.push(ExchangeCacheEvent::ExchangeUpdated {
                        change: ExchangeCacheExchangeChangeKind::SpotCollaterals,
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

/// Cloneable, thread-safe handle around a live [`PhoenixExchangeCacheStore`].
///
/// Intended to be shared between a websocket pump that applies exchange
/// snapshots/deltas and consumers that read live exchange state at use time,
/// such as [`crate::PhoenixFlightClient`] resolving the current Phoenix root
/// authority.
///
/// Obtain it from `PhoenixWSClient::exchange_store()`, which owns the
/// subscription pump and returns the store already populated. The store can
/// also start empty ([`Self::new_empty`]) and reports no data (e.g.
/// [`Self::root_authority`] returns `None`) until the first snapshot is
/// applied.
#[derive(Debug, Clone)]
pub struct SharedExchangeCacheStore {
    inner: Arc<SharedExchangeCacheStoreInner>,
}

#[derive(Debug)]
struct SharedExchangeCacheStoreInner {
    store: RwLock<Option<PhoenixExchangeCacheStore>>,
    /// Latched to `true` once the first snapshot has been applied.
    populated_tx: watch::Sender<bool>,
}

impl SharedExchangeCacheStore {
    /// Store pre-seeded with `initial_snapshot` (e.g. from
    /// `PhoenixHttpClient::get_exchange_snapshot`).
    ///
    /// This is the secondary path for tests and advanced setups that manage
    /// their own snapshot feed; prefer `PhoenixWSClient::exchange_store()`,
    /// which creates, feeds, and populates the store for you.
    pub fn new(initial_snapshot: ExchangeSnapshotView) -> Self {
        Self::from_store(PhoenixExchangeCacheStore::new(initial_snapshot))
    }

    /// Empty store awaiting its first snapshot. Read accessors return `None`
    /// and [`Self::apply_delta`] fails with
    /// [`ExchangeCacheApplyError::MissingSequenceBaseline`] until a snapshot
    /// is applied.
    pub fn new_empty() -> Self {
        Self {
            inner: Arc::new(SharedExchangeCacheStoreInner {
                store: RwLock::new(None),
                populated_tx: watch::channel(false).0,
            }),
        }
    }

    pub fn from_store(store: PhoenixExchangeCacheStore) -> Self {
        Self {
            inner: Arc::new(SharedExchangeCacheStoreInner {
                store: RwLock::new(Some(store)),
                populated_tx: watch::channel(true).0,
            }),
        }
    }

    /// True once at least one snapshot has been applied (stores created via
    /// [`Self::new`] or [`Self::from_store`] start populated).
    pub fn is_populated(&self) -> bool {
        *self.inner.populated_tx.borrow()
    }

    /// Wait until at least one snapshot has been applied; returns
    /// immediately when the store is already populated.
    ///
    /// There is no internal timeout: if no snapshot ever arrives (e.g. the
    /// feeding pump stopped), this future never resolves. Wrap it in
    /// `tokio::time::timeout` to bound the wait.
    pub async fn wait_until_populated(&self) {
        let mut populated_rx = self.inner.populated_tx.subscribe();
        // `wait_for` only fails when the sender is dropped, which cannot
        // happen while `self` holds it.
        let _ = populated_rx.wait_for(|populated| *populated).await;
    }

    /// True when both handles point at the same underlying store.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Run `f` against the current store contents under the read lock, or
    /// return `None` while the store is still awaiting its first snapshot.
    pub fn with_store<R>(&self, f: impl FnOnce(&PhoenixExchangeCacheStore) -> R) -> Option<R> {
        self.inner.store.read().as_ref().map(f)
    }

    /// Clone of the current exchange snapshot, or `None` while the store is
    /// still awaiting its first snapshot.
    pub fn snapshot(&self) -> Option<ExchangeSnapshotView> {
        self.inner
            .store
            .read()
            .as_ref()
            .map(|store| store.snapshot().clone())
    }

    pub fn spot_collaterals(&self) -> Option<Vec<CollateralAssetMetadata>> {
        self.inner
            .store
            .read()
            .as_ref()
            .map(|store| store.spot_collaterals().to_vec())
    }

    /// The current Phoenix root authority, or `None` when the store has not
    /// applied a snapshot yet or the authority is unset or not a valid
    /// pubkey.
    pub fn root_authority(&self) -> Option<Pubkey> {
        let guard = self.inner.store.read();
        let store = guard.as_ref()?;
        Pubkey::from_str(&store.snapshot().exchange.current_authorities.root_authority).ok()
    }

    pub fn apply_snapshot(
        &self,
        snapshot: ExchangeSnapshotView,
        source: ExchangeCacheSnapshotSource,
    ) -> Vec<ExchangeCacheEvent> {
        let events = {
            let mut guard = self.inner.store.write();
            match guard.as_mut() {
                Some(store) => store.apply_snapshot(snapshot, source),
                None => {
                    let store = PhoenixExchangeCacheStore::new(snapshot);
                    let events = vec![ExchangeCacheEvent::SnapshotApplied {
                        source,
                        slot: store.snapshot().slot,
                        slot_index: store.snapshot().slot_index,
                        market_count: store.snapshot().markets.len(),
                    }];
                    *guard = Some(store);
                    events
                }
            }
        };
        self.inner.populated_tx.send_if_modified(|populated| {
            let was_unpopulated = !*populated;
            *populated = true;
            was_unpopulated
        });
        events
    }

    pub fn apply_snapshot_message(
        &self,
        message: &ExchangeSnapshotMessage,
    ) -> Vec<ExchangeCacheEvent> {
        self.apply_snapshot(
            ExchangeSnapshotView::from(message),
            ExchangeCacheSnapshotSource::Websocket,
        )
    }

    pub fn apply_delta(
        &self,
        delta: &ExchangeDeltaMessage,
    ) -> Result<Vec<ExchangeCacheEvent>, ExchangeCacheApplyError> {
        self.inner
            .store
            .write()
            .as_mut()
            .ok_or(ExchangeCacheApplyError::MissingSequenceBaseline {
                found: delta.sequence_number.into_inner(),
            })?
            .apply_delta(delta)
    }
}

impl From<PhoenixExchangeCacheStore> for SharedExchangeCacheStore {
    fn from(store: PhoenixExchangeCacheStore) -> Self {
        Self::from_store(store)
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
        ExchangeMarketParameterUpdate::MaxLiquidationSizeUpdated { new_base_lots, .. } => {
            market.max_liquidation_size_base_lots = *new_base_lots;
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
        ExchangeMarketParameterUpdate::MaxLiquidationSizeUpdated { .. } => {
            Some(ExchangeCacheMarketChangeKind::MaxLiquidationSize)
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
    use phoenix_rise_types::prelude::{
        AuthoritySet, CommodityMarketState, Decimal, ExchangeMarketParameterUpdate,
        ExchangeSnapshotMessage, ExchangeWsCommodityMetadata, ExchangeWsFundingConfig,
        ExchangeWsLeverageTier, ExchangeWsMarkPriceParameters, ExchangeWsMarketPriceBand,
        ExchangeWsMarketPriceBand as MarketPriceBand, ExchangeWsRiskActionPriceValidityRules,
        ExchangeWsValidationRule, JsSafeU64, MarketCalendar, SpotAssetConfig,
    };

    use super::*;

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
                withdrawals_available: true,
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
                risk_factors: phoenix_rise_types::exchange::ExchangeRiskFactors {
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
            spot_collaterals: vec![CollateralAssetMetadata {
                asset_index: 0xffff0000,
                symbol: "SOL".to_string(),
                decimals: 9,
                spot: Some(SpotAssetConfig {
                    is_active: true,
                    perp_asset_index: Some(1),
                    max_per_trader_balance: 1_000_000_000u64.into(),
                    max_global_balance: 10_000_000_000u64.into(),
                    curr_global_balance: 500_000_000u64.into(),
                    min_margin_discount_bps: 100,
                    max_margin_discount_bps: 1_000,
                    max_liquidation_discount_bps: 500,
                    min_liquidation_slippage_bps: 50,
                    max_liquidation_size: 1_000_000_000u64.into(),
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
            version: snapshot.version,
            sequence_number: sequence_number.into(),
            slot: snapshot.slot,
            slot_index: snapshot.slot_index,
            reason: phoenix_rise_types::exchange_ws::ExchangeSnapshotReason::Snapshot,
            exchange: snapshot.exchange,
            markets: snapshot.markets,
            spot_collaterals: snapshot.spot_collaterals,
        }
    }

    fn build_open_interest_cap_delta(
        sequence_number: u64,
        new_base_lots: u64,
    ) -> ExchangeDeltaMessage {
        ExchangeDeltaMessage {
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

    fn build_spot_collaterals_delta(
        sequence_number: u64,
        curr_global_balance: u64,
    ) -> ExchangeDeltaMessage {
        let mut asset = build_snapshot(1, 0).spot_collaterals.remove(0);
        asset
            .spot
            .as_mut()
            .expect("SOL spot collateral config")
            .curr_global_balance = curr_global_balance.into();
        ExchangeDeltaMessage {
            version: 1,
            sequence_number: sequence_number.into(),
            slot: 3,
            slot_index: 2,
            ops: vec![ExchangeDeltaOp::SpotCollateralsUpdated {
                assets: vec![asset],
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
            version: 1,
            sequence_number: sequence_number.into(),
            slot: 5,
            slot_index: 0,
            ops: vec![ExchangeDeltaOp::Unknown],
        }
    }

    fn build_unknown_market_parameter_delta(sequence_number: u64) -> ExchangeDeltaMessage {
        ExchangeDeltaMessage {
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
    fn applies_collateral_registry_deltas() {
        let mut cache = PhoenixExchangeCacheStore::new(build_snapshot(1, 0));
        cache.apply_snapshot_message(&build_snapshot_message(10));

        let events = cache
            .apply_delta(&build_spot_collaterals_delta(11, 750_000_000))
            .expect("collateral registry delta should apply");

        assert_eq!(
            cache.spot_collaterals()[0]
                .spot
                .as_ref()
                .expect("SOL spot collateral config")
                .curr_global_balance
                .into_inner(),
            750_000_000
        );
        assert!(matches!(
            events.get(1),
            Some(ExchangeCacheEvent::ExchangeUpdated {
                change: ExchangeCacheExchangeChangeKind::SpotCollaterals,
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
    fn shared_store_tracks_root_authority_rotation() {
        let initial_root = Pubkey::new_unique();
        let rotated_root = Pubkey::new_unique();
        let mut snapshot = build_snapshot(1, 0);
        snapshot.sequence_number = Some(10u64.into());
        snapshot.exchange.current_authorities.root_authority = initial_root.to_string();
        let store = SharedExchangeCacheStore::new(snapshot.clone());

        assert_eq!(store.root_authority(), Some(initial_root));

        let mut rotated_exchange = snapshot.exchange.clone();
        rotated_exchange.current_authorities.root_authority = rotated_root.to_string();
        store
            .apply_delta(&ExchangeDeltaMessage {
                version: 1,
                sequence_number: 11u64.into(),
                slot: 2,
                slot_index: 0,
                ops: vec![ExchangeDeltaOp::ExchangeKeysUpdated {
                    exchange: rotated_exchange,
                }],
            })
            .expect("keys delta should apply");

        assert_eq!(store.root_authority(), Some(rotated_root));
        assert_eq!(
            store
                .snapshot()
                .expect("populated store should expose a snapshot")
                .sequence_number
                .map(JsSafeU64::into_inner),
            Some(11)
        );
    }

    #[test]
    fn shared_store_root_authority_is_none_for_invalid_pubkey() {
        // `build_snapshot` uses the placeholder string "root", which is not a
        // valid pubkey.
        let store = SharedExchangeCacheStore::new(build_snapshot(1, 0));
        assert_eq!(store.root_authority(), None);
    }

    #[test]
    fn empty_shared_store_starts_unpopulated() {
        let store = SharedExchangeCacheStore::new_empty();

        assert!(!store.is_populated());
        assert_eq!(store.root_authority(), None);
        assert!(store.snapshot().is_none());
        assert_eq!(store.with_store(|store| store.snapshot().slot), None);

        let error = store
            .apply_delta(&build_open_interest_cap_delta(2, 7_500))
            .expect_err("delta should fail before the first snapshot");
        assert_eq!(
            error,
            ExchangeCacheApplyError::MissingSequenceBaseline { found: 2 }
        );
    }

    #[tokio::test]
    async fn empty_shared_store_populates_after_first_snapshot() {
        let store = SharedExchangeCacheStore::new_empty();
        let waiter = {
            let store = store.clone();
            tokio::spawn(async move { store.wait_until_populated().await })
        };

        let events = store.apply_snapshot_message(&build_snapshot_message(10));

        tokio::time::timeout(std::time::Duration::from_secs(2), waiter)
            .await
            .expect("wait_until_populated should resolve after the first snapshot")
            .expect("waiter task should not panic");
        assert!(matches!(
            events.first(),
            Some(ExchangeCacheEvent::SnapshotApplied {
                source: ExchangeCacheSnapshotSource::Websocket,
                ..
            })
        ));
        assert!(store.is_populated());
        assert_eq!(store.snapshot().map(|snapshot| snapshot.slot), Some(2));
        store
            .apply_delta(&build_open_interest_cap_delta(11, 7_500))
            .expect("delta should apply once populated");
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
