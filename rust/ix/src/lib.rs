//! Raw Phoenix instruction construction for Rust.
//!
//! This crate builds Phoenix, Ember, Flight, Hawkeye, and related helper
//! instructions from explicit account addresses. It does not fetch RPC state or
//! resolve API metadata for you, which keeps the interface predictable for
//! clients that already know the accounts they want to pass.
//!
//! # Start Here
//!
//! - If you want the old flat API, use [`prelude`].
//! - If you want a smaller and clearer import surface, use module paths such as
//!   [`market_order::create_place_market_order_ix`],
//!   [`withdraw_funds::create_withdraw_funds_ix`], and
//!   [`register_trader::create_register_trader_ix`].
//! - If you are writing an on-chain program, start with [`cpi`].
//! - If you need account views alongside instruction builders, enable the
//!   `accounts` feature and use [`accounts`] or `accounts::prelude`.
//!
//! Example programs live in `rise/rust/sdk/examples` in this repository and are
//! mirrored into the `rise-public` repository after merge.
//!
//! # Common Builders
//!
//! - Orders: [`market_order::create_place_market_order_ix`],
//!   [`market_order::create_place_market_order_delegated_ix`],
//!   [`limit_order::create_place_limit_order_ix`], and
//!   [`multi_limit_order::create_place_multi_limit_order_ix`].
//! - Cancels: [`cancel_orders::create_cancel_orders_by_id_ix`],
//!   [`cancel_orders::create_cancel_all_ix`],
//!   [`cancel_orders::create_cancel_up_to_ix`], and
//!   [`cancel_stop_loss::create_cancel_stop_loss_ix`].
//! - Collateral: [`ember_deposit::create_ember_deposit_ix`],
//!   [`deposit_funds::create_deposit_funds_ix`],
//!   [`withdraw_funds::create_withdraw_funds_ix`],
//!   [`ember_withdraw::create_ember_withdraw_ix`],
//!   [`transfer_collateral::create_transfer_collateral_ix`], and
//!   [`sync_parent_to_child::create_sync_parent_to_child_ix`].
//! - Trader lifecycle and views:
//!   [`register_trader::create_register_trader_ix`],
//!   [`delegate_trader::create_delegate_trader_ix`],
//!   [`conditional_order::create_create_conditional_orders_account_ix`],
//!   [`hawkeye::create_hawkeye_view_margin_ix`], and
//!   [`hawkeye::decode_hawkeye_return_data`].
//!
//! Enable the `cpi` feature for on-chain CPI context helpers. Enable `serde`
//! when instruction discriminants and params should serialize cleanly for
//! logging, fixtures, or JSON tooling. For product behavior and protocol
//! background, see <https://docs.phoenix.trade/>.
//!
//! # Example
//!
//! ```no_run
//! use phoenix_rise_ix::prelude::{LimitOrderParams, Side, create_place_limit_order_ix};
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
//! let ix = create_place_limit_order_ix(params).unwrap();
//! ```

#[cfg(feature = "accounts")]
pub mod accounts;

use sha2_const_stable::Sha256;

pub const fn sha2_const(input: &[u8]) -> u64 {
    let hash = Sha256::new().update(input).finalize();
    u64::from_le_bytes([
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
    ])
}

pub mod cancel_orders;
pub mod cancel_stop_loss;
pub mod claim_fees;
pub mod conditional_order;
pub mod constants;
#[cfg(feature = "cpi")]
pub mod cpi;
pub mod create_ata;
pub mod data;
pub mod delegate_trader;
pub mod deposit_funds;
pub mod discriminants;
pub mod ember_deposit;
pub mod ember_withdraw;
pub mod error;
pub mod flight;
pub mod hawkeye;
pub mod limit_order;
pub mod market_order;
pub mod multi_limit_order;
pub mod onboard_trader_delegated;
pub mod order_packet;
pub mod place_stop_loss;
pub mod register_trader;
pub mod return_data;
#[cfg(feature = "serde")]
mod serde_helpers;
pub mod spl_approve;
pub mod spline;
pub mod sync_parent_to_child;
pub mod transfer_collateral;
pub mod types;
pub mod uncross_crank;
pub mod withdraw_funds;

pub use constants::{
    EMBER_PROGRAM_ID, PHOENIX_INSTRUCTION_ADDRESSES, PHOENIX_PROGRAM_ID,
    PhoenixInstructionAddresses, phoenix_instruction_addresses, phoenix_program_id,
    resolve_phoenix_instruction_addresses_for_env,
};
pub use discriminants::{
    EmberInstruction, FlightAccount, FlightInstruction, HawkeyeInstruction, PhoenixInstruction,
};
pub use error::PhoenixIxError;
pub use hawkeye::HAWKEYE_PROGRAM_ID;

/// Broad import surface for callers that want the old flat IX API explicitly.
pub mod prelude {
    #[cfg(feature = "accounts")]
    pub use super::accounts::*;
    pub use super::cancel_orders::*;
    pub use super::cancel_stop_loss::*;
    pub use super::claim_fees::*;
    pub use super::conditional_order::*;
    pub use super::constants::*;
    pub use super::create_ata::*;
    pub use super::data::*;
    pub use super::delegate_trader::*;
    pub use super::deposit_funds::*;
    pub use super::discriminants::*;
    pub use super::ember_deposit::*;
    pub use super::ember_withdraw::*;
    pub use super::error::*;
    pub use super::flight::*;
    pub use super::hawkeye::*;
    pub use super::limit_order::*;
    pub use super::market_order::*;
    pub use super::multi_limit_order::*;
    pub use super::onboard_trader_delegated::*;
    pub use super::order_packet::*;
    pub use super::place_stop_loss::*;
    pub use super::register_trader::*;
    pub use super::return_data::*;
    pub use super::spl_approve::*;
    pub use super::spline::*;
    pub use super::sync_parent_to_child::*;
    pub use super::transfer_collateral::*;
    pub use super::types::*;
    pub use super::uncross_crank::*;
    pub use super::withdraw_funds::*;
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use serde_json::Value;
    use solana_pubkey::Pubkey;

    use super::prelude::*;

    #[test]
    fn pretty_prints_instruction_discriminants_and_params() {
        let instruction_json =
            serde_json::to_string_pretty(&PhoenixInstruction::PlaceLimitOrder).unwrap();
        assert_eq!(instruction_json, "\"PlaceLimitOrder\"");

        let trader = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let params = LimitOrderParams::builder()
            .trader(trader)
            .trader_account(trader_account)
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(Side::Bid)
            .price_in_ticks(50_000)
            .num_base_lots(1_000)
            .self_trade_behavior(SelfTradeBehavior::CancelProvide)
            .order_flags(OrderFlags::ReduceOnly)
            .build()
            .unwrap();

        let value = serde_json::to_value(&params).unwrap();
        assert_eq!(value["trader"], Value::String(trader.to_string()));
        assert_eq!(
            value["trader_account"],
            Value::String(trader_account.to_string())
        );
        assert_eq!(value["side"], Value::String("bid".to_owned()));
        assert_eq!(
            value["self_trade_behavior"],
            Value::String("CancelProvide".to_owned())
        );
        assert_eq!(value["order_flags"], Value::String("ReduceOnly".to_owned()));

        let params_json = serde_json::to_string_pretty(&params).unwrap();
        assert!(params_json.contains("\"price_in_ticks\": 50000"));
    }
}
