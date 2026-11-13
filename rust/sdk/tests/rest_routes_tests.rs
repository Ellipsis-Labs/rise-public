//! Mock-server tests for typed REST routes. Response fixtures under
//! `tests/fixtures/rest/` are serialized from the Phoenix server's DTOs.

use std::sync::Arc;

use axum::Router;
use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{TimeZone, Utc};
use parking_lot::Mutex;
use phoenix_rise::api::{ExternalBearer, PhoenixHttpClient, PhoenixHttpError};
use phoenix_rise::types::prelude::{
    CommodityMarketState, ExchangeMarketConfig, ExchangeRiskFactors, MarketStatus,
    OrderHistoryV2QueryParams, PriceHistoryQuery,
};
use serde_json::Value;
use solana_pubkey::Pubkey;
use tokio::net::TcpListener;

const ORDERS_V2: &str = include_str!("fixtures/rest/orders_v2.json");
const MARKET_VIEW: &str = include_str!("fixtures/rest/market_view.json");
const PRICE_HISTORY: &str = include_str!("fixtures/rest/price_history.json");
const TRADER_STATE_SNAPSHOT: &str = include_str!("fixtures/rest/trader_state_snapshot.json");
const TRADER_VIEW: &str = include_str!("fixtures/rest/trader_view.json");

/// Canned response for one request path (query excluded).
struct Route {
    path: String,
    status: StatusCode,
    body: String,
}

impl Route {
    fn ok(path: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            status: StatusCode::OK,
            body: body.into(),
        }
    }

    fn status(path: impl Into<String>, status: StatusCode) -> Self {
        Self {
            path: path.into(),
            status,
            body: "{\"error\":\"rejected\"}".to_string(),
        }
    }
}

#[derive(Clone)]
struct Observed {
    uri: String,
    authorization: Option<String>,
}

#[derive(Clone)]
struct MockServer {
    routes: Arc<Vec<Route>>,
    observed: Arc<Mutex<Vec<Observed>>>,
}

