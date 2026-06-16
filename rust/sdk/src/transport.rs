use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use parking_lot::Mutex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;
use url::Url;

use crate::auth::{
    AuthError, AuthResponseBody, AuthSession, AuthSessionStore, PhoenixAuthSigner,
    PhoenixSessionManager, RefreshRequestBody, login_with_auth_signer,
};
use crate::auth_lifecycle::{
    AuthLifecycleController, AuthLifecycleError, AuthLifecycleErrorReason, AuthLifecycleState,
};
use crate::phoenix_rise_types::PhoenixHttpError;

const ACCESS_REFRESH_WINDOW: Duration = Duration::from_secs(60);
pub(crate) const PHOENIX_CLIENT_HEADER_VALUE: &str = concat!(
    "language=rust;sdk=",
    env!("CARGO_PKG_NAME"),
    ";version=",
    env!("CARGO_PKG_VERSION")
);

#[derive(Debug, Error)]
pub(crate) enum PhoenixApiError {
    #[error(
        "HTTP request failed{status}{details}: {source}",
        status = status
            .as_ref()
            .map(|status| format!(" (status: {status})"))
            .unwrap_or_default(),
        details = if let Some(url) = url {
            format!(" [url: {url}]")
        } else {
            String::new()
        }
    )]
    RequestFailed {
        status: Option<StatusCode>,
        url: Option<String>,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to build http client: {source}")]
    ClientBuildFailed {
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to parse URL: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("API returned error {status}: {message}")]
    ApiError {
        status: StatusCode,
        message: String,
        error_code: Option<String>,
    },

    #[error(
        "API rate limited {status}: {message}{retry_after}",
        retry_after = retry_after_seconds
            .map(|seconds| format!(" (retry_after={seconds}s)"))
            .unwrap_or_default()
    )]
    RateLimited {
        status: StatusCode,
        message: String,
        error_code: Option<String>,
        retry_after_seconds: Option<u64>,
    },

    #[error("failed to serialize JSON for {context}: {source}")]
    JsonSerialize {
        context: &'static str,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "failed to deserialize JSON for {context}: {source}{body}",
        body = body_preview
            .as_ref()
            .map(|preview| format!(" (body: {preview})"))
            .unwrap_or_default()
    )]
    JsonDeserialize {
        context: &'static str,
        #[source]
        source: serde_json::Error,
        body_preview: Option<String>,
    },

    #[error("failed to serialize query parameters for {context}: {source}")]
    QuerySerialize {
        context: &'static str,
        #[source]
        source: serde_urlencoded::ser::Error,
    },

    #[error("authentication error: {0}")]
    Authentication(#[from] AuthError),

    #[error("session manager error: {0}")]
    SessionManager(#[from] PhoenixHttpError),
}

impl PhoenixApiError {
    pub(crate) fn is_rate_limited(&self) -> bool {
        match self {
            PhoenixApiError::RateLimited { .. } => true,
            PhoenixApiError::ApiError {
                status, error_code, ..
            } => is_rate_limited_error(*status, error_code.as_deref()),
            PhoenixApiError::RequestFailed {
                status: Some(status),
                ..
            } => *status == StatusCode::TOO_MANY_REQUESTS,
            _ => false,
        }
    }

    pub(crate) fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            PhoenixApiError::RateLimited {
                retry_after_seconds,
                ..
            } => *retry_after_seconds,
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct PhoenixApiClient {
    client: Client,
    base_url: Url,
    auth_session: Option<AuthSession>,
    auth_session_store: Option<Arc<dyn AuthSessionStore>>,
    auth_signer: Option<Arc<dyn PhoenixAuthSigner>>,
    session_manager: Option<Arc<dyn PhoenixSessionManager>>,
    auth_lifecycle: Arc<Mutex<AuthLifecycleController>>,
}

pub(crate) struct PhoenixApiClientBuilder {
    api_url: String,
    client: Option<Client>,
    auth_session: Option<AuthSession>,
    auth_session_store: Option<Arc<dyn AuthSessionStore>>,
    auth_signer: Option<Arc<dyn PhoenixAuthSigner>>,
    session_manager: Option<Arc<dyn PhoenixSessionManager>>,
}

impl PhoenixApiClient {
    pub(crate) fn builder(api_url: impl Into<String>) -> PhoenixApiClientBuilder {
        PhoenixApiClientBuilder::new(api_url)
    }

    pub(crate) fn auth_lifecycle_state(&self) -> AuthLifecycleState {
        self.auth_lifecycle.lock().state()
    }

    pub(crate) fn auth_lifecycle_last_error(&self) -> Option<AuthLifecycleError> {
        self.auth_lifecycle.lock().last_error().cloned()
    }

    pub(crate) async fn auth_session_for_request(
        &self,
    ) -> Result<Option<AuthSession>, PhoenixApiError> {
        self.maybe_refresh_before_request().await?;

        match self.current_auth_session().await? {
            Some(session) if !access_expires_at_is_expired(&session) => Ok(Some(session)),
            Some(session) => {
                if session.can_refresh()
                    || self.auth_signer.is_some()
                    || self.session_manager.is_some()
                {
                    self.refresh_session().await?;
                    return self.current_auth_session().await.map(|session| {
                        session.filter(|session| !access_expires_at_is_expired(session))
                    });
                }

                Ok(None)
            }
            None if self.auth_signer.is_some() || self.session_manager.is_some() => {
                self.refresh_session().await?;
                self.current_auth_session()
                    .await
                    .map(|session| session.filter(|session| !access_expires_at_is_expired(session)))
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn refresh_auth_session(&self) -> Result<(), PhoenixApiError> {
        self.refresh_session().await
    }

    pub(crate) async fn get_json_typed<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, PhoenixApiError> {
        self.get(path).await
    }

    pub(crate) async fn get_json_with_query<T: DeserializeOwned, Q: Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T, PhoenixApiError> {
        let mut url = self.base_url.join(path.trim_start_matches('/'))?;
        let query_string = serde_urlencoded::to_string(query).map_err(|source| {
            PhoenixApiError::QuerySerialize {
                context: "api query params",
                source,
            }
        })?;
        if !query_string.is_empty() {
            url.set_query(Some(&query_string));
        }
        self.send_request_json(Method::GET, url, None, true).await
    }

    pub(crate) async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, PhoenixApiError> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        let body_bytes =
            serde_json::to_vec(body).map_err(|source| PhoenixApiError::JsonSerialize {
                context: "request body",
                source,
            })?;
        self.send_request_json(Method::POST, url, Some(body_bytes), true)
            .await
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, PhoenixApiError> {
        let url = self.base_url.join(path.trim_start_matches('/'))?;
        self.send_request_json(Method::GET, url, None, true).await
    }

    async fn send_request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        url: Url,
        body: Option<Vec<u8>>,
        allow_auth_retry: bool,
    ) -> Result<T, PhoenixApiError> {
        let response = self
            .send_request_raw(method, url, body, allow_auth_retry)
            .await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|source| PhoenixApiError::RequestFailed {
                status: Some(status),
                url: None,
                source,
            })?;
        serde_json::from_str(&text).map_err(|source| PhoenixApiError::JsonDeserialize {
            context: "api response",
            source,
            body_preview: Some(body_preview(&text, 512)),
        })
    }

    async fn send_request_raw(
        &self,
        method: Method,
        url: Url,
        body: Option<Vec<u8>>,
        allow_auth_retry: bool,
    ) -> Result<reqwest::Response, PhoenixApiError> {
        self.maybe_refresh_before_request().await?;
        let mut allow_auth_retry = allow_auth_retry;
        let mut allow_store_reload = allow_auth_retry;

        loop {
            let session = self.current_auth_session().await?;
            let valid_session = session
                .as_ref()
                .filter(|session| !access_expires_at_is_expired(session));
            let response = self
                .send_request_once(method.clone(), url.clone(), body.clone(), valid_session)
                .await?;

            if response.status().is_success() {
                return Ok(response);
            }

            let status = response.status();
            let retry_after_seconds = parse_retry_after_seconds(response.headers());
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let error_code = parse_error_code(&message);

            let can_retry_auth = valid_session.is_some()
                || self.auth_session_store.is_some()
                || self.auth_signer.is_some()
                || self.session_manager.is_some();

            if allow_auth_retry && status == StatusCode::UNAUTHORIZED && can_retry_auth {
                if allow_store_reload
                    && should_reload_for_error_code(error_code.as_deref())
                    && self.reload_session_from_store(session.as_ref()).await?
                {
                    allow_store_reload = false;
                    continue;
                }

                self.refresh_session().await?;
                allow_auth_retry = false;
                continue;
            }

            return Err(map_api_error_response(
                status,
                message,
                error_code,
                retry_after_seconds,
            ));
        }
    }

    async fn maybe_refresh_before_request(&self) -> Result<(), PhoenixApiError> {
        let Some(session) = self.current_auth_session().await? else {
            return Ok(());
        };
        let should_rotate_access_token = session
            .access_expires_at()
            .map(|expires_at| expires_at <= Instant::now() + ACCESS_REFRESH_WINDOW)
            .unwrap_or(false);
        let can_renew_session =
            session.can_refresh() || self.auth_signer.is_some() || self.session_manager.is_some();

        if !should_rotate_access_token || !can_renew_session {
            return Ok(());
        }

        match self.refresh_session().await {
            Ok(()) => Ok(()),
            Err(error) if access_expires_at_is_expired(&session) => Err(error),
            Err(_) => Ok(()),
        }
    }

    async fn send_request_once(
        &self,
        method: Method,
        url: Url,
        body: Option<Vec<u8>>,
        session: Option<&AuthSession>,
    ) -> Result<reqwest::Response, PhoenixApiError> {
        let url_string = url.to_string();
        let mut request = self.client.request(method, url);

        if let Some(session) = session {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", session.access_token()),
            );
        }

        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }

        request
            .send()
            .await
            .map_err(|error| map_reqwest_error(error, Some(url_string)))
    }

    async fn current_auth_session(&self) -> Result<Option<AuthSession>, PhoenixApiError> {
        if let Some(manager) = &self.session_manager {
            return manager.current_session().await.map_err(Into::into);
        }

        if let Some(session) = &self.auth_session {
            return Ok(Some(session.clone()));
        }

        let Some(store) = &self.auth_session_store else {
            self.auth_lifecycle.lock().on_session_loaded(false);
            return Ok(None);
        };

        let session = store.load_session()?;
        self.auth_lifecycle
            .lock()
            .on_session_loaded(session.is_some());
        Ok(session)
    }

    async fn refresh_session(&self) -> Result<(), PhoenixApiError> {
        if let Some(manager) = &self.session_manager {
            self.auth_lifecycle.lock().on_refresh_started();
            return match manager.refresh_session().await {
                Ok(()) => {
                    self.auth_lifecycle.lock().on_refresh_succeeded(true);
                    Ok(())
                }
                Err(error) => {
                    self.auth_lifecycle.lock().on_refresh_failed(
                        AuthLifecycleErrorReason::RefreshFailed,
                        Some(error.to_string()),
                    );
                    Err(error.into())
                }
            };
        }

        self.auth_lifecycle.lock().on_refresh_started();

        let Some(session) = self.current_auth_session().await? else {
            return self
                .reauthenticate_or_fail(
                    AuthLifecycleErrorReason::NoSession,
                    PhoenixApiError::Authentication(AuthError::NoAuthSession),
                )
                .await;
        };

        let Some(refresh_token) = session.refresh_token() else {
            return self
                .reauthenticate_or_fail(
                    AuthLifecycleErrorReason::NoSession,
                    PhoenixApiError::Authentication(AuthError::MissingRefreshToken),
                )
                .await;
        };

        if !session.can_refresh() {
            return self
                .reauthenticate_or_fail(
                    AuthLifecycleErrorReason::RefreshExpired,
                    PhoenixApiError::Authentication(AuthError::RefreshExpired),
                )
                .await;
        }

        let url = self.base_url.join("v1/auth/refresh")?;
        let url_string = url.to_string();
        let mut request = self
            .client
            .post(url)
            .json(&RefreshRequestBody { refresh_token });
        if !access_expires_at_is_expired(&session) {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", session.access_token()),
            );
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                let mapped = map_reqwest_error(error, Some(url_string));
                self.auth_lifecycle.lock().on_refresh_failed(
                    AuthLifecycleErrorReason::RefreshFailed,
                    Some(mapped.to_string()),
                );
                return Err(mapped);
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let retry_after_seconds = parse_retry_after_seconds(response.headers());
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let error_code = parse_error_code(&message);
            let reason = match error_code.as_deref() {
                Some("invalid_refresh_token") | Some("session_missing") => {
                    AuthLifecycleErrorReason::InvalidRefreshToken
                }
                Some("refresh_expired") => AuthLifecycleErrorReason::RefreshExpired,
                _ => AuthLifecycleErrorReason::RefreshFailed,
            };
            let error = map_api_error_response(status, message, error_code, retry_after_seconds);

            if matches!(
                reason,
                AuthLifecycleErrorReason::InvalidRefreshToken
                    | AuthLifecycleErrorReason::RefreshExpired
            ) {
                return self.reauthenticate_or_fail(reason, error).await;
            }

            self.auth_lifecycle
                .lock()
                .on_refresh_failed(reason, Some(error.to_string()));
            return Err(error);
        }

        let status = response.status();
        let text = match response.text().await {
            Ok(text) => text,
            Err(source) => {
                let mapped = PhoenixApiError::RequestFailed {
                    status: Some(status),
                    url: None,
                    source,
                };
                self.auth_lifecycle.lock().on_refresh_failed(
                    AuthLifecycleErrorReason::RefreshFailed,
                    Some(mapped.to_string()),
                );
                return Err(mapped);
            }
        };

        let auth_response: AuthResponseBody =
            match serde_json::from_str(&text).map_err(|source| PhoenixApiError::JsonDeserialize {
                context: "auth refresh response",
                source,
                body_preview: Some(body_preview(&text, 512)),
            }) {
                Ok(auth_response) => auth_response,
                Err(error) => {
                    self.auth_lifecycle.lock().on_refresh_failed(
                        AuthLifecycleErrorReason::RefreshFailed,
                        Some(error.to_string()),
                    );
                    return Err(error);
                }
            };

        if let Err(error) = session.update_from_auth_response(auth_response) {
            self.auth_lifecycle.lock().on_refresh_failed(
                AuthLifecycleErrorReason::RefreshFailed,
                Some(error.to_string()),
            );
            return Err(error.into());
        }

        if let Some(store) = &self.auth_session_store {
            if let Err(error) = store.store_session(&session) {
                self.auth_lifecycle.lock().on_refresh_failed(
                    AuthLifecycleErrorReason::RefreshFailed,
                    Some(error.to_string()),
                );
                return Err(error.into());
            }
        }

        self.auth_lifecycle.lock().on_refresh_succeeded(true);
        Ok(())
    }

    async fn reauthenticate_or_fail(
        &self,
        reason: AuthLifecycleErrorReason,
        fallback_error: PhoenixApiError,
    ) -> Result<(), PhoenixApiError> {
        let Some(signer) = &self.auth_signer else {
            self.auth_lifecycle
                .lock()
                .on_refresh_failed(reason, Some(fallback_error.to_string()));
            return Err(fallback_error);
        };

        let session = match login_with_auth_signer(
            &self.client,
            &self.base_url,
            self.auth_session_store.as_ref(),
            signer.as_ref(),
        )
        .await
        {
            Ok(session) => session,
            Err(error) => {
                self.auth_lifecycle
                    .lock()
                    .on_refresh_failed(reason, Some(error.to_string()));
                return Err(error);
            }
        };

        if let Some(current_session) = &self.auth_session {
            current_session.update_from_snapshot(session.snapshot())?;
        }

        self.auth_lifecycle.lock().on_refresh_succeeded(true);
        Ok(())
    }

    async fn reload_session_from_store(
        &self,
        current_session: Option<&AuthSession>,
    ) -> Result<bool, PhoenixApiError> {
        if let Some(manager) = &self.session_manager {
            return manager
                .reload_session(current_session)
                .await
                .map_err(Into::into);
        }

        let Some(store) = &self.auth_session_store else {
            return Ok(false);
        };

        let Some(loaded) = store.load_session()? else {
            self.auth_lifecycle.lock().on_session_loaded(false);
            return Ok(false);
        };

        if let Some(current_session) = current_session {
            if sessions_match_for_reload(current_session, &loaded) {
                return Ok(false);
            }
        }

        let loaded_snapshot = loaded.snapshot();
        if let Some(session) = &self.auth_session {
            session.update_from_snapshot(loaded_snapshot)?;
        }

        self.auth_lifecycle.lock().on_session_loaded(true);
        Ok(true)
    }
}

