use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{OriginalUri, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use parking_lot::Mutex;
use phoenix_rise::api::{PhoenixHttpClient, PhoenixHttpError};
use phoenix_rise::types::order_history_v2::OrderHistoryV2QueryParams;
use phoenix_rise::types::trader_http::OrderHistoryQueryParams;
use serde_json::{Value, json};
use solana_pubkey::Pubkey;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
struct ObservedRequest {
    path: String,
    query: HashMap<String, String>,
}

#[derive(Clone)]
struct TestState {
    requests: Arc<Mutex<Vec<ObservedRequest>>>,
    status: StatusCode,
    response: Value,
}

struct TestServer {
    client: PhoenixHttpClient,
    requests: Arc<Mutex<Vec<ObservedRequest>>>,
    task: JoinHandle<()>,
}

impl TestServer {
    async fn new(status: StatusCode, response: Value) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/v1/users/{authority}/orders_v2", get(history_handler))
            .route("/v1/traders/{trader_pda}/orders_v2", get(history_handler))
            .route("/v1/trader/{authority}/order-history", get(history_handler))
            .with_state(TestState {
                requests: requests.clone(),
                status,
                response,
            });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client =
            PhoenixHttpClient::builder(format!("http://{}", listener.local_addr().unwrap()))
                .build()
                .unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            client,
            requests,
            task,
        }
    }

    async fn shutdown(mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn history_handler(
    State(state): State<TestState>,
    OriginalUri(uri): OriginalUri,
    Query(query): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    state.requests.lock().push(ObservedRequest {
        path: uri.path().to_string(),
        query,
    });
    (state.status, Json(state.response))
}

fn child_order() -> Value {
    json!({
        "parentOrderId": "twap:100:2:7", "parentOrderType": "twap", "sortKey": "opaque-child-key",
        "userId": 1, "traderId": 2, "traderPdaIndex": 7, "traderSubaccountIndex": 0,
        "slot": 101, "slotIndex": 2, "instructionIndex": 3, "eventIndex": 4,
        "marketSymbol": "SOL", "instructionType": "PlaceMarketOrder", "orderType": "market",
        "createdAt": "2026-10-12T10:00:00Z", "price": "150", "status": "filled",
        "isReduceOnly": false, "isStopLoss": false,
        "intended": {
            "orderKind": "ImmediateOrCancel", "side": "buy", "hasPriceLimit": false,
            "baseLots": 100, "baseQty": "1", "orderFlags": 0, "isReduceOnly": false,
            "isStopLoss": false, "isPostOnly": false, "isTakeOnly": true, "cancelExisting": false
        },
        "fillBeforePlacement": {
            "totalBaseQtyFilled": "1", "totalQuoteQtyFilled": "150", "totalTakerFee": "0.01",
            "totalNumFills": 1, "numTrades": 1
        },
        "placedOrder": null, "currentOrderState": null
    })
}

fn flat_response() -> Value {
    json!({ "data": [child_order(), standalone_order()], "prevCursor": "prev:opaque+cursor=", "nextCursor": "next/opaque", "hasMore": true })
}

fn standalone_order() -> Value {
    let mut order = child_order();
    let fields = order.as_object_mut().unwrap();
    for field in ["parentOrderId", "parentOrderType", "sortKey"] {
        fields.remove(field);
    }
    order["eventIndex"] = json!(5);
    order
}

fn organized_response() -> Value {
    json!({
        "data": {
            "twapOrders": [{
                "parentOrderId": "twap:100:2:7", "parentOrderType": "twap", "marketSymbol": "SOL",
                "traderPdaIndex": 7, "traderSubaccountIndex": 0, "side": "buy", "isReduceOnly": false,
                "nChildOrders": 2, "nDustOrders": null, "cooldownSlots": 75,
                "status": "filled", "requestedBaseQty": null, "filledBaseQty": "1",
                "filledQuoteQty": "150", "placedAt": null, "terminalAt": "2026-10-12T10:00:00Z",
                "sortKey": "opaque-parent-key", "children": [child_order()]
            }],
            "singleOrders": [standalone_order()]
        },
        "prevCursor": "ohg1:opaque+cursor=", "nextCursor": "ohg1:other/opaque", "hasMore": true
    })
}

#[tokio::test]
async fn flat_v2_methods_preserve_children_filters_and_account_scope() {
    let server = TestServer::new(StatusCode::OK, flat_response()).await;
    let authority = Pubkey::new_unique();
    let trader_pda = Pubkey::new_unique();
    let params = OrderHistoryV2QueryParams::new(25)
        .with_pda_index(0)
        .with_market_symbol("SOL")
        .with_cursor("cursor:+/=");

    let user = server
        .client
        .get_user_order_history_v2(&authority, params.clone())
        .await
        .unwrap();
    let trader = server
        .client
        .get_trader_order_history_v2(&trader_pda, params)
        .await
        .unwrap();
    for response in [user, trader] {
        assert_eq!(
            response.data[0].parent_order_id.as_deref(),
            Some("twap:100:2:7")
        );
        assert_eq!(response.data[0].parent_order_type.as_deref(), Some("twap"));
        assert_eq!(
            response.data[0].sort_key.as_deref(),
            Some("opaque-child-key")
        );
        assert_eq!(response.data[1].parent_order_id, None);
        assert_eq!(response.data[1].parent_order_type, None);
        assert_eq!(response.data[1].sort_key, None);
        assert_eq!(response.prev_cursor.as_deref(), Some("prev:opaque+cursor="));
        assert_eq!(response.next_cursor.as_deref(), Some("next/opaque"));
        assert!(response.has_more);
    }
    let requests = server.requests.lock().clone();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, format!("/v1/users/{authority}/orders_v2"));
    assert_eq!(
        requests[1].path,
        format!("/v1/traders/{trader_pda}/orders_v2")
    );
    let common_query = HashMap::from([
        ("limit".to_string(), "25".to_string()),
        ("market_symbol".to_string(), "SOL".to_string()),
        ("cursor".to_string(), "cursor:+/=".to_string()),
        ("organize_twap_orders".to_string(), "false".to_string()),
    ]);
    assert_eq!(requests[1].query, common_query);
    let mut user_query = common_query;
    user_query.insert("trader_pda_index".to_string(), "0".to_string());
    assert_eq!(requests[0].query, user_query);
    server.shutdown().await;
}

