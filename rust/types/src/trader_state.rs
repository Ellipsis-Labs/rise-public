//! Trader state container with snapshot and delta handling.

use std::collections::HashMap;

use rust_decimal::Decimal;
use tracing::{debug, warn};

use crate::phoenix_rise_math::{
    BaseLots, LimitOrder as MarginLimitOrder, SequenceNumberU8, SignedBaseLots, SignedQuoteLots,
    SignedQuoteLotsI56, SignedQuoteLotsPerBaseLot, Ticks, TraderPortfolio, TraderPosition,
    WrapperNum,
};
use crate::types::core::Side;
use crate::types::trader_key::TraderKey;
use crate::types::{
    CooldownStatus, TraderStateCapabilities, TraderStateMarketLimitOrderEvent, TraderStatePayload,
    TraderStatePositionRow, TraderStatePositionSnapshot, TraderStateRowChangeKind,
    TraderStateServerMessage, TraderStateSplineRow, TraderStateSplineSnapshot,
    TraderStateSubaccountDelta, TraderStateSubaccountSnapshot,
};

/// A position held by the trader in a specific market.
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub base_position_lots: i64,
    pub entry_price_ticks: i64,
    pub entry_price_usd: Decimal,
    pub virtual_quote_position_lots: i64,
    pub unsettled_funding_quote_lots: i64,
    pub accumulated_funding_quote_lots: i64,
}

impl Position {
    fn from_snapshot(snapshot: &TraderStatePositionSnapshot) -> Self {
        Self::from_row(&snapshot.symbol, &snapshot.position)
    }

    fn from_row(symbol: &str, row: &TraderStatePositionRow) -> Self {
        Self {
            symbol: symbol.to_string(),
            base_position_lots: row.base_position_lots.parse().unwrap_or(0),
            entry_price_ticks: row.entry_price_ticks.parse().unwrap_or(0),
            entry_price_usd: row.entry_price_usd.parse().unwrap_or(Decimal::ZERO),
            virtual_quote_position_lots: row.virtual_quote_position_lots.parse().unwrap_or(0),
            unsettled_funding_quote_lots: row.unsettled_funding_quote_lots.parse().unwrap_or(0),
            accumulated_funding_quote_lots: row.accumulated_funding_quote_lots.parse().unwrap_or(0),
        }
    }

    /// Convert to TraderPosition for margin calculations.
    pub fn to_trader_position(&self) -> TraderPosition {
        TraderPosition {
            base_lot_position: SignedBaseLots::new(self.base_position_lots),
            virtual_quote_lot_position: SignedQuoteLots::new(self.virtual_quote_position_lots),
            cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot::ZERO,
            position_sequence_number: SequenceNumberU8::default(),
            accumulated_funding_for_active_position: SignedQuoteLotsI56::default(),
        }
    }
}

/// A limit order on the orderbook.
#[derive(Debug, Clone)]
pub struct LimitOrder {
    pub symbol: String,
    pub order_sequence_number: u64,
    pub side: String,
    pub order_type: String,
    pub price_ticks: i64,
    pub price_usd: Decimal,
    pub size_remaining_lots: u64,
    pub initial_size_lots: u64,
    pub reduce_only: bool,
    pub is_stop_loss: bool,
    pub status: String,
}

impl LimitOrder {
    fn from_event(symbol: &str, event: &TraderStateMarketLimitOrderEvent) -> Self {
        Self {
            symbol: symbol.to_string(),
            order_sequence_number: event.order_sequence_number.parse().unwrap_or(0),
            side: format!("{:?}", event.side),
            order_type: event.order_type.clone(),
            price_ticks: event.price_ticks.parse().unwrap_or(0),
            price_usd: event.price_usd.parse().unwrap_or(Decimal::ZERO),
            size_remaining_lots: event.size_remaining_lots.parse().unwrap_or(0),
            initial_size_lots: event.initial_size_lots.parse().unwrap_or(0),
            reduce_only: event.reduce_only,
            is_stop_loss: event.is_stop_loss,
            status: event.status.clone(),
        }
    }
}

