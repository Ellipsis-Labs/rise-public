//! Unified Phoenix client managing WebSocket subscriptions, reconnection,
//! and receiver-based events.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{BoxStream, StreamExt};
use parking_lot::Mutex;
use phoenix_rise_types::market_state::Market;
use phoenix_rise_types::ws::ServerMessage;
use solana_pubkey::Pubkey;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio_stream::StreamMap;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info, warn};

use crate::client_types::{
    ClientCommand, ClientSubscriptionId, LogicalSubscription, MarginTrigger, PhoenixClientError,
    PhoenixClientEvent, PhoenixClientSubscriptionHandle, PhoenixSubscription, RuntimeState,
};
use crate::env::PhoenixEnv;
use crate::http_client::PhoenixHttpClient;
use crate::metadata::PhoenixMetadata;
use crate::subscription_key::SubscriptionKey;
use crate::trader_key::TraderKey;
use crate::trader_state::Trader;
use crate::ws_client::{PhoenixWSClient, SubscriptionHandle, WsConnectionStatus};
use crate::ws_error::PhoenixWsError;

/// Internal shared state for PhoenixClient handle clones.
struct PhoenixClientInner {
    http_client: PhoenixHttpClient,
    cmd_tx: mpsc::UnboundedSender<ClientCommand>,
    connection_status_rx: watch::Receiver<WsConnectionStatus>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
}

/// Unified high-level client for the Phoenix perpetuals exchange.
///
/// - Auto-reconnects WebSocket connections
/// - Keeps state lock-free (single-owner state inside background task)
/// - Emits subscription updates via receivers (no callbacks)
pub struct PhoenixClient {
    inner: Arc<PhoenixClientInner>,
}

impl Clone for PhoenixClient {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PhoenixClient {
    /// Create a new PhoenixClient from environment variables.
    pub async fn new_from_env() -> Result<Self, PhoenixClientError> {
        Self::from_env(PhoenixEnv::load()).await
    }

    /// Create a new authenticated PhoenixClient from environment variables.
    pub async fn new_from_env_with_auth() -> Result<Self, PhoenixClientError> {
        Self::from_env_with_auth(PhoenixEnv::load()).await
    }

    /// Create a new PhoenixClient from a [`PhoenixEnv`].
    pub async fn from_env(env: PhoenixEnv) -> Result<Self, PhoenixClientError> {
        let http_client =
            PhoenixHttpClient::from_env(env.clone()).map_err(PhoenixClientError::Http)?;
        Self::from_env_with_http_client(env, http_client).await
    }

    /// Create a new authenticated PhoenixClient from a [`PhoenixEnv`].
    pub async fn from_env_with_auth(env: PhoenixEnv) -> Result<Self, PhoenixClientError> {
        let http_client =
            PhoenixHttpClient::from_env_with_auth(env.clone()).map_err(PhoenixClientError::Http)?;
        Self::from_env_with_http_client(env, http_client).await
    }

    /// Create a new PhoenixClient from an environment and HTTP client.
    ///
    /// If the HTTP client has auth configured, the WebSocket connection loop
    /// uses the same auth lifecycle for handshake JWT refreshes.
    pub async fn from_env_with_http_client(
        env: PhoenixEnv,
        http_client: PhoenixHttpClient,
    ) -> Result<Self, PhoenixClientError> {
        let exchange_response = http_client
            .get_exchange()
            .await
            .map_err(PhoenixClientError::Http)?;
        let metadata = PhoenixMetadata::new(exchange_response.into());

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (connection_status_tx, connection_status_rx) = watch::channel(
            WsConnectionStatus::Disconnected("not connected".to_string()),
        );
        let ws_http_client = http_client.clone();
        let inner = Arc::new(PhoenixClientInner {
            http_client,
            cmd_tx,
            connection_status_rx,
            task_handle: Mutex::new(None),
        });

        let client = Self {
            inner: Arc::clone(&inner),
        };

        let task_handle = tokio::spawn(Self::connection_loop(
            env,
            ws_http_client,
            cmd_rx,
            metadata,
            connection_status_tx,
        ));
        *client.inner.task_handle.lock() = Some(task_handle);

        Ok(client)
    }

