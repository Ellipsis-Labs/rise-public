//! HTTP client for Phoenix API.
//!
//! This module provides a client for making HTTP requests to the Phoenix API
//! to fetch exchange configuration and market data.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use tracing::debug;

use crate::BracketLegOrders;
use crate::api::{
    CandlesClient, CollateralClient, ExchangeClient, FundingClient, InviteClient, MarketsClient,
    OrdersClient, TradersClient, TradesClient,
};
use crate::auth::{
    AuthError, AuthSession, AuthSessionStore, PhoenixAuthSigner, PhoenixHttpAuthConfig,
    PhoenixServiceAuthClient,
};
use crate::auth_lifecycle::{AuthLifecycleError, AuthLifecycleState};
use crate::env::PhoenixEnv;
use crate::phoenix_rise_ix::{IsolatedCollateralFlow, Side};
use crate::phoenix_rise_types::{
    ApiCandle, CancelStopLossOrderRequest, CandlesQueryParams, CollateralHistoryQueryParams,
    CollateralHistoryResponse, CommodityMarketCalendarResponse, ExchangeKeysView,
    ExchangeMarketConfig, ExchangeResponse, ExchangeSnapshotView, FundingHistoryQueryParams,
    FundingHistoryResponse, FundingHourlyHistoryResponse, FundingHourlyQuery,
    FundingRateHistoryQuery, FundingRateHistoryResponse, MarketCalendarResponse,
    NextCommodityMarketTransition, NextMarketCalendarTransition, OrderHistoryQueryParams,
    OrderHistoryResponse, PhoenixHttpError, PlaceAttachedConditionalOrderRequest,
    PlaceIsolatedLimitOrderRequest, PlaceIsolatedLimitOrderWithConditionalsRequest,
    PlaceIsolatedMarketOrderRequest, PlacePositionConditionalOrderRequest,
    PlaceStopLossOrderRequest, PnlPoint, PnlQueryParams, TradeHistoryQueryParams,
    TradeHistoryResponse, TraderKey, TraderStateResponse, TraderView,
};
use crate::transport::{PhoenixApiClient, PhoenixApiError};

/// Automatic retry behavior for HTTP 429 (rate-limited) responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRetryConfig {
    /// Enable automatic retry on HTTP 429.
    pub enabled: bool,
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
    /// Maximum total time spent sleeping between retries.
    pub max_total_wait: Duration,
    /// Fallback delay if `Retry-After` is missing or invalid.
    pub fallback_delay: Duration,
    /// Maximum delay per retry attempt.
    pub max_delay: Duration,
}

impl Default for RateLimitRetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 2,
            max_total_wait: Duration::from_secs(15),
            fallback_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        }
    }
}

impl RateLimitRetryConfig {
    /// Returns a retry configuration with automatic rate-limit retry disabled.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

/// Shared HTTP transport used by all resource sub-clients.
#[derive(Clone)]
pub(crate) struct HttpClientInner {
    transport: PhoenixApiClient,
    auth: Option<Arc<PhoenixServiceAuthClient>>,
    pub rate_limit_retry: RateLimitRetryConfig,
}

impl HttpClientInner {
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, PhoenixHttpError> {
        self.execute_with_rate_limit_retry(true, || self.transport.get_json_typed(path))
            .await
    }

    pub async fn get_json_with_query<T: DeserializeOwned, Q: Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T, PhoenixHttpError> {
        self.execute_with_rate_limit_retry(true, || self.transport.get_json_with_query(path, query))
            .await
    }

    pub async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, PhoenixHttpError> {
        self.execute_with_rate_limit_retry(false, || self.transport.post_json(path, body))
            .await
    }

    pub fn auth(&self) -> Option<&PhoenixServiceAuthClient> {
        self.auth.as_deref()
    }

    pub fn auth_lifecycle_state(&self) -> AuthLifecycleState {
        self.transport.auth_lifecycle_state()
    }

    pub fn auth_lifecycle_last_error(&self) -> Option<AuthLifecycleError> {
        self.transport.auth_lifecycle_last_error()
    }

