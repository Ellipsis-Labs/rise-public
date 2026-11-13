//! WebSocket client for connecting to the Phoenix API.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "opentelemetry")]
use opentelemetry::{Context, global};
#[cfg(feature = "opentelemetry")]
use opentelemetry_http::HeaderInjector;
use parking_lot::Mutex;
use phoenix_rise_types::prelude::{
    AllMidsData, CandleData, CandlesSubscriptionRequest, ClientMessage, ErrorMessage,
    ExchangeMessage, ExchangeSnapshotEncoding, ExchangeSubscriptionRequest, FillsMessage,
    FillsSubscriptionRequest, FundingRateMessage, FundingRateSubscriptionRequest, L2BookMessage,
    L2BookUpdate, L2OrderbookSubscriptionRequest, MarkPriceSubscriptionRequest, MarkPriceUpdate,
    MarketStatsUpdate, MarketStatsV2SubscriptionRequest, MarketStatsV2Update,
    OrderbookSubscriptionRequest, ServerMessage, SubscriptionRequest, SubscriptionStatusMessage,
    Timeframe, TraderStateServerMessage, TraderStateSubscriptionRequest, TradesMessage,
    TradesSubscriptionRequest,
};
use solana_pubkey::Pubkey;
use tokio::sync::mpsc;
use tokio::time::{Instant, Interval, MissedTickBehavior};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
#[cfg(feature = "opentelemetry")]
use tokio_tungstenite::tungstenite::http::HeaderMap;
use tokio_tungstenite::tungstenite::http::header::{AUTHORIZATION, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::{Bytes, Error as TungsteniteError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{Instrument, debug, error, info, warn};
#[cfg(feature = "opentelemetry")]
use tracing_opentelemetry::OpenTelemetrySpanExt;
use url::Url;

use crate::auth::AuthSession;
use crate::env::PhoenixEnv;
use crate::exchange_cache::SharedExchangeCacheStore;
use crate::http_client::PhoenixHttpClient;
use crate::subscription_key::SubscriptionKey;
use crate::transport::PHOENIX_CLIENT_HEADER_VALUE;
#[cfg(feature = "opentelemetry")]
use crate::transport::TraceContextProvider;
use crate::ws_error::PhoenixWsError;

#[cfg(not(feature = "opentelemetry"))]
type TraceContextProvider = ();

/// WebSocket connection status events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsConnectionStatus {
    /// Attempting to connect to the WebSocket server.
    Connecting,
    /// Successfully connected to the WebSocket server.
    Connected,
    /// Connection attempt failed.
    ConnectionFailed,
    /// Connection was closed or lost.
    Disconnected(String),
}

/// WebSocket subscription lifecycle events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSubscriptionEvent {
    /// The server acknowledged a subscription status.
    Status {
        status: String,
        subscription: SubscriptionRequest,
    },
    /// The server rejected a subscription or sent a protocol error.
    Error {
        subscription: Option<SubscriptionRequest>,
        code: String,
        message: String,
    },
}

/// Client-side liveness checks for a [`PhoenixWSClient`] connection.
///
/// Any inbound frame (data, ping, pong, close) counts as activity. After
/// `ping_interval` of inbound silence the client sends a Ping so a live
/// server answers with a Pong. After `idle_timeout` of inbound silence the
/// client closes the socket and reports [`WsConnectionStatus::Disconnected`],
/// so a half-open connection is detected. The client does not reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WsHeartbeatConfig {
    /// Send a client Ping after this much inbound silence. `None` = never.
    pub ping_interval: Option<Duration>,
    /// Report `Disconnected` after this much inbound silence. `None` = never.
    pub idle_timeout: Option<Duration>,
}

impl Default for WsHeartbeatConfig {
    /// Ping after 30 s of silence; disconnect after 90 s (three server ping
    /// periods).
    fn default() -> Self {
        Self {
            ping_interval: Some(Duration::from_secs(30)),
            idle_timeout: Some(Duration::from_secs(90)),
        }
    }
}

impl WsHeartbeatConfig {
    /// Period of the liveness check, or `None` when both checks are off.
    fn check_period(&self) -> Option<Duration> {
        let shortest = match (self.ping_interval, self.idle_timeout) {
            (Some(ping), Some(idle)) => ping.min(idle),
            (Some(period), None) | (None, Some(period)) => period,
            (None, None) => return None,
        };
        Some((shortest / 2).max(Duration::from_millis(1)))
    }
}

/// Builder for [`PhoenixWSClient`], created by [`PhoenixWSClient::builder`].
#[must_use]
pub struct PhoenixWsClientBuilder {
    ws_url: String,
    connection_status: bool,
    auth_client: Option<PhoenixHttpClient>,
    heartbeat: WsHeartbeatConfig,
    #[cfg(feature = "opentelemetry")]
    trace_context_provider: Option<TraceContextProvider>,
}

impl PhoenixWsClientBuilder {
    fn new(ws_url: &str) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            connection_status: false,
            auth_client: None,
            heartbeat: WsHeartbeatConfig::default(),
            #[cfg(feature = "opentelemetry")]
            trace_context_provider: None,
        }
    }

    /// Enable connection status updates; take the receiver with
    /// [`PhoenixWSClient::connection_status_receiver`].
    pub fn with_connection_status(mut self) -> Self {
        self.connection_status = true;
        self
    }

    /// Authenticate the websocket with the session of `auth_client`, and
    /// refresh and reconnect on recoverable auth closes.
    pub fn with_auth(mut self, auth_client: PhoenixHttpClient) -> Self {
        self.auth_client = Some(auth_client);
        self
    }

    /// Override the liveness checks. Defaults to
    /// [`WsHeartbeatConfig::default`].
    pub fn with_heartbeat(mut self, heartbeat: WsHeartbeatConfig) -> Self {
        self.heartbeat = heartbeat;
        self
    }

    /// Set an OpenTelemetry parent context provider for the outbound connect
    /// span and trace header propagation. A provider configured on the auth
    /// client takes precedence.
    #[cfg(feature = "opentelemetry")]
    pub fn with_trace_context_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn() -> Context + Send + Sync + 'static,
    {
        self.trace_context_provider = Some(Arc::new(provider));
        self
    }

    /// Build the client and spawn its connection task.
    pub fn build(self) -> Result<PhoenixWSClient, PhoenixWsError> {
        #[cfg(feature = "opentelemetry")]
        let trace_context_provider = self.trace_context_provider;
        #[cfg(not(feature = "opentelemetry"))]
        let trace_context_provider = None;
        PhoenixWSClient::new_internal(
            &self.ws_url,
            self.connection_status,
            self.auth_client,
            trace_context_provider,
            self.heartbeat,
        )
    }
}

/// Handle for managing an active subscription.
///
/// When dropped, automatically sends an unsubscribe message to the server
/// (if this is the last subscriber for this key).
///
/// # Example
///
/// ```ignore
/// let (mut rx, handle) = client.subscribe_to_orderbook("SOL".to_string())?;
///
/// // Process messages...
/// while let Some(msg) = rx.recv().await {
///     // ...
/// }
///
/// // Unsubscribe by dropping the handle (or let it go out of scope)
/// drop(handle);
/// ```
pub struct SubscriptionHandle {
    control_tx: mpsc::UnboundedSender<ControlMessage>,
    key: SubscriptionKey,
    subscriber_id: u64,
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        let _ = self.control_tx.send(ControlMessage::Unsubscribe {
            key: self.key.clone(),
            subscriber_id: self.subscriber_id,
        });
    }
}

/// Handle for the exchange-cache pump task spawned by
/// [`PhoenixWSClient::subscribe_to_exchange_cache`].
///
/// Dropping the handle stops the pump and unsubscribes from the exchange
/// channel. The primary path, [`PhoenixWSClient::exchange_store`], keeps
/// this handle inside the client so the pump lives exactly as long as the
/// client; only custom-store users of `subscribe_to_exchange_cache` manage
/// it themselves.
pub struct ExchangeCachePump {
    task: tokio::task::JoinHandle<()>,
}

impl ExchangeCachePump {
    /// True once the pump task has stopped, e.g. because the websocket
    /// connection closed. The fed store keeps its last-applied state but no
    /// longer receives updates.
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }
}

impl Drop for ExchangeCachePump {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Subscriber channel for different message types.
enum Subscriber {
    AllMids(mpsc::UnboundedSender<AllMidsData>),
    FundingRate(mpsc::UnboundedSender<FundingRateMessage>),
    Orderbook(mpsc::UnboundedSender<L2BookUpdate>),
    L2Book(mpsc::UnboundedSender<L2BookMessage>),
    MarkPrice(mpsc::UnboundedSender<MarkPriceUpdate>),
    Fills(mpsc::UnboundedSender<FillsMessage>),
    TraderState(mpsc::UnboundedSender<TraderStateServerMessage>),
    MarketStats(mpsc::UnboundedSender<MarketStatsUpdate>),
    MarketStatsV2(mpsc::UnboundedSender<MarketStatsV2Update>),
    Trades(mpsc::UnboundedSender<TradesMessage>),
    Candles(mpsc::UnboundedSender<CandleData>),
    Exchange(mpsc::UnboundedSender<ExchangeMessage>),
}

/// Internal control messages for the connection manager.
enum ControlMessage {
    Subscribe {
        key: SubscriptionKey,
        request: SubscriptionRequest,
        subscriber: Subscriber,
        subscriber_id: u64,
    },
    Unsubscribe {
        key: SubscriptionKey,
        subscriber_id: u64,
    },
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingOp {
    Subscribe,
    Unsubscribe,
}

/// A subscribe or unsubscribe frame sent on the current socket that has no
/// server response yet.
struct PendingFrame {
    key: SubscriptionKey,
    request: SubscriptionRequest,
    op: PendingOp,
}

/// The most recent status for one subscription on this socket. Other
/// subscriptions' statuses must not erase a possible trailing error's owner.
#[derive(Debug)]
enum PrevResponse {
    TraderStateAck,
    ServerEnded(SubscriptionRequest),
    MaybeEnded(SubscriptionRequest),
}

/// The server's responses can no longer be matched to the frames that caused
/// them, so the socket must be dropped.
#[derive(Debug)]
struct ResponseMismatch(String);

/// Per-socket subscription state owned by the connection manager.
///
/// Responses for different subscriptions can arrive out of send order.
/// Echoes identify subscriptions, not request attempts; an error must have
/// only one possible owner among the matching pending frames and ended context.
#[derive(Default)]
struct SocketState {
    subscribers: HashMap<SubscriptionKey, HashMap<u64, Subscriber>>,
    active_subscriptions: HashMap<SubscriptionKey, SubscriptionRequest>,
    /// Wire frames with no response yet, in send order.
    pending: VecDeque<PendingFrame>,
    prev: HashMap<SubscriptionKey, PrevResponse>,
}

impl SocketState {
    fn push_pending(&mut self, key: SubscriptionKey, request: SubscriptionRequest, op: PendingOp) {
        self.pending.push_back(PendingFrame { key, request, op });
    }

    /// Drop the subscription for `key`: every receiver for it yields `None`,
    /// and the later handle drop sends nothing.
    fn close(&mut self, key: &SubscriptionKey) {
        self.subscribers.remove(key);
        self.active_subscriptions.remove(key);
    }

    /// A response to an older frame must not close a replacement that is
    /// still awaiting its own subscribe response.
    fn close_unless_replaced(&mut self, key: &SubscriptionKey) {
        if self
            .oldest_pending(key, Some(PendingOp::Subscribe))
            .is_none()
        {
            self.close(key);
        }
    }

    /// Position of the oldest pending frame for `key`, optionally of one op.
    fn oldest_pending(&self, key: &SubscriptionKey, op: Option<PendingOp>) -> Option<usize> {
        self.pending
            .iter()
            .position(|frame| frame.key == *key && op.is_none_or(|op| frame.op == op))
    }

    fn handle_status(
        &mut self,
        status: SubscriptionStatusMessage,
        event_tx: &mpsc::UnboundedSender<WsSubscriptionEvent>,
    ) -> Result<(), ResponseMismatch> {
        let SubscriptionStatusMessage {
            status,
            subscription: echo,
            ..
        } = status;
        let Some(key) = SubscriptionKey::from_request(&echo) else {
            Self::emit_status(event_tx, status, echo);
            return Ok(());
        };
        let prev = self.prev.remove(&key);

        match status.as_str() {
            "subscribed" => {
                let Some(position) = self.oldest_pending(&key, Some(PendingOp::Subscribe)) else {
                    Self::emit_status(event_tx, status, echo);
                    return Ok(());
                };
                self.pending.remove(position);
                if matches!(key, SubscriptionKey::TraderState { .. }) {
                    self.prev.insert(key, PrevResponse::TraderStateAck);
                }
            }
            "unsubscribed" if matches!(prev, Some(PrevResponse::TraderStateAck)) => {
                if let Some(position) = self.oldest_pending(&key, Some(PendingOp::Unsubscribe)) {
                    // Either our unsubscribe reply or the server ending the
                    // older subscription; a pending replacement stays open.
                    self.pending.remove(position);
                    self.prev
                        .insert(key, PrevResponse::MaybeEnded(echo.clone()));
                } else {
                    self.close_unless_replaced(&key);
                    self.prev
                        .insert(key, PrevResponse::ServerEnded(echo.clone()));
                }
            }
            "unsubscribed" => match self.oldest_pending(&key, None) {
                Some(position) => {
                    let frame = self.pending.remove(position).expect("pending frame exists");
                    if frame.op == PendingOp::Subscribe {
                        self.close_unless_replaced(&key);
                        self.prev
                            .insert(key, PrevResponse::ServerEnded(echo.clone()));
                    }
                }
                None => {
                    if self.active_subscriptions.contains_key(&key) {
                        self.close_unless_replaced(&key);
                        self.prev
                            .insert(key, PrevResponse::ServerEnded(echo.clone()));
                    }
                }
            },
            _ => {}
        }
        Self::emit_status(event_tx, status, echo);
        Ok(())
    }

    fn handle_error(
        &mut self,
        error: ErrorMessage,
        event_tx: &mpsc::UnboundedSender<WsSubscriptionEvent>,
    ) -> Result<(), ResponseMismatch> {
        let ErrorMessage {
            error: message,
            code,
            subscription: echo,
        } = error;
        let emit = |subscription: Option<SubscriptionRequest>| {
            let _ = event_tx.send(WsSubscriptionEvent::Error {
                subscription,
                code: code.to_string(),
                message: message.clone(),
            });
        };

        if let Some(echo) = echo {
            let Some(key) = SubscriptionKey::from_request(&echo) else {
                // A channel this SDK never subscribes to.
                emit(Some(echo));
                return Ok(());
            };
            // Status and error queues are independent. An echo cannot tell
            // whether an older operation or a same-key replacement failed.
            let ended = matches!(
                self.prev.get(&key),
                Some(PrevResponse::ServerEnded(_) | PrevResponse::MaybeEnded(_))
            );
            let pending_count = self
                .pending
                .iter()
                .filter(|frame| frame.key == key)
                .take(2)
                .count();
            if pending_count + usize::from(ended) > 1 {
                emit(Some(echo));
                return Err(ResponseMismatch(
                    "echoed error has multiple possible owners for the same subscription"
                        .to_string(),
                ));
            }
            if let Some(PrevResponse::ServerEnded(request) | PrevResponse::MaybeEnded(request)) =
                self.prev.remove(&key)
            {
                emit(Some(request));
                return Ok(());
            }
            if let Some(position) = self.oldest_pending(&key, None) {
                let frame = self.pending.remove(position).expect("pending frame exists");
                if frame.op == PendingOp::Subscribe {
                    self.close_unless_replaced(&key);
                }
            }
            emit(Some(echo));
            return Ok(());
        }

        let mut ended = self.prev.iter().filter(|(_, response)| {
            matches!(
                response,
                PrevResponse::ServerEnded(_) | PrevResponse::MaybeEnded(_)
            )
        });
        let ended_count = ended.clone().count();
        if self.pending.len() + ended_count > 1 {
            emit(None);
            return Err(ResponseMismatch(
                "legacy error without subscription echo has multiple possible owners".to_string(),
            ));
        }
        if let Some((key, _)) = ended.next() {
            let key = key.clone();
            let response = self.prev.remove(&key).expect("ended response exists");
            if let PrevResponse::ServerEnded(request) | PrevResponse::MaybeEnded(request) = response
            {
                emit(Some(request));
            }
        } else if let Some(frame) = self.pending.pop_front() {
            if frame.op == PendingOp::Subscribe {
                self.close_unless_replaced(&frame.key);
            }
            emit(Some(frame.request));
        } else {
            emit(None);
        }
        Ok(())
    }