/// A spline (market making curve) for a specific market.
#[derive(Debug, Clone)]
pub struct Spline {
    pub symbol: String,
    pub mid_price_ticks: i64,
    pub bid_filled_amount_lots: i64,
    pub ask_filled_amount_lots: i64,
}

impl Spline {
    fn from_snapshot(snapshot: &TraderStateSplineSnapshot) -> Self {
        Self::from_row(&snapshot.symbol, &snapshot.spline)
    }

    fn from_row(symbol: &str, row: &TraderStateSplineRow) -> Self {
        Self {
            symbol: symbol.to_string(),
            mid_price_ticks: row.mid_price_ticks.parse().unwrap_or(0),
            bid_filled_amount_lots: row.bid_filled_amount_lots.parse().unwrap_or(0),
            ask_filled_amount_lots: row.ask_filled_amount_lots.parse().unwrap_or(0),
        }
    }
}

/// State for a single subaccount.
#[derive(Debug, Clone, Default)]
pub struct SubaccountState {
    pub subaccount_index: u8,
    pub sequence: u64,
    pub collateral: SignedQuoteLots,
    pub capabilities: Option<TraderStateCapabilities>,
    pub cooldown_status: Option<CooldownStatus>,
    /// Positions keyed by market symbol.
    pub positions: HashMap<String, Position>,
    /// Orders keyed by (symbol, order_sequence_number).
    pub orders: HashMap<(String, u64), LimitOrder>,
    /// Splines keyed by market symbol.
    pub splines: HashMap<String, Spline>,
}

impl SubaccountState {
    fn new(subaccount_index: u8) -> Self {
        Self {
            subaccount_index,
            ..Default::default()
        }
    }

    /// Build a TraderPortfolio from this subaccount's positions and orders.
    pub fn to_trader_portfolio(&self) -> TraderPortfolio {
        let mut builder = TraderPortfolio::builder().quote_lot_collateral(self.collateral);

        for (symbol, position) in &self.positions {
            builder = builder.position(symbol, position.to_trader_position());
        }

        // Group orders by symbol and convert to margin LimitOrders
        let mut orders_by_symbol: HashMap<String, Vec<MarginLimitOrder>> = HashMap::new();
        for ((symbol, _), order) in &self.orders {
            let side = match order.side.as_str() {
                "Buy" => Side::Bid,
                _ => Side::Ask,
            };
            orders_by_symbol
                .entry(symbol.clone())
                .or_default()
                .push(MarginLimitOrder {
                    price: Ticks::new(order.price_ticks as u64),
                    side,
                    order_sequence_number: order.order_sequence_number,
                    base_lot_size: BaseLots::new(order.size_remaining_lots),
                    initial_trade_size: BaseLots::new(order.initial_size_lots),
                    reduce_only: order.reduce_only,
                    is_stop_loss: order.is_stop_loss,
                });
        }
        for (symbol, orders) in orders_by_symbol {
            builder = builder.limit_orders(symbol, orders);
        }

        builder.build()
    }

    fn apply_snapshot(&mut self, snapshot: &TraderStateSubaccountSnapshot) {
        self.sequence = snapshot.sequence;
        self.collateral = snapshot
            .collateral
            .parse::<i64>()
            .map(SignedQuoteLots::new)
            .unwrap_or(SignedQuoteLots::ZERO);
        self.capabilities = snapshot.capabilities.clone();
        self.cooldown_status = snapshot.cooldown_status.clone();

        self.positions.clear();
        for pos in &snapshot.positions {
            let position = Position::from_snapshot(pos);
            self.positions.insert(pos.symbol.clone(), position);
        }

        self.orders.clear();
        for order_group in &snapshot.orders {
            for order in &order_group.orders {
                let limit_order = LimitOrder::from_event(&order_group.symbol, order);
                self.orders.insert(
                    (
                        order_group.symbol.clone(),
                        limit_order.order_sequence_number,
                    ),
                    limit_order,
                );
            }
        }

        self.splines.clear();
        for spline in &snapshot.splines {
            let s = Spline::from_snapshot(spline);
            self.splines.insert(spline.symbol.clone(), s);
        }
    }

