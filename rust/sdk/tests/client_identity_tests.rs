use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use phoenix_rise::PhoenixHttpClient;
use serde_json::{Value, json};
use tokio::net::TcpListener;

#[derive(Clone)]
struct TestState {
    observed_headers: Arc<Mutex<Vec<String>>>,
}

#[tokio::test]
async fn http_client_sends_rise_client_identity_header() {
    let state = TestState {
        observed_headers: Arc::new(Mutex::new(Vec::new())),
    };
    let (base_url, server) = spawn_test_server(state.clone()).await;

    let client = PhoenixHttpClient::builder(base_url).build().unwrap();
    let keys = client.exchange().get_keys().await.unwrap();

    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert_eq!(
        state.observed_headers.lock().unwrap().as_slice(),
        &[expected_client_header()]
    );

    server.abort();
}

#[tokio::test]
async fn auth_client_sends_rise_client_identity_header() {
    let state = TestState {
        observed_headers: Arc::new(Mutex::new(Vec::new())),
    };
    let (base_url, server) = spawn_test_server(state.clone()).await;

    let client = PhoenixHttpClient::builder(base_url)
        .enable_auth()
        .build()
        .unwrap();
    let challenge = client
        .auth()
        .unwrap()
        .service_challenge("client-id", Some("key-id"))
        .await
        .unwrap();

    assert_eq!(challenge.nonce, "nonce");
    assert_eq!(
        state.observed_headers.lock().unwrap().as_slice(),
        &[expected_client_header()]
    );

    server.abort();
}

async fn spawn_test_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .route(
            "/v1/auth/login/service/challenge",
            post(service_challenge_handler),
        )
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), server)
}

async fn exchange_keys_handler(State(state): State<TestState>, headers: HeaderMap) -> Json<Value> {
    record_client_header(&state, &headers);

    Json(json!({
        "globalConfig": "11111111111111111111111111111111",
        "currentAuthorities": {
            "rootAuthority": "22222222222222222222222222222222",
            "riskAuthority": "33333333333333333333333333333333",
            "marketAuthority": "44444444444444444444444444444444",
            "oracleAuthority": "55555555555555555555555555555555"
        },
        "pendingAuthorities": {
            "rootAuthority": "66666666666666666666666666666666",
            "riskAuthority": "77777777777777777777777777777777",
            "marketAuthority": "88888888888888888888888888888888",
            "oracleAuthority": "99999999999999999999999999999999"
        },
        "canonicalMint": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "globalVault": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
        "perpAssetMap": "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        "globalTraderIndex": ["idx1", "idx2"],
        "activeTraderBuffer": ["buf1", "buf2"],
        "withdrawQueue": "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD"
    }))
}

async fn service_challenge_handler(
    State(state): State<TestState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    record_client_header(&state, &headers);

    assert_eq!(body["client_id"], "client-id");
    assert_eq!(body["key_id"], "key-id");

    Json(json!({
        "nonce": "nonce",
        "message": "service-auth:timestamp=2024-01-01T00:00:00Z",
        "expires_at": "2024-01-01T00:05:00Z",
        "key_id": "key-id"
    }))
}

fn record_client_header(state: &TestState, headers: &HeaderMap) {
    let header = headers
        .get("x-phoenix-client")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();
    state.observed_headers.lock().unwrap().push(header);
}

fn expected_client_header() -> String {
    format!(
        "language=rust;sdk={};version={}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
}
