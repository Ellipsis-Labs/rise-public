//! Phoenix instruction construction for Rust.
//!
//! This crate provides functions for building Solana instructions
//! to interact with the Phoenix perpetuals exchange.
//!
//! # Example
//!
//! ```no_run
//! use phoenix_rise::ix::{LimitOrderParams, Side, create_place_limit_order_ix};
//! use solana_pubkey::Pubkey;
//!
//! let params = LimitOrderParams::builder()
//!     .trader(Pubkey::new_unique())
//!     .trader_account(Pubkey::new_unique())
//!     .perp_asset_map(Pubkey::new_unique())
//!     .orderbook(Pubkey::new_unique())
//!     .spline_collection(Pubkey::new_unique())
//!     .global_trader_index(vec![Pubkey::new_unique()])
//!     .active_trader_buffer(vec![Pubkey::new_unique()])
//!     .side(Side::Bid)
//!     .price_in_ticks(50000)
//!     .num_base_lots(1000)
//!     .build()
//!     .unwrap();
//!
//! let ix = create_place_limit_order_ix(params);
//! ```

mod cancel_orders;
mod cancel_stop_loss;
mod conditional_order;
mod constants;
mod create_ata;
mod deposit_funds;
mod ember_deposit;
mod ember_withdraw;
mod error;
pub mod flight;
mod hawkeye;
mod limit_order;
mod market_order;
mod multi_limit_order;
mod onboard_trader_delegated;
mod order_packet;
mod place_stop_loss;
mod register_trader;
mod spl_approve;
mod sync_parent_to_child;
mod transfer_collateral;
mod types;
mod withdraw_funds;

pub use cancel_orders::{CancelOrdersByIdParams, create_cancel_orders_by_id_ix};
pub use cancel_stop_loss::{CancelStopLossParams, create_cancel_stop_loss_ix};
pub use conditional_order::{
    CancelConditionalOrderParams, CreateConditionalOrdersAccountParams,
    PlaceAttachedConditionalOrderParams, PlaceLimitOrderWithConditionalsParams,
    PlacePositionConditionalOrderParams, TriggerOrderParams, create_cancel_conditional_order_ix,
    create_create_conditional_orders_account_ix, create_place_attached_conditional_order_ix,
    create_place_limit_order_with_conditionals_ix, create_place_position_conditional_order_ix,
};
pub use constants::*;
pub use create_ata::create_associated_token_account_idempotent_ix;
pub use deposit_funds::{DepositFundsParams, create_deposit_funds_ix};
pub use ember_deposit::{EmberDepositParams, create_ember_deposit_ix};
pub use ember_withdraw::{EmberWithdrawParams, create_ember_withdraw_ix};
pub use error::PhoenixIxError;
pub use hawkeye::*;
pub use limit_order::{
    IsolatedLimitOrderParams, LimitOrderParams, LimitOrderParamsBuilder,
    create_place_limit_order_ix,
};
pub use market_order::{
    IsolatedMarketOrderParams, MarketOrderDelegatedParams, MarketOrderParams,
    create_place_market_order_delegated_ix, create_place_market_order_ix,
};
pub use multi_limit_order::{
    MultiLimitOrderParams, MultiLimitOrderParamsBuilder, create_place_multi_limit_order_ix,
};
pub use onboard_trader_delegated::{
    OnboardTraderDelegatedParams, create_onboard_trader_delegated_ix,
};
pub use order_packet::{
    CondensedOrder, MultipleOrderPacket, OrderPacket, client_order_id_to_bytes,
};
pub use place_stop_loss::{PlaceStopLossParams, create_place_stop_loss_ix};
pub use register_trader::{RegisterTraderParams, create_register_trader_ix};
pub use spl_approve::{SplApproveParams, create_spl_approve_ix};
pub use sync_parent_to_child::{SyncParentToChildParams, create_sync_parent_to_child_ix};
pub use transfer_collateral::{
    TransferCollateralChildToParentParams, TransferCollateralParams,
    create_transfer_collateral_child_to_parent_ix, create_transfer_collateral_ix,
};
pub use types::*;
pub use withdraw_funds::{WithdrawFundsParams, create_withdraw_funds_ix};