#[tokio::test]
async fn organized_v2_methods_preserve_parent_metadata_children_and_cursors() {
    let server = TestServer::new(StatusCode::OK, organized_response()).await;
    let authority = Pubkey::new_unique();
    let trader_pda = Pubkey::new_unique();
    let params = OrderHistoryV2QueryParams::new(2)
        .with_pda_index(7)
        .with_market_symbol("SOL")
        .with_cursor("ohg1:+/=");

    let user = server
        .client
        .get_user_organized_order_history_v2(&authority, params.clone())
        .await
        .unwrap();
    let trader = server
        .client
        .get_trader_organized_order_history_v2(&trader_pda, params)
        .await
        .unwrap();
    for response in [user, trader] {
        let parent = &response.data.twap_orders[0];
        assert_eq!(parent.parent_order_id, "twap:100:2:7");
        assert_eq!(parent.n_child_orders, Some(2));
        assert_eq!(parent.n_dust_orders, None);
        assert_eq!(parent.requested_base_qty, None);
        assert_eq!(parent.filled_base_qty.as_deref(), Some("1"));
        assert_eq!(
            parent.children[0]
                .fill_before_placement
                .as_ref()
                .unwrap()
                .total_quote_qty_filled,
            "150"
        );
        assert_eq!(response.data.single_orders.len(), 1);
        assert_eq!(response.data.single_orders[0].parent_order_type, None);
        assert_eq!(response.prev_cursor.as_deref(), Some("ohg1:opaque+cursor="));
        assert_eq!(response.next_cursor.as_deref(), Some("ohg1:other/opaque"));
        assert!(response.has_more);
    }
    let requests = server.requests.lock().clone();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, format!("/v1/users/{authority}/orders_v2"));
    assert_eq!(
        requests[1].path,
        format!("/v1/traders/{trader_pda}/orders_v2")
    );
    let common_query = HashMap::from([
        ("limit".to_string(), "2".to_string()),
        ("market_symbol".to_string(), "SOL".to_string()),
        ("cursor".to_string(), "ohg1:+/=".to_string()),
        ("organize_twap_orders".to_string(), "true".to_string()),
    ]);
    assert_eq!(requests[1].query, common_query);
    let mut user_query = common_query;
    user_query.insert("trader_pda_index".to_string(), "7".to_string());
    assert_eq!(requests[0].query, user_query);
    server.shutdown().await;
}

#[tokio::test]
async fn organized_method_rejects_flat_response_instead_of_reporting_empty_history() {
    let server = TestServer::new(StatusCode::OK, flat_response()).await;
    let result = server
        .client
        .orders()
        .get_user_organized_order_history_v2(
            &Pubkey::new_unique(),
            OrderHistoryV2QueryParams::new(25),
        )
        .await;
    assert!(result.is_err());
    server.shutdown().await;
}

#[tokio::test]
async fn legacy_history_method_preserves_its_route_query_and_response() {
    let server = TestServer::new(
        StatusCode::OK,
        json!({ "data": [], "prevCursor": null, "nextCursor": "legacy", "hasMore": true }),
    )
    .await;
    let authority = Pubkey::new_unique();
    let response = server
        .client
        .get_order_history(
            &authority,
            OrderHistoryQueryParams::new(10)
                .with_pda_index(7)
                .with_market_symbol("SOL")
                .with_cursor("old"),
        )
        .await
        .unwrap();
    assert_eq!(response.next_cursor.as_deref(), Some("legacy"));
    assert!(response.has_more);
    let request = server.requests.lock()[0].clone();
    assert_eq!(
        request.path,
        format!("/v1/trader/{authority}/order-history")
    );
    assert_eq!(
        request.query,
        HashMap::from([
            ("traderPdaIndex".to_string(), "7".to_string()),
            ("marketSymbol".to_string(), "SOL".to_string()),
            ("limit".to_string(), "10".to_string()),
            ("cursor".to_string(), "old".to_string()),
        ])
    );
    server.shutdown().await;
}

#[tokio::test]
async fn organized_history_preserves_api_error_handling() {
    let server = TestServer::new(
        StatusCode::BAD_REQUEST,
        json!({ "error": "Invalid cursor" }),
    )
    .await;
    let result = server
        .client
        .get_user_organized_order_history_v2(
            &Pubkey::new_unique(),
            OrderHistoryV2QueryParams::new(25).with_cursor("invalid"),
        )
        .await;
    assert!(matches!(
        result,
        Err(PhoenixHttpError::ApiError { status: 400, .. })
    ));
    server.shutdown().await;
}
