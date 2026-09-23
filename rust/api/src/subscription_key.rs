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
    MarketStatsV2 {
        symbols: Option<Vec<String>>,
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
    pub(crate) fn routing_key(&self) -> Self {
        match self {
            Self::FundingRate { symbol } => Self::funding_rate(symbol.to_ascii_lowercase()),
            Self::Orderbook {
                symbol,
                bypass_execution_band,
            } => Self::orderbook_with_options(symbol.to_ascii_lowercase(), *bypass_execution_band),
            Self::Market { symbol } => Self::market(symbol.to_ascii_lowercase()),
            Self::Trades { symbol } => Self::trades(symbol.to_ascii_lowercase()),
            Self::Candles { symbol, timeframe } => {
                Self::candles(symbol.to_ascii_lowercase(), *timeframe)
            }
            Self::MarketStatsV2 { symbols } => Self::MarketStatsV2 {
                symbols: symbols.as_ref().map(|symbols| {
                    let mut symbols = symbols
                        .iter()
                        .map(|symbol| symbol.to_ascii_lowercase())
                        .collect::<Vec<_>>();
                    symbols.sort_unstable();
                    symbols.dedup();
                    symbols
                }),
            },
            _ => self.clone(),
        }
    }

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

    pub fn market_stats_v2(symbols: Option<Vec<String>>) -> Self {
        let symbols = symbols.map(|mut symbols| {
            for symbol in &mut symbols {
                *symbol = symbol.trim().to_string();
            }
            symbols.sort_unstable();
            symbols.dedup();
            symbols
        });
        Self::MarketStatsV2 { symbols }
    }

    pub(crate) fn market_stats_v2_symbols(&self) -> Option<&[String]> {
        match self {
            Self::MarketStatsV2 {
                symbols: Some(symbols),
            } => Some(symbols),
            _ => None,
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
