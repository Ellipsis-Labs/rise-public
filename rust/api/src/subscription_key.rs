//! Subscription key for routing messages to the correct subscriber.

use solana_pubkey::Pubkey;

use crate::types::prelude::{
    CandleData, FundingRateMessage, L2BookMessage, L2BookUpdate, MarketStatsUpdate,
    SubscriptionRequest, Timeframe, TraderStateServerMessage, TradesMessage,
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
    L2Book {
        symbol: String,
        bypass_execution_band: bool,
    },
    MarkPrice {
        symbol: String,
    },
    Fills {
        symbol: Option<String>,
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
            Self::L2Book {
                symbol,
                bypass_execution_band,
            } => Self::l2_book_with_options(symbol.to_ascii_lowercase(), *bypass_execution_band),
            Self::MarkPrice { symbol } => Self::mark_price(symbol.to_ascii_lowercase()),
            Self::Fills { symbol } => {
                Self::fills(symbol.as_ref().map(|symbol| symbol.to_ascii_lowercase()))
            }
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

    /// Routing key for a wire subscription request, e.g. the echo in a server
    /// acknowledgement. Folds the server's canonicalization (symbol case,
    /// `marketStatsV2` sort and dedup), so an echo maps to the same key as the
    /// request that produced it. Returns `None` for channels this SDK does not
    /// subscribe to.
    pub(crate) fn from_request(request: &SubscriptionRequest) -> Option<Self> {
        let key = match request {
            SubscriptionRequest::AllMids => Self::all_mids(),
            SubscriptionRequest::FundingRate(request) => Self::funding_rate(request.symbol.clone()),
            SubscriptionRequest::Orderbook(request) => Self::orderbook_with_options(
                request.symbol.clone(),
                request.bypass_execution_band.unwrap_or(false),
            ),
            SubscriptionRequest::L2Book(request) => Self::l2_book_with_options(
                request.coin.clone(),
                request.bypass_execution_band.unwrap_or(false),
            ),
            SubscriptionRequest::MarkPrice(request) => Self::mark_price(request.symbol.clone()),
            SubscriptionRequest::Fills(request) => Self::fills(request.market_symbol.clone()),
            SubscriptionRequest::TraderState(request) => Self::TraderState {
                authority: request.authority.clone(),
                trader_pda_index: request.trader_pda_index,
            },
            SubscriptionRequest::Market(request) => Self::market(request.symbol.clone()),
            SubscriptionRequest::MarketStatsV2(request) => {
                Self::market_stats_v2(request.symbols.clone())
            }
            SubscriptionRequest::Trades(request) => Self::trades(request.symbol.clone()),
            SubscriptionRequest::Candles(request) => {
                Self::candles(request.symbol.clone(), request.timeframe)
            }
            SubscriptionRequest::Exchange(_) => Self::exchange(),
            SubscriptionRequest::Other => return None,
        };
        Some(key.routing_key())
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

    pub fn l2_book(symbol: String) -> Self {
        Self::l2_book_with_options(symbol, false)
    }

    pub fn l2_book_with_options(symbol: String, bypass_execution_band: bool) -> Self {
        Self::L2Book {
            symbol,
            bypass_execution_band,
        }
    }

    pub fn l2_book_from_message(msg: &L2BookMessage) -> Self {
        Self::L2Book {
            symbol: msg.coin.clone(),
            bypass_execution_band: msg.bypass_execution_band,
        }
    }

    pub fn mark_price(symbol: String) -> Self {
        Self::MarkPrice { symbol }
    }

    pub fn fills(symbol: Option<String>) -> Self {
        Self::Fills { symbol }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::prelude::{
        FillsSubscriptionRequest, L2OrderbookSubscriptionRequest, MarkPriceSubscriptionRequest,
    };

    /// A server echo must map to the key the client subscribes with, including
    /// the server's symbol canonicalization.
    #[test]
    fn echo_keys_match_the_subscribe_side_key() {
        let cases = [
            (
                SubscriptionRequest::L2Book(L2OrderbookSubscriptionRequest {
                    coin: "KBONK".to_string(),
                    bypass_execution_band: Some(true),
                }),
                SubscriptionKey::l2_book_with_options("KBONK".to_string(), true),
            ),
            (
                SubscriptionRequest::MarkPrice(MarkPriceSubscriptionRequest {
                    symbol: "kBONK".to_string(),
                }),
                SubscriptionKey::mark_price("KBONK".to_string()),
            ),
            (
                SubscriptionRequest::Fills(FillsSubscriptionRequest {
                    market_symbol: Some("kBONK".to_string()),
                }),
                SubscriptionKey::fills(Some("KBONK".to_string())),
            ),
            (
                SubscriptionRequest::Fills(FillsSubscriptionRequest {
                    market_symbol: None,
                }),
                SubscriptionKey::fills(None),
            ),
        ];

        for (request, expected) in cases {
            assert_eq!(
                SubscriptionKey::from_request(&request),
                Some(expected.routing_key()),
                "echo {request:?} should map to {expected:?}"
            );
        }
    }
}
