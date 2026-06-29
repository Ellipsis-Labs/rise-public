//! Client-side types used by higher-level SDK clients.

use std::collections::{HashMap, HashSet};

use phoenix_rise_math::TraderPortfolioMargin;
use phoenix_rise_types::market_state::Market;
use solana_pubkey::Pubkey;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::http_error::PhoenixHttpError;
use crate::metadata::PhoenixMetadata;
use crate::subscription_key::SubscriptionKey;
use crate::trader_state::Trader;
use crate::types::prelude::{
    AllMidsData, CandleData, FundingRateMessage, L2BookUpdate, MarketStatsUpdate, Timeframe,
    TraderStateServerMessage, TradesMessage,
};
use crate::ws_error::PhoenixWsError;

/// Client-side logical subscription identifier.
pub type ClientSubscriptionId = u64;

/// Errors that can occur when using higher-level Phoenix clients.
#[derive(Debug, Error)]
pub enum PhoenixClientError {
    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(PhoenixWsError),
    /// HTTP error.
    #[error("HTTP error: {0}")]
    Http(PhoenixHttpError),
    /// Client is shutting down.
    #[error("Client is shutting down")]
    Shutdown,
    /// Failed to send command to background task.
    #[error("Failed to send command")]
    SendFailed,
    /// Failed to receive response from background task.
    #[error("Failed to receive response")]
    ResponseDropped,
}

/// High-level client subscription request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhoenixSubscription {
    /// Subscribe directly to a single low-level key.
    Key(SubscriptionKey),
    /// Subscribe to a market bundle.
    ///
    /// Includes market stats, orderbook, funding rate, optional trades,
    /// and optional candle streams.
    Market {
        symbol: String,
        candle_timeframes: Vec<Timeframe>,
        include_trades: bool,
    },
    /// Subscribe to trader margin updates.
    ///
    /// If `market_symbols` is empty, all markets from metadata are tracked.
    TraderMargin {
        authority: Pubkey,
        trader_pda_index: u8,
        subaccount_index: u8,
        market_symbols: Vec<String>,
    },
}

impl PhoenixSubscription {
    /// Create a market bundle subscription with default options.
    pub fn market(symbol: impl Into<String>) -> Self {
        Self::Market {
            symbol: symbol.into().to_ascii_uppercase(),
            candle_timeframes: Vec::new(),
            include_trades: false,
        }
    }

    /// Create a trader margin subscription for a trader.
    pub fn trader_margin(authority: Pubkey, trader_pda_index: u8) -> Self {
        Self::TraderMargin {
            authority,
            trader_pda_index,
            subaccount_index: 0,
            market_symbols: Vec::new(),
        }
    }
}

/// Message that triggered a margin recomputation.
#[derive(Debug, Clone)]
pub enum MarginTrigger {
    /// Trader state update triggered recomputation.
    Trader(TraderStateServerMessage),
    /// Market stats update triggered recomputation.
    Market(MarketStatsUpdate),
}

/// Event emitted by high-level client subscription receivers.
#[derive(Debug, Clone)]
pub enum PhoenixClientEvent {
    /// Market stats update and previous market snapshot.
    MarketUpdate {
        symbol: String,
        prev_market: Option<Market>,
        update: MarketStatsUpdate,
    },
    /// Orderbook update and previous market snapshot.
    OrderbookUpdate {
        symbol: String,
        prev_market: Option<Market>,
        update: L2BookUpdate,
    },
    /// Trader state update and previous trader snapshot.
    TraderUpdate {
        key: SubscriptionKey,
        prev_trader: Option<Trader>,
        update: TraderStateServerMessage,
    },
    /// All mids update and previous mids snapshot.
    MidsUpdate {
        prev_mids: HashMap<String, f64>,
        update: AllMidsData,
    },
    /// Funding rate update and previous funding rate snapshot.
    FundingRateUpdate {
        symbol: String,
        prev_funding_rate: Option<FundingRateMessage>,
        update: FundingRateMessage,
    },
    /// Candle update and previous candle snapshot.
    CandleUpdate {
        symbol: String,
        timeframe: Timeframe,
        prev_candle: Option<CandleData>,
        update: CandleData,
    },
    /// Trades update and previous trades snapshot.
    TradesUpdate {
        symbol: String,
        prev_trades: Option<TradesMessage>,
        update: TradesMessage,
    },
    /// Margin update carrying trigger + computed margin + metadata snapshot.
    MarginUpdate {
        trader_key: SubscriptionKey,
        trigger: MarginTrigger,
        margin: Option<TraderPortfolioMargin>,
        metadata: PhoenixMetadata,
        prev_trader: Option<Trader>,
    },
}

/// Internal command channel messages for higher-level clients.
pub enum ClientCommand {
    /// Register a logical subscription.
    Subscribe {
        subscription: PhoenixSubscription,
        response_tx: oneshot::Sender<
            Result<
                (
                    ClientSubscriptionId,
                    mpsc::UnboundedReceiver<PhoenixClientEvent>,
                ),
                PhoenixClientError,
            >,
        >,
    },
    /// Remove a logical subscription.
    Unsubscribe {
        subscription_id: ClientSubscriptionId,
    },
    /// Shut down the client loop.
    Shutdown,
}

/// Handle for a high-level client subscription.
///
/// Dropping the handle unsubscribes this logical subscription.
pub struct PhoenixClientSubscriptionHandle {
    pub cmd_tx: mpsc::UnboundedSender<ClientCommand>,
    pub subscription_id: ClientSubscriptionId,
}

impl Drop for PhoenixClientSubscriptionHandle {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(ClientCommand::Unsubscribe {
            subscription_id: self.subscription_id,
        });
    }
}

/// Logical subscription state tracked by high-level clients.
pub struct LogicalSubscription {
    pub subscription: PhoenixSubscription,
    pub dependencies: HashSet<SubscriptionKey>,
    pub event_tx: mpsc::UnboundedSender<PhoenixClientEvent>,
}

/// Mutable runtime state owned by high-level client loops.
pub struct RuntimeState {
    pub metadata: PhoenixMetadata,
    pub markets: HashMap<String, Market>,
    pub traders: HashMap<SubscriptionKey, Trader>,
    pub mids: HashMap<String, f64>,
    pub funding_rates: HashMap<String, FundingRateMessage>,
    pub candles: HashMap<(String, Timeframe), CandleData>,
    pub trades: HashMap<String, TradesMessage>,
}

impl RuntimeState {
    /// Create a new runtime state with initialized metadata.
    pub fn new(metadata: PhoenixMetadata) -> Self {
        Self {
            metadata,
            markets: HashMap::new(),
            traders: HashMap::new(),
            mids: HashMap::new(),
            funding_rates: HashMap::new(),
            candles: HashMap::new(),
            trades: HashMap::new(),
        }
    }
}
