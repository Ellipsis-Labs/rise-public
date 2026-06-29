use crate::PhoenixAccount;

#[deprecated(
    since = "0.1.13",
    note = "use phoenix_rise::accounts::PhoenixAccount or phoenix_rise::ix::PhoenixAccount instead"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountDiscriminants {
    pub conditional_order_collection: [u8; 8],
    pub global_configuration: [u8; 8],
    pub market: [u8; 8],
    pub orderbook: [u8; 8],
    pub orderbook_header: [u8; 8],
    pub active_trader_buffer_header: [u8; 8],
    pub active_trader_buffer_arena_header: [u8; 8],
    pub global_trader_index_header: [u8; 8],
    pub global_trader_index_arena_header: [u8; 8],
    pub spline_collection: [u8; 8],
    pub trader: [u8; 8],
    pub perp_asset_map: [u8; 8],
    pub permission_account: [u8; 8],
    pub stop_losses: [u8; 8],
    pub withdraw_queue_header: [u8; 8],
}

#[allow(deprecated)]
#[deprecated(
    since = "0.1.13",
    note = "use phoenix_rise::accounts::PhoenixAccount or phoenix_rise::ix::PhoenixAccount instead"
)]
pub const ACCOUNT_DISCRIMINANTS: AccountDiscriminants = AccountDiscriminants {
    conditional_order_collection: PhoenixAccount::ConditionalOrderCollection.discriminant(),
    global_configuration: PhoenixAccount::GlobalConfiguration.discriminant(),
    market: PhoenixAccount::Market.discriminant(),
    orderbook: PhoenixAccount::Orderbook.discriminant(),
    orderbook_header: PhoenixAccount::OrderbookHeader.discriminant(),
    active_trader_buffer_header: PhoenixAccount::ActiveTraderBufferHeader.discriminant(),
    active_trader_buffer_arena_header: PhoenixAccount::ActiveTraderBufferArenaHeader.discriminant(),
    global_trader_index_header: PhoenixAccount::GlobalTraderIndexHeader.discriminant(),
    global_trader_index_arena_header: PhoenixAccount::GlobalTraderIndexArenaHeader.discriminant(),
    spline_collection: PhoenixAccount::SplineCollection.discriminant(),
    trader: PhoenixAccount::Trader.discriminant(),
    perp_asset_map: PhoenixAccount::PerpAssetMap.discriminant(),
    permission_account: PhoenixAccount::PermissionAccount.discriminant(),
    stop_losses: PhoenixAccount::StopLosses.discriminant(),
    withdraw_queue_header: PhoenixAccount::WithdrawQueueHeader.discriminant(),
};
