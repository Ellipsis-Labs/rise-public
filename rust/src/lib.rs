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

pub mod ix;
pub mod math;
#[cfg(feature = "types")]
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

#[cfg(feature = "types")]
pub mod phoenix_rise_types {
    pub use crate::types::{
        accounts, auth, candles, client, conversions, core, exchange, exchange_ws, funding,
        http_error, ix, js_safe_ints, l2book, market, market_state, market_stats, metadata,
        next_commodity_market_transition, next_market_calendar_transition, service_accounts,
        subscription_key, trader, trader_http, trader_key, trader_state, trades, ws, ws_error, *,
    };
}

#[cfg(feature = "sdk")]
pub mod accounts;
#[cfg(feature = "sdk")]
pub mod api;
#[cfg(feature = "sdk")]
mod auth;
#[cfg(feature = "sdk")]
mod auth_lifecycle;
#[cfg(feature = "sdk")]
mod auth_signers;
#[cfg(feature = "sdk")]
mod client;
#[cfg(feature = "sdk")]
mod env;
#[cfg(feature = "sdk")]
mod exchange_cache;
#[cfg(feature = "sdk")]
mod flight_client;
#[cfg(feature = "sdk")]
mod hawkeye_client;
#[cfg(feature = "sdk")]
mod http_client;
#[cfg(feature = "sdk")]
mod order_tickets;
#[cfg(feature = "test-fixture")]
pub mod test_fixture;
#[cfg(feature = "sdk")]
mod transport;
#[cfg(feature = "sdk")]
mod tx_builder;
#[cfg(feature = "sdk")]
mod ws_client;

// Re-export main types
#[cfg(feature = "sdk")]
pub use accounts::{AccountDataFetcher, PhoenixAccountClient, PhoenixAccountClientError};
#[cfg(feature = "sdk")]
pub use api::{
    ActivateReferralTxRequest, ActivateReferralTxResponse, CandlesClient, CollateralClient,
    ExchangeClient, FundingClient, InviteClient, MarketsClient, OrdersClient,
    ReferralActivationPermissionResponse, ReferralActivationTraderStatus,
    ReferralActivationTraderStatusError, TradersClient, TradesClient,
    fetch_referral_activation_trader_status, has_referral_activation_capabilities,
    referral_activation_trader_status,
};
#[cfg(all(feature = "sdk", feature = "solana-keypair"))]
pub use auth::PhoenixWalletSessionManager;
#[cfg(feature = "sdk")]
pub use auth::{
    AuthError, AuthSession, AuthSessionSnapshot, AuthSessionStore, FileAuthSessionStore,
    MemoryAuthSessionStore, PhoenixAuthSigner, PhoenixHttpAuthConfig, PhoenixMemorySessionManager,
    PhoenixServiceAuthClient, PhoenixServiceChallenge, PhoenixServiceLoginRequest,
    PhoenixSessionManager, PhoenixWalletNonce, PhoenixWalletNonceRequest,
    PhoenixWalletTransactionChallenge, default_auth_session_store_path, is_auth_recovery_error,
};
#[cfg(feature = "sdk")]
pub use auth_lifecycle::{AuthLifecycleError, AuthLifecycleErrorReason, AuthLifecycleState};
#[cfg(all(feature = "sdk", feature = "ed25519-dalek"))]
pub use auth_signers::PhoenixEd25519ServiceAuthSigner;
#[cfg(feature = "sdk")]
pub use auth_signers::{
    PhoenixAuthSignerKind, PhoenixHttpAuthConfigSignerExt, PhoenixHttpClientBuilderSignerExt,
    PhoenixServiceAccountCredential,
};
#[cfg(all(feature = "sdk", feature = "solana-keypair"))]
pub use auth_signers::{PhoenixSolanaKeypairAuthSigner, default_solana_keypair_path};
#[cfg(feature = "sdk")]
pub use client::PhoenixClient;
#[cfg(feature = "sdk")]
pub use env::PhoenixEnv;
#[cfg(feature = "sdk")]
pub use exchange_cache::{
    ExchangeCacheApplyError, ExchangeCacheEvent, ExchangeCacheExchangeChangeKind,
    ExchangeCacheMarketChangeKind, ExchangeCacheSnapshotSource, ExchangeInstructionContext,
    PhoenixExchangeCacheStore,
};
#[cfg(feature = "sdk")]
pub use flight_client::PhoenixFlightClient;
#[cfg(feature = "sdk")]
pub use hawkeye_client::{HawkeyeSimulation, PhoenixHawkeyeClient, PhoenixHawkeyeClientError};
#[cfg(feature = "sdk")]
pub use http_client::{PhoenixHttpClient, PhoenixHttpClientBuilder, RateLimitRetryConfig};
#[cfg(feature = "sdk")]
pub use order_tickets::{
    BracketLeg, BracketLegExecution, BracketLegOrders, BracketLegSize, BracketLegTicket,
    DEFAULT_BRACKET_LEG_SLIPPAGE_BPS, LimitOrderTicket, LimitOrderTicketBuilder, MarketOrderTicket,
    MarketOrderTicketBuilder, OrderTicketMetadata,
};
#[cfg(feature = "types")]
pub use rust_decimal::Decimal;
#[cfg(feature = "sdk")]
pub use tx_builder::{PhoenixTxBuilder, PhoenixTxBuilderError};
#[cfg(feature = "sdk")]
pub use ws_client::{PhoenixWSClient, SubscriptionHandle, WsConnectionStatus, WsSubscriptionEvent};

