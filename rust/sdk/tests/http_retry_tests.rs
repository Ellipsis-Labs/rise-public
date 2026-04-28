use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::State;
use axum::http::header::{CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use phoenix_rise::{PhoenixHttpClient, PhoenixHttpError};
use tokio::net::TcpListener;

#[derive(Clone)]
struct TestState {
    exchange_keys_calls: Arc<AtomicUsize>,
}

#[tokio::test]
async fn get_requests_retry_rate_limited_responses_by_default() {
    let state = TestState {
        exchange_keys_calls: Arc::new(AtomicUsize::new(0)),
    };
    let (base_url, server) = spawn_test_server(state.clone()).await;

    let client = PhoenixHttpClient::builder(base_url).build().unwrap();
    let keys = client.exchange().get_keys().await.unwrap();

    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert_eq!(state.exchange_keys_calls.load(Ordering::SeqCst), 2);

    server.abort();
}

#[tokio::test]
async fn get_requests_can_disable_rate_limit_retry_at_creation() {
    let state = TestState {
        exchange_keys_calls: Arc::new(AtomicUsize::new(0)),
    };
    let (base_url, server) = spawn_test_server(state.clone()).await;

    let client = PhoenixHttpClient::builder(base_url)
        .with_rate_limit_retry_enabled(false)
        .build()
        .unwrap();

    let error = client.exchange().get_keys().await.unwrap_err();
    match error {
        PhoenixHttpError::RateLimited {
            retry_after_seconds,
            error_code,
            attempts,
            ..
        } => {
            assert_eq!(retry_after_seconds, Some(0));
            assert_eq!(error_code.as_deref(), Some("rate_limited"));
            assert_eq!(attempts, 1);
        }
        other => panic!("expected rate-limited error, got {other:?}"),
    }

    assert_eq!(state.exchange_keys_calls.load(Ordering::SeqCst), 1);
    server.abort();
}

async fn spawn_test_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/exchange/keys", get(exchange_keys_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), server)
}

async fn exchange_keys_handler(State(state): State<TestState>) -> Response {
    let attempt = state.exchange_keys_calls.fetch_add(1, Ordering::SeqCst);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if attempt == 0 {
        headers.insert(RETRY_AFTER, HeaderValue::from_static("0"));
        return (
            StatusCode::TOO_MANY_REQUESTS,
            headers,
            r#"{"error":"rate_limited","message":"slow down"}"#,
        )
            .into_response();
    }

    (
        StatusCode::OK,
        headers,
        r#"{
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
        }"#,
    )
        .into_response()
}
