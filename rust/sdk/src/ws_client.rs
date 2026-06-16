//! WebSocket client for connecting to the Phoenix API.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use solana_pubkey::Pubkey;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{AUTHORIZATION, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::auth::AuthSession;
use crate::env::PhoenixEnv;
use crate::http_client::PhoenixHttpClient;
use crate::phoenix_rise_types::{
    AllMidsData, CandleData, CandlesSubscriptionRequest, ClientMessage, FundingRateMessage,
    FundingRateSubscriptionRequest, L2BookUpdate, MarketStatsUpdate, MarketSubscriptionRequest,
    OrderbookSubscriptionRequest, PhoenixWsError, ServerMessage, SubscriptionConfirmedMessage,
    SubscriptionErrorMessage, SubscriptionKey, SubscriptionRequest, Timeframe,
    TraderStateServerMessage, TraderStateSubscriptionRequest, TradesMessage,
    TradesSubscriptionRequest,
};

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

/// Subscriber channel for different message types.
enum Subscriber {
    AllMids(mpsc::UnboundedSender<AllMidsData>),
    FundingRate(mpsc::UnboundedSender<FundingRateMessage>),
    L2Book(mpsc::UnboundedSender<L2BookUpdate>),
    TraderState(mpsc::UnboundedSender<TraderStateServerMessage>),
    MarketStats(mpsc::UnboundedSender<MarketStatsUpdate>),
    Trades(mpsc::UnboundedSender<TradesMessage>),
    Candles(mpsc::UnboundedSender<CandleData>),
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

/// WebSocket client for Phoenix API.
///
/// Handles connection management and message routing to subscribers.
pub struct PhoenixWSClient {
    control_tx: mpsc::UnboundedSender<ControlMessage>,
    ws_url: Url,
    ws_connection_status_rx: Option<mpsc::UnboundedReceiver<WsConnectionStatus>>,
    next_subscriber_id: AtomicU64,
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
        Self::new_internal(&env.ws_url, false, None)
    }

    /// Create a new WebSocket client from a `PhoenixEnv` with connection status
    /// updates.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    pub fn from_env_with_connection_status(env: PhoenixEnv) -> Result<Self, PhoenixWsError> {
        Self::new_internal(&env.ws_url, true, None)
    }

    pub(crate) fn from_env_with_connection_status_and_auth(
        env: PhoenixEnv,
        auth_client: PhoenixHttpClient,
    ) -> Result<Self, PhoenixWsError> {
        Self::new_internal(&env.ws_url, true, Some(auth_client))
    }

    /// Create a new WebSocket client and connect to the server.
    ///
    /// # Arguments
    /// * `ws_url` - The WebSocket URL (e.g., "wss://api.phoenix.trade/ws")
    pub fn new(ws_url: &str) -> Result<Self, PhoenixWsError> {
        Self::new_internal(ws_url, false, None)
    }

    /// Create a new WebSocket client with connection status updates enabled.
    ///
    /// Use `connection_status_receiver()` to get the receiver for status
    /// updates.
    ///
    /// # Arguments
    /// * `ws_url` - The WebSocket URL (e.g., "wss://api.phoenix.trade/ws")
    pub fn new_with_connection_status(ws_url: &str) -> Result<Self, PhoenixWsError> {
        Self::new_internal(ws_url, true, None)
    }

    /// Internal constructor.
    fn new_internal(
        ws_url: &str,
        receiver_connection_status: bool,
        auth_client: Option<PhoenixHttpClient>,
    ) -> Result<Self, PhoenixWsError> {
        let url = Url::parse(ws_url)?;
        let (control_tx, control_rx) = mpsc::unbounded_channel();

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
            next_subscriber_id: AtomicU64::new(0),
        };

        // Spawn the connection manager task
        tokio::spawn(Self::connection_manager(
            url,
            auth_client,
            control_rx,
            ws_connection_status_tx,
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
    ) {
        let mut subscribers: HashMap<SubscriptionKey, HashMap<u64, Subscriber>> = HashMap::new();
        let mut active_subscriptions: HashMap<SubscriptionKey, SubscriptionRequest> =
            HashMap::new();

        // Send connecting status
        if let Some(ref tx) = ws_connection_status_tx {
            let _ = tx.send(WsConnectionStatus::Connecting);
        }

        // Connect to WebSocket
        let ws_stream = match Self::connect(&url, auth_client.as_ref()).await {
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

        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        // Main message loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                ws_msg = ws_stream.next() => {
                    match ws_msg {
                        Some(Ok(Message::Text(text))) => {
                            Self::process_message(text.as_bytes(), &subscribers);
                        }
                        Some(Ok(Message::Binary(data))) => {
                            Self::process_message(&data, &subscribers);
                        }
                        Some(Ok(Message::Ping(data))) => {
                            if let Err(e) = ws_sink.send(Message::Pong(data)).await {
                                debug!("Failed to respond to Ping: {e:?}");
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                        }
                        Some(Ok(Message::Close(frame))) => {
                            let reason = if let Some(frame) = frame {
                                warn!("WebSocket closed: code={}, reason={}", frame.code, frame.reason);
                                format!("closed: code={}, reason={}", frame.code, frame.reason)
                            } else {
                                warn!("WebSocket closed without frame");
                                "closed without frame".to_string()
                            };
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
                                    if let Err(e) = ws_sink.send(Message::Binary(bytes.into())).await {
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
                                        let _ = ws_sink.send(Message::Binary(bytes.into())).await;
                                    }
                                }
                            }
                        }
                        Some(ControlMessage::Shutdown) | None => {
                            info!("Shutting down WebSocket client");
                            let _ = ws_sink.close().await;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Connect to the WebSocket server.
    async fn connect(
        url: &Url,
        auth_client: Option<&PhoenixHttpClient>,
    ) -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, PhoenixWsError> {
        let auth_session = Self::auth_session_for_connect(auth_client).await?;
        let request = Self::connect_request(url, auth_session.as_ref())?;
        let ws_stream = match connect_async(request).await {
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
                let (ws_stream, _response) = connect_async(request).await?;
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
            return;
        }

        // Handle subscription error messages
        if let Ok(error) = serde_json::from_slice::<SubscriptionErrorMessage>(data) {
            error!(
                "Subscription error: code={}, message={}",
                error.code, error.message
            );
            return;
        }

        Self::handle_message(data, text, subscribers);
    }

    /// Handle a data message and route to subscribers.
    fn handle_message(
        data: &[u8],
        text: &str,
        subscribers: &HashMap<SubscriptionKey, HashMap<u64, Subscriber>>,
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
            Ok(ServerMessage::Error(err)) => {
                error!("Server error: code={}, error={}", err.code, err.error);
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

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    use super::*;

    #[test]
    fn connect_request_includes_auth_headers_when_session_is_available() {
        let access_token = test_jwt("ws-jti");
        let session = AuthSession::from_tokens(
            access_token.clone(),
            Some("refresh-token".to_string()),
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
            &format!("Bearer {access_token}")
        );
        assert_eq!(
            request.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(),
            &format!("phoenix-jwt, {access_token}")
        );
    }

    #[test]
    fn connect_request_omits_auth_headers_without_session() {
        let url = Url::parse("wss://api.example.test/ws").expect("url should parse");

        let request = PhoenixWSClient::connect_request(&url, None).expect("request should build");

        assert!(request.headers().get(AUTHORIZATION).is_none());
        assert!(request.headers().get(SEC_WEBSOCKET_PROTOCOL).is_none());
    }

    fn test_jwt(jti: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"jti":"{jti}"}}"#));
        format!("{header}.{payload}.sig")
    }
}
