//! Trader-facing order ticket and bracket configuration types.

use std::str::FromStr;
use std::sync::Arc;

use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::phoenix_rise_ix::{
    LimitOrderParams, MarketOrderParams, OrderFlags, SelfTradeBehavior, Side,
};
use crate::phoenix_rise_math::{MarketCalculator, WrapperNum};
use crate::phoenix_rise_types::{
    CROSS_MARGIN_SUBACCOUNT_IDX, ExchangeKeysView, ExchangeMarketConfig,
};
use crate::tx_builder::{ParsedAddresses, PhoenixTxBuilderError};

/// Metadata required to convert an order ticket into ix-level params.
#[derive(Debug, Clone, Copy)]
pub struct OrderTicketMetadata<'a> {
    pub market_calc: &'a MarketCalculator,
    pub market_config: &'a ExchangeMarketConfig,
    pub exchange_keys: &'a ExchangeKeysView,
}

impl OrderTicketMetadata<'_> {
    fn parse_addresses(&self) -> Result<ParsedAddresses, PhoenixTxBuilderError> {
        Ok(ParsedAddresses {
            perp_asset_map: Pubkey::from_str(&self.exchange_keys.perp_asset_map)?,
            global_trader_index: parse_pubkey_vec(&self.exchange_keys.global_trader_index)?,
            active_trader_buffer: parse_pubkey_vec(&self.exchange_keys.active_trader_buffer)?,
            orderbook: Pubkey::from_str(&self.market_config.market_pubkey)?,
            spline_collection: Pubkey::from_str(&self.market_config.spline_pubkey)?,
        })
    }
}

/// Trader-facing market order input.
#[derive(Debug, Clone)]
pub struct MarketOrderTicket {
    authority: Pubkey,
    trader_account: Pubkey,
    symbol: String,
    side: Side,
    price: Option<f64>,
    num_base_lots: u64,
    num_quote_lots: Option<u64>,
    min_base_lots_to_fill: u64,
    min_quote_lots_to_fill: u64,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: Option<u64>,
    client_order_id: u128,
    last_valid_slot: Option<u64>,
    order_flags: OrderFlags,
    cancel_existing: bool,
    subaccount_index: u8,
    bracket_leg_ticket: Option<BracketLegTicket>,
}

impl MarketOrderTicket {
    pub fn builder() -> MarketOrderTicketBuilder {
        MarketOrderTicketBuilder::new()
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn bracket_leg_ticket(&self) -> Option<&BracketLegTicket> {
        self.bracket_leg_ticket.as_ref()
    }

    pub fn to_params(
        &self,
        metadata: OrderTicketMetadata<'_>,
    ) -> Result<MarketOrderParams, PhoenixTxBuilderError> {
        let addrs = metadata.parse_addresses()?;

        let mut builder = MarketOrderParams::builder()
            .trader(self.authority)
            .trader_account(self.trader_account)
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index)
            .active_trader_buffer(addrs.active_trader_buffer)
            .side(self.side)
            .num_base_lots(self.num_base_lots)
            .symbol(self.symbol.clone())
            .subaccount_index(self.subaccount_index)
            .self_trade_behavior(self.self_trade_behavior)
            .order_flags(self.order_flags)
            .cancel_existing(self.cancel_existing)
            .client_order_id(self.client_order_id)
            .min_base_lots_to_fill(self.min_base_lots_to_fill)
            .min_quote_lots_to_fill(self.min_quote_lots_to_fill);

        if let Some(price) = self.price {
            builder =
                builder.price_in_ticks(metadata.market_calc.price_to_ticks(price)?.as_inner());
        }
        if let Some(num_quote_lots) = self.num_quote_lots {
            builder = builder.num_quote_lots(num_quote_lots);
        }
        if let Some(match_limit) = self.match_limit {
            builder = builder.match_limit(match_limit);
        }
        if let Some(last_valid_slot) = self.last_valid_slot {
            builder = builder.last_valid_slot(last_valid_slot);
        }

        Ok(builder.build()?)
    }
}

#[derive(Default)]
pub struct MarketOrderTicketBuilder {
    authority: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    symbol: Option<String>,
    side: Option<Side>,
    price: Option<f64>,
    num_base_lots: Option<u64>,
    num_quote_lots: Option<u64>,
    min_base_lots_to_fill: Option<u64>,
    min_quote_lots_to_fill: Option<u64>,
    self_trade_behavior: Option<SelfTradeBehavior>,
    match_limit: Option<u64>,
    client_order_id: Option<u128>,
    last_valid_slot: Option<u64>,
    order_flags: Option<OrderFlags>,
    cancel_existing: Option<bool>,
    subaccount_index: Option<u8>,
    bracket_leg_ticket: Option<BracketLegTicket>,
}