    fn apply_delta(&mut self, delta: &TraderStateSubaccountDelta) -> bool {
        if delta.sequence <= self.sequence {
            warn!(
                "Ignoring stale delta: received sequence {} but current is {}",
                delta.sequence, self.sequence
            );
            return false;
        }

        self.sequence = delta.sequence;
        self.collateral = delta
            .collateral
            .parse::<i64>()
            .map(SignedQuoteLots::new)
            .unwrap_or(self.collateral);
        if delta.capabilities.is_some() {
            self.capabilities = delta.capabilities.clone();
        }
        if delta.cooldown_status.is_some() {
            self.cooldown_status = delta.cooldown_status.clone();
        }

        for pos_delta in &delta.positions {
            match pos_delta.change {
                TraderStateRowChangeKind::Closed => {
                    self.positions.remove(&pos_delta.symbol);
                }
                TraderStateRowChangeKind::Updated => {
                    if let Some(row) = &pos_delta.position {
                        let position = Position::from_row(&pos_delta.symbol, row);
                        self.positions.insert(pos_delta.symbol.clone(), position);
                    }
                }
            }
        }

        for order_group in &delta.orders {
            for order in &order_group.orders {
                let osn: u64 = order.order_sequence_number.parse().unwrap_or(0);
                let key = (order_group.symbol.clone(), osn);

                match order.change {
                    Some(TraderStateRowChangeKind::Closed) => {
                        self.orders.remove(&key);
                    }
                    Some(TraderStateRowChangeKind::Updated) | None => {
                        let limit_order = LimitOrder::from_event(&order_group.symbol, order);
                        self.orders.insert(key, limit_order);
                    }
                }
            }
        }

        for spline_delta in &delta.splines {
            match spline_delta.change {
                TraderStateRowChangeKind::Closed => {
                    self.splines.remove(&spline_delta.symbol);
                }
                TraderStateRowChangeKind::Updated => {
                    if let Some(row) = &spline_delta.spline {
                        let spline = Spline::from_row(&spline_delta.symbol, row);
                        self.splines.insert(spline_delta.symbol.clone(), spline);
                    }
                }
            }
        }

        true
    }
}

/// Complete trader state across all subaccounts.
#[derive(Debug, Clone)]
pub struct Trader {
    pub key: TraderKey,
    pub last_slot: u64,
    pub maker_fee_override_multiplier: f64,
    pub taker_fee_override_multiplier: f64,
    pub capabilities: Option<TraderStateCapabilities>,
    pub subaccounts: HashMap<u8, SubaccountState>,
}

impl Trader {
    pub fn new(key: TraderKey) -> Self {
        Self {
            key,
            last_slot: 0,
            maker_fee_override_multiplier: 1.0,
            taker_fee_override_multiplier: 1.0,
            capabilities: None,
            subaccounts: HashMap::new(),
        }
    }

    pub fn apply_update(&mut self, msg: &TraderStateServerMessage) {
        self.last_slot = msg.slot;

        match &msg.content {
            TraderStatePayload::Snapshot(snapshot) => {
                debug!("Applying snapshot at slot {}", msg.slot);
                self.maker_fee_override_multiplier = snapshot.maker_fee_override_multiplier;
                self.taker_fee_override_multiplier = snapshot.taker_fee_override_multiplier;
                self.capabilities = Some(snapshot.capabilities.clone());

                self.subaccounts.clear();
                for sub_snapshot in &snapshot.subaccounts {
                    let mut subaccount = SubaccountState::new(sub_snapshot.subaccount_index);
                    subaccount.apply_snapshot(sub_snapshot);
                    self.subaccounts
                        .insert(sub_snapshot.subaccount_index, subaccount);
                }
            }
            TraderStatePayload::Delta(delta) => {
                debug!("Applying delta at slot {}", msg.slot);
                for sub_delta in &delta.deltas {
                    let subaccount = self
                        .subaccounts
                        .entry(sub_delta.subaccount_index)
                        .or_insert_with(|| SubaccountState::new(sub_delta.subaccount_index));
                    subaccount.apply_delta(sub_delta);
                }
            }
        }
    }

