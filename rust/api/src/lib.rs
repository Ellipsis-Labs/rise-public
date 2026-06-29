//! Phoenix HTTP, websocket, auth, and Flight clients.
//!
//! This crate is the transport layer of the Rise SDK. It is batteries-included
//! for off-chain applications that need typed REST calls, typed websocket
//! subscriptions, auth/session management, exchange metadata caching, or Flight
//! order wrapping. General protocol and product documentation lives at
//! <https://docs.phoenix.trade/>.
//!
//! # Common Entry Points
//!
//! - [`PhoenixHttpClient`] for direct REST access. Start with
//!   [`PhoenixHttpClient::new_from_env`] or [`PhoenixHttpClient::builder`],
//!   then use route clients such as `markets()`, `exchange()`, `traders()`,
//!   `orders()`, `trades()`, `candles()`, `collateral()`, `funding()`, and
//!   `invite()`.
//! - [`PhoenixWSClient`] for direct websocket subscriptions to mids, funding
//!   rates, orderbooks, market state, trades, candles, and trader state. This
//!   requires the `ws` feature.
//! - [`PhoenixClient`] for the higher-level reconnecting client that bootstraps
//!   metadata over HTTP and maintains live subscriptions. This requires the
//!   `ws` feature.
//! - [`PhoenixFlightClient`] for wrapping supported Phoenix order instructions
//!   through Flight.
//!
//! The `ws` feature enables [`PhoenixWSClient`] and [`PhoenixClient`]. It is
//! enabled by default, but HTTP/auth-only users can disable default features to
//! avoid websocket dependencies. `PhoenixHttpClient::new_from_env` and
//! websocket constructors read `PHOENIX_API_URL` and `PHOENIX_WS_URL` when
//! present.

pub use phoenix_rise_types as types;

mod auth;
mod auth_lifecycle;
mod auth_signers;
#[cfg(feature = "ws")]
mod client;
#[cfg(feature = "ws")]
mod client_types;
mod conversions;
mod env;
mod exchange_cache;
mod flight_client;
mod http_client;
mod http_error;
mod metadata;
mod routes;
#[cfg(feature = "ws")]
mod subscription_key;
mod trader_key;
mod trader_state;
mod transport;
#[cfg(feature = "ws")]
mod ws_client;
mod ws_error;

pub use auth::{
    AuthError, AuthSession, AuthSessionSnapshot, AuthSessionStore, FileAuthSessionStore,
    MemoryAuthSessionStore, PhoenixAuthSigner, PhoenixHttpAuthConfig, PhoenixMemorySessionManager,
    PhoenixServiceAuthClient, PhoenixServiceChallenge, PhoenixServiceLoginRequest,
    PhoenixSessionManager, PhoenixWalletNonce, PhoenixWalletNonceRequest,
    PhoenixWalletSessionManager, PhoenixWalletTransactionChallenge,
    default_auth_session_store_path, is_auth_recovery_error,
};
pub use auth_lifecycle::{AuthLifecycleError, AuthLifecycleErrorReason, AuthLifecycleState};
pub use auth_signers::{
    PhoenixAuthSignerKind, PhoenixEd25519ServiceAuthSigner, PhoenixHttpAuthConfigSignerExt,
    PhoenixHttpClientBuilderSignerExt, PhoenixServiceAccountCredential,
    PhoenixSolanaKeypairAuthSigner, default_solana_keypair_path,
};
#[cfg(feature = "ws")]
pub use client::PhoenixClient;
#[cfg(feature = "ws")]
pub use client_types::{
    ClientCommand, ClientSubscriptionId, LogicalSubscription, MarginTrigger, PhoenixClientError,
    PhoenixClientEvent, PhoenixClientSubscriptionHandle, PhoenixSubscription, RuntimeState,
};
pub use conversions::{
    calculator_for_metadata, decimal_from_quote_lots, decimal_from_signed_base_lots,
    decimal_from_signed_quote_lots, decimal_str_to_quote_lots, price_to_decimal,
};
pub use env::PhoenixEnv;
pub use exchange_cache::{
    ExchangeCacheApplyError, ExchangeCacheEvent, ExchangeCacheExchangeChangeKind,
    ExchangeCacheMarketChangeKind, ExchangeCacheSnapshotSource, ExchangeInstructionContext,
    PhoenixExchangeCacheStore,
};
pub use flight_client::PhoenixFlightClient;
pub use http_client::{PhoenixHttpClient, PhoenixHttpClientBuilder, RateLimitRetryConfig};
pub use http_error::PhoenixHttpError;
pub use metadata::PhoenixMetadata;
pub use phoenix_rise_types::exchange::{ExchangeKeysView, ExchangeMarketConfig, ExchangeView};
pub use routes::{
    ActivateReferralTxRequest, ActivateReferralTxResponse, ApiAccountMeta, ApiInstructionResponse,
    BuildRegisterIxsRequest, BuildRegisterIxsResponse, CandlesClient, CollateralClient,
    ExchangeClient, FundingClient, InviteClient, MarketsClient, OrdersClient,
    ReferralActivationPermissionResponse, ReferralActivationTraderStatus,
    ReferralActivationTraderStatusError, SendRegisterIxsRequest, SendRegisterIxsResponse,
    TradersClient, TradesClient, fetch_referral_activation_trader_status,
    has_referral_activation_capabilities, referral_activation_trader_status,
};
#[cfg(feature = "ws")]
pub use subscription_key::SubscriptionKey;
pub use trader_key::{CROSS_MARGIN_SUBACCOUNT_IDX, ETERNAL_PROGRAM_ID, TraderKey};
pub use trader_state::{LimitOrder, Position, Spline, SubaccountState, Trader};
#[cfg(feature = "ws")]
pub use ws_client::{PhoenixWSClient, SubscriptionHandle, WsConnectionStatus, WsSubscriptionEvent};
pub use ws_error::PhoenixWsError;