    fn emit_status(
        event_tx: &mpsc::UnboundedSender<WsSubscriptionEvent>,
        status: String,
        subscription: SubscriptionRequest,
    ) {
        let _ = event_tx.send(WsSubscriptionEvent::Status {
            status,
            subscription,
        });
    }
}

type PhoenixWsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type PhoenixWsSink = futures_util::stream::SplitSink<PhoenixWsStream, Message>;
type PhoenixWsRead = futures_util::stream::SplitStream<PhoenixWsStream>;

struct ActiveWebSocket {
    sink: PhoenixWsSink,
    read: PhoenixWsRead,
}

impl ActiveWebSocket {
    fn new(stream: PhoenixWsStream) -> Self {
        let (sink, read) = stream.split();
        Self { sink, read }
    }
}

/// Memoized ws-fed exchange store together with the pump that keeps it
/// fresh. Owned by [`PhoenixWSClient`] so the pump lives exactly as long as
/// the client: dropping the client aborts the pump.
struct OwnedExchangeStore {
    store: SharedExchangeCacheStore,
    _pump: ExchangeCachePump,
}

/// WebSocket client for Phoenix API.
///
/// Handles connection management and message routing to subscribers.
pub struct PhoenixWSClient {
    control_tx: mpsc::UnboundedSender<ControlMessage>,
    ws_url: Url,
    ws_connection_status_rx: Option<mpsc::UnboundedReceiver<WsConnectionStatus>>,
    ws_subscription_event_rx: Option<mpsc::UnboundedReceiver<WsSubscriptionEvent>>,
    next_subscriber_id: Arc<AtomicU64>,
    exchange_store_cell: Mutex<Option<OwnedExchangeStore>>,
}

impl PhoenixWSClient {
    /// Start building a client for `ws_url` (e.g.
    /// "wss://api.phoenix.trade/ws"). The builder defaults to no connection
    /// status updates, no auth, and [`WsHeartbeatConfig::default`].
    pub fn builder(ws_url: &str) -> PhoenixWsClientBuilder {
        PhoenixWsClientBuilder::new(ws_url)
    }

    /// Create a new WebSocket client using environment variables.
    ///
    /// Uses `PhoenixEnv::load()` to read configuration from environment.
    pub fn new_from_env() -> Result<Self, PhoenixWsError> {
        Self::from_env(PhoenixEnv::load())
    }

    /// Create a new WebSocket client using environment variables with
    /// connection status updates.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    pub fn new_from_env_with_connection_status() -> Result<Self, PhoenixWsError> {
        Self::from_env_with_connection_status(PhoenixEnv::load())
    }

    /// Create a new WebSocket client from a `PhoenixEnv`.
    pub fn from_env(env: PhoenixEnv) -> Result<Self, PhoenixWsError> {
        Self::builder(&env.ws_url).build()
    }

    /// Create a new WebSocket client from a `PhoenixEnv` with connection status
    /// updates.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    pub fn from_env_with_connection_status(env: PhoenixEnv) -> Result<Self, PhoenixWsError> {
        Self::builder(&env.ws_url).with_connection_status().build()
    }

    pub(crate) fn from_env_with_connection_status_and_auth(
        env: PhoenixEnv,
        auth_client: PhoenixHttpClient,
    ) -> Result<Self, PhoenixWsError> {
        Self::new_with_connection_status_and_auth(&env.ws_url, auth_client)
    }

    /// Create a new authenticated WebSocket client with connection status
    /// updates enabled.
    pub fn new_with_connection_status_and_auth(
        ws_url: &str,
        auth_client: PhoenixHttpClient,
    ) -> Result<Self, PhoenixWsError> {
        Self::builder(ws_url)
            .with_connection_status()
            .with_auth(auth_client)
            .build()
    }

    /// Create a new WebSocket client and connect to the server.
    ///
    /// # Arguments
    /// * `ws_url` - The WebSocket URL (e.g., "wss://api.phoenix.trade/ws")
    pub fn new(ws_url: &str) -> Result<Self, PhoenixWsError> {
        Self::builder(ws_url).build()
    }

    /// Create a new WebSocket client with an OpenTelemetry parent context
    /// provider for the outbound connect span and trace header propagation.
    #[cfg(feature = "opentelemetry")]
    pub fn new_with_trace_context_provider<F>(
        ws_url: &str,
        provider: F,
    ) -> Result<Self, PhoenixWsError>
    where
        F: Fn() -> Context + Send + Sync + 'static,
    {
        Self::builder(ws_url)
            .with_trace_context_provider(provider)
            .build()
    }

    /// Create a new WebSocket client with connection status updates enabled.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    ///
    /// # Arguments
    /// * `ws_url` - The WebSocket URL (e.g., "wss://api.phoenix.trade/ws")
    pub fn new_with_connection_status(ws_url: &str) -> Result<Self, PhoenixWsError> {
        Self::builder(ws_url).with_connection_status().build()
    }

    /// Create a new WebSocket client with connection status updates and an
    /// OpenTelemetry parent context provider for the outbound connect span and
    /// trace header propagation.
    #[cfg(feature = "opentelemetry")]
    pub fn new_with_connection_status_and_trace_context_provider<F>(
        ws_url: &str,
        provider: F,
    ) -> Result<Self, PhoenixWsError>
    where
        F: Fn() -> Context + Send + Sync + 'static,
    {
        Self::builder(ws_url)
            .with_connection_status()
            .with_trace_context_provider(provider)
            .build()
    }

