use std::fs;
use std::path::PathBuf;

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use phoenix_rise::phoenix_rise_ix::TRADER_CAPABILITY_CAN_DEPOSIT;
use phoenix_rise::phoenix_rise_types::accounts::Trader;
use phoenix_rise::{
    ActivateReferralTxRequest, PhoenixHttpClient, ReferralActivationTraderStatus,
    has_referral_activation_capabilities, referral_activation_trader_status,
};
use serde_json::{Value, json};
use solana_pubkey::Pubkey;
use tokio::net::TcpListener;

#[tokio::test]
async fn activate_invite_parses_object_response() {
    let app = Router::new()
        .route("/v1/invite/activate", post(activate_invite_handler))
        .route("/v1/referral/activate", post(activate_referral_handler))
        .route(
            "/v1/referral/activation-permission",
            get(referral_activation_permission_handler),
        )
        .route(
            "/v1/referral/activate-tx",
            post(activate_referral_tx_handler),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .build()
        .unwrap();
    let authority = expected_authority();

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

    let permission = client
        .invite()
        .get_referral_activation_permission()
        .await
        .unwrap();
    assert_eq!(
        permission.trader_onboarder,
        "Onboarder111111111111111111111111111111111"
    );
    assert_eq!(
        permission.risk_authority,
        "Risk1111111111111111111111111111111111111"
    );
    assert_eq!(
        permission.permission_account,
        "Permission11111111111111111111111111111111"
    );

    let activation = client
        .invite()
        .activate_referral_tx(&ActivateReferralTxRequest {
            referral_code: "ref-code".to_string(),
            trader_authority: authority.to_string(),
            trader_pda_index: Some(1),
            trader_subaccount_index: Some(2),
            recent_blockhash: "recent-blockhash".to_string(),
            transaction: "base64-transaction".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(activation.signature.as_deref(), Some("tx-signature"));
    assert_eq!(
        activation.trader_pda,
        "Trader3333333333333333333333333333333333"
    );
    assert_eq!(activation.referral_code, "REFCODE");
    assert_eq!(activation.status, "submitted");

    server.abort();
}

#[tokio::test]
async fn activate_referral_explains_user_auth_requirement() {
    let app = Router::new().route(
        "/v1/referral/activate",
        post(activate_referral_requires_auth_handler),
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

    let error = client
        .invite()
        .activate_referral(&authority, "ref-code")
        .await
        .unwrap_err();

    assert!(error.is_auth_error());
    assert_eq!(error.status(), Some(401));
    assert_eq!(error.error_code(), Some("missing_access_token"));
    assert!(
        error
            .to_string()
            .contains("Referral activation requires an authenticated user session")
    );

    server.abort();
}

#[test]
fn referral_activation_trader_status_tracks_deposit_capability() {
    assert_eq!(
        referral_activation_trader_status(None),
        ReferralActivationTraderStatus::Missing
    );
    assert!(!ReferralActivationTraderStatus::Registered.should_include_register_trader());
    assert!(ReferralActivationTraderStatus::Missing.should_include_register_trader());

    let mut trader = Trader::try_from_account_bytes(&mock_account_bytes("trader.json"))
        .expect("trader fixture should decode");
    assert_eq!(
        referral_activation_trader_status(Some(&trader)),
        ReferralActivationTraderStatus::Activated
    );
    assert!(has_referral_activation_capabilities(trader.state.flags));

    trader.state.flags &= !TRADER_CAPABILITY_CAN_DEPOSIT;
    assert_eq!(
        referral_activation_trader_status(Some(&trader)),
        ReferralActivationTraderStatus::Registered
    );
    assert!(!has_referral_activation_capabilities(trader.state.flags));
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

async fn referral_activation_permission_handler() -> Json<Value> {
    Json(json!({
        "trader_onboarder": "Onboarder111111111111111111111111111111111",
        "risk_authority": "Risk1111111111111111111111111111111111111",
        "permission_account": "Permission11111111111111111111111111111111"
    }))
}

async fn activate_referral_tx_handler(Json(body): Json<Value>) -> Json<Value> {
    let expected_authority = expected_authority().to_string();
    assert_eq!(
        body.get("referral_code").and_then(Value::as_str),
        Some("ref-code")
    );
    assert_eq!(
        body.get("trader_authority").and_then(Value::as_str),
        Some(expected_authority.as_str())
    );
    assert_eq!(
        body.get("trader_pda_index").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.get("trader_subaccount_index").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        body.get("recent_blockhash").and_then(Value::as_str),
        Some("recent-blockhash")
    );
    assert_eq!(
        body.get("transaction").and_then(Value::as_str),
        Some("base64-transaction")
    );
    Json(json!({
        "signature": "tx-signature",
        "trader_pda": "Trader3333333333333333333333333333333333",
        "referral_code": "REFCODE",
        "status": "submitted"
    }))
}

fn expected_authority() -> Pubkey {
    Pubkey::new_from_array([9; 32])
}

fn mock_account_bytes(file_name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../ts/tests/mocks")
        .join(file_name);
    let fixture: Value =
        serde_json::from_str(&fs::read_to_string(path).expect("fixture should be readable"))
            .expect("fixture should parse");
    let data = fixture["account"]["data"][0]
        .as_str()
        .expect("fixture account data should be base64");
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("fixture should contain base64 account bytes")
}

async fn activate_referral_requires_auth_handler() -> (StatusCode, Json<Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "missing_access_token",
            "message": "missing access token"
        })),
    )
}