    async fn execute_with_rate_limit_retry<T, F, Fut>(
        &self,
        retryable: bool,
        mut operation: F,
    ) -> Result<T, PhoenixHttpError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, PhoenixApiError>>,
    {
        let mut retries: u32 = 0;
        let mut total_wait = Duration::ZERO;

        loop {
            match operation().await {
                Ok(value) => return Ok(value),
                Err(error) if retryable && error.is_rate_limited() => {
                    let retry_after_seconds = error.retry_after_seconds();
                    let attempts = retries.saturating_add(1);
                    let can_retry = self.rate_limit_retry.enabled
                        && retries < self.rate_limit_retry.max_retries;

                    if !can_retry {
                        return Err(map_rate_limited_error(
                            error,
                            attempts,
                            Some(self.auth_lifecycle_state()),
                        ));
                    }

                    let wait = retry_after_seconds
                        .map(Duration::from_secs)
                        .unwrap_or(self.rate_limit_retry.fallback_delay)
                        .min(self.rate_limit_retry.max_delay);
                    let next_total_wait = total_wait.saturating_add(wait);

                    if next_total_wait > self.rate_limit_retry.max_total_wait {
                        return Err(map_rate_limited_error(
                            error,
                            attempts,
                            Some(self.auth_lifecycle_state()),
                        ));
                    }

                    debug!(
                        "Rise HTTP rate limited, retrying attempt {} in {:?} (retry_after={:?})",
                        attempts + 1,
                        wait,
                        retry_after_seconds
                    );

                    tokio::time::sleep(wait).await;
                    total_wait = next_total_wait;
                    retries = retries.saturating_add(1);
                }
                Err(error) => {
                    return Err(map_transport_error(
                        error,
                        Some(self.auth_lifecycle_state()),
                        self.auth_lifecycle_last_error(),
                    ));
                }
            }
        }
    }
}

pub struct PhoenixHttpClientBuilder {
    api_url: String,
    pub(crate) auth: Option<PhoenixHttpAuthConfig>,
    rate_limit_retry: RateLimitRetryConfig,
}

impl PhoenixHttpClientBuilder {
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            auth: None,
            rate_limit_retry: RateLimitRetryConfig::default(),
        }
    }

    pub fn enable_auth(mut self) -> Self {
        self.auth = Some(PhoenixHttpAuthConfig::default());
        self
    }

    pub fn with_auth(mut self, auth: PhoenixHttpAuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_auth_session(mut self, session: AuthSession) -> Self {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_initial_session(session);
        self.auth = Some(auth);
        self
    }

    pub fn with_auth_session_store(mut self, store: Arc<dyn AuthSessionStore>) -> Self {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_session_store(store);
        self.auth = Some(auth);
        self
    }

    pub fn with_auth_signer(mut self, signer: Arc<dyn PhoenixAuthSigner>) -> Self {
        let auth = self.auth.take().unwrap_or_default().with_signer(signer);
        self.auth = Some(auth);
        self
    }

    pub fn with_rate_limit_retry_config(mut self, config: RateLimitRetryConfig) -> Self {
        self.rate_limit_retry = config;
        self
    }

    /// Enables or disables automatic rate-limit retry for this client.
    pub fn with_rate_limit_retry_enabled(mut self, enabled: bool) -> Self {
        self.rate_limit_retry.enabled = enabled;
        self
    }

    /// Disables automatic rate-limit retry for this client.
    pub fn disable_rate_limit_retry(self) -> Self {
        self.with_rate_limit_retry_enabled(false)
    }

    pub fn build(self) -> Result<PhoenixHttpClient, PhoenixHttpError> {
        let auth_parts = self.auth.map(PhoenixHttpAuthConfig::into_parts);

        if let Some(parts) = auth_parts.as_ref() {
            if let Some(session) = parts.initial_session.as_ref() {
                parts
                    .session_store
                    .store_session(session)
                    .map_err(|error| {
                        map_transport_error(PhoenixApiError::Authentication(error), None, None)
                    })?;
            }
        }

        let mut transport_builder = PhoenixApiClient::builder(&self.api_url);
        if let Some(parts) = auth_parts.as_ref() {
            if let Some(session) = parts.initial_session.clone() {
                transport_builder = transport_builder.with_auth_session(session);
            }
            transport_builder =
                transport_builder.with_auth_session_store(parts.session_store.clone());
            if let Some(signer) = parts.signer.clone() {
                transport_builder = transport_builder.with_auth_signer(signer);
            }
        }
        let transport = transport_builder
            .build()
            .map_err(|error| map_transport_error(error, None, None))?;

        let auth = if let Some(parts) = auth_parts {
            Some(Arc::new(PhoenixServiceAuthClient::new(
                &self.api_url,
                parts.session_store,
            )?))
        } else {
            None
        };

        Ok(PhoenixHttpClient {
            inner: HttpClientInner {
                transport,
                auth,
                rate_limit_retry: self.rate_limit_retry,
            },
        })
    }
}

