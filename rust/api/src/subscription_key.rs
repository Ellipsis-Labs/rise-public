//! Subscription key for routing messages to the correct subscriber.

use solana_pubkey::Pubkey;

use crate::types::prelude::{
    CandleData, FundingRateMessage, L2BookUpdate, MarketStatsUpdate, Timeframe,
    TraderStateServerMessage, TradesMessage,
};

/// Subscription key for routing messages to the correct subscriber.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionKey {
    AllMids,
    FundingRate {
        symbol: String,
    },
    Orderbook {
        symbol: String,
        bypass_execution_band: bool,
    },
    TraderState {
        authority: String,
        trader_pda_index: u8,
    },
    Market {
        symbol: String,
    },
    Trades {
        symbol: String,
    },
    Candles {
        symbol: String,
        timeframe: Timeframe,
    },
    Exchange,
}

impl SubscriptionKey {
    pub fn all_mids() -> Self {
        Self::AllMids
    }

    pub fn funding_rate(symbol: String) -> Self {
        Self::FundingRate { symbol }
    }

    pub fn funding_rate_from_message(msg: &FundingRateMessage) -> Self {
        Self::FundingRate {
            symbol: msg.symbol.clone(),
        }
    }

    pub fn orderbook(symbol: String) -> Self {
        Self::orderbook_with_options(symbol, false)
    }

    pub fn orderbook_with_options(symbol: String, bypass_execution_band: bool) -> Self {
        Self::Orderbook {
            symbol,
            bypass_execution_band,
        }
    }

    pub fn orderbook_from_message(msg: &L2BookUpdate) -> Self {
        Self::Orderbook {
            symbol: msg.symbol.clone(),
            bypass_execution_band: msg.bypass_execution_band,
        }
    }

    pub fn trader(authority: &Pubkey, trader_pda_index: u8) -> Self {
        Self::TraderState {
            authority: authority.to_string(),
            trader_pda_index,
        }
    }

    pub fn trader_state_from_message(msg: &TraderStateServerMessage) -> Self {
        Self::TraderState {
            authority: msg.authority.clone(),
            trader_pda_index: msg.trader_pda_index,
        }
    }

    pub fn market(symbol: String) -> Self {
        Self::Market { symbol }
    }

    pub fn market_from_message(msg: &MarketStatsUpdate) -> Self {
        Self::Market {
            symbol: msg.symbol.clone(),
        }
    }

    pub fn trades(symbol: String) -> Self {
        Self::Trades { symbol }
    }

    pub fn trades_from_message(msg: &TradesMessage) -> Self {
        Self::Trades {
            symbol: msg.symbol.clone(),
        }
    }

    pub fn candles(symbol: String, timeframe: Timeframe) -> Self {
        Self::Candles { symbol, timeframe }
    }

    pub fn candles_from_message(msg: &CandleData) -> Option<Self> {
        let timeframe = msg.timeframe.parse().ok()?;
        Some(Self::Candles {
            symbol: msg.symbol.clone(),
            timeframe,
        })
    }

    pub fn exchange() -> Self {
        Self::Exchange
    }
}
