use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use phoenix_rise::{
    AuthError, AuthSession, AuthSessionStore, MemoryAuthSessionStore, PhoenixAuthSigner,
    PhoenixHttpClient, PhoenixMemorySessionManager, PhoenixServiceChallenge,
    PhoenixServiceLoginRequest,
};
use serde_json::{Value, json};
#[cfg(feature = "solana-keypair")]
use solana_keypair::Keypair;
#[cfg(feature = "solana-keypair")]
use solana_signer::Signer;
use tokio::net::TcpListener;

#[derive(Clone)]
struct TestState {
    login_access_token: String,
    pop_key: String,
    saw_auth_header: Arc<AtomicBool>,
    service_login_calls: Arc<AtomicUsize>,
    wallet_login_calls: Arc<AtomicUsize>,
    refresh_calls: Arc<AtomicUsize>,
}

struct TestSigner {
    client_id: &'static str,
    key_id: &'static str,
}

impl PhoenixAuthSigner for TestSigner {
    fn client_id(&self) -> &str {
        self.client_id
    }

    fn key_id(&self) -> Option<&str> {
        Some(self.key_id)
    }

    fn sign_challenge(&self, _challenge: &PhoenixServiceChallenge) -> Result<String, AuthError> {
        Ok("signed".to_string())
    }
}