/// HTTP client for Phoenix API.
///
/// Provides resource sub-client accessors (e.g. `client.markets()`,
/// `client.traders()`) that mirror the TypeScript SDK's `V1ApiClients`
/// shape. Existing flat methods remain for backwards compatibility and
/// delegate to the sub-clients.
///
/// # Example
///
/// ```no_run
/// use phoenix_rise::{PhoenixHttpClient, PhoenixHttpClientBuilder};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = PhoenixHttpClientBuilder::new("https://perp-api.phoenix.trade").build()?;
///
///     let markets = client.markets().get_markets().await?;
///     let keys = client.get_exchange_keys().await?;
///
///     assert!(!markets.is_empty());
///     assert!(!keys.global_config.is_empty());
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct PhoenixHttpClient {
    inner: HttpClientInner,
}

impl PhoenixHttpClient {
    /// Creates a builder for an HTTP client.
    pub fn builder(api_url: impl Into<String>) -> PhoenixHttpClientBuilder {
        PhoenixHttpClientBuilder::new(api_url)
    }

    /// Creates a new HTTP client using environment variables.
    pub fn new_from_env() -> Result<Self, PhoenixHttpError> {
        Self::from_env(PhoenixEnv::load())
    }

    /// Creates a new HTTP client using environment variables and
    /// auto-configured auth.
    pub fn new_from_env_with_auth() -> Result<Self, PhoenixHttpError> {
        Self::from_env_with_auth(PhoenixEnv::load())
    }

    /// Creates a new HTTP client from a `PhoenixEnv`.
    pub fn from_env(env: PhoenixEnv) -> Result<Self, PhoenixHttpError> {
        Self::builder(env.api_url).build()
    }

    /// Creates a new HTTP client from a `PhoenixEnv` and auto-configured auth.
    pub fn from_env_with_auth(env: PhoenixEnv) -> Result<Self, PhoenixHttpError> {
        Self::builder(env.api_url)
            .with_auth_from_env()
            .map_err(|error| {
                map_transport_error(PhoenixApiError::Authentication(error), None, None)
            })?
            .build()
    }

    /// Creates a new HTTP client with the given API URL.
    pub fn new(api_url: impl Into<String>) -> Result<Self, PhoenixHttpError> {
        Self::builder(api_url).build()
    }

    /// Creates a new unauthenticated HTTP client.
    pub fn new_public(api_url: impl Into<String>) -> Result<Self, PhoenixHttpError> {
        Self::builder(api_url).build()
    }

    /// Sets automatic rate-limit retry behavior for this client.
    pub fn set_rate_limit_retry_config(&mut self, config: RateLimitRetryConfig) {
        self.inner.rate_limit_retry = config;
    }

    /// Builder-style variant of [`Self::set_rate_limit_retry_config`].
    pub fn with_rate_limit_retry_config(mut self, config: RateLimitRetryConfig) -> Self {
        self.inner.rate_limit_retry = config;
        self
    }

