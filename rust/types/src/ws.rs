//! WebSocket protocol types for Phoenix API.
//!
//! These types handle subscription management, client/server message
//! envelopes, and error responses.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::candles::{CandleData, Timeframe};
use crate::exchange_ws::{ExchangeMessage, ExchangeSnapshotEncoding};
use crate::market::{L2BookUpdate, MarketStatsUpdate};
use crate::trader::TraderStateServerMessage;
use crate::trades::{TradesMessage, TradesSubscriptionRequest};

// ============================================================================
// Subscription Types
// ============================================================================

/// Subscription request for the funding-rate channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct FundingRateSubscriptionRequest {
    /// Market symbol (e.g., "SOL" or "BTC")
    pub symbol: String,
}

/// Subscription request for the orderbook channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookSubscriptionRequest {
    /// Market symbol (e.g., "SOL" or "BTC")
    pub symbol: String,
    /// Opt in to receive the full orderbook during commodities after-hours,
    /// bypassing the tradeable execution price band filter. Filtered and
    /// unfiltered subscribers receive independent snapshot streams.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypass_execution_band: Option<bool>,
}

/// Subscription request for the trader-state channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateSubscriptionRequest {
    pub authority: String,
    pub trader_pda_index: u8,
}

/// Subscription request for the market channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct MarketSubscriptionRequest {
    /// Market symbol (e.g., "SOL" or "BTC")
    pub symbol: String,
}

/// Subscription request for the candles channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CandlesSubscriptionRequest {
    pub symbol: String,
    pub timeframe: Timeframe,
}

/// Subscription request for the exchange snapshot/delta channel.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeSubscriptionRequest {
    /// Snapshot encoding. When omitted, the server defaults to
    /// `base64+zstd`; clients that cannot decode zstd should request
    /// [`ExchangeSnapshotEncoding::Json`] explicitly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<ExchangeSnapshotEncoding>,
}

/// Subscription request from client.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(tag = "channel")]
pub enum SubscriptionRequest {
    #[serde(rename = "allMids")]
    AllMids,
    #[serde(rename = "fundingRate")]
    FundingRate(FundingRateSubscriptionRequest),
    #[serde(rename = "orderbook")]
    Orderbook(OrderbookSubscriptionRequest),
    #[serde(rename = "traderState")]
    TraderState(TraderStateSubscriptionRequest),
    #[serde(rename = "market")]
    Market(MarketSubscriptionRequest),
    #[serde(rename = "trades")]
    Trades(TradesSubscriptionRequest),
    #[serde(rename = "candles")]
    Candles(CandlesSubscriptionRequest),
    #[serde(rename = "exchange")]
    Exchange(ExchangeSubscriptionRequest),
    /// Other subscription types exist but are not used by this SDK.
    #[serde(other)]
    Other,
}

// ============================================================================
// Client Messages
// ============================================================================

/// WebSocket message types from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { subscription: SubscriptionRequest },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { subscription: SubscriptionRequest },
}

// ============================================================================
// Server Messages
// ============================================================================

/// Mid price snapshot for all markets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllMidsData {
    pub mids: HashMap<String, f64>,
    pub slot: u64,
    pub slot_index: u32,
}

/// Funding rate update for a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRateMessage {
    pub symbol: String,
    pub funding: f64,
}

/// WebSocket message types from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "channel")]
#[serde(rename_all = "camelCase")]
pub enum ServerMessage {
    #[serde(rename = "allMids")]
    AllMids(AllMidsData),
    #[serde(rename = "fundingRate")]
    FundingRate(FundingRateMessage),
    #[serde(rename = "orderbook")]
    Orderbook(L2BookUpdate),
    #[serde(rename = "traderState")]
    TraderState(TraderStateServerMessage),
    #[serde(rename = "market")]
    Market(MarketStatsUpdate),
    #[serde(rename = "trades")]
    Trades(TradesMessage),
    #[serde(rename = "candle", alias = "candles")]
    Candles(CandleData),
    #[serde(rename = "exchange")]
    Exchange(ExchangeMessage),
    #[serde(rename = "error")]
    Error(ErrorMessage),
    #[serde(rename = "subscriptionStatus")]
    SubscriptionStatus(SubscriptionStatusMessage),
    /// Other message types exist but are not used by this SDK.
    #[serde(other)]
    Other,
}

/// Subscription confirmed message from server.
/// Expected format: `{"type":"subscriptionConfirmed","subscription":{...}}`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename = "subscriptionConfirmed")]
pub struct SubscriptionConfirmedMessage {
    pub subscription: SubscriptionRequest,
}

