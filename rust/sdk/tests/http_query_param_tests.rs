use std::collections::HashMap;

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use phoenix_rise::{
    CollateralHistoryQueryParams, OrderHistoryQueryParams, PhoenixHttpClient,
    TradeHistoryQueryParams,
};
use solana_pubkey::Pubkey;
use tokio::net::TcpListener;

#[tokio::test]
async fn order_history_omits_empty_query_fields() {
    let (base_url, server) = spawn_test_server().await;
    let client = PhoenixHttpClient::builder(base_url).build().unwrap();
    let authority = Pubkey::new_unique();

    let response = client
        .orders()
        .get_trader_order_history(
            &authority,
            OrderHistoryQueryParams::new(10)
                .with_pda_index(0)
                .with_market_symbol("SOL-PERP"),
        )
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(!response.has_more);

    server.abort();
}

#[tokio::test]
async fn trade_history_omits_empty_query_fields() {
    let (base_url, server) = spawn_test_server().await;
    let client = PhoenixHttpClient::builder(base_url).build().unwrap();
    let authority = Pubkey::new_unique();

    let response = client
        .trades()
        .get_trader_trade_history(
            &authority,
            TradeHistoryQueryParams::new()
                .with_pda_index(0)
                .with_market_symbol("SOL-PERP")
                .with_limit(10),
        )
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(!response.has_more);

    server.abort();
}

#[tokio::test]
async fn collateral_history_omits_empty_query_fields() {
    let (base_url, server) = spawn_test_server().await;
    let client = PhoenixHttpClient::builder(base_url).build().unwrap();
    let authority = Pubkey::new_unique();

    let response = client
        .collateral()
        .get_user_collateral_history(&authority, CollateralHistoryQueryParams::new(10))
        .await
        .unwrap();

    assert!(response.data.is_empty());
    assert!(!response.has_more);

    server.abort();
}

async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/trader/{authority}/order-history",
            get(order_history_handler),
        )
        .route(
            "/trader/{authority}/trades-history",
            get(trades_history_handler),
        )
        .route(
            "/trader/{authority}/collateral-history",
            get(collateral_history_handler),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), server)
}

async fn order_history_handler(
    Path(_authority): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    match validate_query(
        &query,
        &[
            ("limit", "10"),
            ("traderPdaIndex", "0"),
            ("marketSymbol", "SOL-PERP"),
        ],
        &["cursor", "privyId"],
    ) {
        Ok(()) => (
            StatusCode::OK,
            r#"{"data":[],"nextCursor":null,"prevCursor":null,"hasMore":false}"#,
        )
            .into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            format!(r#"{{"error":"invalid_query","message":"{message}"}}"#),
        )
            .into_response(),
    }
}

async fn trades_history_handler(
    Path(_authority): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    match validate_query(
        &query,
        &[
            ("limit", "10"),
            ("pdaIndex", "0"),
            ("marketSymbol", "SOL-PERP"),
        ],
        &["cursor"],
    ) {
        Ok(()) => (
            StatusCode::OK,
            r#"{"data":[],"nextCursor":null,"prevCursor":null,"hasMore":false}"#,
        )
            .into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            format!(r#"{{"error":"invalid_query","message":"{message}"}}"#),
        )
            .into_response(),
    }
}

async fn collateral_history_handler(
    Path(_authority): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    match validate_query(
        &query,
        &[("limit", "10"), ("pdaIndex", "0")],
        &["cursor", "nextCursor", "prevCursor"],
    ) {
        Ok(()) => (
            StatusCode::OK,
            r#"{"data":[],"nextCursor":null,"prevCursor":null,"hasMore":false}"#,
        )
            .into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            format!(r#"{{"error":"invalid_query","message":"{message}"}}"#),
        )
            .into_response(),
    }
}

fn validate_query(
    query: &HashMap<String, String>,
    expected: &[(&str, &str)],
    absent: &[&str],
) -> Result<(), String> {
    for (key, value) in expected {
        match query.get(*key) {
            Some(actual) if actual == value => {}
            Some(actual) => {
                return Err(format!("expected query param {key}={value}, got {actual}"));
            }
            None => return Err(format!("missing expected query param {key}")),
        }
    }

    for key in absent {
        if query.contains_key(*key) {
            return Err(format!("unexpected query param {key}"));
        }
    }

    Ok(())
}
