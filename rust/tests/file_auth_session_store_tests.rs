#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use phoenix_rise::{AuthSession, AuthSessionStore, FileAuthSessionStore};

#[test]
fn file_auth_session_store_writes_owner_only_permissions() {
    let path = temp_path("session.json");
    let store = FileAuthSessionStore::new(&path);
    let session = AuthSession::from_tokens(
        make_jwt("jti-test"),
        Some("refresh-token".to_string()),
        URL_SAFE_NO_PAD.encode(b"pop-key"),
        Some(3600),
        Some(3600),
    )
    .unwrap();

    store.store_session(&session).unwrap();

    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    let _ = fs::remove_file(path);
}

fn temp_path(name: &str) -> PathBuf {
    let unique = format!(
        "phoenix-rise-{}-{}-{}",
        name,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    );
    std::env::temp_dir().join(unique)
}

fn make_jwt(jti: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload =
        URL_SAFE_NO_PAD.encode(format!(r#"{{"jti":"{jti}","exp":4102444800}}"#).as_bytes());
    format!("{header}.{payload}.sig")
}