/// Subscription status message from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionStatusMessage {
    pub status: String,
    pub subscription: SubscriptionRequest,
    pub client_id: String,
}

/// Subscription error message from server.
/// Expected format:
/// `{"type":"subscriptionError","subscription":{...},"code":"...","message":"..
/// ."}`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename = "subscriptionError")]
pub struct SubscriptionErrorMessage {
    pub subscription: SubscriptionRequest,
    pub code: String,
    pub message: String,
}

/// Error message from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub error: String,
    pub code: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange_ws::{
        AuthoritySet, ExchangeDeltaMessage, ExchangeDeltaOp, ExchangeSnapshotMessage,
        ExchangeSnapshotReason, ExchangeStateSnapshot,
    };

    fn sample_exchange_state_snapshot() -> ExchangeStateSnapshot {
        ExchangeStateSnapshot {
            program_id: "program".to_string(),
            global_config: "global-config".to_string(),
            current_authorities: AuthoritySet {
                root_authority: "root".to_string(),
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

    #[test]
    fn test_exchange_subscription_request_round_trip() {
        let msg = ClientMessage::Subscribe {
            subscription: SubscriptionRequest::Exchange(ExchangeSubscriptionRequest {
                encoding: Some(ExchangeSnapshotEncoding::Json),
            }),
        };

        let value = serde_json::to_value(&msg).unwrap();
        assert_eq!(value["type"], "subscribe");
        assert_eq!(value["subscription"]["channel"], "exchange");
        assert_eq!(value["subscription"]["encoding"], "json");

        let decoded: ClientMessage = serde_json::from_value(value).unwrap();
        let ClientMessage::Subscribe {
            subscription: SubscriptionRequest::Exchange(request),
        } = decoded
        else {
            panic!("Expected exchange subscribe message");
        };
        assert_eq!(request.encoding, Some(ExchangeSnapshotEncoding::Json));
    }

    #[test]
    fn test_exchange_subscription_request_omits_missing_encoding() {
        let subscription = SubscriptionRequest::Exchange(ExchangeSubscriptionRequest::default());

        let value = serde_json::to_value(&subscription).unwrap();
        assert_eq!(value["channel"], "exchange");
        assert!(value.get("encoding").is_none());

        let decoded: SubscriptionRequest =
            serde_json::from_value(serde_json::json!({ "channel": "exchange" })).unwrap();
        assert_eq!(decoded, subscription);
    }

    #[test]
    fn test_exchange_snapshot_server_message_round_trip() {
        let message = ServerMessage::Exchange(ExchangeMessage::Snapshot(Box::new(
            ExchangeSnapshotMessage {
                version: 1,
                sequence_number: 10u64.into(),
                slot: 42,
                slot_index: 7,
                reason: ExchangeSnapshotReason::Snapshot,
                exchange: sample_exchange_state_snapshot(),
                markets: Vec::new(),
                spot_collaterals: Vec::new(),
            },
        )));

        let value = serde_json::to_value(&message).unwrap();
        assert_eq!(value["channel"], "exchange");
        assert_eq!(value["messageType"], "snapshot");
        // JsSafeU64 serializes as a string for JS number-precision safety.
        assert_eq!(value["sequenceNumber"], "10");

        let decoded: ServerMessage = serde_json::from_value(value).unwrap();
        let ServerMessage::Exchange(ExchangeMessage::Snapshot(snapshot)) = decoded else {
            panic!("Expected exchange snapshot message");
        };
        assert_eq!(snapshot.sequence_number, 10u64);
        assert_eq!(snapshot.exchange.current_authorities.root_authority, "root");
    }

    #[test]
    fn test_exchange_delta_server_message_round_trip() {
        let message = ServerMessage::Exchange(ExchangeMessage::Delta(ExchangeDeltaMessage {
            version: 1,
            sequence_number: 11u64.into(),
            slot: 43,
            slot_index: 0,
            ops: vec![ExchangeDeltaOp::ExchangeKeysUpdated {
                exchange: sample_exchange_state_snapshot(),
            }],
        }));

        let value = serde_json::to_value(&message).unwrap();
        assert_eq!(value["channel"], "exchange");
        assert_eq!(value["messageType"], "delta");
        assert_eq!(value["ops"][0]["kind"], "exchangeKeysUpdated");

        let decoded: ServerMessage = serde_json::from_value(value).unwrap();
        let ServerMessage::Exchange(ExchangeMessage::Delta(delta)) = decoded else {
            panic!("Expected exchange delta message");
        };
        assert_eq!(delta.sequence_number, 11u64);
        assert!(matches!(
            delta.ops.as_slice(),
            [ExchangeDeltaOp::ExchangeKeysUpdated { .. }]
        ));
    }

    #[test]
    fn test_deserialize_client_message() {
        let json = r#"{
            "type": "subscribe",
            "subscription": {
                "channel": "traderState",
                "authority": "ABC123",
                "traderPdaIndex": 0
            }
        }"#;

        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Subscribe { .. }));
    }

    #[test]
    fn test_serialize_client_message() {
        let msg = ClientMessage::Subscribe {
            subscription: SubscriptionRequest::TraderState(TraderStateSubscriptionRequest {
                authority: "ABC123".to_string(),
                trader_pda_index: 0,
            }),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));
        assert!(json.contains("traderState"));
    }

    #[test]
    fn test_deserialize_subscription_status_message() {
        let json = r#"{
            "channel": "subscriptionStatus",
            "status": "subscribed",
            "clientId": "client-1",
            "subscription": {
                "channel": "traderState",
                "authority": "ABC123",
                "traderPdaIndex": 0
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        let ServerMessage::SubscriptionStatus(status) = msg else {
            panic!("Expected subscriptionStatus message");
        };
        assert_eq!(status.status, "subscribed");
        assert_eq!(status.client_id, "client-1");
        assert!(matches!(
            status.subscription,
            SubscriptionRequest::TraderState(TraderStateSubscriptionRequest {
                trader_pda_index: 0,
                ..
            })
        ));
    }

    #[test]
    fn test_orderbook_subscription_request() {
        let msg = ClientMessage::Subscribe {
            subscription: SubscriptionRequest::Orderbook(OrderbookSubscriptionRequest {
                symbol: "SOL".to_string(),
                bypass_execution_band: None,
            }),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));
        assert!(json.contains("orderbook"));
        assert!(json.contains("SOL"));
    }

    #[test]
    fn test_deserialize_orderbook_server_message() {
        let json = r#"{
            "channel": "orderbook",
            "symbol": "SOL",
            "orderbook": {
                "bids": [[150.25, 100.0], [150.20, 200.0]],
                "asks": [[150.30, 150.0], [150.35, 250.0]],
                "mid": 150.275
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        if let ServerMessage::Orderbook(update) = msg {
            assert_eq!(update.symbol, "SOL");
            assert_eq!(update.orderbook.bids.len(), 2);
            assert_eq!(update.orderbook.asks.len(), 2);
            assert_eq!(update.orderbook.mid, Some(150.275));
        } else {
            panic!("Expected Orderbook message");
        }
    }

    #[test]
    fn test_deserialize_funding_rate_server_message() {
        let json = r#"{
            "channel": "fundingRate",
            "symbol": "SOL",
            "funding": 0.0125
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        if let ServerMessage::FundingRate(update) = msg {
            assert_eq!(update.symbol, "SOL");
            assert_eq!(update.funding, 0.0125);
        } else {
            panic!("Expected FundingRate message");
        }
    }

    #[test]
    fn test_deserialize_trades_server_message() {
        let json = r#"{
            "channel": "trades",
            "symbol": "SOL",
            "trades": [{
                "slot": "123456789",
                "slotIndex": 5,
                "timestamp": "1775578550",
                "symbol": "SOL",
                "taker": "ABC123pubkey",
                "tradeSequenceNumber": "100",
                "side": "bid",
                "baseLotsFilled": "1000",
                "quoteLotsFilled": "150000",
                "feeInQuoteLots": "30",
                "baseAmount": 10.0,
                "quoteAmount": 1500.0,
                "numFills": 2
            }]
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        if let ServerMessage::Trades(update) = msg {
            assert_eq!(update.symbol, "SOL");
            assert_eq!(update.trades.len(), 1);
        } else {
            panic!("Expected Trades message");
        }
    }

    #[test]
    fn test_serialize_candles_subscription_request() {
        let req = CandlesSubscriptionRequest {
            symbol: "SOL".to_string(),
            timeframe: Timeframe::Minute1,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"symbol\":\"SOL\""));
        assert!(json.contains("\"timeframe\":\"1m\""));
    }

    #[test]
    fn test_deserialize_candle_server_message() {
        let json = r#"{
            "channel": "candle",
            "symbol": "SOL",
            "timeframe": "1m",
            "candle": {
                "time": 1776801600,
                "open": 85.0,
                "high": 85.5,
                "low": 84.9,
                "close": 85.2
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        if let ServerMessage::Candles(update) = msg {
            assert_eq!(update.symbol, "SOL");
            assert_eq!(update.timeframe, "1m");
            assert_eq!(update.candle.close, 85.2);
        } else {
            panic!("Expected Candles message");
        }
    }
}