impl MarketOrderTicketBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    pub fn price(mut self, price: f64) -> Self {
        self.price = Some(price);
        self
    }

    pub fn num_base_lots(mut self, num_base_lots: u64) -> Self {
        self.num_base_lots = Some(num_base_lots);
        self
    }

    pub fn num_quote_lots(mut self, num_quote_lots: u64) -> Self {
        self.num_quote_lots = Some(num_quote_lots);
        self
    }

    pub fn min_base_lots_to_fill(mut self, min_base_lots_to_fill: u64) -> Self {
        self.min_base_lots_to_fill = Some(min_base_lots_to_fill);
        self
    }

    pub fn min_quote_lots_to_fill(mut self, min_quote_lots_to_fill: u64) -> Self {
        self.min_quote_lots_to_fill = Some(min_quote_lots_to_fill);
        self
    }

    pub fn self_trade_behavior(mut self, self_trade_behavior: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = Some(self_trade_behavior);
        self
    }

    pub fn match_limit(mut self, match_limit: u64) -> Self {
        self.match_limit = Some(match_limit);
        self
    }

    pub fn client_order_id(mut self, client_order_id: u128) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn last_valid_slot(mut self, last_valid_slot: u64) -> Self {
        self.last_valid_slot = Some(last_valid_slot);
        self
    }

    pub fn order_flags(mut self, order_flags: OrderFlags) -> Self {
        self.order_flags = Some(order_flags);
        self
    }

    pub fn cancel_existing(mut self, cancel_existing: bool) -> Self {
        self.cancel_existing = Some(cancel_existing);
        self
    }

    pub fn subaccount_index(mut self, subaccount_index: u8) -> Self {
        self.subaccount_index = Some(subaccount_index);
        self
    }

    pub fn bracket_leg_ticket(mut self, bracket_leg_ticket: BracketLegTicket) -> Self {
        self.bracket_leg_ticket = Some(bracket_leg_ticket);
        self
    }

    pub fn build(self) -> Result<MarketOrderTicket, crate::phoenix_rise_ix::PhoenixIxError> {
        Ok(MarketOrderTicket {
            authority: self.authority.ok_or(
                crate::phoenix_rise_ix::PhoenixIxError::MissingField("authority"),
            )?,
            trader_account: self.trader_account.ok_or(
                crate::phoenix_rise_ix::PhoenixIxError::MissingField("trader_account"),
            )?,
            symbol: self
                .symbol
                .ok_or(crate::phoenix_rise_ix::PhoenixIxError::MissingField(
                    "symbol",
                ))?,
            side: self
                .side
                .ok_or(crate::phoenix_rise_ix::PhoenixIxError::MissingField("side"))?,
            price: self.price,
            num_base_lots: self.num_base_lots.ok_or(
                crate::phoenix_rise_ix::PhoenixIxError::MissingField("num_base_lots"),
            )?,
            num_quote_lots: self.num_quote_lots,
            min_base_lots_to_fill: self.min_base_lots_to_fill.unwrap_or(0),
            min_quote_lots_to_fill: self.min_quote_lots_to_fill.unwrap_or(0),
            self_trade_behavior: self.self_trade_behavior.unwrap_or(SelfTradeBehavior::Abort),
            match_limit: self.match_limit,
            client_order_id: self.client_order_id.unwrap_or(0),
            last_valid_slot: self.last_valid_slot,
            order_flags: self.order_flags.unwrap_or(OrderFlags::None),
            cancel_existing: self.cancel_existing.unwrap_or(false),
            subaccount_index: self.subaccount_index.unwrap_or(CROSS_MARGIN_SUBACCOUNT_IDX),
            bracket_leg_ticket: self.bracket_leg_ticket,
        })
    }
}

/// Trader-facing limit order input.
#[derive(Debug, Clone)]
pub struct LimitOrderTicket {
    authority: Pubkey,
    trader_account: Pubkey,
    symbol: String,
    side: Side,
    price: f64,
    num_base_lots: u64,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: Option<u64>,
    client_order_id: u128,
    last_valid_slot: Option<u64>,
    order_flags: OrderFlags,
    cancel_existing: bool,
    subaccount_index: u8,
    bracket_leg_ticket: Option<BracketLegTicket>,
}

