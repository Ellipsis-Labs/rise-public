//! WebSocket client for connecting to the Phoenix API.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "opentelemetry")]
use opentelemetry::{Context, global};
#[cfg(feature = "opentelemetry")]
use opentelemetry_http::HeaderInjector;
use parking_lot::Mutex;
use phoenix_rise_types::prelude::{
    AllMidsData, CandleData, CandlesSubscriptionRequest, ClientMessage, ExchangeMessage,
    ExchangeSnapshotEncoding, ExchangeSubscriptionRequest, FundingRateMessage,
    FundingRateSubscriptionRequest, L2BookUpdate, MarketStatsUpdate, MarketSubscriptionRequest,
    OrderbookSubscriptionRequest, ServerMessage, SubscriptionConfirmedMessage,
    SubscriptionErrorMessage, SubscriptionRequest, Timeframe, TraderStateServerMessage,
    TraderStateSubscriptionRequest, TradesMessage, TradesSubscriptionRequest,
};
use solana_pubkey::Pubkey;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
#[cfg(feature = "opentelemetry")]
use tokio_tungstenite::tungstenite::http::HeaderMap;
use tokio_tungstenite::tungstenite::http::header::{AUTHORIZATION, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};
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
    L2Book(mpsc::UnboundedSender<L2BookUpdate>),
    TraderState(mpsc::UnboundedSender<TraderStateServerMessage>),
    MarketStats(mpsc::UnboundedSender<MarketStatsUpdate>),
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
        Self::new_internal(&env.ws_url, false, None, None)
    }

    /// Create a new WebSocket client from a `PhoenixEnv` with connection status
    /// updates.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    pub fn from_env_with_connection_status(env: PhoenixEnv) -> Result<Self, PhoenixWsError> {
        Self::new_internal(&env.ws_url, true, None, None)
    }

    pub(crate) fn from_env_with_connection_status_and_auth(
        env: PhoenixEnv,
        auth_client: PhoenixHttpClient,
    ) -> Result<Self, PhoenixWsError> {
        Self::new_internal(&env.ws_url, true, Some(auth_client), None)
    }

    /// Create a new authenticated WebSocket client with connection status
    /// updates enabled.
    pub fn new_with_connection_status_and_auth(
        ws_url: &str,
        auth_client: PhoenixHttpClient,
    ) -> Result<Self, PhoenixWsError> {
        Self::new_internal(ws_url, true, Some(auth_client), None)
    }

    /// Create a new WebSocket client and connect to the server.
    ///
    /// # Arguments
    /// * `ws_url` - The WebSocket URL (e.g., "wss://api.phoenix.trade/ws")
    pub fn new(ws_url: &str) -> Result<Self, PhoenixWsError> {
        Self::new_internal(ws_url, false, None, None)
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
        Self::new_internal(ws_url, false, None, Some(Arc::new(provider)))
    }

    /// Create a new WebSocket client with connection status updates enabled.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    ///
    /// # Arguments
    /// * `ws_url` - The WebSocket URL (e.g., "wss://api.phoenix.trade/ws")
    pub fn new_with_connection_status(ws_url: &str) -> Result<Self, PhoenixWsError> {
        Self::new_internal(ws_url, true, None, None)
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
        Self::new_internal(ws_url, true, None, Some(Arc::new(provider)))
    }

    /// Internal constructor.
    fn new_internal(
        ws_url: &str,
        receiver_connection_status: bool,
        auth_client: Option<PhoenixHttpClient>,
        trace_context_provider: Option<TraceContextProvider>,
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

    /// Subscribe to orderbook updates for a given symbol.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `L2BookUpdate` messages. Drop the handle to unsubscribe.
    pub fn subscribe_to_orderbook(
        &self,
        symbol: String,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookUpdate>, SubscriptionHandle), PhoenixWsError> {
        self.subscribe_to_orderbook_with_options(symbol, false)
    }

    /// Subscribe to orderbook updates for a given symbol, optionally
    /// bypassing the commodities after-hours execution-price-band filter.
    ///
    /// Setting `bypass_execution_band = true` routes the subscription to a
    /// distinct stream that includes price levels outside the tradeable band
    /// (only observable during commodities after-hours).
    pub fn subscribe_to_orderbook_with_options(
        &self,
        symbol: String,
        bypass_execution_band: bool,
    ) -> Result<(mpsc::UnboundedReceiver<L2BookUpdate>, SubscriptionHandle), PhoenixWsError> {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key =
            SubscriptionKey::orderbook_with_options(symbol.clone(), bypass_execution_band);
        let request = SubscriptionRequest::Orderbook(OrderbookSubscriptionRequest {
            symbol,
            bypass_execution_band: Some(bypass_execution_band),
        });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::L2Book(tx),
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

    /// Subscribe to market updates for a given symbol.
    ///
    /// # Arguments
    /// * `symbol` - Market symbol (e.g., "SOL" or "BTC")
    ///
    /// Returns a tuple of (receiver, handle). The receiver will receive
    /// `MarketStatsUpdate` messages. Drop the handle to unsubscribe.
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
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let sub_key = SubscriptionKey::market(symbol.clone());
        let request = SubscriptionRequest::Market(MarketSubscriptionRequest { symbol });

        self.control_tx
            .send(ControlMessage::Subscribe {
                key: sub_key.clone(),
                request,
                subscriber: Subscriber::MarketStats(tx),
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
    ) {
        let mut subscribers: HashMap<SubscriptionKey, HashMap<u64, Subscriber>> = HashMap::new();
        let mut active_subscriptions: HashMap<SubscriptionKey, SubscriptionRequest> =
            HashMap::new();

        // Send connecting status
        if let Some(ref tx) = ws_connection_status_tx {
            let _ = tx.send(WsConnectionStatus::Connecting);
        }

        let ws_stream = match Self::connect(
            &url,
            auth_client.as_ref(),
            trace_context_provider.as_ref(),
        )
        .await
        {
            Ok(stream) => {
                info!("Connected to WebSocket: {}", url);
                if let Some(ref tx) = ws_connection_status_tx {
                    let _ = tx.send(WsConnectionStatus::Connected);
                }
                stream
            }
            Err(e) => {
                error!("Failed to connect: {:?}", e);
                if let Some(ref tx) = ws_connection_status_tx {
                    let _ = tx.send(WsConnectionStatus::ConnectionFailed);
                }
                return;
            }
        };

        let mut active_ws = ActiveWebSocket::new(ws_stream);

        // Main message loop
        loop {
            tokio::select! {
                            // Handle incoming WebSocket messages
                            ws_msg = active_ws.read.next() => {
                                match ws_msg {
                                    Some(Ok(Message::Text(text))) => {
                                        Self::process_message(
                                            text.as_bytes(),
                                            &subscribers,
                                            &ws_subscription_event_tx,
                                        );
                                    }
                                    Some(Ok(Message::Binary(data))) => {
                                        Self::process_message(
                                            &data,
                                            &subscribers,
                                            &ws_subscription_event_tx,
                                        );
                                    }
                                    Some(Ok(Message::Ping(data))) => {
                                        if let Err(e) = active_ws.sink.send(Message::Pong(data)).await {
                                            debug!("Failed to respond to Ping: {e:?}");
                                        }
                                    }
                                    Some(Ok(Message::Pong(_))) => {
                                        debug!("Received pong");
                                    }
                                    Some(Ok(Message::Close(frame))) => {
                                        let reason = Self::close_reason(frame.as_ref());
                                        if let Some(frame) = frame.as_ref() {
                                            warn!("WebSocket closed: code={}, reason={}", frame.code, frame.reason);
                                        } else {
                                            warn!("WebSocket closed without frame");
                                        }
                                        if Self::is_recoverable_auth_close(frame.as_ref()) {
                                            let Some(auth_client) = auth_client.as_ref() else {
                                                if let Some(ref tx) = ws_connection_status_tx {
                                                    let _ = tx.send(WsConnectionStatus::Disconnected(reason));
                                                }
                                                return;
                                            };
                                            match Self::refresh_and_reconnect(
                                                &url,
                                                auth_client,
                                                &active_subscriptions,
                                                &mut active_ws,
                                                ws_connection_status_tx.as_ref(),
            trace_context_provider.as_ref(),
                                            )
                                            .await
                                            {
                                                Ok(()) => continue,
                                                Err(error) => {
                                                    error!("Failed to refresh and reconnect WebSocket auth: {:?}", error);
                                                    if let Some(ref tx) = ws_connection_status_tx {
                                                        let _ = tx.send(WsConnectionStatus::Disconnected(reason));
                                                    }
                                                    return;
                                                }
                                            }
                                        }
                                        if let Some(ref tx) = ws_connection_status_tx {
                                            let _ = tx.send(WsConnectionStatus::Disconnected(reason));
                                        }
                                        return;
                                    }
                                    Some(Ok(_)) => {} // Ignore other message types
                                    Some(Err(e)) => {
                                        error!("WebSocket error: {:?}", e);
                                        if let Some(ref tx) = ws_connection_status_tx {
                                            let _ = tx.send(WsConnectionStatus::Disconnected(format!("error: {:?}", e)));
                                        }
                                        return;
                                    }
                                    None => {
                                        warn!("WebSocket stream ended");
                                        if let Some(ref tx) = ws_connection_status_tx {
                                            let _ = tx.send(WsConnectionStatus::Disconnected("stream ended".to_string()));
                                        }
                                        return;
                                    }
                                }
                            }

                            // Handle control messages
                            control_msg = control_rx.recv() => {
                                match control_msg {
                                    Some(ControlMessage::Subscribe { key, request, subscriber, subscriber_id }) => {
                                        // Get or create the inner HashMap for this key
                                        let key_subscribers = subscribers.entry(key.clone()).or_default();

                                        // Check if this is the first subscriber for this key
                                        let is_first_subscriber = key_subscribers.is_empty();

                                        // Insert the new subscriber
                                        key_subscribers.insert(subscriber_id, subscriber);

                                        // Only send wire subscription on first subscriber
                                        if is_first_subscriber {
                                            active_subscriptions.insert(key.clone(), request.clone());

                                            // Send subscription request
                                            let msg = ClientMessage::Subscribe { subscription: request };
                                            if let Ok(bytes) = serde_json::to_vec(&msg) {
                                                debug!("Sending subscription: {}", String::from_utf8_lossy(&bytes));
                                                if let Err(e) = active_ws.sink.send(Message::Binary(bytes.into())).await {
                                                    error!("Failed to send subscription: {:?}", e);
                                                }
                                            }
                                        }
                                    }
                                    Some(ControlMessage::Unsubscribe { key, subscriber_id }) => {
                                        // Remove this specific subscriber
                                        let should_unsubscribe = if let Some(key_subscribers) = subscribers.get_mut(&key) {
                                            key_subscribers.remove(&subscriber_id);
                                            key_subscribers.is_empty()
                                        } else {
                                            false
                                        };

                                        // If no subscribers remain for this key, unsubscribe from server
                                        if should_unsubscribe {
                                            subscribers.remove(&key);
                                            if let Some(request) = active_subscriptions.remove(&key) {
                                                // Send unsubscription request
                                                let msg = ClientMessage::Unsubscribe { subscription: request };
                                                if let Ok(bytes) = serde_json::to_vec(&msg) {
                                                    let _ = active_ws.sink.send(Message::Binary(bytes.into())).await;
                                                }
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

    async fn refresh_and_reconnect(
        url: &Url,
        auth_client: &PhoenixHttpClient,
        active_subscriptions: &HashMap<SubscriptionKey, SubscriptionRequest>,
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
        Self::resend_active_subscriptions(&mut active_ws.sink, active_subscriptions).await;
        Ok(())
    }

    async fn resend_active_subscriptions(
        ws_sink: &mut PhoenixWsSink,
        active_subscriptions: &HashMap<SubscriptionKey, SubscriptionRequest>,
    ) {
        for request in active_subscriptions.values() {
            let msg = ClientMessage::Subscribe {
                subscription: request.clone(),
            };
            if let Ok(bytes) = serde_json::to_vec(&msg) {
                debug!(
                    "Resending subscription after auth reconnect: {}",
                    String::from_utf8_lossy(&bytes)
                );
                if let Err(error) = ws_sink.send(Message::Binary(bytes.into())).await {
                    error!(
                        "Failed to resend subscription after auth reconnect: {:?}",
                        error
                    );
                }
            }
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
        if let Some(key_subscribers) = subscribers.get(key) {
            for (id, subscriber) in key_subscribers {
                if try_send(subscriber) {
                    debug!("Subscriber {} channel closed for {:?}", id, key);
                }
            }
        }
    }

    /// Handle an incoming WebSocket message.
    /// Process an incoming WebSocket data message (subscription confirmations,
    /// errors, and channel payloads).
    fn process_message(
        data: &[u8],
        subscribers: &HashMap<SubscriptionKey, HashMap<u64, Subscriber>>,
        subscription_event_tx: &mpsc::UnboundedSender<WsSubscriptionEvent>,
    ) {
        let text = match std::str::from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                debug!("Received non-UTF8 binary message: {:?}", e);
                return;
            }
        };
        debug!("Received message: {}", text);

        // Handle subscription confirmed messages
        if let Ok(confirmed) = serde_json::from_slice::<SubscriptionConfirmedMessage>(data) {
            debug!("Subscription confirmed: {:?}", confirmed.subscription);
            let _ = subscription_event_tx.send(WsSubscriptionEvent::Status {
                status: "subscribed".to_string(),
                subscription: confirmed.subscription,
            });
            return;
        }

        // Handle subscription error messages
        if let Ok(error) = serde_json::from_slice::<SubscriptionErrorMessage>(data) {
            error!(
                "Subscription error: code={}, message={}",
                error.code, error.message
            );
            let _ = subscription_event_tx.send(WsSubscriptionEvent::Error {
                subscription: Some(error.subscription),
                code: error.code,
                message: error.message,
            });
            return;
        }

        Self::handle_message(data, text, subscribers, subscription_event_tx);
    }

    /// Handle a data message and route to subscribers.
    fn handle_message(
        data: &[u8],
        text: &str,
        subscribers: &HashMap<SubscriptionKey, HashMap<u64, Subscriber>>,
        subscription_event_tx: &mpsc::UnboundedSender<WsSubscriptionEvent>,
    ) {
        match serde_json::from_slice::<ServerMessage>(data) {
            Ok(ServerMessage::AllMids(msg)) => {
                let key = SubscriptionKey::all_mids();
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::AllMids(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            Ok(ServerMessage::FundingRate(msg)) => {
                let key = SubscriptionKey::funding_rate(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::FundingRate(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            Ok(ServerMessage::Orderbook(msg)) => {
                let key = SubscriptionKey::orderbook_with_options(
                    msg.symbol.clone(),
                    msg.bypass_execution_band,
                );
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::L2Book(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            Ok(ServerMessage::TraderState(msg)) => {
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
            Ok(ServerMessage::Market(msg)) => {
                let key = SubscriptionKey::market(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub: &Subscriber| matches!(sub, Subscriber::MarketStats(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            Ok(ServerMessage::Trades(msg)) => {
                let key = SubscriptionKey::trades(msg.symbol.clone());
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::Trades(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            Ok(ServerMessage::Candles(msg)) => {
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
            Ok(ServerMessage::Exchange(msg)) => {
                let key = SubscriptionKey::exchange();
                Self::broadcast_to_subscribers(
                    subscribers,
                    &key,
                    |sub| matches!(sub, Subscriber::Exchange(tx) if tx.send(msg.clone()).is_err()),
                );
            }
            Ok(ServerMessage::Error(err)) => {
                error!("Server error: code={}, error={}", err.code, err.error);
                let _ = subscription_event_tx.send(WsSubscriptionEvent::Error {
                    subscription: None,
                    code: err.code.to_string(),
                    message: err.error,
                });
            }
            Ok(ServerMessage::SubscriptionStatus(status)) => {
                debug!(
                    "Subscription status: status={}, subscription={:?}",
                    status.status, status.subscription
                );
                let _ = subscription_event_tx.send(WsSubscriptionEvent::Status {
                    status: status.status,
                    subscription: status.subscription,
                });
            }
            Ok(_) => {
                // Ignore other message types
            }
            Err(e) => {
                debug!("Failed to parse message: {} - {}", e, text);
            }
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
    use futures_util::StreamExt as _;
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
    }

    #[test]
    fn process_message_forwards_subscription_status_event() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let subscribers = HashMap::new();
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

        PhoenixWSClient::process_message(json, &subscribers, &tx);

        let event = rx.try_recv().expect("subscription event should be sent");
        assert!(matches!(
            event,
            WsSubscriptionEvent::Status {
                status,
                subscription: SubscriptionRequest::TraderState(_),
            } if status == "subscribed"
        ));
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
        let mut client = PhoenixWSClient::new_internal(&ws_url, true, Some(auth_client), None)
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
        let mut client = PhoenixWSClient::new_internal(&ws_url, true, Some(auth_client), None)
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
    async fn exchange_store_keeps_receiving_updates_via_client_owned_pump() {
        let (ws_url, feed_tx) = spawn_exchange_feed_ws_server().await;
        let client = PhoenixWSClient::new(&ws_url).expect("ws client should build");
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

        // The pump owned by the client keeps applying deltas after
        // `exchange_store()` returned; no guard for the caller to hold.
        feed_tx
            .send(exchange_keys_delta_json(11, &rotated_root))
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
            exchange_status_bits: 129,
            exchange_status_features: vec!["initialized".to_string(), "active".to_string()],
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
}
