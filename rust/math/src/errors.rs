//! Application-level error types

use crate::quantities::MathError;
use crate::risk::{MarginError, ProgramError};

#[derive(Debug, thiserror::Error)]
pub enum PhoenixStateError {
    #[error("Market {symbol} not found. Available markets: [{markets:?}]")]
    MarketNotFound {
        symbol: String,
        markets: Vec<String>,
    },

    #[error("Trader {trader} not found.")]
    TraderNotFound { trader: String },

    #[error("Oracle {oracle} not found for market {market}.")]
    OracleNotFound { oracle: String, market: String },

    #[error("Failed to deserialize pubkey: {pubkey}")]
    PubkeyDeserializeError { pubkey: String },

    #[error("Orderbook sequence number for market {symbol} out of order: {expected} != {actual}")]
    OrderbookSequenceNumberOutOfOrder {
        symbol: String,
        expected: u64,
        actual: u64,
    },

    #[error("Duplicate extension registered")]
    DuplicateExtensionType,

    #[error("Market ID {market_id} not found. Available market IDs: [{market_ids:?}]")]
    MarketIdNotFound {
        market_id: u32,
        market_ids: Vec<u32>,
    },

    #[error("Stop loss for asset {symbol} not found. Available stop losses: [{symbols:?}]")]
    StopLossNotFound {
        symbol: String,
        symbols: Vec<String>,
    },

    #[error("Margin error: {0}")]
    MarginError(#[from] MarginError),

    #[error("Math error: {0}")]
    MathError(#[from] MathError),

    #[error("Mark price error: {0:?}")]
    MarkPriceError(ProgramError),
}
