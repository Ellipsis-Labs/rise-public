use axum::routing::post;
use axum::{Json, Router};
use phoenix_rise::PhoenixHttpClient;
use serde_json::{Value, json};
use solana_pubkey::Pubkey;
use tokio::net::TcpListener;

#[tokio::test]
async fn activate_invite_parses_object_response() {
    let app = Router::new()
        .route("/v1/invite/activate", post(activate_invite_handler))
        .route(
            "/v1/invite/activate-with-referral",
            post(activate_referral_handler),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .build()
        .unwrap();
    let authority = Pubkey::new_unique();

    let trader_pda = client
        .register_trader(&authority, "invite-code")
        .await
        .unwrap();
    assert_eq!(trader_pda, "Trader1111111111111111111111111111111111");

    let referral_pda = client
        .invite()
        .activate_referral(&authority, "ref-code")
        .await
        .unwrap();
    assert_eq!(referral_pda, "Trader2222222222222222222222222222222222");

    server.abort();
}

async fn activate_invite_handler(Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(
        body.get("code").and_then(Value::as_str),
        Some("invite-code")
    );
    Json(json!({
        "trader_pda": "Trader1111111111111111111111111111111111"
    }))
}

async fn activate_referral_handler(Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(
        body.get("referral_code").and_then(Value::as_str),
        Some("ref-code")
    );
    Json(json!({
        "trader_pda": "Trader2222222222222222222222222222222222"
    }))
}