impl LimitOrderTicket {
    pub fn builder() -> LimitOrderTicketBuilder {
        LimitOrderTicketBuilder::new()
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn bracket_leg_ticket(&self) -> Option<&BracketLegTicket> {
        self.bracket_leg_ticket.as_ref()
    }

    pub fn to_params(
        &self,
        metadata: OrderTicketMetadata<'_>,
    ) -> Result<LimitOrderParams, PhoenixTxBuilderError> {
        let addrs = metadata.parse_addresses()?;
        let price_in_ticks = metadata.market_calc.price_to_ticks(self.price)?.as_inner();

        let mut builder = LimitOrderParams::builder()
            .trader(self.authority)
            .trader_account(self.trader_account)
            .perp_asset_map(addrs.perp_asset_map)
            .orderbook(addrs.orderbook)
            .spline_collection(addrs.spline_collection)
            .global_trader_index(addrs.global_trader_index)
            .active_trader_buffer(addrs.active_trader_buffer)
            .side(self.side)
            .price_in_ticks(price_in_ticks)
            .num_base_lots(self.num_base_lots)
            .symbol(self.symbol.clone())
            .subaccount_index(self.subaccount_index)
            .self_trade_behavior(self.self_trade_behavior)
            .order_flags(self.order_flags)
            .cancel_existing(self.cancel_existing)
            .client_order_id(self.client_order_id);

        if let Some(match_limit) = self.match_limit {
            builder = builder.match_limit(match_limit);
        }
        if let Some(last_valid_slot) = self.last_valid_slot {
            builder = builder.last_valid_slot(last_valid_slot);
        }

        Ok(builder.build()?)
    }
}

#[derive(Default)]
pub struct LimitOrderTicketBuilder {
    authority: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    symbol: Option<String>,
    side: Option<Side>,
    price: Option<f64>,
    num_base_lots: Option<u64>,
    self_trade_behavior: Option<SelfTradeBehavior>,
    match_limit: Option<u64>,
    client_order_id: Option<u128>,
    last_valid_slot: Option<u64>,
    order_flags: Option<OrderFlags>,
    cancel_existing: Option<bool>,
    subaccount_index: Option<u8>,
    bracket_leg_ticket: Option<BracketLegTicket>,
}

impl LimitOrderTicketBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    pub fn price(mut self, price: f64) -> Self {
        self.price = Some(price);
        self
    }

    pub fn num_base_lots(mut self, num_base_lots: u64) -> Self {
        self.num_base_lots = Some(num_base_lots);
        self
    }

    pub fn self_trade_behavior(mut self, self_trade_behavior: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = Some(self_trade_behavior);
        self
    }

    pub fn match_limit(mut self, match_limit: u64) -> Self {
        self.match_limit = Some(match_limit);
        self
    }

    pub fn client_order_id(mut self, client_order_id: u128) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn last_valid_slot(mut self, last_valid_slot: u64) -> Self {
        self.last_valid_slot = Some(last_valid_slot);
        self
    }

    pub fn order_flags(mut self, order_flags: OrderFlags) -> Self {
        self.order_flags = Some(order_flags);
        self
    }

    pub fn cancel_existing(mut self, cancel_existing: bool) -> Self {
        self.cancel_existing = Some(cancel_existing);
        self
    }

    pub fn subaccount_index(mut self, subaccount_index: u8) -> Self {
        self.subaccount_index = Some(subaccount_index);
        self
    }

    pub fn bracket_leg_ticket(mut self, bracket_leg_ticket: BracketLegTicket) -> Self {
        self.bracket_leg_ticket = Some(bracket_leg_ticket);
        self
    }

    pub fn build(self) -> Result<LimitOrderTicket, crate::phoenix_rise_ix::PhoenixIxError> {
        Ok(LimitOrderTicket {
            authority: self.authority.ok_or(
                crate::phoenix_rise_ix::PhoenixIxError::MissingField("authority"),
            )?,
            trader_account: self.trader_account.ok_or(
                crate::phoenix_rise_ix::PhoenixIxError::MissingField("trader_account"),
            )?,
            symbol: self
                .symbol
                .ok_or(crate::phoenix_rise_ix::PhoenixIxError::MissingField(
                    "symbol",
                ))?,
            side: self
                .side
                .ok_or(crate::phoenix_rise_ix::PhoenixIxError::MissingField("side"))?,
            price: self
                .price
                .ok_or(crate::phoenix_rise_ix::PhoenixIxError::MissingField(
                    "price",
                ))?,
            num_base_lots: self.num_base_lots.ok_or(
                crate::phoenix_rise_ix::PhoenixIxError::MissingField("num_base_lots"),
            )?,
            self_trade_behavior: self.self_trade_behavior.unwrap_or(SelfTradeBehavior::Abort),
            match_limit: self.match_limit,
            client_order_id: self.client_order_id.unwrap_or(0),
            last_valid_slot: self.last_valid_slot,
            order_flags: self.order_flags.unwrap_or(OrderFlags::None),
            cancel_existing: self.cancel_existing.unwrap_or(false),
            subaccount_index: self.subaccount_index.unwrap_or(CROSS_MARGIN_SUBACCOUNT_IDX),
            bracket_leg_ticket: self.bracket_leg_ticket,
        })
    }
}