    /// Enables or disables automatic rate-limit retry for this client.
    pub fn set_rate_limit_retry_enabled(&mut self, enabled: bool) {
        self.inner.rate_limit_retry.enabled = enabled;
    }

    /// Builder-style variant of [`Self::set_rate_limit_retry_enabled`].
    pub fn with_rate_limit_retry_enabled(mut self, enabled: bool) -> Self {
        self.inner.rate_limit_retry.enabled = enabled;
        self
    }

    /// Returns the current automatic rate-limit retry configuration.
    pub fn rate_limit_retry_config(&self) -> &RateLimitRetryConfig {
        &self.inner.rate_limit_retry
    }

    /// GET a typed JSON response using the client's configured rate-limit
    /// retry behavior.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, PhoenixHttpError> {
        self.inner.get_json(path).await
    }

    /// GET a typed JSON response with query parameters using the client's
    /// configured rate-limit retry behavior.
    pub async fn get_json_with_query<T: DeserializeOwned, Q: Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T, PhoenixHttpError> {
        self.inner.get_json_with_query(path, query).await
    }

    /// Returns the optional shared auth client when auth was enabled for this
    /// HTTP client.
    pub fn auth(&self) -> Option<&PhoenixServiceAuthClient> {
        self.inner.auth()
    }

    pub fn auth_lifecycle_state(&self) -> AuthLifecycleState {
        self.inner.auth_lifecycle_state()
    }

    pub fn auth_lifecycle_last_error(&self) -> Option<AuthLifecycleError> {
        self.inner.auth_lifecycle_last_error()
    }

    // --- Resource sub-client accessors ---

    pub fn markets(&self) -> MarketsClient<'_> {
        MarketsClient { http: &self.inner }
    }

    pub fn exchange(&self) -> ExchangeClient<'_> {
        ExchangeClient { http: &self.inner }
    }

    pub fn traders(&self) -> TradersClient<'_> {
        TradersClient { http: &self.inner }
    }

    pub fn collateral(&self) -> CollateralClient<'_> {
        CollateralClient { http: &self.inner }
    }

    pub fn funding(&self) -> FundingClient<'_> {
        FundingClient { http: &self.inner }
    }

    pub fn orders(&self) -> OrdersClient<'_> {
        OrdersClient { http: &self.inner }
    }

    pub fn trades(&self) -> TradesClient<'_> {
        TradesClient { http: &self.inner }
    }

    pub fn candles(&self) -> CandlesClient<'_> {
        CandlesClient { http: &self.inner }
    }

    pub fn invite(&self) -> InviteClient<'_> {
        InviteClient { http: &self.inner }
    }

    // --- Backwards-compatible flat methods (delegate to sub-clients) ---

    pub async fn get_exchange_keys(&self) -> Result<ExchangeKeysView, PhoenixHttpError> {
        self.exchange().get_keys().await
    }

    pub async fn get_markets(&self) -> Result<Vec<ExchangeMarketConfig>, PhoenixHttpError> {
        self.markets().get_markets().await
    }

    pub async fn get_market(&self, symbol: &str) -> Result<ExchangeMarketConfig, PhoenixHttpError> {
        self.markets().get_market(symbol).await
    }

    pub async fn get_next_market_calendar_transition(
        &self,
        symbol: &str,
    ) -> Result<NextMarketCalendarTransition, PhoenixHttpError> {
        self.markets()
            .get_next_market_calendar_transition(symbol)
            .await
    }

    pub async fn get_next_commodity_market_transition(
        &self,
    ) -> Result<NextCommodityMarketTransition, PhoenixHttpError> {
        self.markets().get_next_commodity_market_transition().await
    }

    pub async fn get_market_calendar(
        &self,
        symbol: &str,
    ) -> Result<MarketCalendarResponse, PhoenixHttpError> {
        self.markets().get_market_calendar(symbol).await
    }

    pub async fn get_commodity_market_calendar(
        &self,
    ) -> Result<CommodityMarketCalendarResponse, PhoenixHttpError> {
        self.markets().get_commodity_market_calendar().await
    }

    pub async fn get_exchange(&self) -> Result<ExchangeResponse, PhoenixHttpError> {
        self.exchange().get_exchange().await
    }

    pub async fn get_exchange_snapshot(&self) -> Result<ExchangeSnapshotView, PhoenixHttpError> {
        self.exchange().get_snapshot().await
    }

    pub async fn get_traders(
        &self,
        authority: &Pubkey,
    ) -> Result<Vec<TraderView>, PhoenixHttpError> {
        self.traders().get_trader(authority).await
    }

    pub async fn get_trader_state(
        &self,
        authority: &Pubkey,
        pda_index: u8,
    ) -> Result<TraderStateResponse, PhoenixHttpError> {
        self.traders().get_trader_state(authority, pda_index).await
    }

    pub async fn get_collateral_history(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.collateral()
            .get_user_collateral_history(authority, params)
            .await
    }

    pub async fn get_collateral_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.collateral()
            .get_trader_collateral_history(trader_key, params)
            .await
    }

    pub async fn get_funding_history(
        &self,
        authority: &Pubkey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.funding()
            .get_user_funding_history(authority, params)
            .await
    }

    pub async fn get_funding_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.funding()
            .get_trader_funding_history(trader_key, params)
            .await
    }

    pub async fn get_user_hourly_funding_history(
        &self,
        user_pubkey: &Pubkey,
        params: FundingHourlyQuery,
    ) -> Result<FundingHourlyHistoryResponse, PhoenixHttpError> {
        self.funding()
            .get_user_hourly_funding_history(user_pubkey, params)
            .await
    }

    pub async fn get_market_funding_rate_history(
        &self,
        symbol: &str,
        params: FundingRateHistoryQuery,
    ) -> Result<FundingRateHistoryResponse, PhoenixHttpError> {
        self.funding()
            .get_market_funding_rate_history(symbol, params)
            .await
    }

    pub async fn get_order_history(
        &self,
        authority: &Pubkey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.orders()
            .get_trader_order_history(authority, params)
            .await
    }

    pub async fn get_order_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.orders()
            .get_trader_order_history_with_trader_key(trader_key, params)
            .await
    }

    pub async fn get_candles(
        &self,
        params: CandlesQueryParams,
    ) -> Result<Vec<ApiCandle>, PhoenixHttpError> {
        self.candles().get_candles(params).await
    }

    pub async fn get_trade_history(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.get_user_trade_history(authority, params).await
    }

    pub async fn get_user_trade_history(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.trades()
            .get_user_trade_history(authority, params)
            .await
    }

    pub async fn get_trade_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.trades()
            .get_trader_trade_history_with_trader_key(trader_key, params)
            .await
    }

    pub async fn get_trade_history_by_trader_pda(
        &self,
        trader_pda: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.trades()
            .get_trader_trade_history_by_pda(trader_pda, params)
            .await
    }

    pub async fn get_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.traders().get_trader_pnl(authority, params).await
    }

    pub async fn get_user_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.traders().get_user_pnl(authority, params).await
    }

    pub async fn get_trader_pda_pnl(
        &self,
        trader_pda: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.traders().get_trader_pda_pnl(trader_pda, params).await
    }

    pub async fn get_trader_pnl_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.traders()
            .get_trader_pnl_with_trader_key(trader_key, params)
            .await
    }

    pub async fn build_isolated_limit_order_tx(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        price: f64,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_limit_order_tx(
                authority,
                symbol,
                side,
                price,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
            )
            .await
    }

    pub async fn build_isolated_limit_order_tx_with_request(
        &self,
        request: PlaceIsolatedLimitOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_limit_order_tx_with_request(request)
            .await
    }

    pub async fn place_isolated_limit_order_with_conditionals(
        &self,
        request: PlaceIsolatedLimitOrderWithConditionalsRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .place_isolated_limit_order_with_conditionals(request)
            .await
    }

    pub async fn place_stop_loss_order(
        &self,
        request: PlaceStopLossOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders().place_stop_loss_order(request).await
    }

    pub async fn cancel_stop_loss_order(
        &self,
        request: CancelStopLossOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders().cancel_stop_loss_order(request).await
    }

    pub async fn place_attached_conditional_order(
        &self,
        request: PlaceAttachedConditionalOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .place_attached_conditional_order(request)
            .await
    }

    pub async fn place_position_conditional_order(
        &self,
        request: PlacePositionConditionalOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .place_position_conditional_order(request)
            .await
    }

    pub async fn build_isolated_limit_order_tx_enhanced(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        price: f64,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        self.orders()
            .build_isolated_limit_order_tx_enhanced(
                authority,
                symbol,
                side,
                price,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
            )
            .await
    }

    pub async fn build_isolated_limit_order_tx_enhanced_with_request(
        &self,
        request: PlaceIsolatedLimitOrderRequest,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        self.orders()
            .build_isolated_limit_order_tx_enhanced_with_request(request)
            .await
    }

    pub async fn build_isolated_market_order_tx(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx(
                authority,
                symbol,
                side,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
                bracket,
            )
            .await
    }

    pub async fn build_isolated_market_order_tx_with_request(
        &self,
        request: PlaceIsolatedMarketOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx_with_request(request)
            .await
    }

    pub async fn build_isolated_market_order_tx_enhanced(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx_enhanced(
                authority,
                symbol,
                side,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
                bracket,
            )
            .await
    }

    pub async fn build_isolated_market_order_tx_enhanced_with_request(
        &self,
        request: PlaceIsolatedMarketOrderRequest,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx_enhanced_with_request(request)
            .await
    }

    pub async fn register_trader(
        &self,
        authority: &Pubkey,
        code: &str,
    ) -> Result<String, PhoenixHttpError> {
        self.invite().activate_invite(authority, code).await
    }
}

