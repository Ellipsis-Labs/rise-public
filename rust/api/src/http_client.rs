//! HTTP client for Phoenix API.
//!
//! This module provides a client for making HTTP requests to the Phoenix API
//! to fetch exchange configuration and market data.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use phoenix_rise_ix::types::{IsolatedCollateralFlow, Side};
use phoenix_rise_types::prelude::{
    ApiCandle, CancelStopLossOrderRequest, CandlesQueryParams, CandlesV2QueryParams,
    CandlesV2Response, CollateralAssetsResponse, CollateralHistoryQueryParams,
    CollateralHistoryResponse, CommodityMarketCalendarResponse, ExchangeKeysView,
    ExchangeMarketConfig, ExchangeResponse, ExchangeSnapshotView, FundingHistoryQueryParams,
    FundingHistoryResponse, FundingHourlyHistoryResponse, FundingHourlyQuery,
    FundingRateHistoryQuery, FundingRateHistoryResponse, MarketCalendarResponse,
    NextCommodityMarketTransition, NextMarketCalendarTransition, OrderHistoryQueryParams,
    OrderHistoryResponse, PlaceAttachedConditionalOrderRequest, PlaceIsolatedLimitOrderRequest,
    PlaceIsolatedLimitOrderWithConditionalsRequest, PlaceIsolatedMarketOrderRequest,
    PlacePositionConditionalOrderRequest, PlaceStopLossOrderRequest, PnlPoint, PnlQueryParams,
    TpSlOrderConfig, TradeHistoryQueryParams, TradeHistoryResponse,
    UserLiquidationHistoryQueryParams, UserLiquidationHistoryResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use tracing::debug;

use crate::auth::{
    AuthError, AuthSession, AuthSessionStore, PhoenixAuthSigner, PhoenixHttpAuthConfig,
    PhoenixServiceAuthClient, PhoenixSessionManager, PhoenixWalletSessionManager,
};
use crate::auth_lifecycle::{AuthLifecycleError, AuthLifecycleState};
use crate::env::PhoenixEnv;
use crate::http_error::PhoenixHttpError;
use crate::routes::{
    CandlesClient, CollateralClient, ExchangeClient, FundingClient, InviteClient, MarketsClient,
    OrdersClient, TradersClient, TradesClient,
};
use crate::trader_key::TraderKey;
use crate::transport::{PhoenixApiClient, PhoenixApiError};

const DEFAULT_RATE_LIMIT_COOLDOWN_MAX_DELAY: Duration = Duration::from_secs(30);
const DEFAULT_RATE_LIMIT_JITTER_PERCENT_MAX: u32 = 15;

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
    /// Maximum fallback delay per retry attempt. Explicit `Retry-After` values
    /// are bounded by `max_total_wait` instead.
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

    fn retry_plan(
        &self,
        retries: u32,
        total_wait: Duration,
        retry_after_seconds: Option<u64>,
    ) -> Option<RateLimitRetryPlan> {
        if !self.enabled || retries >= self.max_retries {
            return None;
        }

        let wait = self.retry_delay(retry_after_seconds);
        let next_total_wait = total_wait.saturating_add(wait);
        if next_total_wait > self.max_total_wait {
            return None;
        }

        Some(RateLimitRetryPlan {
            wait,
            next_total_wait,
        })
    }

    fn retry_delay(&self, retry_after_seconds: Option<u64>) -> Duration {
        retry_after_seconds
            .map(Duration::from_secs)
            .map(rate_limit_delay_with_positive_jitter)
            .unwrap_or_else(|| fallback_delay_with_jitter(self.fallback_delay, self.max_delay))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RateLimitRetryPlan {
    wait: Duration,
    next_total_wait: Duration,
}

/// Shared cross-request cooldown behavior for HTTP 429 responses.
///
/// This is separate from [`RateLimitRetryConfig`]. Retry config controls what
/// happens to the request that received HTTP 429. Cooldown config controls what
/// later independent retryable GET requests on the same [`PhoenixHttpClient`]
/// do while a server `Retry-After` window is active.
///
/// By default, cooldown is enabled. A valid `Retry-After` header extends a
/// client-local cooldown deadline, capped at 30 seconds by default. Later
/// retryable GET requests from clones of the same client wait for that deadline
/// plus a small positive release jitter before sending. POST requests are not
/// delayed by this cooldown.
///
/// The cooldown is client-local, not process-global. Constructing a separate
/// [`PhoenixHttpClient`] creates a separate cooldown state.
///
/// # Example
///
/// Tune the shared cooldown while leaving per-request 429 retry enabled:
///
/// ```no_run
/// use std::time::Duration;
///
/// use phoenix_rise_api::{PhoenixHttpClient, RateLimitCooldownConfig, RateLimitRetryConfig};
///
/// # fn build_client() -> Result<PhoenixHttpClient, Box<dyn std::error::Error>> {
/// let client = PhoenixHttpClient::builder("https://perp-api.phoenix.trade")
///     .with_rate_limit_retry_config(RateLimitRetryConfig {
///         max_retries: 2,
///         max_total_wait: Duration::from_secs(15),
///         ..RateLimitRetryConfig::default()
///     })
///     .with_rate_limit_cooldown_config(RateLimitCooldownConfig {
///         enabled: true,
///         fallback_delay: Duration::from_secs(1),
///         max_delay: Duration::from_secs(30),
///     })
///     .build()?;
///
/// # Ok(client)
/// # }
/// ```
///
/// Disable only the cross-request cooldown when a caller intentionally wants
/// independent GETs to keep their historical behavior:
///
/// ```no_run
/// use phoenix_rise_api::{PhoenixHttpClient, RateLimitCooldownConfig};
///
/// # fn build_client() -> Result<PhoenixHttpClient, Box<dyn std::error::Error>> {
/// let client = PhoenixHttpClient::builder("https://perp-api.phoenix.trade")
///     .with_rate_limit_cooldown_config(RateLimitCooldownConfig::disabled())
///     .build()?;
///
/// # Ok(client)
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitCooldownConfig {
    /// Enable client-wide cooldown waits after retryable GET requests receive
    /// HTTP 429.
    pub enabled: bool,
    /// Fallback cooldown if `Retry-After` is missing or invalid.
    pub fallback_delay: Duration,
    /// Maximum honored cooldown from `Retry-After` or fallback delay.
    pub max_delay: Duration,
}

impl Default for RateLimitCooldownConfig {
    fn default() -> Self {
        let retry_defaults = RateLimitRetryConfig::default();
        Self {
            enabled: true,
            fallback_delay: retry_defaults.fallback_delay,
            max_delay: DEFAULT_RATE_LIMIT_COOLDOWN_MAX_DELAY,
        }
    }
}

impl RateLimitCooldownConfig {
    fn from_retry_config(retry_config: &RateLimitRetryConfig) -> Self {
        Self {
            fallback_delay: retry_config.fallback_delay,
            ..Self::default()
        }
    }

    /// Returns a cooldown configuration with client-wide cooldown disabled.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    fn cooldown_delay(&self, retry_after_seconds: Option<u64>) -> Option<Duration> {
        if !self.enabled {
            return None;
        }

        let delay = retry_after_seconds
            .map(Duration::from_secs)
            .unwrap_or(self.fallback_delay)
            .min(self.max_delay);

        (!delay.is_zero()).then_some(delay)
    }
}

#[derive(Debug, Default)]
struct RateLimitCooldown {
    deadline_ms: AtomicU64,
}

impl RateLimitCooldown {
    async fn wait_if_needed(&self, config: &RateLimitCooldownConfig) {
        if !config.enabled {
            return;
        }

        loop {
            let deadline_ms = self.deadline_ms.load(Ordering::Acquire);
            if deadline_ms == 0 {
                return;
            }

            let now_ms = monotonic_ms();
            if deadline_ms <= now_ms {
                let _ = self.deadline_ms.compare_exchange(
                    deadline_ms,
                    0,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                continue;
            }

            let wait = Duration::from_millis(deadline_ms.saturating_sub(now_ms));
            let jittered_wait = rate_limit_delay_with_positive_jitter(wait);
            debug!(
                cooldown_wait_ms = jittered_wait.as_millis() as u64,
                release_jitter_ms = jittered_wait.saturating_sub(wait).as_millis() as u64,
                "Rise HTTP rate limit cooldown active; waiting before retryable GET"
            );
            tokio::time::sleep(jittered_wait).await;
        }
    }

    fn record_rate_limit(
        &self,
        config: &RateLimitCooldownConfig,
        retry_after_seconds: Option<u64>,
    ) {
        let Some(delay) = config.cooldown_delay(retry_after_seconds) else {
            return;
        };

        let delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        let deadline_ms = monotonic_ms().saturating_add(delay_ms);
        self.deadline_ms.fetch_max(deadline_ms, Ordering::AcqRel);

        debug!(
            cooldown_ms = delay.as_millis() as u64,
            retry_after_seconds, "Rise HTTP rate limit cooldown updated"
        );
    }
}

fn monotonic_ms() -> u64 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();

    let millis = STARTED_AT.get_or_init(Instant::now).elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

/// Shared HTTP transport used by all resource sub-clients.
#[derive(Clone)]
pub(crate) struct HttpClientInner {
    transport: PhoenixApiClient,
    auth: Option<Arc<PhoenixServiceAuthClient>>,
    pub rate_limit_retry: RateLimitRetryConfig,
    pub rate_limit_cooldown_config: RateLimitCooldownConfig,
    rate_limit_cooldown: Arc<RateLimitCooldown>,
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

    #[cfg(feature = "ws")]
    pub(crate) async fn auth_session_for_request(
        &self,
    ) -> Result<Option<AuthSession>, PhoenixHttpError> {
        self.transport
            .auth_session_for_request()
            .await
            .map_err(|error| {
                map_transport_error(
                    error,
                    Some(self.auth_lifecycle_state()),
                    self.auth_lifecycle_last_error(),
                )
            })
    }

    #[cfg(feature = "ws")]
    pub(crate) async fn refresh_auth_session(&self) -> Result<(), PhoenixHttpError> {
        self.transport
            .refresh_auth_session()
            .await
            .map_err(|error| {
                map_transport_error(
                    error,
                    Some(self.auth_lifecycle_state()),
                    self.auth_lifecycle_last_error(),
                )
            })
    }

    #[cfg(all(feature = "opentelemetry", feature = "ws"))]
    pub(crate) fn trace_context_provider(&self) -> Option<crate::transport::TraceContextProvider> {
        self.transport.trace_context_provider()
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
            if retryable {
                self.rate_limit_cooldown
                    .wait_if_needed(&self.rate_limit_cooldown_config)
                    .await;
            }

            match operation().await {
                Ok(value) => return Ok(value),
                Err(error) if retryable && error.is_rate_limited() => {
                    let retry_after_seconds = error.retry_after_seconds();
                    self.rate_limit_cooldown
                        .record_rate_limit(&self.rate_limit_cooldown_config, retry_after_seconds);
                    let attempts = retries.saturating_add(1);
                    let Some(plan) =
                        self.rate_limit_retry
                            .retry_plan(retries, total_wait, retry_after_seconds)
                    else {
                        return Err(map_rate_limited_error(
                            error,
                            attempts,
                            Some(self.auth_lifecycle_state()),
                        ));
                    };

                    debug!(
                        "Rise HTTP rate limited, retrying attempt {} in {:?} (retry_after={:?})",
                        attempts + 1,
                        plan.wait,
                        retry_after_seconds
                    );

                    tokio::time::sleep(plan.wait).await;
                    total_wait = plan.next_total_wait;
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
    rate_limit_cooldown: Option<RateLimitCooldownConfig>,
    #[cfg(feature = "opentelemetry")]
    trace_context_provider: Option<crate::transport::TraceContextProvider>,
}

impl PhoenixHttpClientBuilder {
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            auth: None,
            rate_limit_retry: RateLimitRetryConfig::default(),
            rate_limit_cooldown: None,
            #[cfg(feature = "opentelemetry")]
            trace_context_provider: None,
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

    pub fn with_session_manager(mut self, manager: Arc<dyn PhoenixSessionManager>) -> Self {
        let auth = self
            .auth
            .take()
            .unwrap_or_default()
            .with_session_manager(manager);
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

    /// Sets client-wide cooldown behavior after retryable GET requests receive
    /// HTTP 429.
    ///
    /// This does not replace [`Self::with_rate_limit_retry_config`]. The retry
    /// config still controls whether the rate-limited request is retried. The
    /// cooldown config controls whether later independent retryable GETs wait
    /// for the shared client-local cooldown deadline.
    pub fn with_rate_limit_cooldown_config(mut self, config: RateLimitCooldownConfig) -> Self {
        self.rate_limit_cooldown = Some(config);
        self
    }

    /// Enables or disables client-wide cooldown waits after rate limits.
    ///
    /// Disabling cooldown preserves per-request retry behavior but stops later
    /// independent GETs from waiting on a shared `Retry-After` deadline.
    pub fn with_rate_limit_cooldown_enabled(mut self, enabled: bool) -> Self {
        let mut config = self
            .rate_limit_cooldown
            .unwrap_or_else(|| RateLimitCooldownConfig::from_retry_config(&self.rate_limit_retry));
        config.enabled = enabled;
        self.rate_limit_cooldown = Some(config);
        self
    }

    /// Disables client-wide cooldown waits after rate limits.
    pub fn disable_rate_limit_cooldown(self) -> Self {
        self.with_rate_limit_cooldown_enabled(false)
    }

    /// Sets the OpenTelemetry parent context provider used for outbound API
    /// request spans and trace header propagation.
    #[cfg(feature = "opentelemetry")]
    pub fn with_trace_context_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn() -> opentelemetry::Context + Send + Sync + 'static,
    {
        self.trace_context_provider = Some(Arc::new(provider));
        self
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
        #[cfg(feature = "opentelemetry")]
        if let Some(provider) = self.trace_context_provider {
            transport_builder = transport_builder.with_trace_context_provider(provider);
        }
        if let Some(parts) = auth_parts.as_ref() {
            if let Some(session) = parts.initial_session.clone() {
                transport_builder = transport_builder.with_auth_session(session);
            }
            transport_builder =
                transport_builder.with_auth_session_store(parts.session_store.clone());
            if let Some(signer) = parts.signer.clone() {
                transport_builder = transport_builder.with_auth_signer(signer);
            }
            if let Some(manager) = parts.session_manager.clone() {
                transport_builder = transport_builder.with_session_manager(manager);
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

        let rate_limit_cooldown_config = self
            .rate_limit_cooldown
            .unwrap_or_else(|| RateLimitCooldownConfig::from_retry_config(&self.rate_limit_retry));

        Ok(PhoenixHttpClient {
            inner: HttpClientInner {
                transport,
                auth,
                rate_limit_retry: self.rate_limit_retry,
                rate_limit_cooldown_config,
                rate_limit_cooldown: Arc::new(RateLimitCooldown::default()),
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
/// use phoenix_rise_api::{PhoenixHttpClient, PhoenixHttpClientBuilder};
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

    /// Creates an HTTP client backed by a wallet session manager.
    pub async fn from_url_with_wallet_keypair(
        api_url: impl AsRef<str>,
        keypair: Arc<Keypair>,
    ) -> Result<Self, PhoenixHttpError> {
        let api_url = api_url.as_ref();
        let session_manager = PhoenixWalletSessionManager::login(api_url, keypair).await?;
        Self::builder(api_url)
            .with_session_manager(Arc::new(session_manager))
            .build()
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

    /// Sets client-wide rate-limit cooldown behavior for this client.
    ///
    /// Existing cloned clients share the same cooldown deadline. Changing this
    /// config changes how this client handle observes and updates that shared
    /// deadline on future requests.
    pub fn set_rate_limit_cooldown_config(&mut self, config: RateLimitCooldownConfig) {
        self.inner.rate_limit_cooldown_config = config;
    }

    /// Builder-style variant of [`Self::set_rate_limit_cooldown_config`].
    pub fn with_rate_limit_cooldown_config(mut self, config: RateLimitCooldownConfig) -> Self {
        self.inner.rate_limit_cooldown_config = config;
        self
    }

    /// Enables or disables client-wide cooldown waits after rate limits.
    pub fn set_rate_limit_cooldown_enabled(&mut self, enabled: bool) {
        self.inner.rate_limit_cooldown_config.enabled = enabled;
    }

    /// Builder-style variant of [`Self::set_rate_limit_cooldown_enabled`].
    pub fn with_rate_limit_cooldown_enabled(mut self, enabled: bool) -> Self {
        self.inner.rate_limit_cooldown_config.enabled = enabled;
        self
    }

    /// Returns the current client-wide rate-limit cooldown configuration.
    pub fn rate_limit_cooldown_config(&self) -> &RateLimitCooldownConfig {
        &self.inner.rate_limit_cooldown_config
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

    /// POST a typed JSON request and response using the client's configured
    /// auth behavior.
    pub async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, PhoenixHttpError> {
        self.inner.post_json(path, body).await
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

    #[cfg(feature = "ws")]
    pub(crate) async fn auth_session_for_request(
        &self,
    ) -> Result<Option<AuthSession>, PhoenixHttpError> {
        self.inner.auth_session_for_request().await
    }

    #[cfg(feature = "ws")]
    pub(crate) async fn refresh_auth_session(&self) -> Result<(), PhoenixHttpError> {
        self.inner.refresh_auth_session().await
    }

    #[cfg(all(feature = "opentelemetry", feature = "ws"))]
    pub(crate) fn trace_context_provider(&self) -> Option<crate::transport::TraceContextProvider> {
        self.inner.trace_context_provider()
    }

    /// Access market list, mark price, stats, and orderbook REST routes.
    pub fn markets(&self) -> MarketsClient<'_> {
        MarketsClient { http: &self.inner }
    }

    /// Access exchange metadata and registration instruction REST routes.
    pub fn exchange(&self) -> ExchangeClient<'_> {
        ExchangeClient { http: &self.inner }
    }

    /// Access trader state, positions, and account-related REST routes.
    pub fn traders(&self) -> TradersClient<'_> {
        TradersClient { http: &self.inner }
    }

    /// Access collateral history and movement REST routes.
    pub fn collateral(&self) -> CollateralClient<'_> {
        CollateralClient { http: &self.inner }
    }

    /// Access funding-rate and funding-history REST routes.
    pub fn funding(&self) -> FundingClient<'_> {
        FundingClient { http: &self.inner }
    }

    /// Access order history and server-assisted order instruction routes.
    pub fn orders(&self) -> OrdersClient<'_> {
        OrdersClient { http: &self.inner }
    }

    /// Access trade-history REST routes.
    pub fn trades(&self) -> TradesClient<'_> {
        TradesClient { http: &self.inner }
    }

    /// Access historical candle REST routes.
    pub fn candles(&self) -> CandlesClient<'_> {
        CandlesClient { http: &self.inner }
    }

    /// Access invite and referral activation REST routes.
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

    pub async fn get_collateral_history(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.collateral()
            .get_user_collateral_history(authority, params)
            .await
    }

    pub async fn get_spot_collaterals(&self) -> Result<CollateralAssetsResponse, PhoenixHttpError> {
        self.collateral().get_assets().await
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

    pub async fn get_candles_v2<Q>(&self, params: Q) -> Result<CandlesV2Response, PhoenixHttpError>
    where
        Q: Into<CandlesV2QueryParams>,
    {
        self.candles().get_candles_v2(params).await
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

    pub async fn get_user_liquidation_history(
        &self,
        authority: &Pubkey,
        params: UserLiquidationHistoryQueryParams,
    ) -> Result<UserLiquidationHistoryResponse, PhoenixHttpError> {
        self.trades()
            .get_user_liquidation_history(authority, params)
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
        tp_sl: Option<TpSlOrderConfig>,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx(
                authority,
                symbol,
                side,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
                tp_sl,
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
        tp_sl: Option<TpSlOrderConfig>,
    ) -> Result<(Vec<Instruction>, Option<f64>), PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx_enhanced(
                authority,
                symbol,
                side,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
                tp_sl,
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
        PhoenixApiError::SessionManager(error) => error,
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

fn fallback_delay_with_jitter(fallback_delay: Duration, max_delay: Duration) -> Duration {
    if fallback_delay.is_zero() {
        return Duration::ZERO;
    }

    let jitter_percent = u128::from(fallback_jitter_percent());
    let jittered_millis = fallback_delay.as_millis().saturating_mul(jitter_percent) / 100;
    let millis = u64::try_from(jittered_millis).unwrap_or(u64::MAX);
    Duration::from_millis(millis).min(max_delay)
}

fn rate_limit_delay_with_positive_jitter(delay: Duration) -> Duration {
    if delay.is_zero() {
        return Duration::ZERO;
    }

    let jitter_percent = u128::from(rand::random_range(
        100..=100 + DEFAULT_RATE_LIMIT_JITTER_PERCENT_MAX,
    ));
    let jittered_millis = delay.as_millis().saturating_mul(jitter_percent) / 100;
    let millis = u64::try_from(jittered_millis).unwrap_or(u64::MAX);
    Duration::from_millis(millis)
}

fn fallback_jitter_percent() -> u32 {
    rand::random_range(85..=115)
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
    fn default_rate_limit_cooldown_config_matches_retry_fallback() {
        let config = RateLimitCooldownConfig::default();

        assert!(config.enabled);
        assert_eq!(
            config.fallback_delay,
            RateLimitRetryConfig::default().fallback_delay
        );
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(
            config.cooldown_delay(Some(300)),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn disabled_rate_limit_cooldown_config_turns_cooldown_off() {
        assert_eq!(
            RateLimitCooldownConfig::disabled(),
            RateLimitCooldownConfig {
                enabled: false,
                ..RateLimitCooldownConfig::default()
            }
        );
    }

    #[test]
    fn retry_after_jitter_never_shortens_server_delay() {
        let delay = Duration::from_secs(2);

        for _ in 0..100 {
            let jittered = rate_limit_delay_with_positive_jitter(delay);
            assert!(jittered >= delay);
            assert!(jittered <= Duration::from_millis(2_300));
        }
    }

    #[test]
    fn builder_can_disable_rate_limit_retry_at_creation() {
        let client = PhoenixHttpClient::builder("https://api.example.com")
            .disable_rate_limit_retry()
            .build()
            .unwrap();

        assert!(!client.rate_limit_retry_config().enabled);
    }

    #[test]
    fn builder_can_disable_rate_limit_cooldown_at_creation() {
        let client = PhoenixHttpClient::builder("https://api.example.com")
            .disable_rate_limit_cooldown()
            .build()
            .unwrap();

        assert!(!client.rate_limit_cooldown_config().enabled);
    }

    #[test]
    fn cloned_client_can_disable_cooldown_without_changing_the_source_config() {
        let client = PhoenixHttpClient::builder("https://api.example.com")
            .build()
            .unwrap();
        let cooldown_disabled_client = client.clone().with_rate_limit_cooldown_enabled(false);

        assert!(client.rate_limit_cooldown_config().enabled);
        assert!(
            !cooldown_disabled_client
                .rate_limit_cooldown_config()
                .enabled
        );
    }

    #[test]
    fn builder_default_rate_limit_cooldown_inherits_custom_retry_fallback() {
        let retry_config = RateLimitRetryConfig {
            fallback_delay: Duration::from_secs(7),
            ..RateLimitRetryConfig::default()
        };

        let client = PhoenixHttpClient::builder("https://api.example.com")
            .with_rate_limit_retry_config(retry_config)
            .build()
            .unwrap();

        assert_eq!(
            client.rate_limit_cooldown_config().fallback_delay,
            Duration::from_secs(7)
        );
    }

    #[test]
    fn builder_explicit_rate_limit_cooldown_preserves_custom_fallback() {
        let retry_config = RateLimitRetryConfig {
            fallback_delay: Duration::from_secs(7),
            ..RateLimitRetryConfig::default()
        };
        let cooldown_config = RateLimitCooldownConfig {
            fallback_delay: Duration::from_secs(3),
            ..RateLimitCooldownConfig::default()
        };

        let client = PhoenixHttpClient::builder("https://api.example.com")
            .with_rate_limit_cooldown_config(cooldown_config)
            .with_rate_limit_retry_config(retry_config)
            .build()
            .unwrap();

        assert_eq!(
            client.rate_limit_cooldown_config().fallback_delay,
            Duration::from_secs(3)
        );
    }
}