/// Sizing mode for an individual bracket leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BracketLegSize {
    /// Apply the trigger to a percentage of the position.
    PositionPercent(u8),
    /// Apply the trigger to a fixed base-lot size.
    BaseLots(u64),
}

impl Default for BracketLegSize {
    fn default() -> Self {
        Self::PositionPercent(100)
    }
}

/// A single TP or SL leg.
#[derive(Debug, Clone, PartialEq)]
pub struct BracketLeg {
    /// Trigger and execution price in USD.
    pub price: f64,
    /// Optional sizing override for this leg.
    pub size: Option<BracketLegSize>,
}

impl BracketLeg {
    pub fn new(price: f64) -> Self {
        Self { price, size: None }
    }

    pub fn with_size(mut self, size: BracketLegSize) -> Self {
        self.size = Some(size);
        self
    }

    pub(crate) fn resolved_size(&self) -> BracketLegSize {
        self.size.unwrap_or_default()
    }
}

/// Bracket leg configuration attached to a primary order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BracketLegOrders {
    /// Stop-loss trigger configuration. Triggers when price moves against the
    /// position.
    pub stop_loss: Option<BracketLeg>,
    /// Take-profit trigger configuration. Triggers when price moves in favor.
    pub take_profit: Option<BracketLeg>,
}

impl BracketLegOrders {
    pub fn is_empty(&self) -> bool {
        self.stop_loss.is_none() && self.take_profit.is_none()
    }

    pub fn has_explicit_sizes(&self) -> bool {
        self.stop_loss.as_ref().and_then(|leg| leg.size).is_some()
            || self.take_profit.as_ref().and_then(|leg| leg.size).is_some()
    }

    /// Convert to a [`TpSlOrderConfig`] for server-side order endpoints.
    ///
    /// Sets both trigger and execution price to the supplied price for each
    /// leg.
    pub fn to_tp_sl_config(&self) -> crate::phoenix_rise_types::TpSlOrderConfig {
        crate::phoenix_rise_types::TpSlOrderConfig {
            take_profit_trigger_price: self.take_profit.as_ref().map(|leg| leg.price),
            take_profit_execution_price: self.take_profit.as_ref().map(|leg| leg.price),
            stop_loss_trigger_price: self.stop_loss.as_ref().map(|leg| leg.price),
            stop_loss_execution_price: self.stop_loss.as_ref().map(|leg| leg.price),
            ..Default::default()
        }
    }

    /// Convert to a [`TpSlOrderConfig`] for server-side order endpoints.
    ///
    /// Sets both trigger and execution price to the supplied price for each
    /// leg. Explicit per-leg sizing is rejected because these endpoints only
    /// support a shared TP/SL size model.
    pub fn try_to_tp_sl_config(
        &self,
    ) -> Result<crate::phoenix_rise_types::TpSlOrderConfig, String> {
        if self.has_explicit_sizes() {
            return Err(
                "custom TP/SL leg sizing is not supported by isolated HTTP order builders"
                    .to_string(),
            );
        }

        Ok(self.to_tp_sl_config())
    }
}

/// Bracket configuration attached directly to an order ticket.
#[derive(Clone)]
pub struct BracketLegTicket {
    pub rpc_client: Arc<RpcClient>,
    pub bracket_legs: BracketLegOrders,
}

impl std::fmt::Debug for BracketLegTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BracketLegTicket")
            .field("rpc_client", &"RpcClient")
            .field("bracket_legs", &self.bracket_legs)
            .finish()
    }
}

impl BracketLegTicket {
    pub fn new(rpc_client: Arc<RpcClient>, bracket_legs: BracketLegOrders) -> Self {
        Self {
            rpc_client,
            bracket_legs,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bracket_legs.is_empty()
    }

    pub fn rpc_client(&self) -> &RpcClient {
        self.rpc_client.as_ref()
    }

    pub fn bracket_legs(&self) -> &BracketLegOrders {
        &self.bracket_legs
    }
}

fn parse_pubkey_vec(strings: &[String]) -> Result<Vec<Pubkey>, PhoenixTxBuilderError> {
    strings
        .iter()
        .map(|s| Pubkey::from_str(s).map_err(PhoenixTxBuilderError::from))
        .collect()
}
