use std::sync::{Arc, Mutex};

use axum::extract::{OriginalUri, State};
use axum::routing::get;
use axum::{Json, Router};
use phoenix_rise::api::PhoenixHttpClient;
use phoenix_rise::types::prelude::{CandlesQueryParams, CandlesV2InitialQueryParams, Timeframe};
use serde_json::{Value, json};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct TestState {
    observed_uris: Arc<Mutex<Vec<String>>>,
}

#[tokio::test]
async fn legacy_candles_preserve_query_and_full_server_response() {
    let state = TestState::default();
    let (base_url, server) = spawn_test_server(state.clone()).await;
    let client = PhoenixHttpClient::builder(base_url).build().unwrap();

    let candles = client
        .get_candles(CandlesQueryParams::new("sol", Timeframe::Minute1).with_limit(10_000))
        .await
        .unwrap();

    assert_eq!(candles.len(), 2_500);
    assert_eq!(
        state.observed_uris.lock().unwrap().as_slice(),
        &["/v1/candles/SOL?symbol=sol&timeframe=1m&limit=10000"]
    );

    server.abort();
}

#[tokio::test]
async fn candles_v2_initial_requests_default_to_one_thousand_bars() {
    let state = TestState::default();
    let (base_url, server) = spawn_test_server(state.clone()).await;
    let client = PhoenixHttpClient::builder(base_url).build().unwrap();

    let response = client
        .get_candles_v2(CandlesV2InitialQueryParams::new(
            "sol",
            Timeframe::Minute1,
            0,
            60_000,
        ))
        .await
        .unwrap();

    assert!(response.bars.is_empty());
    assert_eq!(
        state.observed_uris.lock().unwrap().as_slice(),
        &["/v1/candles_v2/SOL?timeframe=1m&from=0&to=60000&limit=1000"]
    );

    server.abort();
}

async fn spawn_test_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/v1/candles/SOL", get(legacy_candles_handler))
        .route("/v1/candles_v2/SOL", get(candles_v2_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), server)
}

async fn legacy_candles_handler(
    State(state): State<TestState>,
    OriginalUri(uri): OriginalUri,
) -> Json<Value> {
    state.observed_uris.lock().unwrap().push(uri.to_string());

    Json(Value::Array(
        (0..2_500)
            .map(|time| {
                json!({
                    "time": time,
                    "open": 1.0,
                    "high": 1.0,
                    "low": 1.0,
                    "close": 1.0,
                    "volume": 0.0,
                    "tradeCount": 0
                })
            })
            .collect(),
    ))
}

async fn candles_v2_handler(
    State(state): State<TestState>,
    OriginalUri(uri): OriginalUri,
) -> Json<Value> {
    state.observed_uris.lock().unwrap().push(uri.to_string());

    Json(json!({
        "symbol": "SOL",
        "timeframe": "1m",
        "from": 0,
        "to": 60_000,
        "bars": [],
        "page": { "hasMore": false }
    }))
}