impl MockServer {
    async fn spawn(mut routes: Vec<Route>) -> (Self, String) {
        let markets = serde_json::to_string(&[sol_market()]).unwrap();
        routes.push(Route::ok("/v1/view/exchange/markets", markets));
        let server = Self {
            routes: Arc::new(routes),
            observed: Arc::default(),
        };
        let app = Router::new().fallback(handle).with_state(server.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (server, format!("http://{addr}"))
    }

    /// Observed request URIs, excluding the market-list lookups used for
    /// symbol canonicalization.
    fn uris(&self) -> Vec<String> {
        self.observed
            .lock()
            .iter()
            .filter(|observed| observed.uri != "/v1/view/exchange/markets")
            .map(|observed| observed.uri.clone())
            .collect()
    }

    fn observed(&self) -> Vec<Observed> {
        self.observed.lock().clone()
    }
}

async fn handle(
    State(server): State<MockServer>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    server.observed.lock().push(Observed {
        uri: uri.to_string(),
        authorization: headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
    });
    match server.routes.iter().find(|route| route.path == uri.path()) {
        Some(route) => (
            route.status,
            [("content-type", "application/json")],
            route.body.clone(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn sol_market() -> ExchangeMarketConfig {
    ExchangeMarketConfig {
        symbol: "SOL".to_string(),
        display_name: None,
        asset_id: 0,
        market_status: MarketStatus::Active,
        metadata: None,
        market_pubkey: "market".to_string(),
        spline_pubkey: "spline".to_string(),
        tick_size: 1,
        base_lots_decimals: 0,
        taker_fee: 0.0,
        maker_fee: 0.0,
        leverage_tiers: vec![],
        risk_factors: ExchangeRiskFactors::default(),
        funding_interval_seconds: 1,
        funding_period_seconds: 1,
        max_funding_rate_per_interval: 0.0,
        open_interest_cap_base_lots: 0_u64.into(),
        max_liquidation_size_base_lots: 0_u64.into(),
        isolated_only: false,
        stats_snapshot: None,
    }
}

fn client(base_url: &str) -> PhoenixHttpClient {
    PhoenixHttpClient::builder(base_url).build().unwrap()
}

/// Returns the trader-view fixture re-keyed to `trader_key`.
fn trader_view_for(trader_key: &Pubkey, subaccount_index: u8) -> String {
    let mut view: Value = serde_json::from_str(TRADER_VIEW).unwrap();
    view["traderKey"] = Value::String(trader_key.to_string());
    view["traderSubaccountIndex"] = Value::from(subaccount_index);
    view.to_string()
}

#[tokio::test]
async fn order_history_v2_sends_snake_case_query_and_decodes_page() {
    let trader = Pubkey::new_unique();
    let path = format!("/v1/traders/{trader}/orders_v2");
    let (server, base_url) = MockServer::spawn(vec![Route::ok(&path, ORDERS_V2)]).await;

    let page = client(&base_url)
        .traders()
        .get_trader_order_history_v2(
            &trader,
            OrderHistoryV2QueryParams::new(50)
                .with_market_symbol("SOL")
                .with_cursor("abc"),
        )
        .await
        .unwrap();

    assert_eq!(
        server.uris(),
        [format!("{path}?market_symbol=SOL&limit=50&cursor=abc")]
    );
    assert!(page.has_more);
    assert_eq!(page.prev_cursor.as_deref(), Some("prev"));
    assert_eq!(page.next_cursor.as_deref(), Some("next"));
    let item = &page.data[0];
    assert_eq!(item.market_symbol, "SOL");
    assert_eq!(
        item.created_at,
        Utc.with_ymd_and_hms(2025, 5, 29, 19, 20, 0).unwrap()
    );
    assert!(item.intended.is_post_only);
    assert_eq!(item.intended.quote_lot_budget, None);
    assert_eq!(
        item.fill_before_placement.as_ref().unwrap().total_num_fills,
        2
    );
    assert_eq!(item.placed_order.as_ref().unwrap().scale_set_id, Some(3));
    let current = item.current_order_state.as_ref().unwrap();
    assert_eq!(current.remaining_base_qty, "0.5");
    assert_eq!(current.cancelled_at, None);
    assert_eq!(current.scale_set_id, None);
}

#[tokio::test]
async fn market_view_uses_canonical_symbol_and_decodes_commodity_metadata() {
    let (server, base_url) =
        MockServer::spawn(vec![Route::ok("/v1/view/market/SOL", MARKET_VIEW)]).await;

    let view = client(&base_url)
        .markets()
        .get_market_view("sol")
        .await
        .unwrap();

    assert_eq!(server.uris(), ["/v1/view/market/SOL"]);
    assert_eq!(view.slot, 355_000_000);
    let market = view.market;
    assert_eq!(market.symbol, "SOL");
    assert_eq!(market.fees.maker_fee_micro, -100);
    assert_eq!(market.risk_action_price_validity_rules[7][3][7], 1);
    assert_eq!(market.mark_price.map(|price| price.price), None);
    assert_eq!(market.l2_orderbook.mid, Some(150.25));
    let commodity = market.commodity_metadata.expect("commodity metadata");
    assert_eq!(commodity.status, CommodityMarketState::AfterHours);
    assert_eq!(
        commodity.mark_price_band.expect("mark band").max.ui,
        "150.25"
    );
    assert!(commodity.execution_price_band.is_none());
    let metadata = market.metadata.expect("public metadata");
    assert_eq!(metadata.name.as_deref(), Some("Solana"));
}

#[tokio::test]
async fn price_history_uses_canonical_symbol_and_serializes_rfc3339_times() {
    let (server, base_url) =
        MockServer::spawn(vec![Route::ok("/v1/prices/SOL/history", PRICE_HISTORY)]).await;
    let start = Utc.with_ymd_and_hms(2025, 5, 29, 19, 20, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2025, 5, 29, 19, 25, 0).unwrap();

    let response = client(&base_url)
        .markets()
        .get_price_history(
            "sol",
            PriceHistoryQuery::new()
                .with_start_time(start)
                .with_end_time(end)
                .with_limit(100)
                .with_timeframe("1m"),
        )
        .await
        .unwrap();

    assert_eq!(
        server.uris(),
        [
            "/v1/prices/SOL/history?start_time=2025-05-29T19%3A20%3A00Z&end_time=2025-05-29T19%\
             3A25%3A00Z&limit=100&timeframe=1m"
        ]
    );
    assert_eq!(response.symbol, "SOL");
    assert_eq!(response.timeframe.as_deref(), Some("1m"));
    let point = &response.prices[0];
    assert_eq!(point.timestamp, start);
    assert_eq!(point.mid_spot_diff_ema_ticks, Some(-3));
    assert_eq!(point.exchange_perp_price, None);
    assert_eq!(point.interval_accumulator, Some(i128::MAX));
}

#[tokio::test]
async fn trader_state_snapshot_sends_pda_index_only_when_set() {
    let authority = Pubkey::new_unique();
    let path = format!("/v1/trader/state/{authority}");
    let (server, base_url) = MockServer::spawn(vec![Route::ok(&path, TRADER_STATE_SNAPSHOT)]).await;
    let client = client(&base_url);

    let response = client
        .traders()
        .get_trader_state_snapshot(&authority, Some(2))
        .await
        .unwrap();
    client
        .traders()
        .get_trader_state_snapshot(&authority, None)
        .await
        .unwrap();

    assert_eq!(
        server.uris(),
        [format!("{path}?traderPdaIndex=2"), path.clone()]
    );
    assert_eq!(response.slot_index, 4);
    let subaccounts: Vec<u8> = response
        .snapshot
        .subaccounts
        .iter()
        .map(|subaccount| subaccount.subaccount_index)
        .collect();
    assert_eq!(subaccounts, [0, 3]);
}

#[tokio::test]
async fn trader_state_snapshot_fetches_per_subaccount_margin_in_one_request() {
    let authority = Pubkey::new_unique();
    let state_path = format!("/v1/trader/state/{authority}");
    let (server, base_url) =
        MockServer::spawn(vec![Route::ok(&state_path, TRADER_STATE_SNAPSHOT)]).await;

    let response = client(&base_url)
        .traders()
        .get_trader_state_snapshot(&authority, Some(0))
        .await
        .unwrap();

    assert_eq!(server.uris(), [format!("{state_path}?traderPdaIndex=0")]);
    assert_eq!(response.slot, 355_000_000);
    assert_eq!(response.slot_index, 4);
    let subaccounts = &response.snapshot.subaccounts;
    assert_eq!(subaccounts[0].subaccount_index, 0);
    assert_eq!(subaccounts[1].subaccount_index, 3);
    let margin = subaccounts[0].margin.as_ref().expect("subaccount margin");
    assert_eq!(margin.effective_collateral.value, 1_000_000_000);
    assert_eq!(margin.effective_collateral.ui, "1000");
    assert_eq!(margin.initial_margin.ui, "100");
    assert_eq!(margin.positions["SOL"].liquidation_price.ui, "125");
    assert_eq!(subaccounts[0].margin_error, None);
    let margin = subaccounts[1].margin.as_ref().expect("subaccount margin");
    assert_eq!(margin.effective_collateral.value, 250_000_000);
    assert_eq!(margin.effective_collateral.ui, "250");
    assert!(margin.positions.is_empty());
    assert_eq!(subaccounts[1].margin_error, None);
    let serialized = serde_json::to_value(&response).unwrap();
    assert!(serialized.get("traderViews").is_none());
    assert!(
        serialized["snapshot"]["subaccounts"][0]
            .get("marginError")
            .is_none()
    );
}

#[tokio::test]
async fn trader_state_snapshot_preserves_other_subaccounts_when_margin_is_unavailable() {
    let authority = Pubkey::new_unique();
    let state_path = format!("/v1/trader/state/{authority}");
    let mut snapshot: Value = serde_json::from_str(TRADER_STATE_SNAPSHOT).unwrap();
    let subaccount = &mut snapshot["snapshot"]["subaccounts"][0];
    subaccount.as_object_mut().unwrap().remove("margin");
    subaccount["marginError"] = Value::String("margin_unavailable".into());
    let (server, base_url) =
        MockServer::spawn(vec![Route::ok(&state_path, snapshot.to_string())]).await;

    let response = client(&base_url)
        .traders()
        .get_trader_state_snapshot(&authority, Some(0))
        .await
        .unwrap();

    let subaccounts = &response.snapshot.subaccounts;
    assert_eq!(subaccounts.len(), 2);
    assert_eq!(subaccounts[0].collateral, "1000");
    assert_eq!(subaccounts[0].sequence, 10);
    assert!(subaccounts[0].margin.is_none());
    assert_eq!(
        subaccounts[0].margin_error.as_deref(),
        Some("margin_unavailable")
    );
    assert_eq!(subaccounts[1].subaccount_index, 3);
    assert_eq!(
        subaccounts[1]
            .margin
            .as_ref()
            .unwrap()
            .effective_collateral
            .ui,
        "250"
    );
    assert_eq!(subaccounts[1].margin_error, None);
    assert_eq!(server.uris(), [format!("{state_path}?traderPdaIndex=0")]);
}

#[tokio::test]
async fn trader_state_snapshot_returns_empty_for_an_authority_without_subaccounts() {
    let authority = Pubkey::new_unique();
    let state_path = format!("/v1/trader/state/{authority}");
    let mut snapshot: Value = serde_json::from_str(TRADER_STATE_SNAPSHOT).unwrap();
    snapshot["snapshot"]["subaccounts"] = serde_json::json!([]);
    let (server, base_url) =
        MockServer::spawn(vec![Route::ok(&state_path, snapshot.to_string())]).await;

    let response = client(&base_url)
        .traders()
        .get_trader_state_snapshot(&authority, Some(0))
        .await
        .unwrap();

    assert!(response.snapshot.subaccounts.is_empty());
    assert_eq!(server.uris(), [format!("{state_path}?traderPdaIndex=0")]);
}

#[tokio::test]
async fn trader_state_snapshot_propagates_not_found() {
    let authority = Pubkey::new_unique();
    let state_path = format!("/v1/trader/state/{authority}");
    let (server, base_url) =
        MockServer::spawn(vec![Route::status(&state_path, StatusCode::NOT_FOUND)]).await;

    let error = client(&base_url)
        .traders()
        .get_trader_state_snapshot(&authority, Some(0))
        .await
        .expect_err("404 snapshot should fail");

    assert_eq!(error.status(), Some(404));
    assert_eq!(server.uris(), [format!("{state_path}?traderPdaIndex=0")]);
}

#[tokio::test]
async fn external_bearer_rejection_surfaces_after_one_request() {
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let expired_token = format!(
        "{}.{}.sig",
        URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#),
        URL_SAFE_NO_PAD
            .encode(serde_json::json!({ "exp": Utc::now().timestamp() - 30 }).to_string()),
    );
    for (error_code, token) in [
        ("invalid_access_token", Some("opaque")),
        ("access_token_expired", Some(expired_token.as_str())),
        ("missing_bearer_token", None),
        ("missing_pop_binding", Some("opaque")),
        ("missing_pop_key", Some("opaque")),
        ("invalid_pop_mac", Some("opaque")),
    ] {
        let trader = Pubkey::new_unique();
        let path = format!("/v1/view/trader/{trader}");
        let body = serde_json::json!({
            "error": error_code,
            "message": "server rejection details",
        })
        .to_string();
        let (server, base_url) = MockServer::spawn(vec![Route {
            path: path.clone(),
            status: StatusCode::UNAUTHORIZED,
            body: body.clone(),
        }])
        .await;
        let bearer = ExternalBearer::new();
        if let Some(token) = token {
            bearer.set_access_token(token.to_string()).unwrap();
        }
        let client = PhoenixHttpClient::builder(&base_url)
            .with_session_manager(bearer)
            .build()
            .unwrap();
        let initial_lifecycle_state = client.auth_lifecycle_state();

        let error = client
            .traders()
            .get_trader_by_pubkey(&trader)
            .await
            .expect_err("401 should surface");

        match error {
            PhoenixHttpError::Authentication {
                status,
                error_code: actual_code,
                message,
            } => {
                assert_eq!(status, Some(401));
                assert_eq!(actual_code.as_deref(), Some(error_code));
                assert_eq!(message, body);
            }
            other => panic!("expected authentication error, got {other:?}"),
        }
        assert_eq!(client.auth_lifecycle_state(), initial_lifecycle_state);
        assert!(client.auth_lifecycle_last_error().is_none());
        let observed = server.observed();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].uri, path);
        assert_eq!(
            observed[0].authorization,
            token.map(|token| format!("Bearer {token}"))
        );
    }
}

#[tokio::test]
async fn external_bearer_clear_sends_requests_anonymously() {
    let trader = Pubkey::new_unique();
    let path = format!("/v1/view/trader/{trader}");
    let (server, base_url) =
        MockServer::spawn(vec![Route::ok(&path, trader_view_for(&trader, 0))]).await;
    let bearer = ExternalBearer::new();
    bearer.set_access_token("opaque".to_string()).unwrap();
    let client = PhoenixHttpClient::builder(&base_url)
        .with_session_manager(bearer.clone())
        .build()
        .unwrap();

    bearer.clear();
    client
        .traders()
        .get_trader_by_pubkey(&trader)
        .await
        .unwrap();

    let observed = server.observed();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].authorization, None);
}