fn should_reload_for_error_code(error_code: Option<&str>) -> bool {
    matches!(
        error_code,
        Some("invalid_access_token") | Some("access_jti_mismatch")
    )
}

fn sessions_match_for_reload(current_session: &AuthSession, loaded_session: &AuthSession) -> bool {
    let mut current_snapshot = current_session.snapshot();
    let mut loaded_snapshot = loaded_session.snapshot();
    current_snapshot.access_expires_at = None;
    current_snapshot.refresh_expires_at = None;
    loaded_snapshot.access_expires_at = None;
    loaded_snapshot.refresh_expires_at = None;
    current_snapshot == loaded_snapshot
}

fn access_expires_at_is_expired(session: &AuthSession) -> bool {
    session
        .access_expires_at()
        .map(|expires_at| expires_at <= Instant::now())
        .unwrap_or(false)
}

impl PhoenixApiClientBuilder {
    fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            client: None,
            auth_session: None,
            auth_session_store: None,
            auth_signer: None,
            session_manager: None,
        }
    }

    pub(crate) fn with_auth_session(mut self, session: AuthSession) -> Self {
        self.auth_session = Some(session);
        self
    }

    pub(crate) fn with_auth_session_store(mut self, store: Arc<dyn AuthSessionStore>) -> Self {
        self.auth_session_store = Some(store);
        self
    }

    pub(crate) fn with_auth_signer(mut self, signer: Arc<dyn PhoenixAuthSigner>) -> Self {
        self.auth_signer = Some(signer);
        self
    }

    pub(crate) fn with_session_manager(mut self, manager: Arc<dyn PhoenixSessionManager>) -> Self {
        self.session_manager = Some(manager);
        self
    }

    pub(crate) fn build(self) -> Result<PhoenixApiClient, PhoenixApiError> {
        let client = match self.client {
            Some(client) => client,
            None => build_default_http_client(Duration::from_secs(30))
                .map_err(|source| PhoenixApiError::ClientBuildFailed { source })?,
        };
        let base_url = normalize_base_url(Url::parse(&self.api_url)?);
        let has_session = self.auth_session.is_some();
        let has_session_manager = self.session_manager.is_some();

        Ok(PhoenixApiClient {
            client,
            base_url,
            auth_session: self.auth_session,
            auth_session_store: self.auth_session_store,
            auth_signer: self.auth_signer,
            session_manager: self.session_manager,
            auth_lifecycle: Arc::new(Mutex::new(AuthLifecycleController::new(
                has_session || has_session_manager,
            ))),
        })
    }
}

