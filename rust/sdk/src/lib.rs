//! Phoenix WebSocket SDK for Rust.
//!
//! This crate provides a client for subscribing to real-time trader state
//! updates from the Phoenix exchange via WebSocket.
//!
//! # Example
//!
//! ```no_run
//! use phoenix_rise::{PhoenixWSClient, Trader, TraderKey};
//! use solana_pubkey::Pubkey;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to the WebSocket server
//!     let client = PhoenixWSClient::new("wss://api.phoenix.trade/ws")?;
//!
//!     // Create a trader key from your authority pubkey
//!     let key = TraderKey::new(Pubkey::new_unique());
//!
//!     // Create a trader state container
//!     let mut trader = Trader::new(key.clone());
//!
//!     // Subscribe to trader state updates using the authority pubkey
//!     let (mut rx, _handle) = client.subscribe_to_trader_state(&key.authority())?;
//!
//!     // Process updates
//!     while let Some(msg) = rx.recv().await {
//!         trader.apply_update(&msg);
//!         println!(
//!             "Collateral: {}, Positions: {}",
//!             trader.total_collateral(),
//!             trader.all_positions().len()
//!         );
//!     }
//!
//!     Ok(())
//! }
//! ```

#[path = "../../ix/src/lib.rs"]
pub mod ix;
#[path = "../../math/src/lib.rs"]
pub mod math;
#[path = "../../types/src/lib.rs"]
pub mod types;

pub mod phoenix_rise_ix {
    pub use crate::ix::{flight, *};
}

pub mod phoenix_rise_math {
    pub use crate::math::{
        direction, errors, fixed, funding, leverage_tiers, limit_order_state, margin, margin_calc,
        market_math, perp_metadata, portfolio, price, quantities, risk, trader_position, *,
    };
}

pub mod phoenix_rise_types {
    pub use crate::types::{
        accounts, auth, candles, client, conversions, core, exchange, exchange_ws, funding,
        http_error, ix, js_safe_ints, l2book, market, market_state, market_stats, metadata,
        next_commodity_market_transition, next_market_calendar_transition, service_accounts,
        subscription_key, trader, trader_http, trader_key, trader_state, trades, ws, ws_error, *,
    };
}

pub mod accounts;
pub mod api;
mod auth;
mod auth_lifecycle;
mod auth_signers;
mod client;
mod env;
mod exchange_cache;
mod flight_client;
mod http_client;
mod order_tickets;
mod transport;
mod tx_builder;
mod ws_client;

// Re-export main types
pub use accounts::{AccountDataFetcher, PhoenixAccountClient, PhoenixAccountClientError};
pub use api::{
    CandlesClient, CollateralClient, ExchangeClient, FundingClient, InviteClient, MarketsClient,
    OrdersClient, TradersClient, TradesClient,
};
pub use auth::{
    AuthError, AuthSession, AuthSessionSnapshot, AuthSessionStore, FileAuthSessionStore,
    MemoryAuthSessionStore, PhoenixAuthSigner, PhoenixHttpAuthConfig, PhoenixServiceAuthClient,
    PhoenixServiceChallenge, PhoenixServiceLoginRequest, PhoenixWalletNonce,
    PhoenixWalletNonceRequest, PhoenixWalletTransactionChallenge, default_auth_session_store_path,
};
pub use auth_lifecycle::{AuthLifecycleError, AuthLifecycleErrorReason, AuthLifecycleState};
#[cfg(feature = "ed25519-dalek")]
pub use auth_signers::PhoenixEd25519ServiceAuthSigner;
pub use auth_signers::{
    PhoenixAuthSignerKind, PhoenixHttpAuthConfigSignerExt, PhoenixHttpClientBuilderSignerExt,
    PhoenixServiceAccountCredential,
};
#[cfg(feature = "solana-keypair")]
pub use auth_signers::{PhoenixSolanaKeypairAuthSigner, default_solana_keypair_path};
pub use client::PhoenixClient;
pub use env::PhoenixEnv;
pub use exchange_cache::{
    ExchangeCacheApplyError, ExchangeCacheEvent, ExchangeCacheExchangeChangeKind,
    ExchangeCacheMarketChangeKind, ExchangeCacheSnapshotSource, ExchangeInstructionContext,
    PhoenixExchangeCacheStore,
};
pub use flight_client::PhoenixFlightClient;
pub use http_client::{PhoenixHttpClient, PhoenixHttpClientBuilder, RateLimitRetryConfig};
pub use order_tickets::{
    BracketLeg, BracketLegExecution, BracketLegOrders, BracketLegSize, BracketLegTicket,
    DEFAULT_BRACKET_LEG_SLIPPAGE_BPS, LimitOrderTicket, LimitOrderTicketBuilder, MarketOrderTicket,
    MarketOrderTicketBuilder, OrderTicketMetadata,
};
pub use rust_decimal::Decimal;
pub use tx_builder::{PhoenixTxBuilder, PhoenixTxBuilderError};
pub use ws_client::{PhoenixWSClient, SubscriptionHandle, WsConnectionStatus};

