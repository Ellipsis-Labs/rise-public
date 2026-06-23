//! Minimal API types for Phoenix WebSocket protocol.
//!
//! These types mirror the wire format used by the Phoenix API without
//! requiring the full phoenix-api-types crate and its dependencies.

pub mod accounts;
pub mod auth;
pub mod candles;
pub mod client;
pub mod conversions;
pub mod core;
pub mod exchange;
pub mod exchange_ws;
pub mod funding;
pub mod http_error;
pub mod ix;
pub mod js_safe_ints;
pub mod l2book;
pub mod market;
pub mod market_state;
pub mod market_stats;
pub mod metadata;
pub mod next_commodity_market_transition;
pub mod next_market_calendar_transition;
pub mod service_accounts;
pub mod subscription_key;
pub mod trader;
pub mod trader_http;
pub mod trader_key;
pub mod trader_state;
pub mod trades;
pub mod ws;
pub mod ws_error;

// Re-export all types at crate root for backwards compatibility
pub use core::*;

pub use auth::*;
pub use candles::*;
pub use client::*;
pub use conversions::*;
pub use exchange::*;
pub use exchange_ws::*;
pub use funding::*;
pub use http_error::*;
pub use ix::*;
pub use js_safe_ints::*;
pub use l2book::*;
pub use market::*;
pub use market_state::*;
pub use market_stats::*;
pub use metadata::*;
pub use next_commodity_market_transition::*;
pub use next_market_calendar_transition::*;
pub use service_accounts::*;
pub use subscription_key::*;
pub use trader::*;
pub use trader_http::*;
pub use trader_key::*;
pub use trader_state::{Position, Spline, SubaccountState, Trader};
pub use trades::*;
pub use ws::*;
pub use ws_error::*;

/// Deprecated module for backwards compatibility.
///
/// Use the specific modules instead:
/// - [`core`] for `Decimal`, `Price`
/// - [`market`] for market config and status types
/// - [`exchange`] for `ExchangeKeysView`, `AuthoritySetView`
#[deprecated(
    since = "0.2.0",
    note = "Use specific modules instead: core, market, exchange"
)]
pub mod http {
    pub use crate::types::core::{Decimal, Price};
    pub use crate::types::exchange::{AuthoritySetView, ExchangeKeysView};
    pub use crate::types::market::{
        L2Orderbook, LeverageTier, MarketFeeConfig, MarketInfo, MarketStatus, MarketSummary,
        MarketUnitConfig, MarketView, MarketsView, RiskFactors, RiskState, RiskTier,
    };
}