pub(crate) fn map_transport_error(
    error: PhoenixApiError,
    auth_lifecycle_state: Option<AuthLifecycleState>,
    _auth_lifecycle_last_error: Option<AuthLifecycleError>,
) -> PhoenixHttpError {
    match error {
        PhoenixApiError::RequestFailed { source, .. } => PhoenixHttpError::RequestFailed(source),
        PhoenixApiError::ApiError {
            status,
            message,
            error_code,
        } => {
            let status = status.as_u16();
            if auth_lifecycle_state == Some(AuthLifecycleState::ReauthRequired)
                || is_auth_status(status)
                || is_auth_error_code(error_code.as_deref())
            {
                PhoenixHttpError::Authentication {
                    status: Some(status),
                    message,
                    error_code,
                }
            } else {
                PhoenixHttpError::ApiError {
                    status,
                    message,
                    error_code,
                }
            }
        }
        PhoenixApiError::RateLimited {
            message,
            error_code,
            retry_after_seconds,
            ..
        } => PhoenixHttpError::RateLimited {
            retry_after_seconds,
            message,
            error_code,
            attempts: 1,
        },
        PhoenixApiError::Authentication(error) => PhoenixHttpError::Authentication {
            status: None,
            message: error.to_string(),
            error_code: auth_error_code(&error),
        },
        other => PhoenixHttpError::ParseFailed(other.to_string()),
    }
}