pub use crate::phoenix_rise_ix::{
    CancelConditionalOrderParams, CancelId, CancelStopLossParams, CondensedOrder,
    CreateConditionalOrdersAccountParams, CreatePermissionParams, Direction, FifoOrderId,
    HAWKEYE_PROGRAM_ID, HAWKEYE_RETURN_VERSION, HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT,
    HAWKEYE_SIMULATION_FEE_PAYER, HawkeyeBboViewAccounts, HawkeyeReturnData,
    HawkeyeReturnDataError, HawkeyeTraderViewAccounts, IsolatedCollateralFlow,
    IsolatedLimitOrderParams, IsolatedMarketOrderParams, MarketOrderDelegatedParams,
    MultiLimitOrderParams, OnboardTraderDelegatedParams, OrderFlags, PhoenixHawkeyeInstruction,
    PlaceAttachedConditionalOrderParams, PlaceLimitOrderWithConditionalsParams,
    PlacePositionConditionalOrderParams, RegisterTraderParams, SelfTradeBehavior,
    SetPermissionDelegatedParams, Side, StopLossOrderKind, TRADER_MANAGEMENT_PERMISSION,
    TRADER_ONBOARDING_PERMISSION, TransferCollateralParams, TriggerOrderParams,
    VIEW_ASSET_RETURN_MAGIC, VIEW_BBO_HAS_ASK, VIEW_BBO_HAS_BID, VIEW_BBO_RETURN_MAGIC,
    VIEW_FUNDING_HAS_ACCUMULATED, VIEW_FUNDING_HAS_UNSETTLED, VIEW_FUNDING_RETURN_MAGIC,
    VIEW_LIQUIDATION_PRICE_RETURN_MAGIC, VIEW_MARGIN_RETURN_MAGIC, ViewAssetParams,
    ViewAssetReturn, ViewBboReturn, ViewFundingReturn, ViewLiquidationPriceReturn,
    ViewMarginReturn, create_create_permission_ix, create_hawkeye_view_bbo_ix,
    create_hawkeye_view_funding_ix, create_hawkeye_view_liquidation_price_ix,
    create_hawkeye_view_margin_for_asset_ix, create_hawkeye_view_margin_ix,
    create_set_permission_delegated_ix, decode_hawkeye_return, decode_hawkeye_return_data,
    get_conditional_orders_address, get_permission_address, phoenix_global_configuration,
    phoenix_instruction_addresses, phoenix_log_authority, phoenix_program_id,
    resolve_phoenix_instruction_addresses_for_env,
};
// Re-export useful types from the types crate
#[cfg(feature = "types")]
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
#[cfg(feature = "types")]
pub use crate::types::conversions::*;