#[tokio::test]
async fn service_account_login_shares_session_with_http_client() {
    let state = TestState {
        login_access_token: make_jwt("jti-1"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-1"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route(
            "/v1/auth/login/service/challenge",
            post(service_challenge_handler),
        )
        .route("/v1/auth/login/service", post(service_login_handler))
        .route("/v1/auth/refresh", post(refresh_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .enable_auth()
        .build()
        .unwrap();

    let auth = client.auth().expect("auth client should be configured");
    let challenge = auth
        .service_challenge("svc-client", Some("svc-key"))
        .await
        .unwrap();

    assert_eq!(challenge.key_id, "svc-key");
    assert_eq!(challenge.timestamp.as_deref(), Some("2026-04-22T00:00:00Z"));

    let session = auth
        .login_with_service_signature(PhoenixServiceLoginRequest {
            client_id: "svc-client".to_string(),
            key_id: Some("svc-key".to_string()),
            nonce: challenge.nonce,
            timestamp: challenge.timestamp.unwrap(),
            signature: "signed".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(session.access_jti(), "jti-1");
    assert!(auth.session().unwrap().is_some());

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert!(state.saw_auth_header.load(Ordering::SeqCst));
    assert_eq!(state.service_login_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 0);

    server.abort();
}

#[cfg(feature = "solana-keypair")]
#[tokio::test]
async fn wallet_login_shares_session_with_http_client() {
    let state = TestState {
        login_access_token: make_jwt("jti-wallet"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-wallet"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route("/v1/auth/nonce", get(wallet_nonce_handler))
        .route("/v1/auth/login/wallet", post(wallet_login_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .enable_auth()
        .build()
        .unwrap();

    let auth = client.auth().expect("auth client should be configured");
    let keypair = Keypair::new();
    let wallet_pubkey = keypair.pubkey().to_string();

    let nonce = auth.get_wallet_nonce(&wallet_pubkey).await.unwrap();
    assert_eq!(nonce.nonce_id, "wallet-nonce-1");

    let session = auth.login_with_wallet_keypair(&keypair).await.unwrap();

    assert_eq!(session.access_jti(), "jti-wallet");
    assert!(auth.session().unwrap().is_some());

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert!(state.saw_auth_header.load(Ordering::SeqCst));
    assert_eq!(state.wallet_login_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.service_login_calls.load(Ordering::SeqCst), 0);
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 0);

    server.abort();
}

#[tokio::test]
async fn signer_reauthenticates_when_refresh_token_is_missing() {
    let state = TestState {
        login_access_token: make_jwt("jti-fresh"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-2"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route(
            "/v1/auth/login/service/challenge",
            post(service_challenge_handler),
        )
        .route("/v1/auth/login/service", post(service_login_handler))
        .route("/v1/auth/refresh", post(refresh_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let initial_session = AuthSession::from_tokens(
        make_jwt("jti-stale"),
        None,
        URL_SAFE_NO_PAD.encode(b"pop-key-stale"),
        Some(3600),
        None,
    )
    .unwrap();

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .with_auth_session(initial_session)
        .with_auth_signer(Arc::new(TestSigner {
            client_id: "svc-client",
            key_id: "svc-key",
        }))
        .build()
        .unwrap();

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert!(state.saw_auth_header.load(Ordering::SeqCst));
    assert_eq!(state.service_login_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        client
            .auth()
            .unwrap()
            .session()
            .unwrap()
            .unwrap()
            .access_jti(),
        "jti-fresh"
    );

    server.abort();
}

#[tokio::test]
async fn signer_reauthenticates_after_invalid_refresh_token() {
    let state = TestState {
        login_access_token: make_jwt("jti-fresh"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-3"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route(
            "/v1/auth/login/service/challenge",
            post(service_challenge_handler),
        )
        .route("/v1/auth/login/service", post(service_login_handler))
        .route("/v1/auth/refresh", post(refresh_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let initial_session = AuthSession::from_tokens(
        make_jwt("jti-stale"),
        Some("refresh-token-stale".to_string()),
        URL_SAFE_NO_PAD.encode(b"pop-key-stale"),
        Some(3600),
        Some(3600),
    )
    .unwrap();

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .with_auth_session(initial_session)
        .with_auth_signer(Arc::new(TestSigner {
            client_id: "svc-client",
            key_id: "svc-key",
        }))
        .build()
        .unwrap();

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.service_login_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        client
            .auth()
            .unwrap()
            .session()
            .unwrap()
            .unwrap()
            .access_jti(),
        "jti-fresh"
    );

    server.abort();
}

#[tokio::test]
async fn store_only_auth_refreshes_after_unauthorized() {
    let state = TestState {
        login_access_token: make_jwt("jti-fresh"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-4"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route("/v1/auth/refresh", post(refresh_success_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let store = Arc::new(MemoryAuthSessionStore::new());
    let initial_session = AuthSession::from_tokens(
        make_jwt("jti-stale"),
        Some("refresh-token-stale".to_string()),
        URL_SAFE_NO_PAD.encode(b"pop-key-stale"),
        Some(3600),
        Some(3600),
    )
    .unwrap();
    store.store_session(&initial_session).unwrap();

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .with_auth_session_store(store.clone())
        .build()
        .unwrap();

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(state.service_login_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        store.load_session().unwrap().unwrap().access_jti(),
        "jti-fresh"
    );

    server.abort();
}

#[tokio::test]
async fn memory_session_manager_refreshes_before_request() {
    let state = TestState {
        login_access_token: make_jwt("jti-memory-fresh"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-memory"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route("/v1/auth/refresh", post(refresh_success_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let initial_session = AuthSession::from_tokens(
        make_jwt_with_exp("jti-memory-expired", 1),
        Some("refresh-token-memory".to_string()),
        URL_SAFE_NO_PAD.encode(b"pop-key-stale"),
        None,
        Some(3600),
    )
    .unwrap();
    let manager = Arc::new(
        PhoenixMemorySessionManager::new(format!("http://{}", addr), initial_session).unwrap(),
    );

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .with_session_manager(manager.clone())
        .build()
        .unwrap();

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert!(state.saw_auth_header.load(Ordering::SeqCst));
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(manager.session().access_jti(), "jti-memory-fresh");
    assert_eq!(
        client
            .auth()
            .expect("manager-backed client should expose shared auth client")
            .session()
            .unwrap()
            .expect("refreshed session should be stored")
            .access_jti(),
        "jti-memory-fresh"
    );

    server.abort();
}

#[tokio::test]
async fn memory_session_manager_coalesces_concurrent_refreshes() {
    let state = TestState {
        login_access_token: make_jwt("jti-memory-fresh"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-memory"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route("/v1/auth/refresh", post(refresh_success_slow_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let initial_session = AuthSession::from_tokens(
        make_jwt_with_exp("jti-memory-expired", 1),
        Some("refresh-token-memory".to_string()),
        URL_SAFE_NO_PAD.encode(b"pop-key-stale"),
        None,
        Some(3600),
    )
    .unwrap();
    let manager = Arc::new(
        PhoenixMemorySessionManager::new(format!("http://{}", addr), initial_session).unwrap(),
    );

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .with_session_manager(manager.clone())
        .build()
        .unwrap();

    let first = async { client.exchange().get_keys().await };
    let second = async { client.exchange().get_keys().await };
    let (first, second) = tokio::join!(first, second);

    assert_eq!(
        first.unwrap().global_config,
        "11111111111111111111111111111111"
    );
    assert_eq!(
        second.unwrap().global_config,
        "11111111111111111111111111111111"
    );
    assert!(state.saw_auth_header.load(Ordering::SeqCst));
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(manager.session().access_jti(), "jti-memory-fresh");

    server.abort();
}

#[tokio::test]
async fn store_reload_does_not_consume_refresh_retry() {
    #[derive(Clone)]
    struct ReloadThenRefreshStore {
        session: Arc<std::sync::Mutex<AuthSession>>,
        load_calls: Arc<AtomicUsize>,
    }

    impl AuthSessionStore for ReloadThenRefreshStore {
        fn load_session(&self) -> Result<Option<AuthSession>, AuthError> {
            let session = self
                .session
                .lock()
                .expect("session mutex should not be poisoned")
                .clone();
            let call_index = self.load_calls.fetch_add(1, Ordering::SeqCst);
            if call_index == 1 {
                let mut snapshot = session.snapshot();
                snapshot.counter = snapshot.counter.saturating_add(1);
                return Ok(Some(AuthSession::from_snapshot(snapshot)?));
            }
            Ok(Some(session))
        }

        fn store_session(&self, session: &AuthSession) -> Result<(), AuthError> {
            *self
                .session
                .lock()
                .expect("session mutex should not be poisoned") = session.clone();
            Ok(())
        }

        fn clear_session(&self) -> Result<(), AuthError> {
            Ok(())
        }
    }

    let state = TestState {
        login_access_token: make_jwt("jti-fresh-after-reload"),
        pop_key: URL_SAFE_NO_PAD.encode(b"pop-key-5"),
        saw_auth_header: Arc::new(AtomicBool::new(false)),
        service_login_calls: Arc::new(AtomicUsize::new(0)),
        wallet_login_calls: Arc::new(AtomicUsize::new(0)),
        refresh_calls: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route("/v1/auth/refresh", post(refresh_success_handler))
        .route("/v1/view/exchange/keys", get(exchange_keys_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let initial_session = AuthSession::from_tokens(
        make_jwt("jti-stale-reload"),
        Some("refresh-token-stale".to_string()),
        URL_SAFE_NO_PAD.encode(b"pop-key-stale"),
        Some(Duration::from_secs(3600).as_secs()),
        Some(Duration::from_secs(3600).as_secs()),
    )
    .unwrap();
    let store = Arc::new(ReloadThenRefreshStore {
        session: Arc::new(std::sync::Mutex::new(initial_session)),
        load_calls: Arc::new(AtomicUsize::new(0)),
    });

    let client = PhoenixHttpClient::builder(format!("http://{}", addr))
        .with_auth_session_store(store.clone())
        .build()
        .unwrap();

    let keys = client.exchange().get_keys().await.unwrap();
    assert_eq!(keys.global_config, "11111111111111111111111111111111");
    assert_eq!(state.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        store.load_session().unwrap().unwrap().access_jti(),
        "jti-fresh-after-reload"
    );

    server.abort();
}

async fn service_challenge_handler(Json(body): Json<Value>) -> Json<Value> {
    let key_id = body
        .get("key_id")
        .and_then(Value::as_str)
        .unwrap_or("svc-key");

    Json(json!({
        "nonce": "nonce-1",
        "message": "timestamp:2026-04-22T00:00:00Z\nchallenge:sign this payload",
        "expires_at": "2026-04-22T00:01:00Z",
        "key_id": key_id,
    }))
}

async fn service_login_handler(State(state): State<TestState>) -> Json<Value> {
    state.service_login_calls.fetch_add(1, Ordering::SeqCst);

    Json(json!({
        "token_type": "Bearer",
        "access_token": state.login_access_token,
        "expires_in": 900,
        "refresh_token": "refresh-token-1",
        "refresh_expires_in": 3600,
        "pop_key": state.pop_key,
    }))
}

async fn wallet_nonce_handler(
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let wallet_pubkey = query
        .get("wallet_pubkey")
        .cloned()
        .unwrap_or_else(|| "missing-wallet".to_string());

    Json(json!({
        "nonce_id": "wallet-nonce-1",
        "wallet_pubkey": wallet_pubkey,
        "message": "Sign this Phoenix API login message",
        "expires_at": "2026-04-22T00:01:00Z",
    }))
}

async fn wallet_login_handler(
    State(state): State<TestState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state.wallet_login_calls.fetch_add(1, Ordering::SeqCst);

    let has_wallet_pubkey = body.get("wallet_pubkey").and_then(Value::as_str).is_some();
    let has_signature = body
        .get("signature")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty());
    let nonce_id = body.get("nonce_id").and_then(Value::as_str);

    if !has_wallet_pubkey || !has_signature || nonce_id != Some("wallet-nonce-1") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_wallet_login",
                "message": "wallet login payload was incomplete",
            })),
        ));
    }

    Ok(Json(json!({
        "token_type": "Bearer",
        "access_token": state.login_access_token,
        "expires_in": 900,
        "refresh_token": "refresh-token-wallet",
        "refresh_expires_in": 3600,
        "pop_key": state.pop_key,
    })))
}

async fn refresh_handler(
    State(state): State<TestState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state.refresh_calls.fetch_add(1, Ordering::SeqCst);
    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "invalid_refresh_token",
            "message": "refresh token rejected",
        })),
    ))
}

async fn refresh_success_handler(
    State(state): State<TestState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state.refresh_calls.fetch_add(1, Ordering::SeqCst);
    Ok(Json(json!({
        "token_type": "Bearer",
        "access_token": state.login_access_token,
        "expires_in": 900,
        "refresh_token": "refresh-token-fresh",
        "refresh_expires_in": 3600,
        "pop_key": state.pop_key,
    })))
}

async fn refresh_success_slow_handler(
    State(state): State<TestState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    tokio::time::sleep(Duration::from_millis(50)).await;
    refresh_success_handler(State(state)).await
}

async fn exchange_keys_handler(
    State(state): State<TestState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let bearer = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let has_auth = bearer.is_some();
    let is_expected_token = bearer == Some(state.login_access_token.as_str());

    state.saw_auth_header.store(has_auth, Ordering::SeqCst);

    if !is_expected_token {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": if has_auth { "invalid_access_token" } else { "missing_access_token" },
                "message": if has_auth { "invalid access token" } else { "missing access token" },
            })),
        ));
    }

    Ok(Json(json!({
        "globalConfig": "11111111111111111111111111111111",
        "currentAuthorities": {
            "rootAuthority": "11111111111111111111111111111111",
            "riskAuthority": "11111111111111111111111111111111",
            "marketAuthority": "11111111111111111111111111111111",
            "oracleAuthority": "11111111111111111111111111111111"
        },
        "pendingAuthorities": {
            "rootAuthority": "11111111111111111111111111111111",
            "riskAuthority": "11111111111111111111111111111111",
            "marketAuthority": "11111111111111111111111111111111",
            "oracleAuthority": "11111111111111111111111111111111"
        },
        "canonicalMint": "11111111111111111111111111111111",
        "globalVault": "11111111111111111111111111111111",
        "perpAssetMap": "11111111111111111111111111111111",
        "globalTraderIndex": [],
        "activeTraderBuffer": [],
        "withdrawQueue": "11111111111111111111111111111111"
    })))
}

fn make_jwt(jti: &str) -> String {
    make_jwt_with_exp(jti, 4_102_444_800)
}

fn make_jwt_with_exp(jti: &str, exp: u64) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"jti":"{jti}","exp":{exp}}}"#).as_bytes());
    format!("{header}.{payload}.sig")
}