pub(crate) fn build_default_http_client(timeout: Duration) -> Result<Client, reqwest::Error> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        HeaderName::from_static("x-phoenix-client"),
        HeaderValue::from_static(PHOENIX_CLIENT_HEADER_VALUE),
    );

    let builder = Client::builder()
        .timeout(timeout)
        .default_headers(default_headers);

    #[cfg(test)]
    let builder = {
        // Reqwest's proxy autodiscovery reaches into macOS system config APIs,
        // which can panic in the hermetic test sandbox. Unit tests only need a
        // plain client, so disable proxies for test builds.
        builder.no_proxy()
    };

    builder.build()
}

pub(crate) fn is_rate_limited_error(status: StatusCode, error_code: Option<&str>) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || matches!(error_code, Some("rate_limited"))
}

pub(crate) fn map_api_error_response(
    status: StatusCode,
    message: String,
    error_code: Option<String>,
    retry_after_seconds: Option<u64>,
) -> PhoenixApiError {
    if is_rate_limited_error(status, error_code.as_deref()) {
        PhoenixApiError::RateLimited {
            status,
            message,
            error_code,
            retry_after_seconds,
        }
    } else {
        PhoenixApiError::ApiError {
            status,
            message,
            error_code,
        }
    }
}

pub(crate) fn map_reqwest_error(error: reqwest::Error, url: Option<String>) -> PhoenixApiError {
    PhoenixApiError::RequestFailed {
        status: error.status(),
        url,
        source: error,
    }
}