    /// Subscribe to a high-level subscription and receive events.
    pub async fn subscribe(
        &self,
        subscription: PhoenixSubscription,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<PhoenixClientEvent>,
            PhoenixClientSubscriptionHandle,
        ),
        PhoenixClientError,
    > {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.inner
            .cmd_tx
            .send(ClientCommand::Subscribe {
                subscription,
                response_tx,
            })
            .map_err(|_| PhoenixClientError::SendFailed)?;

        let (subscription_id, rx) = response_rx
            .await
            .map_err(|_| PhoenixClientError::ResponseDropped)??;

        Ok((
            rx,
            PhoenixClientSubscriptionHandle {
                cmd_tx: self.inner.cmd_tx.clone(),
                subscription_id,
            },
        ))
    }

    /// Access the HTTP client for REST API calls.
    pub fn http(&self) -> &PhoenixHttpClient {
        &self.inner.http_client
    }

    /// Return the latest observed WebSocket connection status.
    pub fn connection_status(&self) -> WsConnectionStatus {
        self.inner.connection_status_rx.borrow().clone()
    }

    /// Subscribe to WebSocket connection status updates for this client.
    pub fn subscribe_connection_status(&self) -> watch::Receiver<WsConnectionStatus> {
        self.inner.connection_status_rx.clone()
    }

    /// Signal the background task to shut down.
    pub fn shutdown(&self) {
        let _ = self.inner.cmd_tx.send(ClientCommand::Shutdown);
    }

    /// Block until the background task exits (e.g. after `shutdown()`).
    pub async fn run(&self) {
        let handle = self.inner.task_handle.lock().take();
        if let Some(handle) = handle {
            let _ = handle.await;
        }
    }

