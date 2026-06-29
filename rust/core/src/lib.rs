//! Off-chain account fetchers and transaction builders for Phoenix Rise.
//!
//! `phoenix-rise-core` sits above raw instruction construction. It combines API
//! exchange metadata, account fetchers, order tickets, and math helpers so an
//! off-chain client can build Solana instructions locally without depending on
//! server-assisted order routes.
//!
//! Use [`PhoenixTxBuilder`] for common flows such as market orders, limit
//! orders, cancels, deposits, withdrawals, trader registration, collateral
//! transfers, and Hawkeye simulation instructions. Use [`MarketOrderTicket`]
//! and [`LimitOrderTicket`] when you want trader-facing price/size inputs that
//! the builder converts into ix-level params.
//!
//! For exact account metas or on-chain CPI, use `phoenix-rise-ix` directly
//! instead of this crate. Product and protocol background is available at
//! <https://docs.phoenix.trade/>.

mod account_client;
mod bracket_pricing;
#[cfg(feature = "events")]
pub mod events;
mod hawkeye_client;
mod order_tickets;
mod tx_builder;

pub use account_client::{
    AccountDataFetcher, FetchedAccount, PhoenixAccountClient, PhoenixAccountClientError,
};
pub use hawkeye_client::{HawkeyeSimulation, PhoenixHawkeyeClient, PhoenixHawkeyeClientError};
pub use order_tickets::{
    BracketLeg, BracketLegExecution, BracketLegOrders, BracketLegSize, BracketLegTicket,
    ConditionalOrdersAccountMode, DEFAULT_BRACKET_LEG_SLIPPAGE_BPS, LimitOrderTicket,
    LimitOrderTicketBuilder, MarketOrderTicket, MarketOrderTicketBuilder, OrderTicketMetadata,
};
pub use phoenix_rise_api::{
    CROSS_MARGIN_SUBACCOUNT_IDX, ExchangeKeysView, ExchangeMarketConfig, ExchangeView,
    PhoenixMetadata, Trader, TraderKey, types,
};
pub use tx_builder::{PhoenixTxBuilder, PhoenixTxBuilderError};
