use borsh::{BorshDeserialize, BorshSerialize};
use phoenix_rise::ix::cpi;
use phoenix_rise::ix::types::{Direction, OrderFlags, SelfTradeBehavior, Side, StopLossOrderKind};

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct DepositEmberThenPhoenixParams {
    pub ember_amount: u64,
    pub phoenix_amount: u64,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct WithdrawPhoenixThenEmberParams {
    pub phoenix_amount: u64,
    pub ember_amount: Option<u64>,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct MarketOrderCpiParams {
    pub side: Side,
    pub price_in_ticks: Option<u64>,
    pub num_base_lots: u64,
    pub num_quote_lots: Option<u64>,
    pub min_base_lots_to_fill: u64,
    pub min_quote_lots_to_fill: u64,
    pub self_trade_behavior: SelfTradeBehavior,
    pub match_limit: Option<u64>,
    pub client_order_id: u128,
    pub last_valid_slot: Option<u64>,
    pub order_flags: OrderFlags,
    pub cancel_existing: bool,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct LimitOrderCpiParams {
    pub side: Side,
    pub price_in_ticks: u64,
    pub num_base_lots: u64,
    pub self_trade_behavior: SelfTradeBehavior,
    pub match_limit: Option<u64>,
    pub client_order_id: u128,
    pub last_valid_slot: Option<u64>,
    pub order_flags: OrderFlags,
    pub cancel_existing: bool,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct CancelLimitOrderParams {
    pub node_pointer: u32,
    pub price_in_ticks: u64,
    pub order_sequence_number: u64,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct StopLossCpiParams {
    pub trigger_price: u64,
    pub execution_price: u64,
    pub trade_side: Side,
    pub execution_direction: Direction,
    pub order_kind: StopLossOrderKind,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct CancelStopLossCpiParams {
    pub execution_direction: Direction,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SubaccountOpenAndTradeParams {
    pub max_positions: u64,
    pub trader_pda_index: u8,
    pub child_subaccount_index: u8,
    pub transfer_amount: u64,
    pub order: MarketOrderWithoutArenas,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct SubaccountCloseAndSweepParams {
    pub order: MarketOrderWithoutArenas,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct MarketOrderWithoutArenas {
    pub side: Side,
    pub price_in_ticks: Option<u64>,
    pub num_base_lots: u64,
    pub num_quote_lots: Option<u64>,
    pub min_base_lots_to_fill: u64,
    pub min_quote_lots_to_fill: u64,
    pub self_trade_behavior: SelfTradeBehavior,
    pub match_limit: Option<u64>,
    pub client_order_id: u128,
    pub last_valid_slot: Option<u64>,
    pub order_flags: OrderFlags,
    pub cancel_existing: bool,
}

impl From<&MarketOrderWithoutArenas> for cpi::phoenix::PlaceMarketOrderArgs {
    fn from(order: &MarketOrderWithoutArenas) -> Self {
        Self {
            side: order.side,
            price_in_ticks: order.price_in_ticks,
            num_base_lots: order.num_base_lots,
            num_quote_lots: order.num_quote_lots,
            min_base_lots_to_fill: order.min_base_lots_to_fill,
            min_quote_lots_to_fill: order.min_quote_lots_to_fill,
            self_trade_behavior: order.self_trade_behavior,
            match_limit: order.match_limit,
            client_order_id: order.client_order_id,
            last_valid_slot: order.last_valid_slot,
            order_flags: order.order_flags,
            cancel_existing: order.cancel_existing,
        }
    }
}