    async fn connection_loop(
        env: PhoenixEnv,
        http_client: PhoenixHttpClient,
        mut cmd_rx: mpsc::UnboundedReceiver<ClientCommand>,
        metadata: PhoenixMetadata,
        connection_status_tx: watch::Sender<WsConnectionStatus>,
    ) {
        let mut runtime_state = RuntimeState::new(metadata);
        let mut logical_subscriptions: HashMap<ClientSubscriptionId, LogicalSubscription> =
            HashMap::new();
        let mut subscribers_by_key: HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>> =
            HashMap::new();
        let mut dependency_refcounts: HashMap<SubscriptionKey, usize> = HashMap::new();
        let mut next_subscription_id: ClientSubscriptionId = 1;

        let mut backoff_ms = 1_000u64;
        const MAX_BACKOFF_MS: u64 = 30_000;

        'reconnect: loop {
            let mut ws_client = match PhoenixWSClient::from_env_with_connection_status_and_auth(
                env.clone(),
                http_client.clone(),
            ) {
                Ok(client) => {
                    backoff_ms = 1_000;
                    client
                }
                Err(e) => {
                    error!("Failed to create WS client: {:?}", e);
                    let _ = connection_status_tx.send(WsConnectionStatus::ConnectionFailed);
                    if !Self::wait_with_command_processing(
                        Duration::from_millis(backoff_ms),
                        &mut cmd_rx,
                        &runtime_state,
                        &mut logical_subscriptions,
                        &mut subscribers_by_key,
                        &mut dependency_refcounts,
                        &mut next_subscription_id,
                    )
                    .await
                    {
                        return;
                    }
                    backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                    continue 'reconnect;
                }
            };

            let mut status_rx = match ws_client.connection_status_receiver() {
                Some(status_rx) => status_rx,
                None => {
                    error!(
                        "PhoenixWSClient missing status receiver; expected \
                         from_env_with_connection_status to enable it"
                    );
                    let _ = connection_status_tx.send(WsConnectionStatus::Disconnected(
                        "connection status unavailable".to_string(),
                    ));
                    return;
                }
            };
            let mut ws_handles: HashMap<SubscriptionKey, SubscriptionHandle> = HashMap::new();
            let mut ws_streams: StreamMap<SubscriptionKey, BoxStream<'static, ServerMessage>> =
                StreamMap::new();

            for key in dependency_refcounts.keys() {
                if let Err(e) =
                    Self::open_dependency(&ws_client, key, &mut ws_handles, &mut ws_streams)
                {
                    warn!("Failed to restore subscription {:?}: {:?}", key, e);
                }
            }

            loop {
                tokio::select! {
                    status = status_rx.recv() => {
                        if let Some(status) = status {
                            let _ = connection_status_tx.send(status.clone());
                            let should_reconnect = match status {
                                WsConnectionStatus::Connected => {
                                    debug!("PhoenixClient WebSocket connected");
                                    backoff_ms = 1_000;
                                    false
                                }
                                WsConnectionStatus::Connecting => {
                                    debug!("PhoenixClient WebSocket connecting");
                                    false
                                }
                                WsConnectionStatus::Disconnected(reason) => {
                                    warn!("PhoenixClient WebSocket disconnected: {}", reason);
                                    true
                                }
                                WsConnectionStatus::ConnectionFailed => {
                                    warn!("PhoenixClient WebSocket connection failed");
                                    true
                                }
                            };

                            if should_reconnect {
                                drop(ws_handles);
                                if !Self::wait_with_command_processing(
                                    Duration::from_millis(backoff_ms),
                                    &mut cmd_rx,
                                    &runtime_state,
                                    &mut logical_subscriptions,
                                    &mut subscribers_by_key,
                                    &mut dependency_refcounts,
                                    &mut next_subscription_id,
                                ).await {
                                    return;
                                }
                                backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                                continue 'reconnect;
                            }
                        } else {
                            warn!("PhoenixClient connection status channel closed");
                            let _ = connection_status_tx.send(WsConnectionStatus::Disconnected(
                                "connection status channel closed".to_string(),
                            ));
                            drop(ws_handles);
                            continue 'reconnect;
                        }
                    }

                    cmd = cmd_rx.recv() => {
                        if !Self::handle_command_connected(
                            cmd,
                            &runtime_state,
                            &ws_client,
                            &mut ws_handles,
                            &mut ws_streams,
                            &mut logical_subscriptions,
                            &mut subscribers_by_key,
                            &mut dependency_refcounts,
                            &mut next_subscription_id,
                        ) {
                            info!("PhoenixClient shutting down");
                            return;
                        }
                    }

                    ws_message = ws_streams.next(), if !ws_streams.is_empty() => {
                        if let Some((key, message)) = ws_message {
                            let stale = Self::handle_ws_message(
                                &key,
                                message,
                                &mut runtime_state,
                                &mut logical_subscriptions,
                                &subscribers_by_key,
                            );

                            if !stale.is_empty() {
                                let deactivated = Self::remove_subscriptions(
                                    &stale,
                                    &mut logical_subscriptions,
                                    &mut subscribers_by_key,
                                    &mut dependency_refcounts,
                                );

                                for key in deactivated {
                                    Self::close_dependency(&key, &mut ws_handles, &mut ws_streams);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn wait_with_command_processing(
        delay: Duration,
        cmd_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
        runtime_state: &RuntimeState,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &mut HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
        dependency_refcounts: &mut HashMap<SubscriptionKey, usize>,
        next_subscription_id: &mut ClientSubscriptionId,
    ) -> bool {
        let sleep = tokio::time::sleep(delay);
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                _ = &mut sleep => {
                    return true;
                }
                cmd = cmd_rx.recv() => {
                    if !Self::handle_command_offline(
                        cmd,
                        runtime_state,
                        logical_subscriptions,
                        subscribers_by_key,
                        dependency_refcounts,
                        next_subscription_id,
                    ) {
                        return false;
                    }
                }
            }
        }
    }

    fn handle_command_offline(
        cmd: Option<ClientCommand>,
        runtime_state: &RuntimeState,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &mut HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
        dependency_refcounts: &mut HashMap<SubscriptionKey, usize>,
        next_subscription_id: &mut ClientSubscriptionId,
    ) -> bool {
        match cmd {
            Some(ClientCommand::Subscribe {
                subscription,
                response_tx,
            }) => {
                let result = Self::register_subscription(
                    subscription,
                    runtime_state,
                    logical_subscriptions,
                    subscribers_by_key,
                    dependency_refcounts,
                    next_subscription_id,
                )
                .map(|(id, rx, _)| (id, rx));
                let _ = response_tx.send(result);
                true
            }
            Some(ClientCommand::Unsubscribe { subscription_id }) => {
                let _ = Self::remove_subscription(
                    subscription_id,
                    logical_subscriptions,
                    subscribers_by_key,
                    dependency_refcounts,
                );
                true
            }
            Some(ClientCommand::Shutdown) | None => false,
        }
    }

    fn handle_command_connected(
        cmd: Option<ClientCommand>,
        runtime_state: &RuntimeState,
        ws_client: &PhoenixWSClient,
        ws_handles: &mut HashMap<SubscriptionKey, SubscriptionHandle>,
        ws_streams: &mut StreamMap<SubscriptionKey, BoxStream<'static, ServerMessage>>,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &mut HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
        dependency_refcounts: &mut HashMap<SubscriptionKey, usize>,
        next_subscription_id: &mut ClientSubscriptionId,
    ) -> bool {
        match cmd {
            Some(ClientCommand::Subscribe {
                subscription,
                response_tx,
            }) => {
                let result = match Self::register_subscription(
                    subscription,
                    runtime_state,
                    logical_subscriptions,
                    subscribers_by_key,
                    dependency_refcounts,
                    next_subscription_id,
                ) {
                    Ok((id, rx, activated_keys)) => {
                        for key in &activated_keys {
                            if let Err(e) =
                                Self::open_dependency(ws_client, key, ws_handles, ws_streams)
                            {
                                warn!("Failed to open subscription {:?}: {:?}", key, e);
                            }
                        }
                        Ok((id, rx))
                    }
                    Err(e) => Err(e),
                };

                let _ = response_tx.send(result);
                true
            }
            Some(ClientCommand::Unsubscribe { subscription_id }) => {
                let deactivated = Self::remove_subscription(
                    subscription_id,
                    logical_subscriptions,
                    subscribers_by_key,
                    dependency_refcounts,
                );

                for key in deactivated {
                    Self::close_dependency(&key, ws_handles, ws_streams);
                }

                true
            }
            Some(ClientCommand::Shutdown) | None => false,
        }
    }

    fn register_subscription(
        subscription: PhoenixSubscription,
        runtime_state: &RuntimeState,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &mut HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
        dependency_refcounts: &mut HashMap<SubscriptionKey, usize>,
        next_subscription_id: &mut ClientSubscriptionId,
    ) -> Result<
        (
            ClientSubscriptionId,
            mpsc::UnboundedReceiver<PhoenixClientEvent>,
            Vec<SubscriptionKey>,
        ),
        PhoenixClientError,
    > {
        let dependencies = Self::resolve_dependencies(&subscription, &runtime_state.metadata);
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let subscription_id = *next_subscription_id;
        *next_subscription_id = next_subscription_id.saturating_add(1);

        let mut activated = Vec::new();

        for key in &dependencies {
            subscribers_by_key
                .entry(key.clone())
                .or_default()
                .insert(subscription_id);

            let count = dependency_refcounts.entry(key.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                activated.push(key.clone());
            }
        }

        logical_subscriptions.insert(
            subscription_id,
            LogicalSubscription {
                subscription,
                dependencies,
                event_tx,
            },
        );

        Ok((subscription_id, event_rx, activated))
    }

    fn remove_subscriptions(
        subscription_ids: &[ClientSubscriptionId],
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &mut HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
        dependency_refcounts: &mut HashMap<SubscriptionKey, usize>,
    ) -> Vec<SubscriptionKey> {
        let mut deactivated = HashSet::new();
        for subscription_id in subscription_ids {
            for key in Self::remove_subscription(
                *subscription_id,
                logical_subscriptions,
                subscribers_by_key,
                dependency_refcounts,
            ) {
                deactivated.insert(key);
            }
        }
        deactivated.into_iter().collect()
    }

    fn remove_subscription(
        subscription_id: ClientSubscriptionId,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &mut HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
        dependency_refcounts: &mut HashMap<SubscriptionKey, usize>,
    ) -> Vec<SubscriptionKey> {
        let mut deactivated = Vec::new();

        let Some(logical) = logical_subscriptions.remove(&subscription_id) else {
            return deactivated;
        };

        for key in logical.dependencies {
            if let Some(ids) = subscribers_by_key.get_mut(&key) {
                ids.remove(&subscription_id);
                if ids.is_empty() {
                    subscribers_by_key.remove(&key);
                }
            }

            if let Some(count) = dependency_refcounts.get_mut(&key) {
                if *count > 1 {
                    *count -= 1;
                } else {
                    dependency_refcounts.remove(&key);
                    deactivated.push(key);
                }
            }
        }

        deactivated
    }

    fn resolve_dependencies(
        subscription: &PhoenixSubscription,
        metadata: &PhoenixMetadata,
    ) -> HashSet<SubscriptionKey> {
        let mut dependencies = HashSet::new();

        match subscription {
            PhoenixSubscription::Key(key) => {
                dependencies.insert(key.clone());
            }
            PhoenixSubscription::Market {
                symbol,
                candle_timeframes,
                include_trades,
            } => {
                let symbol = symbol.to_ascii_uppercase();

                dependencies.insert(SubscriptionKey::market(symbol.clone()));
                dependencies.insert(SubscriptionKey::orderbook(symbol.clone()));
                dependencies.insert(SubscriptionKey::funding_rate(symbol.clone()));

                for timeframe in candle_timeframes {
                    dependencies.insert(SubscriptionKey::candles(symbol.clone(), *timeframe));
                }

                if *include_trades {
                    dependencies.insert(SubscriptionKey::trades(symbol));
                }
            }
            PhoenixSubscription::TraderMargin {
                authority,
                trader_pda_index,
                market_symbols,
                ..
            } => {
                dependencies.insert(SubscriptionKey::trader(authority, *trader_pda_index));

                if market_symbols.is_empty() {
                    for symbol in metadata.exchange().markets.keys() {
                        dependencies.insert(SubscriptionKey::market(symbol.clone()));
                    }
                } else {
                    for symbol in market_symbols {
                        dependencies.insert(SubscriptionKey::market(symbol.to_ascii_uppercase()));
                    }
                }
            }
        }

        dependencies
    }

    fn open_dependency(
        ws_client: &PhoenixWSClient,
        key: &SubscriptionKey,
        ws_handles: &mut HashMap<SubscriptionKey, SubscriptionHandle>,
        ws_streams: &mut StreamMap<SubscriptionKey, BoxStream<'static, ServerMessage>>,
    ) -> Result<(), PhoenixWsError> {
        if ws_handles.contains_key(key) {
            return Ok(());
        }

        match key {
            SubscriptionKey::AllMids => {
                let (rx, handle) = ws_client.subscribe_to_all_mids()?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::AllMids)
                        .boxed(),
                );
            }
            SubscriptionKey::FundingRate { symbol } => {
                let (rx, handle) = ws_client.subscribe_to_funding_rate(symbol.clone())?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::FundingRate)
                        .boxed(),
                );
            }
            SubscriptionKey::Orderbook {
                symbol,
                bypass_execution_band,
            } => {
                let (rx, handle) = ws_client
                    .subscribe_to_orderbook_with_options(symbol.clone(), *bypass_execution_band)?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::Orderbook)
                        .boxed(),
                );
            }
            SubscriptionKey::TraderState {
                authority,
                trader_pda_index,
            } => {
                let authority = match authority.parse::<Pubkey>() {
                    Ok(authority) => authority,
                    Err(e) => {
                        warn!(
                            "Invalid trader authority in subscription key {}: {}",
                            authority, e
                        );
                        return Ok(());
                    }
                };

                let (rx, handle) =
                    ws_client.subscribe_to_trader_state_with_pda(&authority, *trader_pda_index)?;

                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::TraderState)
                        .boxed(),
                );
            }
            SubscriptionKey::Market { symbol } => {
                let (rx, handle) = ws_client.subscribe_to_market(symbol.clone())?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::Market)
                        .boxed(),
                );
            }
            SubscriptionKey::Trades { symbol } => {
                let (rx, handle) = ws_client.subscribe_to_trades(symbol.clone())?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::Trades)
                        .boxed(),
                );
            }
            SubscriptionKey::Candles { symbol, timeframe } => {
                let (rx, handle) = ws_client.subscribe_to_candles(symbol.clone(), *timeframe)?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::Candles)
                        .boxed(),
                );
            }
            SubscriptionKey::Exchange => {
                let (rx, handle) = ws_client.subscribe_to_exchange()?;
                ws_handles.insert(key.clone(), handle);
                ws_streams.insert(
                    key.clone(),
                    UnboundedReceiverStream::new(rx)
                        .map(ServerMessage::Exchange)
                        .boxed(),
                );
            }
        }

        Ok(())
    }

    fn close_dependency(
        key: &SubscriptionKey,
        ws_handles: &mut HashMap<SubscriptionKey, SubscriptionHandle>,
        ws_streams: &mut StreamMap<SubscriptionKey, BoxStream<'static, ServerMessage>>,
    ) {
        ws_handles.remove(key);
        ws_streams.remove(key);
    }

    fn handle_ws_message(
        key: &SubscriptionKey,
        message: ServerMessage,
        runtime_state: &mut RuntimeState,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
    ) -> Vec<ClientSubscriptionId> {
        let mut stale = Vec::new();

        match message {
            ServerMessage::Market(update) => {
                let symbol = update.symbol.clone();
                let prev_market = runtime_state.markets.get(&symbol).cloned();

                let market = runtime_state
                    .markets
                    .entry(symbol.clone())
                    .or_insert_with(|| Market::from_symbol(symbol.clone()));
                market.apply_market_stats_update(&update);

                if let Err(e) = runtime_state.metadata.apply_market_stats(&update) {
                    warn!("Failed to apply market stats for {}: {}", symbol, e);
                }

                stale.extend(Self::dispatch_raw_event(
                    key,
                    PhoenixClientEvent::MarketUpdate {
                        symbol,
                        prev_market,
                        update: update.clone(),
                    },
                    logical_subscriptions,
                    subscribers_by_key,
                ));

                stale.extend(Self::dispatch_margin_events(
                    key,
                    MarginTrigger::Market(update),
                    None,
                    runtime_state,
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::Orderbook(update) => {
                let symbol = update.symbol.clone();
                let prev_market = runtime_state.markets.get(&symbol).cloned();

                let market = runtime_state
                    .markets
                    .entry(symbol.clone())
                    .or_insert_with(|| Market::from_symbol(symbol.clone()));
                market.apply_l2_book_update(&update);

                stale.extend(Self::dispatch_raw_event(
                    key,
                    PhoenixClientEvent::OrderbookUpdate {
                        symbol,
                        prev_market,
                        update,
                    },
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::TraderState(update) => {
                let trader_key = match key {
                    SubscriptionKey::TraderState {
                        authority,
                        trader_pda_index,
                    } => {
                        let authority_pubkey = match authority.parse::<Pubkey>() {
                            Ok(authority) => authority,
                            Err(e) => {
                                warn!("Invalid trader authority {}: {}", authority, e);
                                return stale;
                            }
                        };
                        SubscriptionKey::trader(&authority_pubkey, *trader_pda_index)
                    }
                    _ => {
                        warn!("Received trader message for non-trader key: {:?}", key);
                        return stale;
                    }
                };

                let prev_trader = runtime_state.traders.get(&trader_key).cloned();

                let (authority, pda_index) = match &trader_key {
                    SubscriptionKey::TraderState {
                        authority,
                        trader_pda_index,
                    } => (authority, *trader_pda_index),
                    _ => unreachable!(),
                };

                let authority_pubkey = match authority.parse::<Pubkey>() {
                    Ok(authority) => authority,
                    Err(e) => {
                        warn!("Invalid trader authority {}: {}", authority, e);
                        return stale;
                    }
                };

                let trader = runtime_state
                    .traders
                    .entry(trader_key.clone())
                    .or_insert_with(|| {
                        Trader::new(TraderKey::from_authority_with_idx(
                            authority_pubkey,
                            pda_index,
                            0,
                        ))
                    });
                trader.apply_update(&update);

                stale.extend(Self::dispatch_raw_event(
                    &trader_key,
                    PhoenixClientEvent::TraderUpdate {
                        key: trader_key.clone(),
                        prev_trader: prev_trader.clone(),
                        update: update.clone(),
                    },
                    logical_subscriptions,
                    subscribers_by_key,
                ));

                stale.extend(Self::dispatch_margin_events(
                    &trader_key,
                    MarginTrigger::Trader(update),
                    prev_trader,
                    runtime_state,
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::AllMids(update) => {
                let prev_mids = runtime_state.mids.clone();
                runtime_state.mids = update.mids.clone();

                stale.extend(Self::dispatch_raw_event(
                    key,
                    PhoenixClientEvent::MidsUpdate { prev_mids, update },
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::FundingRate(update) => {
                let symbol = update.symbol.clone();
                let prev_funding_rate = runtime_state
                    .funding_rates
                    .insert(symbol.clone(), update.clone());

                stale.extend(Self::dispatch_raw_event(
                    key,
                    PhoenixClientEvent::FundingRateUpdate {
                        symbol,
                        prev_funding_rate,
                        update,
                    },
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::Candles(update) => {
                let (symbol, timeframe) = match key {
                    SubscriptionKey::Candles { symbol, timeframe } => (symbol.clone(), *timeframe),
                    _ => {
                        warn!("Received candle message for non-candle key: {:?}", key);
                        return stale;
                    }
                };

                let prev_candle = runtime_state
                    .candles
                    .insert((symbol.clone(), timeframe), update.clone());

                stale.extend(Self::dispatch_raw_event(
                    key,
                    PhoenixClientEvent::CandleUpdate {
                        symbol,
                        timeframe,
                        prev_candle,
                        update,
                    },
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::Trades(update) => {
                let symbol = update.symbol.clone();
                let prev_trades = runtime_state.trades.insert(symbol.clone(), update.clone());

                stale.extend(Self::dispatch_raw_event(
                    key,
                    PhoenixClientEvent::TradesUpdate {
                        symbol,
                        prev_trades,
                        update,
                    },
                    logical_subscriptions,
                    subscribers_by_key,
                ));
            }
            ServerMessage::Error(_)
            | ServerMessage::SubscriptionStatus(_)
            | ServerMessage::Exchange(_)
            | ServerMessage::Other => {}
        }

        stale
    }

    fn dispatch_raw_event(
        key: &SubscriptionKey,
        event: PhoenixClientEvent,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
    ) -> Vec<ClientSubscriptionId> {
        let mut stale = Vec::new();

        let Some(subscription_ids) = subscribers_by_key.get(key) else {
            return stale;
        };

        for subscription_id in subscription_ids {
            let Some(logical) = logical_subscriptions.get(subscription_id) else {
                continue;
            };

            if matches!(
                logical.subscription,
                PhoenixSubscription::TraderMargin { .. }
            ) {
                continue;
            }

            if logical.event_tx.send(event.clone()).is_err() {
                stale.push(*subscription_id);
            }
        }

        stale
    }

    fn dispatch_margin_events(
        trigger_key: &SubscriptionKey,
        trigger: MarginTrigger,
        prev_trader: Option<Trader>,
        runtime_state: &RuntimeState,
        logical_subscriptions: &mut HashMap<ClientSubscriptionId, LogicalSubscription>,
        subscribers_by_key: &HashMap<SubscriptionKey, HashSet<ClientSubscriptionId>>,
    ) -> Vec<ClientSubscriptionId> {
        let mut stale = Vec::new();

        let Some(subscription_ids) = subscribers_by_key.get(trigger_key) else {
            return stale;
        };

        for subscription_id in subscription_ids {
            let Some(logical) = logical_subscriptions.get(subscription_id) else {
                continue;
            };

            let PhoenixSubscription::TraderMargin {
                authority,
                trader_pda_index,
                subaccount_index,
                ..
            } = &logical.subscription
            else {
                continue;
            };

            let trader_key = SubscriptionKey::trader(authority, *trader_pda_index);
            let margin = runtime_state
                .traders
                .get(&trader_key)
                .and_then(|trader| trader.subaccount(*subaccount_index))
                .and_then(|subaccount| {
                    subaccount
                        .to_trader_portfolio()
                        .compute_margin(runtime_state.metadata.all_perp_asset_metadata())
                        .ok()
                });

            let event = PhoenixClientEvent::MarginUpdate {
                trader_key,
                trigger: trigger.clone(),
                margin,
                metadata: runtime_state.metadata.clone(),
                prev_trader: prev_trader.clone(),
            };

            if logical.event_tx.send(event).is_err() {
                stale.push(*subscription_id);
            }
        }

        stale
    }
}
