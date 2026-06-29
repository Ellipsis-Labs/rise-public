//! Rust SDK facade for Phoenix perpetuals.
//!
//! `phoenix-rise` is the umbrella crate for the split Rise SDK. It gives you a
//! single entry point for typed HTTP/WebSocket clients, local transaction
//! building, raw instruction construction, account decoding, math helpers, and
//! on-chain CPI helpers.
//!
//! # Start Here
//!
//! Pick the crate and feature set that matches the job:
//!
//! - Build a transaction from fetched market data: enable `api` + `tx-builder`
//!   and start with [`core::PhoenixTxBuilder`].
//! - Build exact instructions from known account addresses: enable `ix` and
//!   start with [`ix::prelude`] or module paths such as
//!   [`ix::market_order::create_place_market_order_ix`].
//! - Decode account bytes: enable `accounts` and start with
//!   [`accounts::perp_asset_map::PerpAssetMap`] or
//!   [`accounts::trader::TraderHeader`].
//! - Invoke Phoenix instructions from an on-chain program: enable `cpi` and use
//!   [`ix::cpi`].
//! - Use typed HTTP, auth, or WebSocket clients: enable `api` and, if needed,
//!   `ws`.
//!
//! Example programs live under `rise/rust/sdk/examples` in this repository and
//! are mirrored into the `rise-public` repository after merge.
//!
//! # Feature Guide
//!
//! - Default features (`api`, `ws`, `sdk`, `tx-builder`): the most ergonomic
//!   off-chain bundle, including HTTP/WebSocket clients, local transaction
//!   builders, account decoders, instruction builders, and math helpers.
//! - `api`: HTTP, auth, exchange-cache, and Flight clients without local
//!   transaction building or websocket transport.
//! - `ws`: websocket clients and live trader-state helpers on top of `api`.
//! - `tx-builder`: off-chain account fetchers and transaction builders such as
//!   [`core::PhoenixTxBuilder`], [`core::MarketOrderTicket`], and
//!   [`core::LimitOrderTicket`]. The legacy `core` feature remains an alias for
//!   this feature.
//! - `ix`: raw Phoenix, Ember, Flight, and Hawkeye instruction builders.
//! - `cpi`: on-chain CPI helpers from `ix::cpi` plus account byte decoders,
//!   without the API/RPC client stack.
//! - `accounts`: borrowed account-data views for accounts such as
//!   [`accounts::perp_asset_map::PerpAssetMap`],
//!   [`accounts::trader::TraderHeader`], and
//!   [`accounts::trader::TraderPositions`].
//! - `math`: pure market math, margin, risk, funding, and quantity wrappers.
//! - `events`: Phoenix market event parsing from `Log` and `LogEventLengths`
//!   instruction batches.
//! - `serde`: JSON-friendly account views, instruction params, math wrappers,
//!   and API DTOs.
//!
//! # Common Paths
//!
//! - REST and WebSocket clients: [`api::PhoenixHttpClient`],
//!   [`api::PhoenixWSClient`], [`api::PhoenixClient`].
//! - Local order and lifecycle builders: [`core::PhoenixTxBuilder`].
//! - Instruction-level builders: [`ix::market_order`], [`ix::limit_order`],
//!   [`ix::deposit_funds`], [`ix::withdraw_funds`], [`ix::register_trader`].
//! - Account decoding: [`accounts::perp_asset_map`], [`accounts::trader`],
//!   [`accounts::global_config`], [`accounts::withdraw_queue`].
//! - CPI helpers: [`ix::cpi`].
//!
//! # Recipes
//!
//! - Fetch exchange metadata, then build and send a limit order:
//!   [`core::PhoenixTxBuilder::place_limit_order`] and
//!   [`examples/send_limit_order.rs`](https://github.com/ellipsis-labs/rise-public/blob/master/rise/rust/sdk/examples/send_limit_order.rs).
//! - Deposit or withdraw collateral through Ember:
//!   [`core::PhoenixTxBuilder::build_deposit_funds`] and
//!   [`core::PhoenixTxBuilder::build_withdraw_funds`], with
//!   [`examples/deposit_funds.rs`](https://github.com/ellipsis-labs/rise-public/blob/master/rise/rust/sdk/examples/deposit_funds.rs).
//! - Build raw instruction bundles when you already know the accounts:
//!   [`ix::prelude`] and [`examples/build_all_ix.rs`](https://github.com/ellipsis-labs/rise-public/blob/master/rise/rust/sdk/examples/build_all_ix.rs).
//! - Decode account bytes for market metadata or trader state:
//!   [`accounts::perp_asset_map::PerpAssetMap`] and
//!   [`accounts::trader::TraderHeader`].
//! - Invoke Phoenix from an on-chain program:
//!   [`ix::cpi`] and the CPI-oriented examples in
//!   [`examples/fetch_on_chain_accounts.rs`](https://github.com/ellipsis-labs/rise-public/blob/master/rise/rust/sdk/examples/fetch_on_chain_accounts.rs).
//! - Subscribe to live market and trader state: [`api::PhoenixWSClient`] and [`examples/compute_trader_margin.rs`](https://github.com/ellipsis-labs/rise-public/blob/master/rise/rust/sdk/examples/compute_trader_margin.rs).
//!
//! For protocol concepts, account semantics, and product behavior, start with
//! the [Phoenix Eternal docs](https://docs.phoenix.trade/).

#[cfg(feature = "accounts")]
pub use phoenix_rise_accounts as accounts;
#[cfg(feature = "api")]
pub use phoenix_rise_api as api;
#[cfg(feature = "tx-builder")]
pub use phoenix_rise_core as core;
#[cfg(feature = "events")]
pub use phoenix_rise_events as events;
#[cfg(any(feature = "cpi", feature = "ix"))]
pub use phoenix_rise_ix as ix;
#[cfg(feature = "cpi")]
pub use phoenix_rise_ix::cpi;
#[cfg(feature = "math")]
pub use phoenix_rise_math as math;
#[cfg(feature = "types")]
pub use phoenix_rise_types;
#[cfg(feature = "types")]
pub use phoenix_rise_types as types;