    /// Internal constructor.
    fn new_internal(
        ws_url: &str,
        receiver_connection_status: bool,
        auth_client: Option<PhoenixHttpClient>,
        trace_context_provider: Option<TraceContextProvider>,
        heartbeat: WsHeartbeatConfig,
    ) -> Result<Self, PhoenixWsError> {
        let url = Url::parse(ws_url)?;
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (ws_subscription_event_tx, ws_subscription_event_rx) = mpsc::unbounded_channel();

        let (ws_connection_status_tx, ws_connection_status_rx) = if receiver_connection_status {
            let (tx, rx) = mpsc::unbounded_channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let client = Self {
            control_tx,
            ws_url: url.clone(),
            ws_connection_status_rx,
            ws_subscription_event_rx: Some(ws_subscription_event_rx),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
            exchange_store_cell: Mutex::new(None),
        };
        #[cfg(feature = "opentelemetry")]
        let trace_context_provider = auth_client
            .as_ref()
            .and_then(PhoenixHttpClient::trace_context_provider)
            .or(trace_context_provider)
            .map(|provider| {
                let context = provider();
                Arc::new(move || context.clone()) as TraceContextProvider
            });

        // Spawn the connection manager task
        tokio::spawn(Self::connection_manager(
            url,
            auth_client,
            control_rx,
            ws_connection_status_tx,
            ws_subscription_event_tx,
            trace_context_provider,
            heartbeat,
        ));

        Ok(client)
    }

    /// Subscribe to all mid prices.
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `AllMidsData` messages. Drop the handle to unsubscribe.
    pub fn subscribe_to_all_mids(
        &self,
    ) -> Result<(mpsc::UnboundedReceiver<AllMidsData>, SubscriptionHandle), PhoenixWsError> {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::all_mids();
        let request = SubscriptionRequest::AllMids;

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::AllMids(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to funding rate updates for a given symbol.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `FundingRateMessage` messages. Drop the handle to unsubscribe.
    pub fn subscribe_to_funding_rate(
        &self,
        symbol: String,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<FundingRateMessage>,
            SubscriptionHandle,
        ),
        PhoenixWsError,
    > {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::funding_rate(symbol.clone());
        let request = SubscriptionRequest::FundingRate(FundingRateSubscriptionRequest { symbol });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::FundingRate(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to mark price updates for a given symbol.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `MarkPriceUpdate` messages. Drop the handle to unsubscribe.
    pub fn subscribe_to_mark_price(
        &self,
        symbol: String,
    ) -> Result<(mpsc::UnboundedReceiver<MarkPriceUpdate>, SubscriptionHandle), PhoenixWsError>
    {
        self.subscribe(
            SubscriptionKey::mark_price(symbol.clone()),
            SubscriptionRequest::MarkPrice(MarkPriceSubscriptionRequest { symbol }),
            Subscriber::MarkPrice,
        )
    }

    /// Subscribe to fills for one market, or for every market when `symbol`
    /// is `None`.
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `FillsMessage` batches. Drop the handle to unsubscribe.
    pub fn subscribe_to_fills(
        &self,
        symbol: Option<String>,
    ) -> Result<(mpsc::UnboundedReceiver<FillsMessage>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe(
            SubscriptionKey::fills(symbol.clone()),
            SubscriptionRequest::Fills(FillsSubscriptionRequest {
                market_symbol: symbol,
            }),
            Subscriber::Fills,
        )
    }

    /// Subscribe to L2 orderbook snapshots for a given symbol.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `L2BookMessage` messages. Drop the handle to unsubscribe.
    pub fn subscribe_to_l2_book(
        &self,
        symbol: String,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookMessage>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe_to_l2_book_with_options(symbol, false)
    }

    /// Subscribe to L2 orderbook snapshots for a given symbol, optionally
    /// bypassing the commodities after-hours execution-price-band filter.
    ///
    /// Setting `bypass_execution_band = true` routes the subscription to a
    /// distinct stream that includes price levels outside the tradeable band
    /// (only observable during commodities after-hours).
    pub fn subscribe_to_l2_book_with_options(
        &self,
        symbol: String,
        bypass_execution_band: bool,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookMessage>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe(
            SubscriptionKey::l2_book_with_options(symbol.clone(), bypass_execution_band),
            SubscriptionRequest::L2Book(L2OrderbookSubscriptionRequest {
                coin: symbol,
                bypass_execution_band: Some(bypass_execution_band),
            }),
            Subscriber::L2Book,
        )
    }

    /// Subscribe to orderbook updates for a given symbol.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `L2BookUpdate` messages. Drop the handle to unsubscribe.
    #[deprecated(note = "use subscribe_to_l2_book")]
    pub fn subscribe_to_orderbook(
        &self,
        symbol: String,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookUpdate>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe_to_orderbook_channel(symbol, false)
    }

    /// Subscribe to orderbook updates for a given symbol, optionally
    /// bypassing the commodities after-hours execution-price-band filter.
    ///
    /// Setting `bypass_execution_band = true` routes the subscription to a
    /// distinct stream that includes price levels outside the tradeable band
    /// (only observable during commodities after-hours).
    #[deprecated(note = "use subscribe_to_l2_book")]
    pub fn subscribe_to_orderbook_with_options(
        &self,
        symbol: String,
        bypass_execution_band: bool,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookUpdate>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe_to_orderbook_channel(symbol, bypass_execution_band)
    }

    pub(crate) fn subscribe_to_orderbook_channel(
        &self,
        symbol: String,
        bypass_execution_band: bool,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookUpdate>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe(
            SubscriptionKey::orderbook_with_options(symbol.clone(), bypass_execution_band),
            SubscriptionRequest::Orderbook(OrderbookSubscriptionRequest {
                symbol,
                bypass_execution_band: Some(bypass_execution_band),
            }),
            Subscriber::Orderbook,
        )
    }

    fn subscribe<T>(
        &self,
        key: SubscriptionKey,
        request: SubscriptionRequest,
        subscriber: fn(mpsc::UnboundedSender<T>) -> Subscriber,
    ) -> Result<(mpsc::UnboundedReceiver<T>, SubscriptionHandle), PhoenixWsError> {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: key.clone(),
                request,
                subscriber: subscriber(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to trader state updates for the given authority (uses PDA
    /// index 0).
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `TraderStateServerMessage` updates. Drop the handle to unsubscribe.
    pub fn subscribe_to_trader_state(
        &self,
        authority: &Pubkey,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<TraderStateServerMessage>,
            SubscriptionHandle,
        ),
        PhoenixWsError,
    > {
        self.subscribe_to_trader_state_with_pda(authority, 0)
    }

    /// Subscribe to trader state updates for the given authority and PDA index.
    ///
    /// # Arguments
    /// * `authority` - The trader's authority pubkey
    /// * `trader_pda_index` - The trader PDA subaccount index
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `TraderStateServerMessage` updates. Drop the handle to unsubscribe.
    pub fn subscribe_to_trader_state_with_pda(
        &self,
        authority: &Pubkey,
        trader_pda_index: u8,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<TraderStateServerMessage>,
            SubscriptionHandle,
        ),
        PhoenixWsError,
    > {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::trader(authority, trader_pda_index);
        let request = SubscriptionRequest::TraderState(TraderStateSubscriptionRequest {
            authority: authority.to_string(),
            trader_pda_index,
        });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::TraderState(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to legacy market updates for a given symbol over the V2
    /// market-stats channel.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `MarketStatsUpdate` messages, with `mid_price: None` when the market
    /// has no orderbook midpoint. Drop the handle to unsubscribe.
    ///
    /// The server acknowledges this subscription as
    /// `SubscriptionRequest::MarketStatsV2 { symbols: Some([<symbol>]) }`
    /// (the one-symbol `marketStatsV2` request), not as
    /// `SubscriptionRequest::Market`.
    pub fn subscribe_to_market(
        &self,
        symbol: String,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<MarketStatsUpdate>,
            SubscriptionHandle,
        ),
        PhoenixWsError,
    > {
        self.subscribe_to_market_stats_v2_channel(Some(vec![symbol]), Subscriber::MarketStats)
    }

    /// Subscribe to one batched market-stats snapshot per server refresh.
    ///
    /// Pass `None` for all markets, or `Some` with one or more symbols. Symbols
    /// retain their case on the wire. Equivalent filters share one subscription
    /// locally. Drop the handle to unsubscribe.
    pub fn subscribe_to_market_stats_v2(
        &self,
        symbols: Option<Vec<String>>,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<MarketStatsV2Update>,
            SubscriptionHandle,
        ),
        PhoenixWsError,
    > {
        self.subscribe_to_market_stats_v2_channel(symbols, Subscriber::MarketStatsV2)
    }

    fn subscribe_to_market_stats_v2_channel<T>(
        &self,
        symbols: Option<Vec<String>>,
        subscriber: fn(mpsc::UnboundedSender<T>) -> Subscriber,
    ) -> Result<(mpsc::UnboundedReceiver<T>, SubscriptionHandle), PhoenixWsError> {
        validate_market_stats_v2_symbols(&symbols)?;
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::market_stats_v2(symbols);
        let request = SubscriptionRequest::MarketStatsV2(MarketStatsV2SubscriptionRequest {
            symbols: sub_key.market_stats_v2_symbols().map(<[String]>::to_vec),
        });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: subscriber(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to trade updates.
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `TradesMessage` messages containing the symbol and array of trades.
    /// Drop the handle to unsubscribe.
    pub fn subscribe_to_trades(
        &self,
        symbol: String,
    ) -> Result<(mpsc::UnboundedReceiver<TradesMessage>, SubscriptionHandle), PhoenixWsError> {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::trades(symbol.clone());
        let request = SubscriptionRequest::Trades(TradesSubscriptionRequest { symbol });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::Trades(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to candle updates.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL").
    /// * `timeframe` - Candle timeframe (e.g., Timeframe::Minute1).
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `CandleData` messages. Drop the handle to unsubscribe.
    pub fn subscribe_to_candles(
        &self,
        symbol: String,
        timeframe: Timeframe,
    ) -> Result<(mpsc::UnboundedReceiver<CandleData>, SubscriptionHandle), PhoenixWsError> {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::candles(symbol.clone(), timeframe);
        let request =
            SubscriptionRequest::Candles(CandlesSubscriptionRequest { symbol, timeframe });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::Candles(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: self.control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Subscribe to exchange snapshot/delta updates.
    ///
    /// The subscription requests JSON snapshot encoding explicitly: the
    /// server defaults to `base64+zstd`, which this SDK does not decode.
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `ExchangeMessage` snapshots and deltas. Drop the handle to
    /// unsubscribe.
    pub fn subscribe_to_exchange(
        &self,
    ) -> Result<(mpsc::UnboundedReceiver<ExchangeMessage>, SubscriptionHandle), PhoenixWsError>
    {
        Self::subscribe_exchange_channel(&self.control_tx, &self.next_subscriber_id)
    }

    /// Shared exchange snapshot store fed by this client — the recommended
    /// source of the store consumed by
    /// [`crate::PhoenixFlightClient::from_exchange_store`].
    ///
    /// The first call creates the store, subscribes to the exchange channel,
    /// and keeps the pump handle inside the client, so the pump lives
    /// exactly as long as the client: dropping the client stops updates and
    /// freezes the store at its last-applied state. It then waits for the
    /// first snapshot to be applied, so the returned store is populated and
    /// [`SharedExchangeCacheStore::root_authority`] works immediately.
    /// Subsequent calls return the same store without creating another
    /// subscription.
    ///
    /// There is no internal timeout: if the server never delivers a snapshot
    /// (e.g. the connection failed), this future does not resolve. Wrap it
    /// in `tokio::time::timeout` to bound the wait.
    pub async fn exchange_store(&self) -> Result<SharedExchangeCacheStore, PhoenixWsError> {
        let store = {
            let mut cell = self.exchange_store_cell.lock();
            match cell.as_ref() {
                Some(owned) => owned.store.clone(),
                None => {
                    let store = SharedExchangeCacheStore::new_empty();
                    let pump = self.subscribe_to_exchange_cache(store.clone())?;
                    *cell = Some(OwnedExchangeStore {
                        store: store.clone(),
                        _pump: pump,
                    });
                    store
                }
            }
        };
        // Await outside the lock: concurrent callers share the same store
        // and wait on the same populated signal.
        store.wait_until_populated().await;
        Ok(store)
    }

    /// Subscribe to the exchange channel and keep `store` synchronized with
    /// the snapshot/delta stream.
    ///
    /// Prefer [`Self::exchange_store`], which creates the store, owns the
    /// pump, and waits for the first snapshot; use this method only to feed
    /// a custom [`SharedExchangeCacheStore`], and keep the returned pump
    /// handle alive yourself.
    ///
    /// Snapshots replace the store contents; deltas are applied in sequence
    /// order. On a sequence gap the pump re-subscribes to obtain a fresh
    /// snapshot, mirroring the TS SDK exchange cache. Drop the returned pump
    /// handle to stop the pump and unsubscribe.
    pub fn subscribe_to_exchange_cache(
        &self,
        store: SharedExchangeCacheStore,
    ) -> Result<ExchangeCachePump, PhoenixWsError> {
        let (rx, handle) = self.subscribe_to_exchange()?;
        let task = tokio::spawn(Self::run_exchange_cache_pump(
            self.control_tx.clone(),
            Arc::clone(&self.next_subscriber_id),
            store,
            rx,
            handle,
        ));
        Ok(ExchangeCachePump { task })
    }

    fn subscribe_exchange_channel(
        control_tx: &mpsc::UnboundedSender<ControlMessage>,
        next_subscriber_id: &Arc<AtomicU64>,
    ) -> Result<(mpsc::UnboundedReceiver<ExchangeMessage>, SubscriptionHandle), PhoenixWsError>
    {
        let subscriber_id = next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::exchange();
        let request = SubscriptionRequest::Exchange(ExchangeSubscriptionRequest {
            encoding: Some(ExchangeSnapshotEncoding::Json),
        });

        control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::Exchange(tx),
                subscriber_id,
            })
            .map_err(|_| PhoenixWsError::SubscriptionClosed)?;

        let handle = SubscriptionHandle {
            control_tx: control_tx.clone(),
            key: sub_key,
            subscriber_id,
        };

        Ok((rx, handle))
    }

    /// Apply exchange messages to `store` until the subscription channel
    /// closes, re-subscribing for a fresh snapshot whenever a delta cannot be
    /// applied in sequence.
    async fn run_exchange_cache_pump(
        control_tx: mpsc::UnboundedSender<ControlMessage>,
        next_subscriber_id: Arc<AtomicU64>,
        store: SharedExchangeCacheStore,
        mut rx: mpsc::UnboundedReceiver<ExchangeMessage>,
        mut handle: SubscriptionHandle,
    ) {
        // The subscribe that created `rx` already triggers a server-side
        // snapshot bootstrap, so start with a refresh outstanding.
        let mut awaiting_snapshot = true;
        let mut refresh_pending = true;

        loop {
            let Some(message) = rx.recv().await else {
                debug!("Exchange cache pump stopped: subscription channel closed");
                return;
            };

            match message {
                ExchangeMessage::Snapshot(snapshot) => {
                    store.apply_snapshot_message(&snapshot);
                    awaiting_snapshot = false;
                    refresh_pending = false;
                }
                ExchangeMessage::EncodedSnapshot(_) => {
                    warn!(
                        "Ignoring encoded exchange snapshot; the exchange cache pump subscribes \
                         with json encoding"
                    );
                }
                ExchangeMessage::Delta(delta) => {
                    if !awaiting_snapshot {
                        match store.apply_delta(&delta) {
                            Ok(_) => continue,
                            Err(error) => {
                                warn!(
                                    "Exchange cache delta rejected ({error}); resubscribing for a \
                                     fresh snapshot"
                                );
                                awaiting_snapshot = true;
                            }
                        }
                    }
                    if refresh_pending {
                        continue;
                    }
                    // Drop the stale subscription first so its wire
                    // unsubscribe precedes the new subscribe; the server then
                    // replays a fresh snapshot bootstrap.
                    drop(handle);
                    drop(rx);
                    match Self::subscribe_exchange_channel(&control_tx, &next_subscriber_id) {
                        Ok((new_rx, new_handle)) => {
                            rx = new_rx;
                            handle = new_handle;
                            refresh_pending = true;
                        }
                        Err(error) => {
                            warn!("Exchange cache pump stopped: failed to resubscribe: {error}");
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Returns the WebSocket URL.
    pub fn url(&self) -> &Url {
        &self.ws_url
    }

    /// Returns the connection status receiver, if enabled during construction.
    ///
    /// This takes ownership of the receiver, so it can only be called once.
    /// Returns `None` if `receiver_connection_status` was `false` during
    /// construction, or if the receiver has already been taken.
    pub fn connection_status_receiver(
        &mut self,
    ) -> Option<mpsc::UnboundedReceiver<WsConnectionStatus>> {
        self.ws_connection_status_rx.take()
    }

    /// Returns the subscription event receiver.
    ///
    /// This takes ownership of the receiver, so it can only be called once.
    pub fn subscription_event_receiver(
        &mut self,
    ) -> Option<mpsc::UnboundedReceiver<WsSubscriptionEvent>> {
        self.ws_subscription_event_rx.take()
    }

    /// Shutdown the client and close the connection.
    pub fn shutdown(&self) {
        let _ = self.control_tx.send(ControlMessage::Shutdown);
    }

    /// Connection manager that handles WebSocket connection and message
    /// routing.
    async fn connection_manager(
        url: Url,
        auth_client: Option<PhoenixHttpClient>,
        mut control_rx: mpsc::UnboundedReceiver<ControlMessage>,
        ws_connection_status_tx: Option<mpsc::UnboundedSender<WsConnectionStatus>>,
        ws_subscription_event_tx: mpsc::UnboundedSender<WsSubscriptionEvent>,
        trace_context_provider: Option<TraceContextProvider>,
        heartbeat: WsHeartbeatConfig,
    ) {
        let mut state = SocketState::default();
        let send_status = |status: WsConnectionStatus| {
            if let Some(tx) = ws_connection_status_tx.as_ref() {
                let _ = tx.send(status);
            }
        };

        send_status(WsConnectionStatus::Connecting);

        let ws_stream = match Self::connect(
            &url,
            auth_client.as_ref(),
            trace_context_provider.as_ref(),
        )
        .await
        {
            Ok(stream) => {
                info!("Connected to WebSocket: {}", url);
                send_status(WsConnectionStatus::Connected);
                stream
            }
            Err(e) => {
                error!("Failed to connect: {:?}", e);
                send_status(WsConnectionStatus::ConnectionFailed);
                return;
            }
        };

        let mut active_ws = ActiveWebSocket::new(ws_stream);
        let mut last_frame_at = Instant::now();
        let mut last_ping_at = last_frame_at;
        let mut heartbeat_timer = heartbeat.check_period().map(|period| {
            let mut timer = tokio::time::interval(period);
            timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
            timer
        });

        // Main message loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                ws_msg = active_ws.read.next() => {
                    if matches!(ws_msg, Some(Ok(_))) {
                        last_frame_at = Instant::now();
                    }
                    let processed = match ws_msg {
                        Some(Ok(Message::Text(text))) => Self::process_message(
                            text.as_bytes(),
                            &mut state,
                            &ws_subscription_event_tx,
                        ),
                        Some(Ok(Message::Binary(data))) => {
                            Self::process_message(&data, &mut state, &ws_subscription_event_tx)
                        }
                        Some(Ok(Message::Ping(data))) => {
                            if let Err(e) = active_ws.sink.send(Message::Pong(data)).await {
                                debug!("Failed to respond to Ping: {e:?}");
                            }
                            Ok(())
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                            Ok(())
                        }
                        Some(Ok(Message::Close(frame))) => {
                            let reason = Self::close_reason(frame.as_ref());
                            if let Some(frame) = frame.as_ref() {
                                warn!("WebSocket closed: code={}, reason={}", frame.code, frame.reason);
                            } else {
                                warn!("WebSocket closed without frame");
                            }
                            if Self::is_recoverable_auth_close(frame.as_ref())
                                && let Some(auth_client) = auth_client.as_ref()
                            {
                                match Self::refresh_and_reconnect(
                                    &url,
                                    auth_client,
                                    &mut state,
                                    &mut active_ws,
                                    ws_connection_status_tx.as_ref(),
                                    trace_context_provider.as_ref(),
                                )
                                .await
                                {
                                    Ok(()) => {
                                        last_frame_at = Instant::now();
                                        last_ping_at = last_frame_at;
                                        continue;
                                    }
                                    Err(error) => {
                                        error!("Failed to refresh and reconnect WebSocket auth: {:?}", error);
                                    }
                                }
                            }
                            send_status(WsConnectionStatus::Disconnected(reason));
                            return;
                        }
                        Some(Ok(_)) => Ok(()), // Ignore other message types
                        Some(Err(e)) => {
                            error!("WebSocket error: {:?}", e);
                            send_status(WsConnectionStatus::Disconnected(format!("error: {:?}", e)));
                            return;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            send_status(WsConnectionStatus::Disconnected("stream ended".to_string()));
                            return;
                        }
                    };
                    if let Err(ResponseMismatch(detail)) = processed {
                        // Once a response is missing or ambiguous, later
                        // responses cannot be attributed safely; only a new
                        // socket can recover.
                        warn!("Subscription response mismatch; closing WebSocket: {detail}");
                        send_status(WsConnectionStatus::Disconnected(format!(
                            "subscription response mismatch: {detail}"
                        )));
                        let _ = tokio::time::timeout(Duration::from_secs(1), active_ws.sink.close()).await;
                        return;
                    }
                }

                // Client-side liveness checks
                _ = Self::next_heartbeat_tick(heartbeat_timer.as_mut()) => {
                    let now = Instant::now();
                    let silence = now.duration_since(last_frame_at);
                    if heartbeat.idle_timeout.is_some_and(|idle| silence >= idle) {
                        warn!("WebSocket idle for {silence:?}; closing");
                        let _ = tokio::time::timeout(Duration::from_secs(1), active_ws.sink.close()).await;
                        send_status(WsConnectionStatus::Disconnected(format!(
                            "idle timeout: no frames for {:.1}s",
                            silence.as_secs_f64()
                        )));
                        return;
                    }
                    if heartbeat.ping_interval.is_some_and(|interval| {
                        silence >= interval && now.duration_since(last_ping_at) >= interval
                    }) {
                        last_ping_at = now;
                        if let Err(e) = active_ws.sink.send(Message::Ping(Bytes::new())).await {
                            error!("Failed to send WebSocket ping: {e:?}");
                            send_status(WsConnectionStatus::Disconnected(format!("ping send failed: {e}")));
                            return;
                        }
                    }
                }

                // Handle control messages
                control_msg = control_rx.recv() => {
                    match control_msg {
                        Some(ControlMessage::Subscribe { key, request, subscriber, subscriber_id }) => {
                            let key = key.routing_key();
                            // Get or create the inner HashMap for this key
                            let key_subscribers = state.subscribers.entry(key.clone()).or_default();

                            // Check if this is the first subscriber for this key
                            let is_first_subscriber = key_subscribers.is_empty();

                            // Insert the new subscriber
                            key_subscribers.insert(subscriber_id, subscriber);

                            // Only send wire subscription on first subscriber
                            if is_first_subscriber {
                                state.active_subscriptions.insert(key.clone(), request.clone());
                                Self::send_subscription_frame(
                                    &mut active_ws.sink,
                                    &mut state,
                                    key,
                                    request,
                                    PendingOp::Subscribe,
                                )
                                .await;
                            }
                        }
                        Some(ControlMessage::Unsubscribe { key, subscriber_id }) => {
                            let key = key.routing_key();
                            // Remove this specific subscriber
                            let should_unsubscribe = if let Some(key_subscribers) = state.subscribers.get_mut(&key) {
                                key_subscribers.remove(&subscriber_id);
                                key_subscribers.is_empty()
                            } else {
                                false
                            };

                            // If no subscribers remain for this key, unsubscribe from server
                            if should_unsubscribe {
                                state.subscribers.remove(&key);
                                if let Some(request) = state.active_subscriptions.remove(&key) {
                                    Self::send_subscription_frame(
                                        &mut active_ws.sink,
                                        &mut state,
                                        key,
                                        request,
                                        PendingOp::Unsubscribe,
                                    )
                                    .await;
                                }
                            }
                        }
                        Some(ControlMessage::Shutdown) | None => {
                            info!("Shutting down WebSocket client");
                            let _ = active_ws.sink.close().await;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Wait for the next liveness check; never resolves when both checks are
    /// disabled.
    async fn next_heartbeat_tick(timer: Option<&mut Interval>) {
        match timer {
            Some(timer) => {
                timer.tick().await;
            }
            None => std::future::pending().await,
        }
    }

    /// Send a subscribe or unsubscribe frame and record it as awaiting a
    /// response.
    async fn send_subscription_frame(
        ws_sink: &mut PhoenixWsSink,
        state: &mut SocketState,
        key: SubscriptionKey,
        request: SubscriptionRequest,
        op: PendingOp,
    ) {
        let msg = match op {
            PendingOp::Subscribe => ClientMessage::Subscribe {
                subscription: request.clone(),
            },
            PendingOp::Unsubscribe => ClientMessage::Unsubscribe {
                subscription: request.clone(),
            },
        };
        let bytes = match serde_json::to_vec(&msg) {
            Ok(bytes) => bytes,
            Err(error) => {
                error!("Failed to serialize {op:?} frame: {error}");
                return;
            }
        };
        debug!("Sending {op:?}: {}", String::from_utf8_lossy(&bytes));
        match ws_sink.send(Message::Binary(bytes.into())).await {
            Ok(()) => state.push_pending(key, request, op),
            Err(error) => error!("Failed to send {op:?} frame: {:?}", error),
        }
    }

    async fn refresh_and_reconnect(
        url: &Url,
        auth_client: &PhoenixHttpClient,
        state: &mut SocketState,
        active_ws: &mut ActiveWebSocket,
        ws_connection_status_tx: Option<&mpsc::UnboundedSender<WsConnectionStatus>>,
        trace_context_provider: Option<&TraceContextProvider>,
    ) -> Result<(), PhoenixWsError> {
        auth_client
            .refresh_auth_session()
            .await
            .map_err(ws_auth_error)?;
        if let Some(tx) = ws_connection_status_tx {
            let _ = tx.send(WsConnectionStatus::Connecting);
        }
        let next_ws_stream = Self::connect(url, Some(auth_client), trace_context_provider).await?;
        *active_ws = ActiveWebSocket::new(next_ws_stream);
        if let Some(tx) = ws_connection_status_tx {
            let _ = tx.send(WsConnectionStatus::Connected);
        }
        Self::resend_active_subscriptions(&mut active_ws.sink, state).await;
        Ok(())
    }

    /// Resubscribe every active subscription on a fresh socket. The old
    /// socket's unanswered frames and response history do not carry over.
    async fn resend_active_subscriptions(ws_sink: &mut PhoenixWsSink, state: &mut SocketState) {
        state.pending.clear();
        state.prev.clear();
        let resend = state
            .active_subscriptions
            .iter()
            .map(|(key, request)| (key.clone(), request.clone()))
            .collect::<Vec<_>>();
        for (key, request) in resend {
            debug!("Resending subscription after auth reconnect: {request:?}");
            Self::send_subscription_frame(ws_sink, state, key, request, PendingOp::Subscribe).await;
        }
    }

    /// Connect to the WebSocket server.
    async fn connect(
        url: &Url,
        auth_client: Option<&PhoenixHttpClient>,
        trace_context_provider: Option<&TraceContextProvider>,
    ) -> Result<PhoenixWsStream, PhoenixWsError> {
        #[cfg(not(feature = "opentelemetry"))]
        let _ = trace_context_provider;

        let auth_session = Self::auth_session_for_connect(auth_client).await?;
        let request = Self::connect_request(url, auth_session.as_ref())?;
        let span = tracing::info_span!(
            "phoenix_rise_ws_client_connect",
            http.url = %url,
            otel.kind = "client"
        );
        #[cfg(feature = "opentelemetry")]
        if let Some(context) = trace_parent_context(trace_context_provider) {
            let _ = span.set_parent(context);
        }
        #[cfg(feature = "opentelemetry")]
        let connect_result = {
            let mut request = request;
            inject_trace_headers(&span, request.headers_mut());
            connect_async(request).instrument(span).await
        };
        #[cfg(not(feature = "opentelemetry"))]
        let connect_result = connect_async(request).instrument(span).await;

        let ws_stream = match connect_result {
            Ok((ws_stream, _response)) => ws_stream,
            Err(error) if is_ws_auth_error(&error) => {
                let Some(auth_client) = auth_client else {
                    return Err(error.into());
                };
                auth_client
                    .refresh_auth_session()
                    .await
                    .map_err(ws_auth_error)?;
                let auth_session = Self::auth_session_for_connect(Some(auth_client)).await?;
                let request = Self::connect_request(url, auth_session.as_ref())?;
                let span = tracing::info_span!(
                    "phoenix_rise_ws_client_connect",
                    http.url = %url,
                    otel.kind = "client",
                    retry = true
                );
                #[cfg(feature = "opentelemetry")]
                if let Some(context) = trace_parent_context(trace_context_provider) {
                    let _ = span.set_parent(context);
                }
                #[cfg(feature = "opentelemetry")]
                let connect_result = {
                    let mut request = request;
                    inject_trace_headers(&span, request.headers_mut());
                    connect_async(request).instrument(span).await
                };
                #[cfg(not(feature = "opentelemetry"))]
                let connect_result = connect_async(request).instrument(span).await;
                let (ws_stream, _response) = connect_result?;
                ws_stream
            }
            Err(error) => return Err(error.into()),
        };
        Ok(ws_stream)
    }

    async fn auth_session_for_connect(
        auth_client: Option<&PhoenixHttpClient>,
    ) -> Result<Option<AuthSession>, PhoenixWsError> {
        let Some(auth_client) = auth_client else {
            return Ok(None);
        };

        auth_client
            .auth_session_for_request()
            .await
            .map_err(ws_auth_error)
    }

    fn connect_request(
        url: &Url,
        auth_session: Option<&AuthSession>,
    ) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request, PhoenixWsError> {
        let mut request = url.as_str().into_client_request()?;
        request.headers_mut().insert(
            "x-phoenix-client",
            HeaderValue::from_static(PHOENIX_CLIENT_HEADER_VALUE),
        );
        let Some(auth_session) = auth_session else {
            return Ok(request);
        };

        let bearer = format!("Bearer {}", auth_session.access_token());
        let protocols = format!("phoenix-jwt, {}", auth_session.access_token());
        request.headers_mut().insert(
            AUTHORIZATION,
            bearer
                .parse::<HeaderValue>()
                .map_err(|error| PhoenixWsError::InvalidHeaderValue(error.to_string()))?,
        );
        request.headers_mut().insert(
            SEC_WEBSOCKET_PROTOCOL,
            protocols
                .parse::<HeaderValue>()
                .map_err(|error| PhoenixWsError::InvalidHeaderValue(error.to_string()))?,
        );
        Ok(request)
    }

    fn close_reason(frame: Option<&CloseFrame>) -> String {
        frame
            .map(|frame| format!("closed: code={}, reason={}", frame.code, frame.reason))
            .unwrap_or_else(|| "closed without frame".to_string())
    }

    fn is_recoverable_auth_close(frame: Option<&CloseFrame>) -> bool {
        let Some(frame) = frame else {
            return false;
        };
        let code: u16 = frame.code.into();
        matches!(code, 4401 | 4403 | 1008)
            || matches!(
                frame.reason.as_str(),
                "access_token_expired"
                    | "invalid_access_token"
                    | "access_jti_mismatch"
                    | "session_missing"
                    | "authz_refresh_required"
            )
    }

    /// Broadcast a message to all subscribers for a given key.
    ///
    /// The `try_send` closure should attempt to send the message if the
    /// subscriber matches the expected variant, returning `true` if the
    /// send failed (channel closed).
    fn broadcast_to_subscribers<F>(
        subscribers: &HashMap<SubscriptionKey, HashMap<u64, Subscriber>>,
        key: &SubscriptionKey,
        try_send: F,
    ) where
        F: Fn(&Subscriber) -> bool,
    {
        if let Some(key_subscribers) = subscribers.get(&key.routing_key()) {
            for (id, subscriber) in key_subscribers {
                if try_send(subscriber) {
                    debug!("Subscriber {} channel closed for {:?}", id, key);
                }
            }
        }
    }

    /// Process an incoming WebSocket text or binary frame: route channel
    /// payloads to subscribers and apply subscription statuses and errors to
    /// the socket state.
    ///
    /// Returns an error when a status or error frame cannot be matched to
    /// the frames that caused it; the caller must then drop the socket.
    fn process_message(
        data: &[u8],
        state: &mut SocketState,
        subscription_event_tx: &mpsc::UnboundedSender<WsSubscriptionEvent>,
    ) -> Result<(), ResponseMismatch> {
        let text = match std::str::from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                debug!("Received non-UTF8 binary message: {:?}", e);
                return Ok(());
            }
        };
        debug!("Received message: {}", text);

        match serde_json::from_slice::<ServerMessage>(data) {
            Ok(ServerMessage::Error(err)) => {
                error!(
                    "Server error: code={}, error={}, subscription={:?}",
                    err.code, err.error, err.subscription
                );
                state.handle_error(err, subscription_event_tx)
            }
            Ok(ServerMessage::SubscriptionStatus(status)) => {
                debug!(
                    "Subscription status: status={}, subscription={:?}",
                    status.status, status.subscription
                );
                state.handle_status(status, subscription_event_tx)
            }
            Ok(message) => {
                Self::route_data_message(message, &state.subscribers);
                Ok(())
            }
            Err(e) => {
                debug!("Failed to parse message: {} - {}", e, text);
                Ok(())
            }
        }
    }

    /// Route a channel payload to its subscribers.
    fn route_data_message(
        message: ServerMessage,
        subscribers: &HashMap<SubscriptionKey, HashMap<u64, Subscriber>>,
    ) {
        match message {
            ServerMessage::AllMids(msg) => {
                let key = SubscriptionKey::all_mids();
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::AllMids(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::FundingRate(msg) => {
                let key = SubscriptionKey::funding_rate(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::FundingRate(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::Orderbook(msg) => {
                let key = SubscriptionKey::orderbook_with_options(
                    msg.symbol.clone(),
                    msg.bypass_execution_band,
                );
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::Orderbook(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::L2Book(msg) => {
                let key = SubscriptionKey::l2_book_from_message(&msg);
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::L2Book(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::MarkPrice(msg) => {
                let key = SubscriptionKey::mark_price(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::MarkPrice(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::Fills(msg) => {
                for key in [
                    SubscriptionKey::fills(Some(msg.symbol.clone())),
                    SubscriptionKey::fills(None),
                ] {
                    Self::broadcast_to_subscribers(
                        subscribers,
                        &key,
                        |sub| matches!(sub, Subscriber::Fills(tx) if tx.send(msg.clone()).is_err()),
                    );
                }
            }
            ServerMessage::TraderState(msg) => {
                let key = SubscriptionKey::TraderState {
                    authority: msg.authority.clone(),
                    trader_pda_index: msg.trader_pda_index,
                };
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::TraderState(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::Market(msg) => {
                let key = SubscriptionKey::market(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub: &Subscriber| matches!(sub, Subscriber::MarketStats(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::MarketStatsV2(msg) => {
                let key = SubscriptionKey::market_stats_v2(msg.symbols.clone());
                let legacy_update = msg.legacy_market_stats_update();
                Self::broadcast_to_subscribers(subscribers, &key, |sub| match sub {
                    Subscriber::MarketStatsV2(tx) => tx.send(msg.clone()).is_err(),
                    Subscriber::MarketStats(tx) => legacy_update
                        .as_ref()
                        .is_some_and(|update| tx.send(update.clone()).is_err()),
                    _ => false,
                });
            }
            ServerMessage::Trades(msg) => {
                let key = SubscriptionKey::trades(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::Trades(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::Candles(msg) => {
                let Some(timeframe) = msg.timeframe.parse().ok() else {
                    debug!(
                        "Failed to parse timeframe from candle message: {}",
                        msg.timeframe
                    );
                    return;
                };
                let key = SubscriptionKey::candles(msg.symbol.clone(), timeframe);
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::Candles(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::Exchange(msg) => {
                let key = SubscriptionKey::exchange();
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::Exchange(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            ServerMessage::Error(_)
            | ServerMessage::SubscriptionStatus(_)
            | ServerMessage::Other => {}
        }
    }
}

impl Drop for PhoenixWSClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn is_ws_auth_error(error: &TungsteniteError) -> bool {
    matches!(
        error,
        TungsteniteError::Http(response)
            if matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::CONFLICT
            )
    )
}

fn ws_auth_error(error: impl std::fmt::Display) -> PhoenixWsError {
    PhoenixWsError::Authentication(error.to_string())
}

fn validate_market_stats_v2_symbols(symbols: &Option<Vec<String>>) -> Result<(), PhoenixWsError> {
    let Some(symbols) = symbols else {
        return Ok(());
    };
    if symbols.is_empty() {
        return Err(PhoenixWsError::EmptyMarketStatsV2Symbols);
    }
    if symbols.iter().any(|symbol| symbol.trim().is_empty()) {
        return Err(PhoenixWsError::BlankMarketStatsV2Symbol);
    }
    Ok(())
}
#[cfg(feature = "opentelemetry")]
fn trace_parent_context(provider: Option<&TraceContextProvider>) -> Option<Context> {
    provider.map(|provider| provider())
}
#[cfg(feature = "opentelemetry")]
fn inject_trace_headers(span: &tracing::Span, headers: &mut HeaderMap) {
    inject_trace_headers_from_context(&span.context(), headers);
}
#[cfg(feature = "opentelemetry")]
fn inject_trace_headers_from_context(context: &Context, headers: &mut HeaderMap) {
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(context, &mut HeaderInjector(headers));
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    #[cfg(feature = "opentelemetry")]
    use std::sync::Arc as TelemetryArc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    #[cfg(feature = "opentelemetry")]
    use opentelemetry::trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState,
    };
    #[cfg(feature = "opentelemetry")]
    use opentelemetry::{Context, global};
    #[cfg(feature = "opentelemetry")]
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::mpsc::error::TryRecvError;
    use tokio::sync::{mpsc, oneshot};
    use tokio_tungstenite::accept_hdr_async;
    use tokio_tungstenite::tungstenite::Utf8Bytes;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::{
        Iana, Normal, Policy,
    };

    use super::*;
    use crate::auth::{AuthSessionStore, MemoryAuthSessionStore};

    const TEST_SYMBOL: &str = "SOL";
    const REFRESH_TOKEN: &str = "refresh-token";
    const FRESH_REFRESH_TOKEN: &str = "refresh-token-fresh";

    #[test]
    fn connect_request_includes_auth_headers_when_session_is_available() {
        let access_token = test_jwt("ws-jti");
        let session = AuthSession::from_tokens(
            access_token.clone(),
            Some(REFRESH_TOKEN.to_string()),
            URL_SAFE_NO_PAD.encode(b"pop-key"),
            Some(300),
            Some(600),
        )
        .expect("session should parse");
        let url = Url::parse("wss://api.example.test/ws").expect("url should parse");

        let request =
            PhoenixWSClient::connect_request(&url, Some(&session)).expect("request should build");

        assert_eq!(
            request.headers().get(AUTHORIZATION).unwrap(),
            &bearer(&access_token)
        );
        assert_eq!(
            request.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(),
            &ws_protocol(&access_token)
        );
    }

    #[test]
    fn connect_request_omits_auth_headers_without_session() {
        let url = Url::parse("wss://api.example.test/ws").expect("url should parse");

        let request = PhoenixWSClient::connect_request(&url, None).expect("request should build");

        assert!(request.headers().get(AUTHORIZATION).is_none());
        assert!(request.headers().get(SEC_WEBSOCKET_PROTOCOL).is_none());
        assert_eq!(
            request.headers().get("x-phoenix-client").unwrap(),
            PHOENIX_CLIENT_HEADER_VALUE
        );
    }

    #[test]
    fn market_subscription_keys_route_all_case_variants() {
        let expected = SubscriptionKey::market("kBONK".to_string()).routing_key();
        for symbol in ["kBONK", "kbonk", "KBONK"] {
            assert_eq!(
                SubscriptionKey::market(symbol.to_string()).routing_key(),
                expected
            );
        }
    }

    #[test]
    fn market_stats_v2_symbols_preserve_wire_case() {
        let key = SubscriptionKey::market_stats_v2(Some(vec![
            " kBONK ".to_string(),
            "SOL-PERP".to_string(),
        ]));
        assert_eq!(
            key.market_stats_v2_symbols(),
            Some(["SOL-PERP".to_string(), "kBONK".to_string()].as_slice())
        );
        assert_eq!(
            key.routing_key(),
            SubscriptionKey::market_stats_v2(Some(vec![
                "kbonk".to_string(),
                "sol-perp".to_string(),
            ]))
        );
        assert!(matches!(
            validate_market_stats_v2_symbols(&Some(Vec::new())),
            Err(PhoenixWsError::EmptyMarketStatsV2Symbols)
        ));
        assert!(validate_market_stats_v2_symbols(&None).is_ok());
    }

    #[test]
    fn process_message_forwards_subscription_status_event() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut state = SocketState::default();
        let json = br#"{
            "channel": "subscriptionStatus",
            "status": "subscribed",
            "clientId": "client-1",
            "subscription": {
                "channel": "traderState",
                "authority": "ABC123",
                "traderPdaIndex": 0
            }
        }"#;

        PhoenixWSClient::process_message(json, &mut state, &tx).expect("status should apply");

        let event = rx.try_recv().expect("subscription event should be sent");
        assert!(matches!(
            event,
            WsSubscriptionEvent::Status {
                status,
                subscription: SubscriptionRequest::TraderState(_),
            } if status == "subscribed"
        ));
    }

    #[test]
    fn trader_subscription_delivers_known_deltas_after_unknown_messages() {
        use phoenix_rise_types::prelude::TraderStateEscrowAction;

        let (tx, mut rx) = mpsc::unbounded_channel();
        let subscribers = HashMap::from([(
            SubscriptionKey::TraderState {
                authority: "authority".to_string(),
                trader_pda_index: 0,
            },
            HashMap::from([(1, Subscriber::TraderState(tx))]),
        )]);
        let (event_tx, mut events) = mpsc::unbounded_channel();
        let mut state = SocketState {
            subscribers,
            ..SocketState::default()
        };
        for message in [
            json!({"channel": "futureChannel"}),
            json!({"channel": "traderState", "messageType": "futureMessage"}),
            json!({
                "channel": "traderState", "messageType": "delta",
                "authority": "authority", "traderPdaIndex": 0, "slot": 11,
                "deltas": [{
                    "subaccountIndex": 0, "sequence": 2, "collateral": "105",
                    "futureField": true,
                    "escrowRequests": [{
                        "sequenceNumber": "7", "senderAuthority": "sender",
                        "senderPdaIndex": 0, "senderSubaccountIndex": 0,
                        "expirationOffset": null, "initialSlot": "10",
                        "expirationSlot": null,
                        "actions": [{"kind": "futureAction"}, {"kind": "cash", "amount": "5"}]
                    }]
                }]
            }),
        ] {
            PhoenixWSClient::process_message(
                &serde_json::to_vec(&message).unwrap(),
                &mut state,
                &event_tx,
            )
            .expect("unknown messages are tolerated");
        }
        let mut trader = crate::trader_state::Trader::new(crate::trader_key::TraderKey::new(
            Pubkey::new_unique(),
        ));
        trader.apply_update(&rx.try_recv().expect("known delta should be delivered"));
        let subaccount = &trader.subaccounts[&0];
        assert_eq!(subaccount.sequence, 2);
        assert_eq!(
            subaccount.collateral,
            phoenix_rise_math::SignedQuoteLots::new(105)
        );
        assert_eq!(
            subaccount.escrow_requests[0].actions,
            vec![
                TraderStateEscrowAction::Unknown,
                TraderStateEscrowAction::Cash { amount: 5.into() },
            ]
        );
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        assert!(matches!(
            events.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn process_message_routes_market_stats_v2_by_exact_filter() {
        let (filtered_tx, mut filtered_rx) = mpsc::unbounded_channel();
        let (legacy_tx, mut legacy_rx) = mpsc::unbounded_channel();
        let (all_tx, mut all_rx) = mpsc::unbounded_channel();
        let mut subscribers = HashMap::new();
        subscribers.insert(
            SubscriptionKey::market_stats_v2(Some(vec!["KBONK".to_string()])).routing_key(),
            HashMap::from([
                (1, Subscriber::MarketStatsV2(filtered_tx)),
                (3, Subscriber::MarketStats(legacy_tx)),
            ]),
        );
        subscribers.insert(
            SubscriptionKey::market_stats_v2(None),
            HashMap::from([(2, Subscriber::MarketStatsV2(all_tx))]),
        );
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let json = serde_json::to_vec(&json!({
            "channel": "marketStatsV2",
            "symbols": ["kBONK"],
            "stats": [{
                "symbol": "kBONK",
                "timestamp": 1,
                "openInterest": 2.0,
                "markPrice": 3.0,
                "midPrice": 3.5,
                "oraclePrice": 4.0,
                "prevDayMarkPrice": 5.0,
                "dayVolumeUsd": 6.0,
                "dayVolumeBase": 7.0,
                "currentFundingRate": 8.0,
                "eightHourFundingRate": 9.0,
                "annualizedFundingRate": 10.0
            }]
        }))
        .unwrap();

        let mut state = SocketState {
            subscribers,
            ..SocketState::default()
        };
        PhoenixWSClient::process_message(&json, &mut state, &event_tx).expect("data frame");

        let update = filtered_rx
            .try_recv()
            .expect("filtered subscriber should receive the batch");
        assert_eq!(update.stats[0].symbol, "kBONK");
        let legacy_update = legacy_rx
            .try_recv()
            .expect("legacy subscriber should receive the adapted V2 entry");
        assert_eq!(legacy_update.symbol, "kBONK");
        assert_eq!(legacy_update.mark_price, 3.0);
        assert_eq!(legacy_update.mid_price, Some(3.5));
        assert!(all_rx.try_recv().is_err());
    }

    #[test]
    fn process_message_routes_l2_book_and_mark_price_case_insensitively() {
        let (book_tx, mut book_rx) = mpsc::unbounded_channel();
        let (mark_tx, mut mark_rx) = mpsc::unbounded_channel();
        let subscribers = HashMap::from([
            (
                SubscriptionKey::l2_book("KBONK".to_string()).routing_key(),
                HashMap::from([(1, Subscriber::L2Book(book_tx))]),
            ),
            (
                SubscriptionKey::mark_price("KBONK".to_string()).routing_key(),
                HashMap::from([(2, Subscriber::MarkPrice(mark_tx))]),
            ),
        ]);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let book = serde_json::to_vec(&json!({
            "channel": "l2Book",
            "coin": "kBONK",
            "timestamp": 1,
            "slot": 42,
            "slotIndex": 3,
            "bids": [[1.5, 10.0]],
            "asks": [[1.6, 20.0]]
        }))
        .unwrap();
        let mark = serde_json::to_vec(&json!({
            "channel": "markPrice",
            "slot": 43,
            "slotIndex": 7,
            "symbol": "kBONK",
            "markPrice": 1.55
        }))
        .unwrap();

        let mut state = SocketState {
            subscribers,
            ..SocketState::default()
        };
        PhoenixWSClient::process_message(&book, &mut state, &event_tx).expect("l2Book data frame");
        PhoenixWSClient::process_message(&mark, &mut state, &event_tx)
            .expect("markPrice data frame");

        let book_update = book_rx
            .try_recv()
            .expect("l2Book subscriber should receive the snapshot");
        assert_eq!(book_update.slot, 42);
        assert_eq!(book_update.slot_index, Some(3));
        assert_eq!(book_update.asks, vec![(1.6, 20.0)]);
        let mark_update = mark_rx
            .try_recv()
            .expect("markPrice subscriber should receive the update");
        assert_eq!(mark_update.slot_index, 7);
        assert_eq!(mark_update.mark_price, 1.55);
        assert!(book_rx.try_recv().is_err());
    }

    #[test]
    fn process_message_routes_fills_to_market_and_all_market_subscribers() {
        let (bonk_tx, mut bonk_rx) = mpsc::unbounded_channel();
        let (sol_tx, mut sol_rx) = mpsc::unbounded_channel();
        let (all_tx, mut all_rx) = mpsc::unbounded_channel();
        let subscribers = HashMap::from([
            (
                SubscriptionKey::fills(Some("KBONK".to_string())).routing_key(),
                HashMap::from([(1, Subscriber::Fills(bonk_tx))]),
            ),
            (
                SubscriptionKey::fills(Some("SOL".to_string())).routing_key(),
                HashMap::from([(2, Subscriber::Fills(sol_tx))]),
            ),
            (
                SubscriptionKey::fills(None).routing_key(),
                HashMap::from([(3, Subscriber::Fills(all_tx))]),
            ),
        ]);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let fills = serde_json::to_vec(&json!({
            "channel": "fills",
            "symbol": "kBONK",
            "fills": [{
                "marketSymbol": "kBONK",
                "baseQty": "10",
                "quoteQty": "15.5",
                "price": "1.55",
                "timestamp": "2026-10-29T18:30:25Z",
                "transactionSignature": "sig",
                "instructionType": "PlaceMarketOrder"
            }]
        }))
        .unwrap();

        let mut state = SocketState {
            subscribers,
            ..SocketState::default()
        };
        PhoenixWSClient::process_message(&fills, &mut state, &event_tx).expect("fills data frame");

        let bonk = bonk_rx
            .try_recv()
            .expect("market subscriber should receive kBONK fills");
        assert_eq!(bonk.fills[0].price, "1.55");
        assert_eq!(
            all_rx
                .try_recv()
                .expect("all-markets subscriber should receive kBONK fills"),
            bonk
        );
        assert!(sol_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn connect_refreshes_near_expiry_access_before_handshake() {
        let stale_token = test_jwt_with_exp("stale-ws-jti", future_unix_secs(30));
        let fresh_token = test_jwt_with_exp("fresh-ws-jti", future_unix_secs(600));
        let (refresh_url, refresh_auth_rx, refresh_count) =
            spawn_refresh_server(fresh_token.clone()).await;
        let (ws_url, mut ws_rx) = spawn_recording_ws_server(None).await;
        let auth_client = test_auth_client(
            refresh_url,
            test_session_store(
                stale_token.clone(),
                Some(REFRESH_TOKEN.to_string()),
                Some(30),
            ),
        );
        let url = Url::parse(&ws_url).expect("ws url should parse");

        let mut stream = PhoenixWSClient::connect(&url, Some(&auth_client), None)
            .await
            .expect("websocket should connect");
        let _ = stream.close(None).await;

        let observed = timeout_recv(&mut ws_rx).await;
        assert_eq!(observed.authorization, Some(bearer(&fresh_token)));
        assert_eq!(observed.protocol, Some(ws_protocol(&fresh_token)));
        assert_eq!(refresh_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            refresh_auth_rx
                .await
                .expect("refresh auth should be recorded"),
            Some(bearer(&stale_token))
        );
    }

    #[tokio::test]
    async fn connect_omits_expired_access_when_refresh_is_unavailable() {
        let expired_token = test_jwt_with_exp("expired-ws-jti", past_unix_secs(30));
        let (ws_url, mut ws_rx) = spawn_recording_ws_server(None).await;
        let auth_client = test_auth_client(
            "http://127.0.0.1:1",
            test_session_store(expired_token, None, None),
        );
        let url = Url::parse(&ws_url).expect("ws url should parse");

        let mut stream = PhoenixWSClient::connect(&url, Some(&auth_client), None)
            .await
            .expect("websocket should connect anonymously");
        let _ = stream.close(None).await;

        let observed = timeout_recv(&mut ws_rx).await;
        assert_eq!(observed.authorization, None);
        assert_eq!(observed.protocol, None);
    }

    #[tokio::test]
    async fn connect_sends_external_bearer_token() {
        let (ws_url, mut ws_rx) = spawn_recording_ws_server(None).await;
        let bearer_manager = crate::auth::ExternalBearer::new();
        bearer_manager
            .set_access_token("opaque-external-token".to_string())
            .expect("token should install");
        let auth_client = PhoenixHttpClient::builder("http://127.0.0.1:1")
            .with_session_manager(bearer_manager)
            .build()
            .expect("auth client should build");
        let url = Url::parse(&ws_url).expect("ws url should parse");

        let mut stream = PhoenixWSClient::connect(&url, Some(&auth_client), None)
            .await
            .expect("websocket should connect");
        let _ = stream.close(None).await;

        let observed = timeout_recv(&mut ws_rx).await;
        assert_eq!(
            observed.authorization,
            Some(bearer("opaque-external-token"))
        );
        assert_eq!(
            observed.protocol,
            Some(ws_protocol("opaque-external-token"))
        );
    }

    #[tokio::test]
    async fn connect_anonymously_with_unset_or_cleared_external_bearer() {
        for clear_installed_token in [false, true] {
            let (ws_url, mut ws_rx) = spawn_recording_ws_server(None).await;
            let manager = crate::auth::ExternalBearer::new();
            let auth_client = PhoenixHttpClient::builder("http://127.0.0.1:1")
                .with_session_manager(manager.clone())
                .build()
                .expect("auth client should build");
            if clear_installed_token {
                manager
                    .set_access_token("opaque-external-token".to_string())
                    .expect("token should install");
                manager.clear();
            }
            let url = Url::parse(&ws_url).expect("ws url should parse");

            let mut stream = PhoenixWSClient::connect(&url, Some(&auth_client), None)
                .await
                .expect("websocket should connect anonymously");
            let _ = stream.close(None).await;

            let observed = timeout_recv(&mut ws_rx).await;
            assert_eq!(observed.authorization, None);
            assert_eq!(observed.protocol, None);
            assert!(auth_client.auth_lifecycle_last_error().is_none());
        }
    }

    #[tokio::test]
    async fn connect_sends_external_bearer_even_after_local_expiry_without_lifecycle_error() {
        for exp in [future_unix_secs(30), past_unix_secs(30)] {
            let token = test_jwt_with_exp("external-ws-jti", exp);
            let (ws_url, mut ws_rx) = spawn_recording_ws_server(None).await;
            let manager = crate::auth::ExternalBearer::new();
            manager
                .set_access_token(token.clone())
                .expect("token should install");
            let auth_client = PhoenixHttpClient::builder("http://127.0.0.1:1")
                .with_session_manager(manager)
                .build()
                .expect("auth client should build");
            let url = Url::parse(&ws_url).expect("ws url should parse");

            let mut stream = PhoenixWSClient::connect(&url, Some(&auth_client), None)
                .await
                .expect("websocket should connect without refreshing the external token");
            let _ = stream.close(None).await;

            let observed = timeout_recv(&mut ws_rx).await;
            assert_eq!(observed.authorization, Some(bearer(&token)));
            assert_eq!(observed.protocol, Some(ws_protocol(&token)));
            assert!(auth_client.auth_lifecycle_last_error().is_none());
        }
    }

    #[tokio::test]
    async fn auth_close_refreshes_reconnects_and_resubscribes() {
        let stale_token = test_jwt_with_exp("stale-close-jti", future_unix_secs(600));
        let fresh_token = test_jwt_with_exp("fresh-close-jti", future_unix_secs(600));
        let (refresh_url, _refresh_auth_rx, refresh_count) =
            spawn_refresh_server(fresh_token.clone()).await;
        let (ws_url, mut ws_rx) =
            spawn_auth_close_then_reconnect_ws_server(Iana(4401), "access_token_expired").await;
        let auth_client = test_auth_client(
            refresh_url,
            test_session_store(
                stale_token.clone(),
                Some(REFRESH_TOKEN.to_string()),
                Some(600),
            ),
        );
        let mut client = PhoenixWSClient::builder(&ws_url)
            .with_connection_status()
            .with_auth(auth_client)
            .build()
            .expect("ws client should build");
        let mut status_rx = client
            .connection_status_receiver()
            .expect("status receiver should exist");
        let (_rx, _handle) = client
            .subscribe_to_trades(TEST_SYMBOL.to_string())
            .expect("subscribe should succeed");

        let first = timeout_recv(&mut ws_rx).await;
        let second = timeout_recv(&mut ws_rx).await;
        assert_eq!(first.authorization, Some(bearer(&stale_token)));
        assert!(first.subscription_seen);
        assert_eq!(second.authorization, Some(bearer(&fresh_token)));
        assert!(second.subscription_seen);
        assert_eq!(refresh_count.load(AtomicOrdering::SeqCst), 1);
        assert_status(&mut status_rx, WsConnectionStatus::Connecting).await;
        assert_status(&mut status_rx, WsConnectionStatus::Connected).await;
        assert_status(&mut status_rx, WsConnectionStatus::Connecting).await;
        assert_status(&mut status_rx, WsConnectionStatus::Connected).await;
    }

    #[tokio::test]
    async fn normal_close_does_not_refresh() {
        let token = test_jwt_with_exp("normal-close-jti", future_unix_secs(600));
        let (refresh_url, _refresh_auth_rx, refresh_count) =
            spawn_refresh_server(test_jwt_with_exp("unused-fresh-jti", future_unix_secs(600)))
                .await;
        let (ws_url, mut ws_rx) = spawn_close_after_subscription_ws_server(Normal, "normal").await;
        let auth_client = test_auth_client(
            refresh_url,
            test_session_store(token, Some(REFRESH_TOKEN.to_string()), Some(600)),
        );
        let mut client = PhoenixWSClient::builder(&ws_url)
            .with_connection_status()
            .with_auth(auth_client)
            .build()
            .expect("ws client should build");
        let mut status_rx = client
            .connection_status_receiver()
            .expect("status receiver should exist");
        let (_rx, _handle) = client
            .subscribe_to_trades(TEST_SYMBOL.to_string())
            .expect("subscribe should succeed");

        let observed = timeout_recv(&mut ws_rx).await;
        assert!(observed.subscription_seen);
        assert_status(&mut status_rx, WsConnectionStatus::Connecting).await;
        assert_status(&mut status_rx, WsConnectionStatus::Connected).await;
        assert_disconnected_contains(&mut status_rx, "normal").await;
        assert_eq!(refresh_count.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn recoverable_auth_close_matches_ts_policy() {
        assert!(PhoenixWSClient::is_recoverable_auth_close(Some(
            &close_frame(Iana(4401), "any")
        )));
        assert!(PhoenixWSClient::is_recoverable_auth_close(Some(
            &close_frame(Policy, "any")
        )));
        assert!(PhoenixWSClient::is_recoverable_auth_close(Some(
            &close_frame(Normal, "invalid_access_token")
        )));
        assert!(!PhoenixWSClient::is_recoverable_auth_close(Some(
            &close_frame(Normal, "normal")
        )));
    }

    #[tokio::test]
    async fn exchange_store_populates_from_first_snapshot_and_memoizes() {
        let (ws_url, feed_tx) = spawn_exchange_feed_ws_server().await;
        let client = PhoenixWSClient::new(&ws_url).expect("ws client should build");
        let root_authority = Pubkey::new_unique();
        feed_tx
            .send(exchange_snapshot_json(10, &root_authority))
            .expect("feed channel should accept the snapshot");

        let store = tokio::time::timeout(Duration::from_secs(5), client.exchange_store())
            .await
            .expect("exchange_store should resolve once the snapshot arrives")
            .expect("exchange_store should succeed");

        assert!(store.is_populated());
        assert_eq!(store.root_authority(), Some(root_authority));

        // A second call returns the SAME store (one subscription total).
        let second = tokio::time::timeout(Duration::from_secs(5), client.exchange_store())
            .await
            .expect("memoized exchange_store should resolve immediately")
            .expect("memoized exchange_store should succeed");
        assert!(store.ptr_eq(&second));
    }

    #[tokio::test]
    async fn exchange_store_keeps_receiving_updates_after_unknown_messages() {
        let (ws_url, feed_tx) = spawn_exchange_feed_ws_server().await;
        let mut client = PhoenixWSClient::new(&ws_url).expect("ws client should build");
        let initial_root = Pubkey::new_unique();
        let rotated_root = Pubkey::new_unique();
        feed_tx
            .send(exchange_snapshot_json(10, &initial_root))
            .expect("feed channel should accept the snapshot");

        let store = tokio::time::timeout(Duration::from_secs(5), client.exchange_store())
            .await
            .expect("exchange_store should resolve once the snapshot arrives")
            .expect("exchange_store should succeed");
        assert_eq!(store.root_authority(), Some(initial_root));

        let mut events = client.subscription_event_receiver().unwrap();
        for message in [
            json!({"channel": "futureChannel", "payload": true}),
            json!({"channel": "exchange", "messageType": "futureMessage"}),
            json!({"channel": "traderState", "messageType": "futureMessage"}),
        ] {
            feed_tx
                .send(message.to_string())
                .expect("feed should stay open");
        }
        let mut delta: serde_json::Value =
            serde_json::from_str(&exchange_keys_delta_json(11, &rotated_root)).unwrap();
        delta["ops"]
            .as_array_mut()
            .unwrap()
            .insert(0, json!({"kind": "futureDelta", "value": 42}));
        feed_tx
            .send(delta.to_string())
            .expect("feed channel should accept the delta");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if store.root_authority() == Some(rotated_root) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("store should observe the rotated root authority");
        assert!(matches!(
            events.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    fn test_exchange_state_snapshot(
        root_authority: &Pubkey,
    ) -> phoenix_rise_types::prelude::ExchangeStateSnapshot {
        phoenix_rise_types::prelude::ExchangeStateSnapshot {
            program_id: "program".to_string(),
            global_config: "global-config".to_string(),
            current_authorities: phoenix_rise_types::prelude::AuthoritySet {
                root_authority: root_authority.to_string(),
                risk_authority: "risk".to_string(),
                market_authority: "market".to_string(),
                oracle_authority: "oracle".to_string(),
                adl_authority: "adl".to_string(),
                cancel_authority: "cancel".to_string(),
                backstop_authority: "backstop".to_string(),
            },
            canonical_mint: "canonical".to_string(),
            usdc_mint: "usdc".to_string(),
            global_vault: "vault".to_string(),
            perp_asset_map: "perp-map".to_string(),
            global_trader_index: vec!["gti-0".to_string()],
            active_trader_buffer: vec!["atb-0".to_string()],
            withdraw_queue: "withdraw-queue".to_string(),
            pending_withdraw_throttle_replenishment: 0_u64.into(),
            exchange_status_bits: 129,
            exchange_status_features: vec!["initialized".to_string(), "active".to_string()],
            running_state: phoenix_rise_types::prelude::ExchangeRunningState::Active,
            active: true,
            gated: false,
            withdrawals_available: true,
        }
    }

    fn exchange_snapshot_json(sequence_number: u64, root_authority: &Pubkey) -> String {
        let message = ServerMessage::Exchange(ExchangeMessage::Snapshot(Box::new(
            phoenix_rise_types::prelude::ExchangeSnapshotMessage {
                version: 1,
                sequence_number: sequence_number.into(),
                slot: 1,
                slot_index: 0,
                reason: phoenix_rise_types::exchange_ws::ExchangeSnapshotReason::Snapshot,
                exchange: test_exchange_state_snapshot(root_authority),
                markets: Vec::new(),
                spot_collaterals: Vec::new(),
            },
        )));
        serde_json::to_string(&message).expect("snapshot message should serialize")
    }

    fn exchange_keys_delta_json(sequence_number: u64, root_authority: &Pubkey) -> String {
        let message = ServerMessage::Exchange(ExchangeMessage::Delta(
            phoenix_rise_types::prelude::ExchangeDeltaMessage {
                version: 1,
                sequence_number: sequence_number.into(),
                slot: 2,
                slot_index: 0,
                ops: vec![
                    phoenix_rise_types::prelude::ExchangeDeltaOp::ExchangeKeysUpdated {
                        exchange: test_exchange_state_snapshot(root_authority),
                    },
                ],
            },
        ));
        serde_json::to_string(&message).expect("delta message should serialize")
    }

    /// WebSocket server that accepts one connection, waits for the client's
    /// exchange subscribe message, then forwards every JSON string pushed to
    /// the returned sender as a text frame.
    async fn spawn_exchange_feed_ws_server() -> (String, mpsc::UnboundedSender<String>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let addr = listener.local_addr().expect("ws addr should exist");
        let (feed_tx, mut feed_rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(mut websocket) = tokio_tungstenite::accept_async(stream).await else {
                return;
            };
            // Wait for the exchange subscribe before feeding messages.
            if websocket.next().await.is_none() {
                return;
            }
            while let Some(json) = feed_rx.recv().await {
                if websocket
                    .send(Message::Text(Utf8Bytes::from(json)))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
        (format!("ws://{addr}/v1/ws"), feed_tx)
    }

    #[cfg(feature = "opentelemetry")]
    #[test]
    fn inject_trace_headers_from_context_includes_traceparent() {
        global::set_text_map_propagator(TraceContextPropagator::new());
        let parent_context = Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").expect("valid trace id"),
            SpanId::from_hex("00f067aa0ba902b7").expect("valid span id"),
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        ));
        let mut headers = HeaderMap::new();

        inject_trace_headers_from_context(&parent_context, &mut headers);

        let traceparent = headers
            .get("traceparent")
            .and_then(|value| value.to_str().ok())
            .expect("traceparent header should be present");
        assert!(traceparent.contains("4bf92f3577b34da6a3ce929d0e0e4736"));
    }
    #[cfg(feature = "opentelemetry")]
    #[test]
    fn trace_parent_context_uses_provider() {
        global::set_text_map_propagator(TraceContextPropagator::new());
        let parent_context = Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").expect("valid trace id"),
            SpanId::from_hex("00f067aa0ba902b7").expect("valid span id"),
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        ));
        let provider_context = parent_context.clone();
        let provider: TraceContextProvider = TelemetryArc::new(move || provider_context.clone());

        let provided_context =
            trace_parent_context(Some(&provider)).expect("provider should return context");
        let mut headers = HeaderMap::new();
        inject_trace_headers_from_context(&provided_context, &mut headers);

        let traceparent = headers
            .get("traceparent")
            .and_then(|value| value.to_str().ok())
            .expect("traceparent header should be present");
        assert!(traceparent.contains("4bf92f3577b34da6a3ce929d0e0e4736"));
    }

    fn test_jwt(jti: &str) -> String {
        test_jwt_with_exp(jti, future_unix_secs(600))
    }

    fn test_jwt_with_exp(jti: &str, exp: u64) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"jti":"{jti}","exp":{exp}}}"#));
        format!("{header}.{payload}.sig")
    }

    fn test_session_store(
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<u64>,
    ) -> Arc<MemoryAuthSessionStore> {
        let store = Arc::new(MemoryAuthSessionStore::new());
        store
            .store_session(
                &AuthSession::from_tokens(
                    access_token,
                    refresh_token,
                    URL_SAFE_NO_PAD.encode(b"pop-key"),
                    expires_in,
                    Some(600),
                )
                .expect("session should parse"),
            )
            .expect("session should store");
        store
    }

    fn test_auth_client(
        api_url: impl Into<String>,
        store: Arc<MemoryAuthSessionStore>,
    ) -> PhoenixHttpClient {
        PhoenixHttpClient::builder(api_url)
            .with_auth_session_store(store)
            .build()
            .expect("auth client should build")
    }

    fn bearer(token: &str) -> String {
        format!("Bearer {token}")
    }

    fn ws_protocol(token: &str) -> String {
        format!("phoenix-jwt, {token}")
    }

    fn future_unix_secs(offset_secs: u64) -> u64 {
        SystemTime::now()
            .checked_add(Duration::from_secs(offset_secs))
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .expect("future timestamp should be after epoch")
            .as_secs()
    }

    fn past_unix_secs(offset_secs: u64) -> u64 {
        SystemTime::now()
            .checked_sub(Duration::from_secs(offset_secs))
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .expect("past timestamp should be after epoch")
            .as_secs()
    }

    fn close_frame(code: CloseCode, reason: &str) -> CloseFrame {
        CloseFrame {
            code,
            reason: Utf8Bytes::from(reason),
        }
    }

    #[derive(Debug)]
    struct WsObservation {
        authorization: Option<String>,
        protocol: Option<String>,
        subscription_seen: bool,
    }

    async fn spawn_refresh_server(
        access_token: String,
    ) -> (String, oneshot::Receiver<Option<String>>, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("refresh listener should bind");
        let addr = listener.local_addr().expect("refresh addr should exist");
        let refresh_count = Arc::new(AtomicUsize::new(0));
        let refresh_count_task = refresh_count.clone();
        let (auth_tx, auth_rx) = oneshot::channel();
        tokio::spawn(async move {
            let mut auth_tx = Some(auth_tx);
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let access_token = access_token.clone();
                let mut buffer = vec![0; 8192];
                let Ok(n) = socket.read(&mut buffer).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buffer[..n]);
                let authorization = request.lines().find_map(|line| {
                    let (name, value) = line.split_once(": ")?;
                    name.eq_ignore_ascii_case("authorization")
                        .then(|| value.to_string())
                });
                refresh_count_task.fetch_add(1, AtomicOrdering::SeqCst);
                if let Some(tx) = auth_tx.take() {
                    let _ = tx.send(authorization);
                }
                let body = json!({
                    "token_type": "Bearer",
                    "access_token": access_token,
                    "expires_in": 900,
                    "refresh_token": FRESH_REFRESH_TOKEN,
                    "refresh_expires_in": 3600,
                    "pop_key": URL_SAFE_NO_PAD.encode(b"fresh-pop-key"),
                })
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: \
                     {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        (format!("http://{addr}"), auth_rx, refresh_count)
    }

    async fn spawn_recording_ws_server(
        close: Option<CloseFrame>,
    ) -> (String, mpsc::UnboundedReceiver<WsObservation>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let addr = listener.local_addr().expect("ws addr should exist");
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let tx_headers = tx.clone();
                let callback = move |request: &Request, mut response: Response| {
                    let _ = tx_headers.send(observation_from_request(request, false));
                    select_ws_protocol(request, &mut response);
                    Ok(response)
                };
                if let Ok(mut websocket) = accept_hdr_async(stream, callback).await {
                    if let Some(frame) = close {
                        let _ = websocket.close(Some(frame)).await;
                    } else {
                        let _ = websocket.next().await;
                    }
                }
            }
        });
        (format!("ws://{addr}/v1/ws"), rx)
    }

    async fn spawn_auth_close_then_reconnect_ws_server(
        close_code: CloseCode,
        close_reason: &'static str,
    ) -> (String, mpsc::UnboundedReceiver<WsObservation>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let addr = listener.local_addr().expect("ws addr should exist");
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            handle_ws_connection(
                &listener,
                tx.clone(),
                Some(close_frame(close_code, close_reason)),
            )
            .await;
            handle_ws_connection(&listener, tx, None).await;
        });
        (format!("ws://{addr}/v1/ws"), rx)
    }

    async fn spawn_close_after_subscription_ws_server(
        close_code: CloseCode,
        close_reason: &'static str,
    ) -> (String, mpsc::UnboundedReceiver<WsObservation>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let addr = listener.local_addr().expect("ws addr should exist");
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            handle_ws_connection(&listener, tx, Some(close_frame(close_code, close_reason))).await;
        });
        (format!("ws://{addr}/v1/ws"), rx)
    }

    async fn handle_ws_connection(
        listener: &TcpListener,
        tx: mpsc::UnboundedSender<WsObservation>,
        close: Option<CloseFrame>,
    ) {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let (headers_tx, mut headers_rx) = mpsc::unbounded_channel();
        let callback = move |request: &Request, mut response: Response| {
            let _ = headers_tx.send(observation_from_request(request, false));
            select_ws_protocol(request, &mut response);
            Ok(response)
        };
        let Ok(mut websocket) = accept_hdr_async(stream, callback).await else {
            return;
        };
        let Some(mut observation) = headers_rx.recv().await else {
            return;
        };
        observation.subscription_seen = websocket.next().await.is_some();
        let _ = tx.send(observation);
        if let Some(frame) = close {
            let _ = websocket.close(Some(frame)).await;
        }
    }

    fn observation_from_request(request: &Request, subscription_seen: bool) -> WsObservation {
        WsObservation {
            authorization: request
                .headers()
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
            protocol: request
                .headers()
                .get(SEC_WEBSOCKET_PROTOCOL)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
            subscription_seen,
        }
    }

    fn select_ws_protocol(request: &Request, response: &mut Response) {
        let Some(protocol) = request
            .headers()
            .get(SEC_WEBSOCKET_PROTOCOL)
            .and_then(|value| value.to_str().ok())
        else {
            return;
        };
        if protocol
            .split(',')
            .map(str::trim)
            .any(|entry| entry.eq_ignore_ascii_case("phoenix-jwt"))
        {
            response.headers_mut().insert(
                SEC_WEBSOCKET_PROTOCOL,
                HeaderValue::from_static("phoenix-jwt"),
            );
        }
    }

    async fn timeout_recv<T>(rx: &mut mpsc::UnboundedReceiver<T>) -> T {
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("channel receive should not time out")
            .expect("channel should remain open")
    }

    async fn assert_status(
        rx: &mut mpsc::UnboundedReceiver<WsConnectionStatus>,
        expected: WsConnectionStatus,
    ) {
        let actual = timeout_recv(rx).await;
        assert_eq!(actual, expected);
    }

    async fn assert_disconnected_contains(
        rx: &mut mpsc::UnboundedReceiver<WsConnectionStatus>,
        expected_reason: &str,
    ) {
        let actual = timeout_recv(rx).await;
        let WsConnectionStatus::Disconnected(reason) = actual else {
            panic!("expected disconnected status, got {actual:?}");
        };
        assert!(
            reason.contains(expected_reason),
            "expected disconnected reason {reason:?} to contain {expected_reason:?}"
        );
    }

    // ------------------------------------------------------------------
    // Heartbeat
    // ------------------------------------------------------------------

    const FAST_HEARTBEAT: WsHeartbeatConfig = WsHeartbeatConfig {
        ping_interval: Some(Duration::from_millis(100)),
        idle_timeout: Some(Duration::from_millis(300)),
    };

    /// Server that completes the handshake, then never reads or writes.
    async fn spawn_silent_ws_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let url = format!("ws://{}", listener.local_addr().expect("ws addr"));
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let _ws = tokio_tungstenite::accept_async(stream)
                .await
                .expect("handshake");
            std::future::pending::<()>().await;
        });
        url
    }

    fn client_with_heartbeat(url: &str, heartbeat: WsHeartbeatConfig) -> PhoenixWSClient {
        PhoenixWSClient::builder(url)
            .with_connection_status()
            .with_heartbeat(heartbeat)
            .build()
            .expect("ws client should build")
    }

    async fn assert_connected(status: &mut mpsc::UnboundedReceiver<WsConnectionStatus>) {
        assert_status(status, WsConnectionStatus::Connecting).await;
        assert_status(status, WsConnectionStatus::Connected).await;
    }

    async fn assert_no_status_for(
        status: &mut mpsc::UnboundedReceiver<WsConnectionStatus>,
        duration: Duration,
    ) {
        if let Ok(next) = tokio::time::timeout(duration, status.recv()).await {
            panic!("expected no status change, got {next:?}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn silent_socket_is_reported_disconnected() {
        let url = spawn_silent_ws_server().await;
        let mut client = PhoenixWSClient::new_with_connection_status(&url).unwrap();
        let mut status = client.connection_status_receiver().unwrap();
        let (_rx, _h) = client
            .subscribe_to_trader_state(&Pubkey::new_unique())
            .unwrap();
        assert_eq!(status.recv().await, Some(WsConnectionStatus::Connecting));
        assert_eq!(status.recv().await, Some(WsConnectionStatus::Connected));
        // Default heartbeat: the idle timeout is 90 s of (paused) time.
        let next = tokio::time::timeout(Duration::from_secs(120), status.recv()).await;
        assert!(matches!(
            next,
            Ok(Some(WsConnectionStatus::Disconnected(_)))
        ));
    }

    #[tokio::test]
    async fn idle_timeout_disconnects_and_closes_receivers() {
        let url = spawn_silent_ws_server().await;
        let mut client = client_with_heartbeat(&url, FAST_HEARTBEAT);
        let mut status = client.connection_status_receiver().unwrap();
        let (mut trader_rx, _trader) = client
            .subscribe_to_trader_state(&Pubkey::new_unique())
            .unwrap();
        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        assert_connected(&mut status).await;

        let next = tokio::time::timeout(Duration::from_secs(1), status.recv())
            .await
            .expect("idle timeout should report within 1s");
        let Some(WsConnectionStatus::Disconnected(reason)) = next else {
            panic!("expected Disconnected, got {next:?}");
        };
        assert!(reason.starts_with("idle timeout"), "{reason}");
        assert!(timeout_closed(&mut trader_rx).await);
        assert!(timeout_closed(&mut mids_rx).await);
    }

    #[tokio::test]
    async fn client_pings_keep_a_quiet_reading_server_alive() {
        let server = spawn_scripted_ws_server(None).await;
        let mut client = client_with_heartbeat(&server.url, FAST_HEARTBEAT);
        let mut status = client.connection_status_receiver().unwrap();
        assert_connected(&mut status).await;

        assert_no_status_for(&mut status, Duration::from_secs(1)).await;
        assert!(server.client_pings.load(AtomicOrdering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn server_pings_count_as_activity() {
        let server = spawn_scripted_ws_server(Some(Duration::from_millis(100))).await;
        let mut client = client_with_heartbeat(
            &server.url,
            WsHeartbeatConfig {
                ping_interval: None,
                idle_timeout: Some(Duration::from_millis(300)),
            },
        );
        let mut status = client.connection_status_receiver().unwrap();
        assert_connected(&mut status).await;

        assert_no_status_for(&mut status, Duration::from_secs(1)).await;
        assert_eq!(server.client_pings.load(AtomicOrdering::SeqCst), 0);
    }

    #[tokio::test]
    async fn disabled_heartbeat_keeps_a_silent_socket_open() {
        let url = spawn_silent_ws_server().await;
        let mut client = client_with_heartbeat(
            &url,
            WsHeartbeatConfig {
                ping_interval: None,
                idle_timeout: None,
            },
        );
        let mut status = client.connection_status_receiver().unwrap();
        let (mut rx, _handle) = client.subscribe_to_all_mids().unwrap();
        assert_connected(&mut status).await;

        assert_no_status_for(&mut status, Duration::from_secs(1)).await;
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn heartbeat_check_period_uses_half_the_shortest_enabled_interval() {
        assert_eq!(
            FAST_HEARTBEAT.check_period(),
            Some(Duration::from_millis(50))
        );
        assert_eq!(
            WsHeartbeatConfig {
                ping_interval: None,
                idle_timeout: Some(Duration::from_millis(300)),
            }
            .check_period(),
            Some(Duration::from_millis(150))
        );
        assert_eq!(
            WsHeartbeatConfig {
                ping_interval: None,
                idle_timeout: None,
            }
            .check_period(),
            None
        );
    }

    // ------------------------------------------------------------------
    // Subscription status and error attribution
    // ------------------------------------------------------------------

    /// Scripted server: records client frames and pings, and sends whatever
    /// the test pushes, in order.
    struct ScriptedWsServer {
        url: String,
        client_frames: mpsc::UnboundedReceiver<serde_json::Value>,
        outbound: mpsc::UnboundedSender<Message>,
        client_pings: Arc<AtomicUsize>,
    }

    impl ScriptedWsServer {
        async fn next_client_frame(&mut self) -> serde_json::Value {
            timeout_recv(&mut self.client_frames).await
        }

        fn send(&self, value: serde_json::Value) {
            self.outbound
                .send(Message::Text(Utf8Bytes::from(value.to_string())))
                .expect("scripted server should be running");
        }

        fn send_status(&self, status: &str, subscription: serde_json::Value) {
            self.send(json!({
                "channel": "subscriptionStatus",
                "status": status,
                "subscription": subscription,
                "clientId": "client-1",
            }));
        }

        fn send_error(&self, code: u16, error: &str) {
            self.send(json!({ "channel": "error", "error": error, "code": code }));
        }

        /// Read one subscribe from the client and acknowledge it.
        async fn ack_next_subscribe(&mut self) -> serde_json::Value {
            let frame = self.next_client_frame().await;
            assert_eq!(frame["type"], "subscribe", "{frame}");
            self.send_status("subscribed", frame["subscription"].clone());
            frame["subscription"].clone()
        }
    }

    async fn spawn_scripted_ws_server(server_ping_every: Option<Duration>) -> ScriptedWsServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let url = format!("ws://{}", listener.local_addr().expect("ws addr"));
        let (frames_tx, client_frames) = mpsc::unbounded_channel();
        let (outbound, mut outbound_rx) = mpsc::unbounded_channel::<Message>();
        let client_pings = Arc::new(AtomicUsize::new(0));
        let pings = Arc::clone(&client_pings);
        tokio::spawn(async move {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
                return;
            };
            let mut ping_timer = server_ping_every.map(tokio::time::interval);
            loop {
                tokio::select! {
                    frame = ws.next() => match frame {
                        Some(Ok(Message::Text(text))) => {
                            let _ = frames_tx.send(serde_json::from_str(text.as_str()).expect("json frame"));
                        }
                        Some(Ok(Message::Binary(data))) => {
                            let _ = frames_tx.send(serde_json::from_slice(&data).expect("json frame"));
                        }
                        Some(Ok(Message::Ping(_))) => {
                            pings.fetch_add(1, AtomicOrdering::SeqCst);
                        }
                        Some(Ok(_)) => {}
                        Some(Err(_)) | None => return,
                    },
                    out = outbound_rx.recv() => {
                        let Some(out) = out else { return };
                        if ws.send(out).await.is_err() {
                            return;
                        }
                    }
                    _ = PhoenixWSClient::next_heartbeat_tick(ping_timer.as_mut()) => {
                        if ws.send(Message::Ping(Bytes::new())).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });
        ScriptedWsServer {
            url,
            client_frames,
            outbound,
            client_pings,
        }
    }

    fn scripted_client(server: &ScriptedWsServer) -> PhoenixWSClient {
        PhoenixWSClient::new_with_connection_status(&server.url).expect("ws client should build")
    }

    fn all_mids_frame() -> serde_json::Value {
        json!({ "channel": "allMids", "mids": { "SOL": 1.0 }, "slot": 1, "slotIndex": 0 })
    }

    fn trader_state_frame(authority: &Pubkey, slot: u64) -> serde_json::Value {
        json!({
            "channel": "traderState",
            "authority": authority.to_string(),
            "traderPdaIndex": 0,
            "slot": slot,
            "messageType": "delta",
            "deltas": [],
        })
    }

    fn trader_state_request(authority: &Pubkey) -> SubscriptionRequest {
        SubscriptionRequest::TraderState(TraderStateSubscriptionRequest {
            authority: authority.to_string(),
            trader_pda_index: 0,
        })
    }

    async fn timeout_closed<T>(rx: &mut mpsc::UnboundedReceiver<T>) -> bool {
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("receiver should resolve")
            .is_none()
    }

    fn assert_status_event(
        event: WsSubscriptionEvent,
        expected_status: &str,
        expected: &SubscriptionRequest,
    ) {
        assert_eq!(
            event,
            WsSubscriptionEvent::Status {
                status: expected_status.to_string(),
                subscription: expected.clone(),
            }
        );
    }

    fn assert_error_event(
        event: WsSubscriptionEvent,
        expected_code: &str,
        expected: Option<&SubscriptionRequest>,
    ) {
        let WsSubscriptionEvent::Error {
            subscription, code, ..
        } = event
        else {
            panic!("expected error event, got {event:?}");
        };
        assert_eq!(code, expected_code);
        assert_eq!(subscription.as_ref(), expected);
    }

    #[test]
    fn unsubscribed_status_keeps_its_status() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        PhoenixWSClient::process_message(
            br#"{"channel":"subscriptionStatus","status":"unsubscribed","subscription":{"channel":"allMids"},"clientId":"c"}"#,
            &mut SocketState::default(),
            &tx,
        )
        .expect("status should apply");
        assert!(
            matches!(rx.try_recv().unwrap(), WsSubscriptionEvent::Status { status, .. } if status == "unsubscribed")
        );
    }

    #[tokio::test]
    async fn trader_end_preserves_replacement_and_cancellation() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut events = client.subscription_event_receiver().unwrap();
        let authority = Pubkey::new_unique();
        let (old_rx, old_handle) = client.subscribe_to_trader_state(&authority).unwrap();
        let echo = server.next_client_frame().await["subscription"].clone();
        drop(old_handle);
        drop(old_rx);
        assert_eq!(server.next_client_frame().await["type"], "unsubscribe");
        let (mut replacement_rx, replacement) =
            client.subscribe_to_trader_state(&authority).unwrap();
        assert_eq!(server.next_client_frame().await["type"], "subscribe");

        server.send_status("subscribed", echo.clone());
        server.send_status("unsubscribed", echo.clone());
        server.send_status("subscribed", echo.clone());
        server.send(trader_state_frame(&authority, 42));
        assert_eq!(timeout_recv(&mut replacement_rx).await.slot, 42);

        // Status-only ordering still permits cancellation of the replacement.
        drop(replacement);
        let unsubscribe = server.next_client_frame().await;
        assert_eq!(unsubscribe["type"], "unsubscribe");
        assert_eq!(unsubscribe["subscription"], echo);
        assert!(timeout_closed(&mut replacement_rx).await);
        let request = trader_state_request(&authority);
        assert_status_event(timeout_recv(&mut events).await, "subscribed", &request);
        assert_status_event(timeout_recv(&mut events).await, "unsubscribed", &request);
        assert_status_event(timeout_recv(&mut events).await, "subscribed", &request);
    }

    #[test]
    fn repeated_subscribe_keeps_newer_receiver_after_trader_end() {
        let authority = Pubkey::new_unique();
        let (subscriber_tx, mut subscriber_rx) = mpsc::unbounded_channel();
        let (event_tx, mut events) = mpsc::unbounded_channel();
        let request = trader_state_request(&authority);
        let key = SubscriptionKey::trader(&authority, 0);
        let mut state = SocketState {
            subscribers: HashMap::from([(
                key.clone(),
                HashMap::from([(2, Subscriber::TraderState(subscriber_tx))]),
            )]),
            active_subscriptions: HashMap::from([(key.clone(), request.clone())]),
            ..SocketState::default()
        };
        state.push_pending(key.clone(), request.clone(), PendingOp::Subscribe);
        state.push_pending(key.clone(), request.clone(), PendingOp::Subscribe);
        let echo = serde_json::to_value(&request).unwrap();
        for status in ["subscribed", "unsubscribed"] {
            PhoenixWSClient::process_message(
                &serde_json::to_vec(&json!({
                    "channel": "subscriptionStatus", "status": status,
                    "subscription": echo, "clientId": "c",
                }))
                .unwrap(),
                &mut state,
                &event_tx,
            )
            .expect("old status should apply");
            assert_status_event(events.try_recv().unwrap(), status, &request);
        }
        assert_eq!(state.pending.len(), 1);
        assert_eq!(state.active_subscriptions.get(&key), Some(&request));
        PhoenixWSClient::process_message(
            &serde_json::to_vec(&json!({
                "channel": "subscriptionStatus", "status": "subscribed",
                "subscription": echo, "clientId": "c",
            }))
            .unwrap(),
            &mut state,
            &event_tx,
        )
        .expect("replacement acknowledgement should apply");
        PhoenixWSClient::process_message(
            &serde_json::to_vec(&trader_state_frame(&authority, 42)).unwrap(),
            &mut state,
            &event_tx,
        )
        .expect("replacement data should apply");
        assert_eq!(subscriber_rx.try_recv().unwrap().slot, 42);
        assert!(state.pending.is_empty());
    }

    #[tokio::test]
    async fn echoed_error_after_exchange_end_and_retry_fails_closed() {
        for replacement_pending in [false, true] {
            let mut server = spawn_scripted_ws_server(None).await;
            let mut client = scripted_client(&server);
            let mut status = client.connection_status_receiver().unwrap();
            let mut events = client.subscription_event_receiver().unwrap();
            assert_connected(&mut status).await;
            let (mut old_rx, old_handle) = client.subscribe_to_exchange().unwrap();
            let echo = server.next_client_frame().await["subscription"].clone();
            let mut old_handle = Some(old_handle);
            let pending_retry = if replacement_pending {
                drop(old_handle.take());
                assert_eq!(server.next_client_frame().await["type"], "unsubscribe");
                let retry = client.subscribe_to_exchange().unwrap();
                assert_eq!(server.next_client_frame().await["type"], "subscribe");
                Some(retry)
            } else {
                None
            };
            server.send_status("unsubscribed", echo.clone());
            let request = SubscriptionRequest::Exchange(ExchangeSubscriptionRequest {
                encoding: Some(ExchangeSnapshotEncoding::Json),
            });
            assert_status_event(timeout_recv(&mut events).await, "unsubscribed", &request);
            assert!(timeout_closed(&mut old_rx).await);
            drop(old_handle);
            let (mut retry_rx, _retry) = if let Some(retry) = pending_retry {
                retry
            } else {
                let retry = client.subscribe_to_exchange().unwrap();
                assert_eq!(server.next_client_frame().await["type"], "subscribe");
                retry
            };
            // A different key's acknowledgement cannot disambiguate the
            // old exchange error from a rejection of its pending retry.
            let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
            let mids_echo = server.next_client_frame().await["subscription"].clone();
            server.send_status("subscribed", mids_echo);
            assert_status_event(
                timeout_recv(&mut events).await,
                "subscribed",
                &SubscriptionRequest::AllMids,
            );
            server.send(json!({
                "channel": "error", "error": "old bootstrap failed", "code": 500,
                "subscription": echo.clone(),
            }));
            assert_error_event(timeout_recv(&mut events).await, "500", Some(&request));
            assert_disconnected_contains(&mut status, "subscription response mismatch").await;
            assert!(timeout_closed(&mut retry_rx).await);
            assert!(timeout_closed(&mut mids_rx).await);
        }
    }

    #[test]
    fn echoed_error_does_not_choose_between_ended_context_and_pending_operation() {
        for definite_end in [false, true] {
            for op in [PendingOp::Subscribe, PendingOp::Unsubscribe] {
                let authority = Pubkey::new_unique();
                let key = SubscriptionKey::trader(&authority, 0);
                let request = trader_state_request(&authority);
                let (subscriber_tx, mut subscriber_rx) = mpsc::unbounded_channel();
                let (event_tx, mut events) = mpsc::unbounded_channel();
                let ended = if definite_end {
                    PrevResponse::ServerEnded(request.clone())
                } else {
                    PrevResponse::MaybeEnded(request.clone())
                };
                let mut state = SocketState {
                    subscribers: HashMap::from([(
                        key.clone(),
                        HashMap::from([(1, Subscriber::TraderState(subscriber_tx))]),
                    )]),
                    prev: HashMap::from([(key.clone(), ended)]),
                    ..SocketState::default()
                };
                state.push_pending(key.clone(), request.clone(), op);

                let result = PhoenixWSClient::process_message(
                    &serde_json::to_vec(&json!({
                        "channel": "error", "error": "retry later", "code": 429,
                        "subscription": request,
                    }))
                    .unwrap(),
                    &mut state,
                    &event_tx,
                );

                assert!(
                    result.is_err(),
                    "the manager must disconnect rather than guess"
                );
                assert_error_event(events.try_recv().unwrap(), "429", Some(&request));
                assert_eq!(state.pending.len(), 1, "no attempt is consumed");
                assert!(state.prev.contains_key(&key), "ended context is retained");
                assert!(matches!(subscriber_rx.try_recv(), Err(TryRecvError::Empty)));
            }
        }
    }

    #[test]
    fn echoed_error_does_not_choose_between_same_key_pending_operations() {
        for second_op in [PendingOp::Subscribe, PendingOp::Unsubscribe] {
            let authority = Pubkey::new_unique();
            let key = SubscriptionKey::trader(&authority, 0);
            let request = trader_state_request(&authority);
            let (event_tx, mut events) = mpsc::unbounded_channel();
            let mut state = SocketState::default();
            state.push_pending(key.clone(), request.clone(), PendingOp::Subscribe);
            state.push_pending(key, request.clone(), second_op);

            let result = PhoenixWSClient::process_message(
                &serde_json::to_vec(&json!({
                    "channel": "error", "error": "quota exceeded", "code": 429,
                    "subscription": request,
                }))
                .unwrap(),
                &mut state,
                &event_tx,
            );

            assert!(
                result.is_err(),
                "the manager must disconnect rather than guess"
            );
            assert_error_event(events.try_recv().unwrap(), "429", Some(&request));
            assert_eq!(state.pending.len(), 2, "neither attempt is consumed");
        }
    }

    #[tokio::test]
    async fn echoed_replacement_rejection_overtaking_old_trader_statuses_fails_closed() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        assert_connected(&mut status).await;
        let authority = Pubkey::new_unique();

        let (mut old_rx, old_handle) = client.subscribe_to_trader_state(&authority).unwrap();
        let first = server.next_client_frame().await;
        assert_eq!(first["type"], "subscribe");
        let echo = first["subscription"].clone();
        drop(old_handle);
        let unsubscribe = server.next_client_frame().await;
        assert_eq!(unsubscribe["type"], "unsubscribe");
        assert_eq!(unsubscribe["subscription"], echo);
        assert!(timeout_closed(&mut old_rx).await);

        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        let mids = server.next_client_frame().await;
        assert_eq!(mids["type"], "subscribe");
        assert_eq!(mids["subscription"]["channel"], "allMids");
        let (mut replacement_rx, _replacement) =
            client.subscribe_to_trader_state(&authority).unwrap();
        let replacement = server.next_client_frame().await;
        assert_eq!(replacement["type"], "subscribe");
        assert_eq!(replacement["subscription"], echo);

        // The shared error queue can overtake traderState's status queue.
        // B's acknowledgement does not identify which A attempt was rejected.
        // Send one batch so the old statuses are already queued at disconnect.
        server.send_status("subscribed", mids["subscription"].clone());
        server.send(json!({
            "channel": "error", "error": "quota exceeded", "code": 429,
            "subscription": echo,
        }));
        server.send_status("subscribed", echo.clone());
        server.send_status("unsubscribed", echo);

        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_error_event(
            timeout_recv(&mut events).await,
            "429",
            Some(&trader_state_request(&authority)),
        );
        assert_disconnected_contains(&mut status, "subscription response mismatch").await;
        assert!(timeout_closed(&mut replacement_rx).await);
        assert!(timeout_closed(&mut mids_rx).await);
        assert!(
            timeout_closed(&mut events).await,
            "old statuses must not revive A"
        );
    }

    /// Without a pending frame the echo can only belong to the subscription
    /// that just ended, so it stays attributed to that request.
    #[test]
    fn echoed_error_with_nothing_pending_keeps_the_ended_subscription() {
        let authority = Pubkey::new_unique();
        let key = SubscriptionKey::trader(&authority, 0);
        let (event_tx, mut events) = mpsc::unbounded_channel();
        let mut state = SocketState {
            prev: HashMap::from([(
                key.clone(),
                PrevResponse::ServerEnded(trader_state_request(&authority)),
            )]),
            ..SocketState::default()
        };

        PhoenixWSClient::process_message(
            &serde_json::to_vec(&json!({
                "channel": "error",
                "error": "retry later",
                "code": 429,
                "subscription": {
                    "channel": "traderState",
                    "authority": authority.to_string(),
                    "traderPdaIndex": 0,
                },
            }))
            .unwrap(),
            &mut state,
            &event_tx,
        )
        .expect("echoed error should apply");

        assert_error_event(
            events.try_recv().unwrap(),
            "429",
            Some(&trader_state_request(&authority)),
        );
    }

    #[test]
    fn legacy_type_tagged_frames_are_not_confirmations() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        PhoenixWSClient::process_message(
            br#"{"type":"subscriptionError","subscription":{"channel":"allMids"},"code":"400","message":"bad"}"#,
            &mut SocketState::default(),
            &tx,
        )
        .expect("unknown frame is ignored");
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn trades_ack_with_market_symbol_echo_parses() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        PhoenixWSClient::process_message(
            br#"{"channel":"subscriptionStatus","status":"subscribed","subscription":{"channel":"trades","marketSymbol":"SOL"},"clientId":"c"}"#,
            &mut SocketState::default(),
            &tx,
        )
        .expect("status should apply");
        assert_status_event(
            rx.try_recv().unwrap(),
            "subscribed",
            &SubscriptionRequest::Trades(TradesSubscriptionRequest {
                symbol: "SOL".to_string(),
            }),
        );
    }

    #[test]
    fn market_update_without_mid_is_delivered() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (event_tx, _events) = mpsc::unbounded_channel();
        let mut state = SocketState {
            subscribers: HashMap::from([(
                SubscriptionKey::market_stats_v2(Some(vec!["SOL".into()])).routing_key(),
                HashMap::from([(1, Subscriber::MarketStats(tx))]),
            )]),
            ..SocketState::default()
        };
        PhoenixWSClient::process_message(br#"{"channel":"marketStatsV2","symbols":["SOL"],"stats":[{"symbol":"SOL","timestamp":1,"openInterest":2.0,"markPrice":3.0,"oraclePrice":4.0,"prevDayMarkPrice":5.0,"dayVolumeUsd":6.0,"dayVolumeBase":7.0,"currentFundingRate":8.0,"eightHourFundingRate":9.0,"annualizedFundingRate":10.0}]}"#,
            &mut state, &event_tx).expect("data frame");
        let update = rx
            .try_recv()
            .expect("update without mid should be delivered");
        assert_eq!(update.mid_price, None);
    }

    #[tokio::test]
    async fn rejected_subscription_is_attributed_and_closed() {
        // The server keeps the socket open and pings every 50 ms, so the
        // heartbeat alone would never surface the rejection.
        let mut server = spawn_scripted_ws_server(Some(Duration::from_millis(50))).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let authority = Pubkey::new_unique();
        let (mut rx, _h) = client.subscribe_to_trader_state(&authority).unwrap();
        assert_connected(&mut status).await;
        server.next_client_frame().await;
        server.send_error(403, "insufficient_scope_trader_state_subscription");

        assert_error_event(
            timeout_recv(&mut events).await,
            "403",
            Some(&trader_state_request(&authority)),
        );
        assert!(timeout_closed(&mut rx).await);

        // The connection stays up and other subscriptions keep receiving.
        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        server.ack_next_subscribe().await;
        server.send(all_mids_frame());
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        timeout_recv(&mut mids_rx).await;
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn register_failure_ends_only_the_failed_trader_state() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut mids_rx, _c) = client.subscribe_to_all_mids().unwrap();
        server.ack_next_subscribe().await;
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );

        let (a, b) = (Pubkey::new_unique(), Pubkey::new_unique());
        let (mut a_rx, _a) = client.subscribe_to_trader_state(&a).unwrap();
        let (mut b_rx, _b) = client.subscribe_to_trader_state(&b).unwrap();
        let a_echo = server.next_client_frame().await["subscription"].clone();
        let b_echo = server.next_client_frame().await["subscription"].clone();
        server.send_status("subscribed", a_echo.clone());
        server.send(trader_state_frame(&a, 1));
        server.send_status("unsubscribed", a_echo.clone());
        server.send(json!({
            "channel": "error", "error": "trader state actor unavailable", "code": 500,
            "subscription": a_echo,
        }));
        server.send_status("subscribed", b_echo);
        server.send(trader_state_frame(&b, 2));
        server.send(all_mids_frame());

        let (a_req, b_req) = (trader_state_request(&a), trader_state_request(&b));
        assert_status_event(timeout_recv(&mut events).await, "subscribed", &a_req);
        assert_status_event(timeout_recv(&mut events).await, "unsubscribed", &a_req);
        assert_error_event(timeout_recv(&mut events).await, "500", Some(&a_req));
        assert_status_event(timeout_recv(&mut events).await, "subscribed", &b_req);
        assert_eq!(timeout_recv(&mut a_rx).await.slot, 1);
        assert!(timeout_closed(&mut a_rx).await);
        assert_eq!(timeout_recv(&mut b_rx).await.slot, 2);
        timeout_recv(&mut mids_rx).await;
        assert!(matches!(b_rx.try_recv(), Err(TryRecvError::Empty)));
        assert!(matches!(mids_rx.try_recv(), Err(TryRecvError::Empty)));
        assert_connected(&mut status).await;
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn exchange_bootstrap_failure_is_attributed_to_exchange() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut exchange_rx, _exchange) = client.subscribe_to_exchange().unwrap();
        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        let exchange_echo = server.next_client_frame().await["subscription"].clone();
        let mids_echo = server.next_client_frame().await["subscription"].clone();
        server
            .send(serde_json::from_str(&exchange_snapshot_json(1, &Pubkey::new_unique())).unwrap());
        server.send_status("unsubscribed", exchange_echo.clone());
        server.send(json!({
            "channel": "error", "error": "exchange bootstrap failed", "code": 500,
            "subscription": exchange_echo,
        }));
        server.send_status("subscribed", mids_echo);
        server.send(all_mids_frame());

        let exchange_request = SubscriptionRequest::Exchange(ExchangeSubscriptionRequest {
            encoding: Some(ExchangeSnapshotEncoding::Json),
        });
        assert_status_event(
            timeout_recv(&mut events).await,
            "unsubscribed",
            &exchange_request,
        );
        assert_error_event(
            timeout_recv(&mut events).await,
            "500",
            Some(&exchange_request),
        );
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert!(matches!(
            timeout_recv(&mut exchange_rx).await,
            ExchangeMessage::Snapshot(_)
        ));
        assert!(timeout_closed(&mut exchange_rx).await);
        timeout_recv(&mut mids_rx).await;
    }

    #[tokio::test]
    async fn error_after_a_status_belongs_to_the_next_pending_frame() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut a_rx, _a) = client.subscribe_to_all_mids().unwrap();
        let (mut b_rx, _b) = client.subscribe_to_trades(TEST_SYMBOL.to_string()).unwrap();
        let a_echo = server.next_client_frame().await["subscription"].clone();
        server.next_client_frame().await;
        server.send_status("subscribed", a_echo);
        server.send_error(400, "Unknown market symbol");
        server.send(all_mids_frame());

        let trades_request = SubscriptionRequest::Trades(TradesSubscriptionRequest {
            symbol: TEST_SYMBOL.to_string(),
        });
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_error_event(
            timeout_recv(&mut events).await,
            "400",
            Some(&trades_request),
        );
        assert!(timeout_closed(&mut b_rx).await);
        timeout_recv(&mut a_rx).await;
    }

    #[tokio::test]
    async fn legacy_error_with_multiple_pending_requests_is_unattributed_and_fails_closed() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut a_rx, _a) = client.subscribe_to_all_mids().unwrap();
        let (mut b_rx, _b) = client.subscribe_to_trades(TEST_SYMBOL.to_string()).unwrap();
        assert_connected(&mut status).await;
        server.next_client_frame().await;
        server.next_client_frame().await;
        server.send_error(403, "subscription rejected without an echo");

        assert_error_event(timeout_recv(&mut events).await, "403", None);
        assert_disconnected_contains(&mut status, "subscription response mismatch").await;
        assert!(timeout_closed(&mut a_rx).await);
        assert!(timeout_closed(&mut b_rx).await);
    }

    #[tokio::test]
    async fn canonical_echoes_match_their_requests() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let (_book_rx, _book) = client.subscribe_to_orderbook("sol".to_string()).unwrap();
        let (_trades_rx, _trades) = client.subscribe_to_trades("sol".to_string()).unwrap();
        server.next_client_frame().await;
        server.next_client_frame().await;
        let book_echo = json!({ "channel": "orderbook", "symbol": "SOL" });
        let trades_echo = json!({ "channel": "trades", "marketSymbol": "SOL" });
        server.send_status("subscribed", book_echo.clone());
        server.send_status("subscribed", trades_echo.clone());
        // With nothing left pending, a stray error is unattributed.
        server.send_error(400, "stray");

        let book_request: SubscriptionRequest = serde_json::from_value(book_echo).unwrap();
        let trades_request: SubscriptionRequest = serde_json::from_value(trades_echo).unwrap();
        assert_status_event(timeout_recv(&mut events).await, "subscribed", &book_request);
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &trades_request,
        );
        assert_error_event(timeout_recv(&mut events).await, "400", None);
        assert_connected(&mut status).await;
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn unsubscribe_reply_only_reports_status() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        server.ack_next_subscribe().await;
        let authority = Pubkey::new_unique();
        let (_a_rx, a_handle) = client.subscribe_to_trader_state(&authority).unwrap();
        server.ack_next_subscribe().await;
        let a_request = trader_state_request(&authority);
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_status_event(timeout_recv(&mut events).await, "subscribed", &a_request);

        drop(a_handle);
        let unsubscribe = server.next_client_frame().await;
        assert_eq!(unsubscribe["type"], "unsubscribe");
        server.send_status("unsubscribed", unsubscribe["subscription"].clone());
        server.send(all_mids_frame());

        assert_status_event(timeout_recv(&mut events).await, "unsubscribed", &a_request);
        timeout_recv(&mut mids_rx).await;
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn unsolicited_error_is_unattributed_and_closes_nothing() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        server.ack_next_subscribe().await;
        server.send_error(400, "Invalid JSON");
        server.send(all_mids_frame());

        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_error_event(timeout_recv(&mut events).await, "400", None);
        timeout_recv(&mut mids_rx).await;
        assert_connected(&mut status).await;
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));
    }

    struct AmbiguousScenario {
        server: ScriptedWsServer,
        _client: PhoenixWSClient,
        status: mpsc::UnboundedReceiver<WsConnectionStatus>,
        events: mpsc::UnboundedReceiver<WsSubscriptionEvent>,
        b_rx: mpsc::UnboundedReceiver<TraderStateServerMessage>,
        _b_handle: SubscriptionHandle,
        a: Pubkey,
        b: Pubkey,
        b_echo: serde_json::Value,
    }

    /// Subscribe `traderState` A, drop A after the server read it, then
    /// subscribe B. The server reads all three frames and answers
    /// `subscribed(A)`, `unsubscribed(A)` and the error built from A's echo.
    async fn ambiguous_error_scenario(
        error: impl FnOnce(serde_json::Value) -> serde_json::Value,
    ) -> AmbiguousScenario {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let events = client.subscription_event_receiver().unwrap();
        assert_connected(&mut status).await;
        let (a, b) = (Pubkey::new_unique(), Pubkey::new_unique());
        let (_a_rx, a_handle) = client.subscribe_to_trader_state(&a).unwrap();
        let a_echo = server.next_client_frame().await["subscription"].clone();
        drop(a_handle);
        let (b_rx, b_handle) = client.subscribe_to_trader_state(&b).unwrap();
        assert_eq!(server.next_client_frame().await["type"], "unsubscribe");
        let b_echo = server.next_client_frame().await["subscription"].clone();
        server.send_status("subscribed", a_echo.clone());
        server.send_status("unsubscribed", a_echo.clone());
        server.send(error(a_echo));
        AmbiguousScenario {
            server,
            _client: client,
            status,
            events,
            b_rx,
            _b_handle: b_handle,
            a,
            b,
            b_echo,
        }
    }

    #[tokio::test]
    async fn ambiguous_error_fails_closed() {
        let mut scenario = ambiguous_error_scenario(|_| {
            json!({ "channel": "error", "error": "trader state actor unavailable", "code": 500 })
        })
        .await;

        let a_request = trader_state_request(&scenario.a);
        let events = &mut scenario.events;
        assert_status_event(timeout_recv(events).await, "subscribed", &a_request);
        assert_status_event(timeout_recv(events).await, "unsubscribed", &a_request);
        assert_error_event(timeout_recv(events).await, "500", None);
        assert_disconnected_contains(&mut scenario.status, "subscription response mismatch").await;
        assert!(timeout_closed(&mut scenario.b_rx).await);
    }

    #[tokio::test]
    async fn echoed_error_resolves_the_ambiguity() {
        let mut scenario = ambiguous_error_scenario(|a_echo| {
            json!({
                "channel": "error",
                "error": "trader state actor unavailable",
                "code": 500,
                "subscription": a_echo,
            })
        })
        .await;
        scenario
            .server
            .send_status("subscribed", scenario.b_echo.clone());
        scenario.server.send(trader_state_frame(&scenario.b, 3));

        let a_request = trader_state_request(&scenario.a);
        let b_request = trader_state_request(&scenario.b);
        let events = &mut scenario.events;
        assert_status_event(timeout_recv(events).await, "subscribed", &a_request);
        assert_status_event(timeout_recv(events).await, "unsubscribed", &a_request);
        assert_error_event(timeout_recv(events).await, "500", Some(&a_request));
        assert_status_event(timeout_recv(events).await, "subscribed", &b_request);
        assert_eq!(timeout_recv(&mut scenario.b_rx).await.slot, 3);
        assert!(matches!(
            scenario.status.try_recv(),
            Err(TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn mixed_channel_acknowledgements_can_arrive_in_reverse_order() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let authority = Pubkey::new_unique();
        let (mut trader_rx, trader) = client.subscribe_to_trader_state(&authority).unwrap();
        let (mut mids_rx, mids) = client.subscribe_to_all_mids().unwrap();
        assert_connected(&mut status).await;
        let trader_echo = server.next_client_frame().await["subscription"].clone();
        let mids_echo = server.next_client_frame().await["subscription"].clone();
        server.send_status("subscribed", mids_echo.clone());
        server.send_status("subscribed", trader_echo.clone());
        server.send(all_mids_frame());
        server.send(trader_state_frame(&authority, 7));

        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &trader_state_request(&authority),
        );
        timeout_recv(&mut mids_rx).await;
        assert_eq!(timeout_recv(&mut trader_rx).await.slot, 7);
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));

        drop(trader);
        assert_eq!(server.next_client_frame().await["type"], "unsubscribe");
        drop(mids);
        assert_eq!(server.next_client_frame().await["type"], "unsubscribe");
        server.send_status("unsubscribed", mids_echo);
        server.send_status("unsubscribed", trader_echo);
        assert_status_event(
            timeout_recv(&mut events).await,
            "unsubscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_status_event(
            timeout_recv(&mut events).await,
            "unsubscribed",
            &trader_state_request(&authority),
        );
        assert!(timeout_closed(&mut mids_rx).await);
        assert!(timeout_closed(&mut trader_rx).await);
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn echoed_error_can_overtake_an_earlier_trader_state_acknowledgement() {
        let mut server = spawn_scripted_ws_server(None).await;
        let mut client = scripted_client(&server);
        let mut status = client.connection_status_receiver().unwrap();
        let mut events = client.subscription_event_receiver().unwrap();
        let authority = Pubkey::new_unique();
        let (mut trader_rx, _trader) = client.subscribe_to_trader_state(&authority).unwrap();
        let (mut trades_rx, _trades) = client.subscribe_to_trades(TEST_SYMBOL.to_string()).unwrap();
        let (mut mids_rx, _mids) = client.subscribe_to_all_mids().unwrap();
        assert_connected(&mut status).await;
        let trader_echo = server.next_client_frame().await["subscription"].clone();
        let trades_echo = server.next_client_frame().await["subscription"].clone();
        let mids_echo = server.next_client_frame().await["subscription"].clone();

        server.send(json!({
            "channel": "error", "error": "Unknown market symbol", "code": 400,
            "subscription": trades_echo,
        }));
        server.send_status("subscribed", mids_echo);
        server.send_status("subscribed", trader_echo);
        server.send(all_mids_frame());
        server.send(trader_state_frame(&authority, 8));

        assert_error_event(
            timeout_recv(&mut events).await,
            "400",
            Some(&SubscriptionRequest::Trades(TradesSubscriptionRequest {
                symbol: TEST_SYMBOL.to_string(),
            })),
        );
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &SubscriptionRequest::AllMids,
        );
        assert_status_event(
            timeout_recv(&mut events).await,
            "subscribed",
            &trader_state_request(&authority),
        );
        assert!(timeout_closed(&mut trades_rx).await);
        timeout_recv(&mut mids_rx).await;
        assert_eq!(timeout_recv(&mut trader_rx).await.slot, 8);
        assert!(matches!(status.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn legacy_error_does_not_consume_ambiguous_same_key_generations() {
        let authority = Pubkey::new_unique();
        let key = SubscriptionKey::trader(&authority, 0);
        let request = trader_state_request(&authority);
        let (subscriber_tx, mut subscriber_rx) = mpsc::unbounded_channel();
        let (event_tx, mut events) = mpsc::unbounded_channel();
        let mut state = SocketState {
            subscribers: HashMap::from([(
                key.clone(),
                HashMap::from([(1, Subscriber::TraderState(subscriber_tx))]),
            )]),
            active_subscriptions: HashMap::from([(key.clone(), request.clone())]),
            ..SocketState::default()
        };
        state.push_pending(key.clone(), request.clone(), PendingOp::Subscribe);
        state.push_pending(key.clone(), request.clone(), PendingOp::Subscribe);

        let result = PhoenixWSClient::process_message(
            br#"{"channel":"error","error":"rejected","code":403}"#,
            &mut state,
            &event_tx,
        );

        assert!(
            result.is_err(),
            "the manager must disconnect rather than guess"
        );
        assert_error_event(events.try_recv().unwrap(), "403", None);
        assert_eq!(state.pending.len(), 2);
        assert_eq!(state.active_subscriptions.get(&key), Some(&request));
        assert!(matches!(subscriber_rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn error_after_auth_reconnect_is_attributed_to_the_resent_request() {
        let stale_token = test_jwt_with_exp("stale-reject-jti", future_unix_secs(600));
        let fresh_token = test_jwt_with_exp("fresh-reject-jti", future_unix_secs(600));
        let (refresh_url, _refresh_auth_rx, _refresh_count) =
            spawn_refresh_server(fresh_token).await;
        let ws_url =
            spawn_auth_close_then_reject_ws_server(Iana(4401), "access_token_expired").await;
        let auth_client = test_auth_client(
            refresh_url,
            test_session_store(stale_token, Some(REFRESH_TOKEN.to_string()), Some(600)),
        );
        let mut client = PhoenixWSClient::builder(&ws_url)
            .with_auth(auth_client)
            .build()
            .expect("ws client should build");
        let mut events = client.subscription_event_receiver().unwrap();
        let (mut rx, _handle) = client
            .subscribe_to_trades(TEST_SYMBOL.to_string())
            .expect("subscribe should succeed");

        assert_error_event(
            timeout_recv(&mut events).await,
            "403",
            Some(&SubscriptionRequest::Trades(TradesSubscriptionRequest {
                symbol: TEST_SYMBOL.to_string(),
            })),
        );
        assert!(timeout_closed(&mut rx).await);
    }

    /// First connection: read the subscribe, then close with an auth code.
    /// Second connection: read the resent subscribe, reject it, stay open.
    async fn spawn_auth_close_then_reject_ws_server(
        close_code: CloseCode,
        close_reason: &'static str,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("ws listener should bind");
        let addr = listener.local_addr().expect("ws addr should exist");
        let (tx, _rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            handle_ws_connection(&listener, tx, Some(close_frame(close_code, close_reason))).await;
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let callback = |request: &Request, mut response: Response| {
                select_ws_protocol(request, &mut response);
                Ok(response)
            };
            let Ok(mut websocket) = accept_hdr_async(stream, callback).await else {
                return;
            };
            if websocket.next().await.is_none() {
                return;
            }
            let error = json!({ "channel": "error", "error": "insufficient_scope", "code": 403 });
            if websocket
                .send(Message::Text(Utf8Bytes::from(error.to_string())))
                .await
                .is_err()
            {
                return;
            }
            while websocket.next().await.is_some() {}
        });
        format!("ws://{addr}/v1/ws")
    }
}
