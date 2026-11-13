use std::sync::Arc;

use axum::extract::{OriginalUri, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_lot::Mutex;
use phoenix_rise::api::{PhoenixHttpClient, TraderKey};
use phoenix_rise::types::prelude::{CollateralHistoryQueryParams, CollateralHistoryRequest};
use phoenix_rise::types::swap::BuildSwapTransactionRequest;
use serde_json::{Value, json};
use solana_pubkey::Pubkey;
use tokio::net::TcpListener;

const SWAP: &str = include_str!("../../../ts/tests/mocks/swap-transaction.json");
const HISTORY: &str = include_str!("../../../ts/tests/mocks/collateral-history-v2.json");
const TOTALS: &str = include_str!("../../../ts/tests/mocks/collateral-totals.json");

type Requests = Arc<Mutex<Vec<(String, Option<Value>)>>>;

#[tokio::test]
async fn collateral_clients_preserve_account_selection_and_mixed_asset_results() {
    let requests = Requests::default();
    let app = Router::new()
        .route("/v1/ix/swap", post(swap_handler))
        .fallback(get(history_handler))
        .with_state(requests.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = PhoenixHttpClient::builder(format!("http://{}", listener.local_addr().unwrap()))
        .build()
        .unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let authority = Pubkey::new_unique();
    let request_json = json!({
        "authority": authority.to_string(),
        "positionAuthority": "delegate",
        "assetIn": {"spot": "mint"},
        "assetOut": "quote",
        "amountAtoms": 1_000_000_000_u64,
        "traderPdaIndex": 2,
        "traderSubaccountIndex": 3,
        "feePayer": "sponsor",
        "transactionVersion": 0
    });
    let request: BuildSwapTransactionRequest =
        serde_json::from_value(request_json.clone()).unwrap();
    let swap = client
        .collateral()
        .build_swap_transaction(&request)
        .await
        .unwrap();
    assert_eq!(swap.transaction, "unsigned-base64");
    assert_eq!(swap.min_amount_out_atoms, 149_000_000);
    assert_eq!(swap.sponsorship_attestation.as_deref(), Some("proof"));
    assert_eq!(requests.lock()[0].1.as_ref().unwrap(), &request_json);

    let default_request_json = json!({
        "authority": authority.to_string(), "assetIn": "sol", "assetOut": "quote", "quantity": 0.5,
    });
    let default_request = serde_json::from_value(default_request_json.clone()).unwrap();
    client
        .collateral()
        .build_swap_transaction(&default_request)
        .await
        .unwrap();
    assert_eq!(
        requests.lock()[1].1.as_ref().unwrap(),
        &default_request_json
    );

    let user_history = client
        .collateral()
        .get_user_collateral_history_v2(
            &authority,
            CollateralHistoryRequest::new(25).with_next_cursor("older+/="),
        )
        .await
        .unwrap();
    assert_eq!(user_history.data.len(), 2);
    assert_eq!(user_history.data[0].symbol, "SOL");
    assert_eq!(user_history.data[0].amount, -1_000_000_000);
    assert_eq!(user_history.data[1].symbol, "USDC");
    assert_eq!(user_history.data[1].balance_after, 100_000_000);
    assert!(user_history.has_more);
    assert_eq!(user_history.next_cursor.as_deref(), Some("older+/="));
    assert!(user_history.prev_cursor.is_none());

    let trader = TraderKey::new_with_idx(authority, 2, 3);
    client
        .collateral()
        .get_trader_collateral_history_v2(&trader, CollateralHistoryRequest::new(25))
        .await
        .unwrap();
    client
        .collateral()
        .get_authority_collateral_history_v2(
            &authority,
            CollateralHistoryQueryParams::new(25).with_pda_index(2),
        )
        .await
        .unwrap();
    let totals = client
        .collateral()
        .get_user_collateral_totals(&authority)
        .await
        .unwrap();
    assert_eq!(totals.total_deposited, 300.0);
    assert_eq!(totals.assets[0].withdrawn.amount, 1_000_000_000);
    assert_eq!(
        requests
            .lock()
            .iter()
            .map(|(uri, _)| uri.clone())
            .collect::<Vec<_>>(),
        vec![
            "/v1/ix/swap".to_string(),
            "/v1/ix/swap".to_string(),
            format!(
                "/v1/users/{authority}/collateral-history-v2?limit=25&nextCursor=older%2B%2F%3D"
            ),
            format!(
                "/v1/traders/{}/collateral-history-v2?limit=25",
                trader.pda()
            ),
            format!("/v1/trader/{authority}/collateral-history-v2?pdaIndex=2&limit=25"),
            format!("/v1/users/{authority}/collateral-totals"),
        ]
    );
    server.abort();
}

async fn swap_handler(State(requests): State<Requests>, Json(body): Json<Value>) -> Json<Value> {
    requests
        .lock()
        .push(("/v1/ix/swap".to_string(), Some(body)));
    Json(serde_json::from_str(SWAP).unwrap())
}

async fn history_handler(
    State(requests): State<Requests>,
    OriginalUri(uri): OriginalUri,
) -> Json<Value> {
    let response = if uri.path().ends_with("collateral-totals") {
        TOTALS
    } else {
        HISTORY
    };
    requests.lock().push((uri.to_string(), None));
    Json(serde_json::from_str(response).unwrap())
}