    pub fn total_collateral(&self) -> SignedQuoteLots {
        self.subaccounts
            .values()
            .fold(SignedQuoteLots::ZERO, |acc, s| acc + s.collateral)
    }

    pub fn all_positions(&self) -> Vec<&Position> {
        self.subaccounts
            .values()
            .flat_map(|s| s.positions.values())
            .collect()
    }

    pub fn all_orders(&self) -> Vec<&LimitOrder> {
        self.subaccounts
            .values()
            .flat_map(|s| s.orders.values())
            .collect()
    }

    pub fn subaccount(&self, index: u8) -> Option<&SubaccountState> {
        self.subaccounts.get(&index)
    }

    pub fn primary_subaccount(&self) -> Option<&SubaccountState> {
        self.subaccount(0)
    }

    /// Return a `TraderKey` for the given subaccount index, inheriting this
    /// trader's authority and PDA index.
    pub fn subaccount_key(&self, subaccount_index: u8) -> TraderKey {
        TraderKey::new_with_idx(self.key.authority, self.key.pda_index, subaccount_index)
    }

    /// Find an isolated subaccount for the given asset.
    ///
    /// Prefers a subaccount with an existing position in this asset. Falls back
    /// to the empty isolated subaccount with the greatest collateral.
    pub fn isolated_subaccount_for_asset(&self, symbol: &str) -> Option<&SubaccountState> {
        if let Some(s) = self
            .subaccounts
            .values()
            .find(|s| s.subaccount_index > 0 && s.positions.contains_key(symbol))
        {
            return Some(s);
        }
        self.subaccounts
            .values()
            .filter(|s| s.subaccount_index > 0 && s.positions.is_empty() && s.orders.is_empty())
            .max_by_key(|s| s.collateral)
    }

    /// Try to find an existing isolated subaccount for `symbol`, falling back
    /// to the next available slot. Returns `None` if no suitable subaccount
    /// exists and all slots are occupied.
    pub fn get_or_create_isolated_subaccount_key(&self, symbol: &str) -> Option<TraderKey> {
        if let Some(sub) = self.isolated_subaccount_for_asset(symbol) {
            return Some(self.subaccount_key(sub.subaccount_index));
        }
        self.get_next_isolated_subaccount_key()
    }

    /// Returns whether the given subaccount index is registered.
    pub fn subaccount_exists(&self, subaccount_index: u8) -> bool {
        self.subaccounts.contains_key(&subaccount_index)
    }

    pub fn get_collateral_for_subaccount(&self, subaccount_index: u8) -> SignedQuoteLots {
        self.subaccounts
            .get(&subaccount_index)
            .map(|s| s.collateral)
            .unwrap_or(SignedQuoteLots::ZERO)
    }

    /// Find the next available isolated subaccount slot and return its
    /// `TraderKey`.
    ///
    /// Scans subaccount indexes 1..=255 and returns the first one not already
    /// registered. Returns `None` if all 255 isolated slots are occupied.
    pub fn get_next_isolated_subaccount_key(&self) -> Option<TraderKey> {
        for idx in 1..=255u8 {
            if !self.subaccounts.contains_key(&idx) {
                return Some(TraderKey::new_with_idx(
                    self.key.authority(),
                    self.key.pda_index,
                    idx,
                ));
            }
        }
        None
    }
}
