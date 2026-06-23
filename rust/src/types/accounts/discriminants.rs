use crate::phoenix_rise_math::sha2_const;

const fn account_discriminant(name: &[u8]) -> [u8; 8] {
    sha2_const(name).to_le_bytes()
}

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
    pub dynamic_trader: [u8; 8],
    pub perp_asset_map: [u8; 8],
    pub permission_account: [u8; 8],
    pub stop_losses: [u8; 8],
    pub withdraw_queue_header: [u8; 8],
}

pub const ACCOUNT_DISCRIMINANTS: AccountDiscriminants = AccountDiscriminants {
    conditional_order_collection: account_discriminant(b"account:conditional_order"),
    global_configuration: account_discriminant(b"account:global_configuration"),
    market: account_discriminant(b"account:market"),
    orderbook: account_discriminant(b"account:orderbook"),
    orderbook_header: account_discriminant(b"account:orderbook"),
    active_trader_buffer_header: account_discriminant(b"account:active_trader_buffer"),
    active_trader_buffer_arena_header: account_discriminant(b"account:active_trader_buffer_arena"),
    global_trader_index_header: account_discriminant(b"account:global_trader_index"),
    global_trader_index_arena_header: account_discriminant(b"account:global_trader_index_arena"),
    spline_collection: account_discriminant(b"account:spline_collection"),
    trader: account_discriminant(b"account:trader"),
    dynamic_trader: account_discriminant(b"account:dynamic_trader"),
    perp_asset_map: account_discriminant(b"account:perp_asset_map"),
    permission_account: account_discriminant(b"account:permission_account"),
    stop_losses: account_discriminant(b"account:stop_losses"),
    withdraw_queue_header: account_discriminant(b"account:withdraw_queue"),
};