fn map_rate_limited_error(
    error: PhoenixApiError,
    attempts: u32,
    auth_lifecycle_state: Option<AuthLifecycleState>,
) -> PhoenixHttpError {
    match error {
        PhoenixApiError::RateLimited {
            message,
            error_code,
            retry_after_seconds,
            ..
        } => PhoenixHttpError::RateLimited {
            retry_after_seconds,
            message,
            error_code,
            attempts,
        },
        PhoenixApiError::ApiError {
            status,
            message,
            error_code,
        } if status.as_u16() == 429 || error_code.as_deref() == Some("rate_limited") => {
            PhoenixHttpError::RateLimited {
                retry_after_seconds: None,
                message,
                error_code,
                attempts,
            }
        }
        other => map_transport_error(other, auth_lifecycle_state, None),
    }
}

fn auth_error_code(error: &AuthError) -> Option<String> {
    match error {
        AuthError::NoAuthSession => Some("no_auth_session".to_string()),
        AuthError::MissingRefreshToken => Some("missing_refresh_token".to_string()),
        AuthError::RefreshExpired => Some("refresh_expired".to_string()),
        AuthError::MissingPopKey => Some("missing_pop_key".to_string()),
        _ => None,
    }
}

fn is_auth_status(status: u16) -> bool {
    matches!(status, 401 | 403)
}