pub use crate::phoenix_rise_ix::{
    CancelConditionalOrderParams, CancelId, CancelStopLossParams, CondensedOrder,
    CreateConditionalOrdersAccountParams, Direction, FifoOrderId, IsolatedCollateralFlow,
    IsolatedLimitOrderParams, IsolatedMarketOrderParams, MultiLimitOrderParams,
    OnboardTraderDelegatedParams, OrderFlags, PlaceAttachedConditionalOrderParams,
    PlaceLimitOrderWithConditionalsParams, PlacePositionConditionalOrderParams,
    RegisterTraderParams, SelfTradeBehavior, Side, StopLossOrderKind, TransferCollateralParams,
    TriggerOrderParams, get_conditional_orders_address, get_permission_address,
    phoenix_global_configuration, phoenix_instruction_addresses, phoenix_log_authority,
    phoenix_program_id, resolve_phoenix_instruction_addresses_for_env,
};
// Re-export useful types from the types crate
pub use crate::phoenix_rise_types::{
    AllMidsData, ApiCandle, AuthoritySet, CROSS_MARGIN_SUBACCOUNT_IDX,
    CancelConditionalOrderRequest, CancelStopLossOrderRequest, CandleData, CandlesQueryParams,
    CandlesSubscriptionRequest, ClientCommand, ClientSubscriptionId, CollateralEvent,
    CollateralHistoryQueryParams, CollateralHistoryResponse, CommodityMarketCalendarResponse,
    CommodityMarketCalendarView, CommodityMarketDaySchedule, CommodityMarketHoursRange,
    CommodityMarketState, CommodityMarketStateView, ConditionalTriggerRequest, ETERNAL_PROGRAM_ID,
    ExchangeDeltaMessage, ExchangeDeltaOp, ExchangeEncodedSnapshotMessage, ExchangeKeysView,
    ExchangeMarketConfig, ExchangeMarketParameterUpdate, ExchangeMarketSnapshot,
    ExchangeSnapshotEncoding, ExchangeSnapshotMessage, ExchangeSnapshotReason,
    ExchangeSnapshotView, ExchangeStateSnapshot, ExchangeView, ExchangeWsCommodityMetadata,
    ExchangeWsFeeConfig, ExchangeWsFundingConfig, ExchangeWsLeverageTier,
    ExchangeWsMarkPriceParameters, ExchangeWsMarketPriceBand, FundingHistoryEvent,
    FundingHistoryQueryParams, FundingHistoryResponse, FundingHourlyEvent,
    FundingHourlyHistoryResponse, FundingHourlyQuery, FundingRateHistoryQuery,
    FundingRateHistoryResponse, FundingRateMessage, FundingRatePoint, L2Book, L2BookUpdate,
    LogicalSubscription, MarginTrigger, MarkPriceResponse, Market, MarketCalendar,
    MarketCalendarDaySchedule, MarketCalendarHoursRange, MarketCalendarResponse,
    MarketCalendarStateView, MarketCalendarView, MarketStats, MarketStatsUpdate, MarketStatus,
    NextCommodityMarketTransition, NextMarketCalendarTransition, OrderHistoryItem,
    OrderHistoryQueryParams, OrderHistoryResponse, OrderStatus, PaginatedResponse,
    PhoenixClientError, PhoenixClientEvent, PhoenixClientSubscriptionHandle, PhoenixHttpError,
    PhoenixMetadata, PhoenixSubscription, PhoenixWsError, PlaceAttachedConditionalOrderRequest,
    PlaceIsolatedLimitOrderRequest, PlaceIsolatedLimitOrderWithConditionalsRequest,
    PlaceIsolatedMarketOrderRequest, PlacePositionConditionalOrderRequest,
    PlaceStopLossOrderRequest, PnlPoint, PnlQueryParams, PnlResolution, Position, PriceLevel,
    RuntimeState, ServerMessage, Spline, StopLossExecutionDirection, SubaccountState,
    SubscriptionKey, Timeframe, TpSlOrderConfig, TradeEvent, TradeHistoryItem,
    TradeHistoryQueryParams, TradeHistoryResponse, Trader, TraderKey, TraderStateDelta,
    TraderStatePayload, TraderStateResponse, TraderStateRowChangeKind, TraderStateServerMessage,
    TraderStateSnapshot, TraderView, TradesMessage, TradesSubscriptionRequest,
    UserLiquidationHistoryKind, UserLiquidationHistoryPoint, UserLiquidationHistoryQueryParams,
    UserLiquidationHistoryResponse, UserLiquidationHistoryRole, UserLiquidationHistoryType,
    WalletNonceResponse,
};
pub use crate::types::conversions::*;