pub(crate) fn normalize_base_url(mut url: Url) -> Url {
    if !url.path().ends_with('/') {
        let mut path = url.path().to_string();
        path.push('/');
        url.set_path(&path);
    }
    url
}

fn parse_error_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|entry| entry.as_str())
                .map(str::to_string)
        })
}

pub(crate) fn parse_retry_after_seconds(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_retry_after_value_seconds(value, Utc::now()))
}

fn parse_retry_after_value_seconds(value: &str, now: chrono::DateTime<Utc>) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds);
    }

    let retry_at = chrono::DateTime::parse_from_rfc2822(trimmed).ok()?;
    let delta_ms = retry_at
        .with_timezone(&Utc)
        .signed_duration_since(now)
        .num_milliseconds();
    if delta_ms <= 0 {
        return Some(0);
    }
    let rounded_seconds = (delta_ms + 999) / 1_000;
    u64::try_from(rounded_seconds).ok()
}

fn body_preview(body: &str, max_chars: usize) -> String {
    let mut preview = body.chars().take(max_chars).collect::<String>();
    if body.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::*;

    fn future_unix_secs(offset: Duration) -> u64 {
        SystemTime::now()
            .checked_add(offset)
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .expect("future timestamp should be after epoch")
            .as_secs()
    }

    fn past_unix_secs(offset: Duration) -> u64 {
        SystemTime::now()
            .checked_sub(offset)
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .expect("past timestamp should be after epoch")
            .as_secs()
    }

    fn test_jwt(jti: &str, exp: u64) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"jti":"{jti}","exp":{exp}}}"#));
        format!("{header}.{payload}.sig")
    }

    fn response(status: &str, body: String, extra_headers: &[(&str, &str)]) -> String {
        let mut headers = format!(
            concat!(
                "HTTP/1.1 {}\r\n",
                "content-type: application/json\r\n",
                "connection: close\r\n",
                "content-length: {}\r\n",
            ),
            status,
            body.len()
        );
        for (name, value) in extra_headers {
            headers.push_str(name);
            headers.push_str(": ");
            headers.push_str(value);
            headers.push_str("\r\n");
        }
        headers.push_str("\r\n");
        headers.push_str(&body);
        headers
    }

    #[derive(Clone, Copy)]
    enum RefreshResponse {
        Success,
        RateLimited,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TestRequest {
        path: String,
        authorization: Option<String>,
    }

    fn spawn_auth_test_server(
        refresh_response: RefreshResponse,
        max_requests: usize,
    ) -> (String, Arc<Mutex<Vec<TestRequest>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_thread = requests.clone();

        thread::spawn(move || {
            for stream in listener.incoming().take(max_requests) {
                let mut stream = stream.expect("test request should connect");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("request should read");
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                let authorization = request.lines().find_map(|line| {
                    line.strip_prefix("authorization: ")
                        .or_else(|| line.strip_prefix("Authorization: "))
                        .map(str::to_string)
                });
                requests_for_thread.lock().push(TestRequest {
                    path: path.clone(),
                    authorization,
                });

                let response = match (path.as_str(), refresh_response) {
                    ("/v1/auth/refresh", RefreshResponse::Success) => {
                        let access_token =
                            test_jwt("fresh-jti", future_unix_secs(Duration::from_secs(900)));
                        response(
                            "200 OK",
                            json!({
                                "token_type": "Bearer",
                                "access_token": access_token,
                                "expires_in": 900,
                                "refresh_token": "refresh-token-2",
                                "refresh_expires_in": 3600,
                                "pop_key": URL_SAFE_NO_PAD.encode(b"pop-key-2"),
                            })
                            .to_string(),
                            &[],
                        )
                    }
                    ("/v1/auth/refresh", RefreshResponse::RateLimited) => response(
                        "429 Too Many Requests",
                        json!({ "error": "rate_limited" }).to_string(),
                        &[("retry-after", "2")],
                    ),
                    _ => response("200 OK", json!({ "ok": true }).to_string(), &[]),
                };
                stream
                    .write_all(response.as_bytes())
                    .expect("response should write");
            }
        });

        (base_url, requests)
    }

    fn expired_session() -> AuthSession {
        AuthSession::from_tokens(
            test_jwt("expired-jti", past_unix_secs(Duration::from_secs(30))),
            Some("refresh-token-1".to_string()),
            URL_SAFE_NO_PAD.encode(b"pop-key-1"),
            None,
            Some(3600),
        )
        .expect("test session should parse")
    }

    #[tokio::test]
    async fn expired_access_refreshes_before_request() {
        let (base_url, requests) = spawn_auth_test_server(RefreshResponse::Success, 2);
        let client = PhoenixApiClient::builder(base_url)
            .with_auth_session(expired_session())
            .build()
            .expect("client should build");

        let value: serde_json::Value = client
            .get_json_typed("/v1/view/markets")
            .await
            .expect("protected request should succeed after refresh");

        assert_eq!(value, json!({ "ok": true }));
        let requests = requests.lock();
        assert_eq!(requests[0].path, "/v1/auth/refresh");
        assert_eq!(requests[0].authorization, None);
        assert_eq!(requests[1].path, "/v1/view/markets");
        assert!(requests[1].authorization.is_some());
    }

    #[tokio::test]
    async fn expired_access_does_not_send_request_when_refresh_fails() {
        let (base_url, requests) = spawn_auth_test_server(RefreshResponse::RateLimited, 1);
        let client = PhoenixApiClient::builder(base_url)
            .with_auth_session(expired_session())
            .build()
            .expect("client should build");

        let error = client
            .get_json_typed::<serde_json::Value>("/v1/view/markets")
            .await
            .expect_err("refresh failure should stop the protected request");

        assert!(matches!(error, PhoenixApiError::RateLimited { .. }));
        let requests = requests.lock();
        assert_eq!(requests[0].path, "/v1/auth/refresh");
        assert_eq!(requests[0].authorization, None);
    }

    #[test]
    fn retry_after_parses_seconds_and_http_dates() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .expect("valid test timestamp")
            .with_timezone(&Utc);

        assert_eq!(parse_retry_after_value_seconds("2", now), Some(2));
        assert_eq!(
            parse_retry_after_value_seconds("Thu, 01 Jan 2026 00:00:02 GMT", now),
            Some(2)
        );
        assert_eq!(
            parse_retry_after_value_seconds("Wed, 31 Dec 2025 23:59:59 GMT", now),
            Some(0)
        );
        assert_eq!(parse_retry_after_value_seconds("invalid", now), None);
    }
}