fn is_auth_error_code(code: Option<&str>) -> bool {
    matches!(
        code,
        Some(
            "missing_access_token"
                | "invalid_access_token"
                | "access_token_expired"
                | "access_jti_mismatch"
                | "session_missing"
                | "missing_refresh_token"
                | "invalid_refresh_token"
                | "refresh_expired"
                | "missing_pop_nonce"
                | "missing_pop_mac"
                | "missing_pop_binding"
                | "invalid_pop_nonce"
                | "invalid_pop_mac"
                | "invalid_pop_key"
                | "pop_binding_mismatch"
                | "pop_replay"
                | "pop_too_far_ahead"
                | "no_auth_session"
        )
    )
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;

    use super::*;

    #[test]
    fn test_client_creation() {
        let client = PhoenixHttpClient::new("https://perp-api.phoenix.trade").unwrap();
        assert_eq!(
            client.inner.rate_limit_retry,
            RateLimitRetryConfig::default()
        );
    }

    #[test]
    fn test_client_with_string() {
        let url = String::from("https://api.example.com");
        let client = PhoenixHttpClient::new(url).unwrap();
        assert_eq!(
            client.inner.rate_limit_retry,
            RateLimitRetryConfig::default()
        );
    }

    #[test]
    fn test_client_public() {
        let client = PhoenixHttpClient::new_public("https://api.example.com").unwrap();
        assert_eq!(
            client.inner.rate_limit_retry,
            RateLimitRetryConfig::default()
        );
    }

    #[test]
    fn invalid_url_returns_error() {
        assert!(PhoenixHttpClient::new("not a url").is_err());
        assert!(
            PhoenixHttpClient::from_env(PhoenixEnv {
                api_url: "not a url".to_string(),
                ws_url: "wss://example.com/v1/ws".to_string(),
            })
            .is_err()
        );
    }

    #[test]
    fn builder_enables_auth_with_shared_memory_store() {
        let client = PhoenixHttpClient::builder("https://api.example.com")
            .enable_auth()
            .build()
            .unwrap();

        assert!(client.auth().is_some());
        assert_eq!(
            client.auth_lifecycle_state(),
            AuthLifecycleState::Unauthenticated
        );
    }

    #[test]
    fn maps_reauth_errors_to_authentication_variant() {
        let error = map_transport_error(
            PhoenixApiError::ApiError {
                status: StatusCode::UNAUTHORIZED,
                message: "refresh expired".to_string(),
                error_code: Some("invalid_refresh_token".to_string()),
            },
            Some(AuthLifecycleState::ReauthRequired),
            None,
        );

        match error {
            PhoenixHttpError::Authentication {
                status, error_code, ..
            } => {
                assert_eq!(status, Some(401));
                assert_eq!(error_code.as_deref(), Some("invalid_refresh_token"));
            }
            other => panic!("expected authentication error, got {other:?}"),
        }
    }

    #[test]
    fn disabled_rate_limit_retry_config_turns_retry_off() {
        assert_eq!(
            RateLimitRetryConfig::disabled(),
            RateLimitRetryConfig {
                enabled: false,
                ..RateLimitRetryConfig::default()
            }
        );
    }

    #[test]
    fn builder_can_disable_rate_limit_retry_at_creation() {
        let client = PhoenixHttpClient::builder("https://api.example.com")
            .disable_rate_limit_retry()
            .build()
            .unwrap();

        assert!(!client.rate_limit_retry_config().enabled);
    }
}
